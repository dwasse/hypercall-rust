use hypercall_hyperliquid::{PerpVenue, PerpVenueOrderRequest, Tif};
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;

use crate::config::HedgeConfig;
use crate::types::PositionDelta;

#[derive(Debug, Clone, PartialEq)]
pub struct DeltaHedgeInput {
    pub underlying: String,
    pub mark_price_usdc: Decimal,
    pub positions: Vec<PositionDelta>,
}

#[derive(Debug, Clone)]
pub struct HedgePlan {
    pub request: PerpVenueOrderRequest,
    pub net_delta: f64,
    pub notional_usdc: Decimal,
}

#[derive(Debug, Clone)]
pub enum HedgePlanDecision {
    Place(HedgePlan),
    Skip(HedgeSkipReason),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HedgeSkipReason {
    HedgeDisabled,
    NonTakerPolicy,
    MissingPositions,
    InvalidMarkPrice,
    BelowDeltaBand {
        notional_usdc: Decimal,
        band_usdc: Decimal,
    },
    BelowMinOrderNotional {
        notional_usdc: Decimal,
        min_order_notional_usdc: Decimal,
    },
    DecimalConversion,
}

pub fn plan_delta_hedge(config: &HedgeConfig, input: &DeltaHedgeInput) -> HedgePlanDecision {
    if !config.enabled {
        return HedgePlanDecision::Skip(HedgeSkipReason::HedgeDisabled);
    }
    if !config.taker_only {
        return HedgePlanDecision::Skip(HedgeSkipReason::NonTakerPolicy);
    }
    if input.positions.is_empty() {
        return HedgePlanDecision::Skip(HedgeSkipReason::MissingPositions);
    }
    if input.mark_price_usdc <= Decimal::ZERO {
        return HedgePlanDecision::Skip(HedgeSkipReason::InvalidMarkPrice);
    }

    let net_delta: f64 = input
        .positions
        .iter()
        .filter(|position| position.underlying.eq_ignore_ascii_case(&input.underlying))
        .map(|position| position.delta)
        .sum();

    let abs_delta_decimal = match Decimal::try_from(net_delta.abs()) {
        Ok(delta) => delta,
        Err(_) => return HedgePlanDecision::Skip(HedgeSkipReason::DecimalConversion),
    };
    let desired_notional = abs_delta_decimal * input.mark_price_usdc;
    if desired_notional < config.delta_band_usd {
        return HedgePlanDecision::Skip(HedgeSkipReason::BelowDeltaBand {
            notional_usdc: desired_notional,
            band_usdc: config.delta_band_usd,
        });
    }
    if desired_notional < config.min_order_notional_usdc {
        return HedgePlanDecision::Skip(HedgeSkipReason::BelowMinOrderNotional {
            notional_usdc: desired_notional,
            min_order_notional_usdc: config.min_order_notional_usdc,
        });
    }

    let order_notional = desired_notional.min(config.max_order_notional_usdc);
    let size = order_notional / input.mark_price_usdc;
    let limit_price = taker_limit_price(
        input.mark_price_usdc,
        net_delta < 0.0,
        config.max_slippage_bps,
    );
    let Some(limit_price_f64) = limit_price.to_f64() else {
        return HedgePlanDecision::Skip(HedgeSkipReason::DecimalConversion);
    };
    let Some(size_f64) = size.to_f64() else {
        return HedgePlanDecision::Skip(HedgeSkipReason::DecimalConversion);
    };

    let symbol = format!("{}-PERP", input.underlying.to_uppercase());
    let request =
        PerpVenueOrderRequest::new(symbol, net_delta < 0.0, limit_price_f64, size_f64, Tif::Ioc);

    HedgePlanDecision::Place(HedgePlan {
        request,
        net_delta,
        notional_usdc: order_notional,
    })
}

pub async fn submit_hedge_plan<V: PerpVenue>(
    venue: &V,
    plan: &HedgePlan,
) -> hypercall_client::error::Result<V::OrderResult> {
    venue.place_order(plan.request.clone()).await
}

fn taker_limit_price(mark_price: Decimal, is_buy: bool, max_slippage_bps: u32) -> Decimal {
    let slippage = Decimal::from(max_slippage_bps) / Decimal::from(10_000u32);
    if is_buy {
        mark_price * (Decimal::ONE + slippage)
    } else {
        mark_price * (Decimal::ONE - slippage)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::HypercallLiquidatorConfig;
    use rust_decimal_macros::dec;

    fn config() -> HedgeConfig {
        HypercallLiquidatorConfig::from_toml_str(include_str!(
            "../examples/liquidator.example.toml"
        ))
        .unwrap()
        .hedge
    }

    #[test]
    fn plans_ioc_buy_to_offset_negative_delta() {
        let decision = plan_delta_hedge(
            &config(),
            &DeltaHedgeInput {
                underlying: "BTC".to_string(),
                mark_price_usdc: dec!(70000),
                positions: vec![PositionDelta {
                    symbol: "BTC-20261231-90000-C".to_string(),
                    underlying: "BTC".to_string(),
                    delta: -0.5,
                }],
            },
        );

        let HedgePlanDecision::Place(plan) = decision else {
            panic!("expected hedge order");
        };
        assert!(plan.request.is_buy);
        assert_eq!(plan.request.symbol, "BTC-PERP");
        assert_eq!(plan.request.tif, Tif::Ioc);
        assert_eq!(plan.request.price, 70525.0);
        assert_eq!(plan.notional_usdc, dec!(10000));
    }

    #[test]
    fn plans_ioc_sell_with_slippage_floor() {
        let decision = plan_delta_hedge(
            &config(),
            &DeltaHedgeInput {
                underlying: "BTC".to_string(),
                mark_price_usdc: dec!(70000),
                positions: vec![PositionDelta {
                    symbol: "BTC-20261231-90000-C".to_string(),
                    underlying: "BTC".to_string(),
                    delta: 0.5,
                }],
            },
        );

        let HedgePlanDecision::Place(plan) = decision else {
            panic!("expected hedge order");
        };
        assert!(!plan.request.is_buy);
        assert_eq!(plan.request.price, 69475.0);
    }

    #[test]
    fn skips_small_delta_inside_band() {
        let decision = plan_delta_hedge(
            &config(),
            &DeltaHedgeInput {
                underlying: "ETH".to_string(),
                mark_price_usdc: dec!(3000),
                positions: vec![PositionDelta {
                    symbol: "ETH-20261231-3000-C".to_string(),
                    underlying: "ETH".to_string(),
                    delta: 0.1,
                }],
            },
        );

        assert!(matches!(
            decision,
            HedgePlanDecision::Skip(HedgeSkipReason::BelowDeltaBand { .. })
        ));
    }
}
