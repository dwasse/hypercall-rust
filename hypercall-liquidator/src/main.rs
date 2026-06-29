use std::path::PathBuf;
use std::str::FromStr;

use clap::{Args, Parser, Subcommand, ValueEnum};
use hypercall_client::HypercallClient;
use hypercall_hyperliquid::{DirectHyperliquidPerpVenue, HyperliquidPerpAssetRegistry};
use hypercall_liquidator::{
    discover_standard_margin_liquidation_cycles, execute_hedge_after_liquidation,
    execute_standard_margin_liquidation_cycle, plaintext_private_key_from_env, plan_delta_hedge,
    read_standard_margin_liquidation_cycle, CycleDecision, DeltaHedgeInput, HedgeExecution,
    HedgePlanDecision, HypercallLiquidatorConfig, LiquidationExecution, PositionDelta,
};
use rust_decimal::Decimal;
use serde::Serialize;

#[derive(Debug, Parser)]
#[command(name = "hypercall-liquidator")]
#[command(about = "Public standard margin liquidator proof-of-concept")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Validate a liquidator config file without executing anything.
    CheckConfig {
        #[arg(long)]
        config: PathBuf,
        /// Explicitly allow live execution configs where dry_run = false.
        #[arg(long)]
        allow_live_execution: bool,
    },
    /// Read Hypercall margin and off-chain standard-margin liquidation state, then print the decision.
    Inspect {
        #[arg(long)]
        config: PathBuf,
        #[arg(long)]
        account: Option<String>,
    },
    /// Run one liquidation cycle. Honors dry_run unless explicitly overridden.
    RunOnce {
        #[arg(long)]
        config: PathBuf,
        #[arg(long)]
        account: Option<String>,
        /// Explicitly allow live execution configs where dry_run = false.
        #[arg(long)]
        allow_live_execution: bool,
        #[command(flatten)]
        hedge: HedgeCliArgs,
    },
}

#[derive(Debug, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
enum RunOnceOutput {
    Skipped {
        decision: CycleDecision,
    },
    Executed {
        execution: LiquidationExecution,
        hedge: Option<RunHedgeOutput>,
    },
}

