use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DashboardSnapshot {
    pub account: String,
    pub hypercall: MarginPanel,
    pub hyperliquid: MarginPanel,
    pub liquidation: LiquidationPanel,
    pub hedge: HedgePanel,
    #[serde(default)]
    pub collateral_prompts: Vec<CollateralPrompt>,
    #[serde(default)]
    pub kill_switches: Vec<KillSwitch>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MarginPanel {
    #[serde(with = "rust_decimal::serde::str")]
    pub equity_usdc: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub initial_margin_usdc: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub maintenance_margin_usdc: Decimal,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LiquidationPanel {
    pub mode: String,
    pub status: String,
    #[serde(with = "rust_decimal::serde::str")]
    pub current_bid_usdc: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub maintenance_excess_usdc: Decimal,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HedgePanel {
    pub venue: String,
    pub status: String,
    pub symbol: Option<String>,
    pub side: Option<String>,
    #[serde(with = "rust_decimal::serde::str")]
    pub notional_usdc: Decimal,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CollateralPrompt {
    pub venue: String,
    pub action: String,
    #[serde(with = "rust_decimal::serde::str")]
    pub amount_usdc: Decimal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KillSwitch {
    pub name: String,
    pub enabled: bool,
}

#[cfg(feature = "ui")]
mod leptos_ui {
    use leptos::prelude::*;

    use super::{DashboardSnapshot, MarginPanel};

    const DASHBOARD_CSS: &str = r#"
.liquidator-dashboard {
  box-sizing: border-box;
  min-height: 100vh;
  padding: 28px;
  color: #f7f8fb;
  background:
    linear-gradient(180deg, rgba(73, 211, 255, 0.10), transparent 280px),
    #07080b;
  font-family: Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
}
.liquidator-dashboard * { box-sizing: border-box; }
.liquidator-dashboard__header {
  display: flex;
  align-items: flex-end;
  justify-content: space-between;
  gap: 20px;
  margin-bottom: 22px;
}
.liquidator-dashboard__eyebrow {
  margin: 0 0 6px;
  color: rgba(247, 248, 251, 0.58);
  font-size: 12px;
  font-weight: 700;
  letter-spacing: 0;
  text-transform: uppercase;
}
.liquidator-dashboard h1 {
  max-width: 100%;
  margin: 0;
  overflow-wrap: anywhere;
  color: #ffffff;
  font-size: 28px;
  line-height: 1.15;
  letter-spacing: 0;
}
.liquidator-dashboard__status {
  flex: 0 0 auto;
  border: 1px solid rgba(169, 250, 56, 0.34);
  border-radius: 8px;
  padding: 10px 12px;
  color: #a9fa38;
  background: rgba(169, 250, 56, 0.10);
  font-size: 13px;
  font-weight: 800;
}
.liquidator-dashboard__grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 14px;
  margin-bottom: 14px;
}
.liquidator-dashboard__card {
  min-width: 0;
  border: 1px solid rgba(255, 255, 255, 0.10);
  border-radius: 8px;
  padding: 16px;
  background: rgba(17, 20, 26, 0.94);
  box-shadow: 0 14px 48px rgba(0, 0, 0, 0.26);
}
.liquidator-dashboard__card h2 {
  margin: 0 0 14px;
  color: rgba(247, 248, 251, 0.88);
  font-size: 13px;
  line-height: 1.2;
  letter-spacing: 0;
  text-transform: uppercase;
}
.liquidator-dashboard dl {
  display: grid;
  grid-template-columns: minmax(96px, 0.8fr) minmax(0, 1.2fr);
  gap: 10px 14px;
  margin: 0;
}
.liquidator-dashboard dt {
  min-width: 0;
  color: rgba(247, 248, 251, 0.50);
  font-size: 12px;
}
.liquidator-dashboard dd {
  min-width: 0;
  margin: 0;
  overflow-wrap: anywhere;
  color: #ffffff;
  font-size: 13px;
  font-weight: 750;
  text-align: right;
}
.liquidator-dashboard ul {
  display: grid;
  gap: 10px;
  margin: 0;
  padding: 0;
  list-style: none;
}
.liquidator-dashboard li {
  display: grid;
  grid-template-columns: minmax(0, 0.9fr) minmax(0, 1fr) minmax(72px, 0.5fr);
  gap: 10px;
  align-items: center;
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 8px;
  padding: 10px 12px;
  background: rgba(255, 255, 255, 0.035);
}
.liquidator-dashboard li span,
.liquidator-dashboard li strong {
  min-width: 0;
  overflow-wrap: anywhere;
  font-size: 12px;
}
.liquidator-dashboard li span {
  color: rgba(247, 248, 251, 0.58);
}
.liquidator-dashboard li strong {
  color: #f7f8fb;
}
.liquidator-dashboard__empty {
  border: 1px dashed rgba(255, 255, 255, 0.16);
  border-radius: 8px;
  padding: 12px;
  color: rgba(247, 248, 251, 0.54);
  font-size: 12px;
}
@media (max-width: 820px) {
  .liquidator-dashboard { padding: 18px; }
  .liquidator-dashboard__header {
    align-items: stretch;
    flex-direction: column;
  }
  .liquidator-dashboard__grid { grid-template-columns: 1fr; }
  .liquidator-dashboard dl { grid-template-columns: 1fr; }
  .liquidator-dashboard dd { text-align: left; }
  .liquidator-dashboard li { grid-template-columns: 1fr; }
}
"#;

    #[component]
    pub fn LiquidatorDashboard(snapshot: DashboardSnapshot) -> impl IntoView {
        let hedge_side = snapshot
            .hedge
            .side
            .clone()
            .unwrap_or_else(|| "none".to_string());
        let hedge_symbol = snapshot
            .hedge
            .symbol
            .clone()
            .unwrap_or_else(|| "none".to_string());

        view! {
            <main class="liquidator-dashboard">
                <style>{DASHBOARD_CSS}</style>
                <header class="liquidator-dashboard__header">
                    <div>
                        <p class="liquidator-dashboard__eyebrow">"Public liquidator POC"</p>
                        <h1>{snapshot.account.clone()}</h1>
                    </div>
                    <div class="liquidator-dashboard__status">{snapshot.liquidation.status.clone()}</div>
                </header>

                <section class="liquidator-dashboard__grid">
                    <MarginCard title="Hypercall" panel=snapshot.hypercall.clone() />
                    <MarginCard title="Hyperliquid" panel=snapshot.hyperliquid.clone() />
                </section>

                <section class="liquidator-dashboard__grid">
                    <article class="liquidator-dashboard__card">
                        <h2>"Liquidation"</h2>
                        <dl>
                            <dt>"Mode"</dt>
                            <dd>{snapshot.liquidation.mode.clone()}</dd>
                            <dt>"Current bid"</dt>
                            <dd>{snapshot.liquidation.current_bid_usdc.to_string()}</dd>
                            <dt>"MM excess"</dt>
                            <dd>{snapshot.liquidation.maintenance_excess_usdc.to_string()}</dd>
                        </dl>
                    </article>

                    <article class="liquidator-dashboard__card">
                        <h2>"Delta hedge"</h2>
                        <dl>
                            <dt>"Venue"</dt>
                            <dd>{snapshot.hedge.venue.clone()}</dd>
                            <dt>"Status"</dt>
                            <dd>{snapshot.hedge.status.clone()}</dd>
                            <dt>"Order"</dt>
                            <dd>{format!("{hedge_side} {hedge_symbol}")}</dd>
                            <dt>"Notional"</dt>
                            <dd>{snapshot.hedge.notional_usdc.to_string()}</dd>
                        </dl>
                    </article>
                </section>

                <section class="liquidator-dashboard__grid">
                    <article class="liquidator-dashboard__card">
                        <h2>"Collateral"</h2>
                        {if snapshot.collateral_prompts.is_empty() {
                            view! { <div class="liquidator-dashboard__empty">"no collateral prompt"</div> }.into_any()
                        } else {
                            view! {
                                <ul>
                                    {snapshot.collateral_prompts.iter().map(|prompt| {
                                        view! {
                                            <li>
                                                <span>{prompt.venue.clone()}</span>
                                                <strong>{prompt.action.clone()}</strong>
                                                <span>{prompt.amount_usdc.to_string()}</span>
                                            </li>
                                        }
                                    }).collect_view()}
                                </ul>
                            }.into_any()
                        }}
                    </article>

                    <article class="liquidator-dashboard__card">
                        <h2>"Kill switches"</h2>
                        {if snapshot.kill_switches.is_empty() {
                            view! { <div class="liquidator-dashboard__empty">"no kill switches configured"</div> }.into_any()
                        } else {
                            view! {
                                <ul>
                                    {snapshot.kill_switches.iter().map(|switch| {
                                        let state = if switch.enabled { "enabled" } else { "disabled" };
                                        view! {
                                            <li>
                                                <span>{switch.name.clone()}</span>
                                                <strong>{state}</strong>
                                                <span>{if switch.enabled { "blocking" } else { "open" }}</span>
                                            </li>
                                        }
                                    }).collect_view()}
                                </ul>
                            }.into_any()
                        }}
                    </article>
                </section>
            </main>
        }
    }

    #[component]
    fn MarginCard(title: &'static str, panel: MarginPanel) -> impl IntoView {
        view! {
            <article class="liquidator-dashboard__card">
                <h2>{title}</h2>
                <dl>
                    <dt>"Equity"</dt>
                    <dd>{panel.equity_usdc.to_string()}</dd>
                    <dt>"IM"</dt>
                    <dd>{panel.initial_margin_usdc.to_string()}</dd>
                    <dt>"MM"</dt>
                    <dd>{panel.maintenance_margin_usdc.to_string()}</dd>
                </dl>
            </article>
        }
    }
}

#[cfg(feature = "ui")]
pub use leptos_ui::LiquidatorDashboard;

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn dashboard_snapshot_serializes_margin_and_prompts() {
        let snapshot = DashboardSnapshot {
            account: "0x0000000000000000000000000000000000000001".to_string(),
            hypercall: MarginPanel {
                equity_usdc: dec!(900),
                initial_margin_usdc: dec!(1200),
                maintenance_margin_usdc: dec!(1000),
            },
            hyperliquid: MarginPanel {
                equity_usdc: dec!(5000),
                initial_margin_usdc: dec!(100),
                maintenance_margin_usdc: dec!(50),
            },
            liquidation: LiquidationPanel {
                mode: "standard".to_string(),
                status: "eligible".to_string(),
                current_bid_usdc: dec!(1000),
                maintenance_excess_usdc: dec!(-100),
            },
            hedge: HedgePanel {
                venue: "hyperliquid".to_string(),
                status: "planned".to_string(),
                symbol: Some("BTC-PERP".to_string()),
                side: Some("buy".to_string()),
                notional_usdc: dec!(10000),
            },
            collateral_prompts: vec![CollateralPrompt {
                venue: "hypercall".to_string(),
                action: "add_liquidation_usdc".to_string(),
                amount_usdc: dec!(1000),
            }],
            kill_switches: vec![KillSwitch {
                name: "liquidation".to_string(),
                enabled: false,
            }],
        };

        let json = serde_json::to_string(&snapshot).unwrap();
        assert!(json.contains("\"hypercall\""));
        assert!(json.contains("\"add_liquidation_usdc\""));
    }
}