#[derive(Debug, Clone, Args)]
struct HedgeCliArgs {
    /// Explicitly allow submitting the Hyperliquid hedge after a live liquidation.
    #[arg(long)]
    allow_live_hedge: bool,
    /// Hyperliquid chain for the direct public adapter.
    #[arg(long, value_enum)]
    hyperliquid_chain: Option<HyperliquidCliChain>,
    /// Underlying to hedge, for example BTC or ETH.
    #[arg(long)]
    hedge_underlying: Option<String>,
    /// Mark price used for the band and slippage-capped IOC order.
    #[arg(long)]
    hedge_mark_price_usdc: Option<String>,
    /// Acquired option delta for the configured underlying.
    #[arg(long)]
    hedge_delta: Option<f64>,
    /// Optional symbol label for the acquired option delta.
    #[arg(long)]
    hedge_position_symbol: Option<String>,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum HyperliquidCliChain {
    Mainnet,
    Testnet,
}

#[derive(Debug, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
enum RunHedgeOutput {
    Skipped {
        reason: String,
    },
    Submitted {
        symbol: String,
        is_buy: bool,
        price: f64,
        size: f64,
        notional_usdc: String,
        result_debug: String,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::CheckConfig {
            config,
            allow_live_execution,
        } => {
            let config = HypercallLiquidatorConfig::from_path(&config)?;
            config.validate_for_live_start(allow_live_execution)?;
            println!(
                "config ok: mode={}, dry_run={}, hedge_enabled={}",
                config.liquidation.mode, config.liquidation.dry_run, config.hedge.enabled
            );
        }
        Command::Inspect { config, account } => {
            let config = HypercallLiquidatorConfig::from_path(&config)?;
            config.validate_for_live_start(false)?;
            let decision = read_decision(&config, account).await?;
            println!("{}", serde_json::to_string_pretty(&decision)?);
        }
        Command::RunOnce {
            config,
            account,
            allow_live_execution,
            hedge,
        } => {
            let config = HypercallLiquidatorConfig::from_path(&config)?;
            config.validate_for_live_start(allow_live_execution)?;
            let decision = read_decision(&config, account).await?;
            let output = match decision {
                CycleDecision::Eligible { .. } => {
                    let prepared_hedge = if config.liquidation.dry_run {
                        None
                    } else {
                        prepare_optional_hyperliquid_hedge(&config, hedge.clone())?
                    };
                    let api = HypercallClient::new(&config.hypercall.api_url);
                    let execution =
                        execute_standard_margin_liquidation_cycle(&config, &api, decision).await?;
                    let hedge = match prepared_hedge {
                        Some(prepared) => {
                            run_prepared_hyperliquid_hedge(&config, &execution, prepared).await?
                        }
                        None => run_optional_hyperliquid_hedge(&config, &execution, hedge).await?,
                    };
                    RunOnceOutput::Executed { execution, hedge }
                }
                skip => RunOnceOutput::Skipped { decision: skip },
            };
            println!("{}", serde_json::to_string_pretty(&output)?);
        }
    }
    Ok(())
}

async fn read_decision(
    config: &HypercallLiquidatorConfig,
    account: Option<String>,
) -> anyhow::Result<CycleDecision> {
    let api = HypercallClient::new(&config.hypercall.api_url);
    if let Some(account) = account {
        return Ok(
            read_standard_margin_liquidation_cycle(config, &api, &account, Vec::new()).await?,
        );
    }

    let decisions = discover_standard_margin_liquidation_cycles(config, &api).await?;
    Ok(decisions
        .iter()
        .find(|decision| matches!(decision, CycleDecision::Eligible { .. }))
        .cloned()
        .or_else(|| decisions.into_iter().next())
        .unwrap_or_else(|| {
            CycleDecision::Skip(hypercall_liquidator::CycleSkipReason::NoActiveAuction {
                account: "public-liquidation-discovery".to_string(),
            })
        }))
}

struct PreparedHyperliquidHedge {
    venue: DirectHyperliquidPerpVenue,
    input: DeltaHedgeInput,
}

async fn run_optional_hyperliquid_hedge(
    config: &HypercallLiquidatorConfig,
    execution: &LiquidationExecution,
    args: HedgeCliArgs,
) -> anyhow::Result<Option<RunHedgeOutput>> {
    if !hedge_args_present(&args) {
        return Ok(None);
    }
    if !matches!(
        execution,
        LiquidationExecution::SubmittedStandardMargin { .. }
    ) {
        return Ok(Some(RunHedgeOutput::Skipped {
            reason: "LiquidationNotSubmitted".to_string(),
        }));
    }
    let prepared = prepare_required_hyperliquid_hedge(config, args)?;
    run_prepared_hyperliquid_hedge(config, execution, prepared).await
}

fn prepare_optional_hyperliquid_hedge(
    config: &HypercallLiquidatorConfig,
    args: HedgeCliArgs,
) -> anyhow::Result<Option<PreparedHyperliquidHedge>> {
    if !hedge_args_present(&args) {
        return Ok(None);
    }
    Ok(Some(prepare_required_hyperliquid_hedge(config, args)?))
}

fn prepare_required_hyperliquid_hedge(
    config: &HypercallLiquidatorConfig,
    args: HedgeCliArgs,
) -> anyhow::Result<PreparedHyperliquidHedge> {
    if !args.allow_live_hedge {
        anyhow::bail!("live hedge args require --allow-live-hedge");
    }
    let hyperliquid_chain = args
        .hyperliquid_chain
        .ok_or_else(|| anyhow::anyhow!("live hedge args require --hyperliquid-chain"))?;

    let underlying = required_arg(args.hedge_underlying, "--hedge-underlying")?;
    let mark_price_usdc = Decimal::from_str(&required_arg(
        args.hedge_mark_price_usdc,
        "--hedge-mark-price-usdc",
    )?)?;
    let delta = args
        .hedge_delta
        .ok_or_else(|| anyhow::anyhow!("missing --hedge-delta"))?;
    let symbol = args
        .hedge_position_symbol
        .unwrap_or_else(|| format!("{}-ACQUIRED-OPTION", underlying.to_uppercase()));
    let input = DeltaHedgeInput {
        underlying: underlying.clone(),
        mark_price_usdc,
        positions: vec![PositionDelta {
            symbol,
            underlying,
            delta,
        }],
    };
    let plan = match plan_delta_hedge(&config.hedge, &input) {
        HedgePlanDecision::Place(plan) => plan,
        HedgePlanDecision::Skip(reason) => {
            anyhow::bail!("requested live hedge is not placeable before liquidation: {reason:?}")
        }
    };

    let private_key = plaintext_private_key_from_env(&config.keys.hyperliquid)?;
    let registry = match hyperliquid_chain {
        HyperliquidCliChain::Mainnet => HyperliquidPerpAssetRegistry::mainnet_defaults(),
        HyperliquidCliChain::Testnet => HyperliquidPerpAssetRegistry::testnet_defaults(),
    };
    let venue = match hyperliquid_chain {
        HyperliquidCliChain::Mainnet => {
            DirectHyperliquidPerpVenue::mainnet(&private_key, registry)?
        }
        HyperliquidCliChain::Testnet => {
            DirectHyperliquidPerpVenue::testnet(&private_key, registry)?
        }
    };
    venue.validate_order(&plan.request)?;

    Ok(PreparedHyperliquidHedge { venue, input })
}

async fn run_prepared_hyperliquid_hedge(
    config: &HypercallLiquidatorConfig,
    execution: &LiquidationExecution,
    prepared: PreparedHyperliquidHedge,
) -> anyhow::Result<Option<RunHedgeOutput>> {
    if !matches!(
        execution,
        LiquidationExecution::SubmittedStandardMargin { .. }
    ) {
        return Ok(Some(RunHedgeOutput::Skipped {
            reason: "LiquidationNotSubmitted".to_string(),
        }));
    }
    let result =
        execute_hedge_after_liquidation(config, execution, &prepared.venue, &prepared.input)
            .await?;
    Ok(Some(match result {
        HedgeExecution::Submitted { plan, result } => RunHedgeOutput::Submitted {
            symbol: plan.request.symbol,
            is_buy: plan.request.is_buy,
            price: plan.request.price,
            size: plan.request.size,
            notional_usdc: plan.notional_usdc.to_string(),
            result_debug: format!("{result:?}"),
        },
        HedgeExecution::Skipped(reason) => RunHedgeOutput::Skipped {
            reason: format!("{reason:?}"),
        },
    }))
}

fn hedge_args_present(args: &HedgeCliArgs) -> bool {
    args.allow_live_hedge
        || args.hyperliquid_chain.is_some()
        || args.hedge_underlying.is_some()
        || args.hedge_mark_price_usdc.is_some()
        || args.hedge_delta.is_some()
        || args.hedge_position_symbol.is_some()
}

fn required_arg(value: Option<String>, name: &'static str) -> anyhow::Result<String> {
    value.ok_or_else(|| anyhow::anyhow!("missing {name}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> HypercallLiquidatorConfig {
        HypercallLiquidatorConfig::from_toml_str(include_str!(
            "../examples/liquidator.example.toml"
        ))
        .unwrap()
    }

    fn hedge_args() -> HedgeCliArgs {
        HedgeCliArgs {
            allow_live_hedge: true,
            hyperliquid_chain: Some(HyperliquidCliChain::Testnet),
            hedge_underlying: Some("BTC".to_string()),
            hedge_mark_price_usdc: Some("70000".to_string()),
            hedge_delta: Some(1.0),
            hedge_position_symbol: None,
        }
    }

    #[test]
    fn live_hedge_requires_explicit_hyperliquid_chain() {
        let mut args = hedge_args();
        args.hyperliquid_chain = None;

        let error = match prepare_required_hyperliquid_hedge(&config(), args) {
            Ok(_) => panic!("expected missing chain error"),
            Err(error) => error,
        };

        assert!(error.to_string().contains("--hyperliquid-chain"));
    }

    #[test]
    fn live_hedge_must_be_placeable_before_liquidation() {
        let mut config = config();
        config.hedge.enabled = false;

        let error = match prepare_required_hyperliquid_hedge(&config, hedge_args()) {
            Ok(_) => panic!("expected unplaceable hedge error"),
            Err(error) => error,
        };

        assert!(error.to_string().contains("not placeable"));
    }

    #[test]
    fn hedge_only_live_opt_in_counts_as_hedge_request() {
        let args = HedgeCliArgs {
            allow_live_hedge: true,
            hyperliquid_chain: None,
            hedge_underlying: None,
            hedge_mark_price_usdc: None,
            hedge_delta: None,
            hedge_position_symbol: None,
        };

        assert!(hedge_args_present(&args));
        let error = match prepare_optional_hyperliquid_hedge(&config(), args) {
            Ok(_) => panic!("expected missing chain error"),
            Err(error) => error,
        };

        assert!(error.to_string().contains("--hyperliquid-chain"));
    }

    #[test]
    fn hedge_only_chain_counts_as_hedge_request() {
        let args = HedgeCliArgs {
            allow_live_hedge: false,
            hyperliquid_chain: Some(HyperliquidCliChain::Testnet),
            hedge_underlying: None,
            hedge_mark_price_usdc: None,
            hedge_delta: None,
            hedge_position_symbol: None,
        };

        assert!(hedge_args_present(&args));
        let error = match prepare_optional_hyperliquid_hedge(&config(), args) {
            Ok(_) => panic!("expected live hedge opt-in error"),
            Err(error) => error,
        };

        assert!(error.to_string().contains("--allow-live-hedge"));
    }
}
