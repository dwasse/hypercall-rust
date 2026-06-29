use crate::api_models::{
    OptionsChainStrikeRow, PortfolioGreeksAggregate, PositionGreeksLeg, PositionWithMetrics,
    SpanMarginSummary,
};
use crate::{Side, WalletAddress};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CandleResolution {
    #[serde(rename = "1m")]
    OneMinute,
    #[serde(rename = "5m")]
    FiveMinutes,
    #[serde(rename = "15m")]
    FifteenMinutes,
    #[serde(rename = "1h")]
    OneHour,
    #[serde(rename = "4h")]
    FourHours,
    #[serde(rename = "1d")]
    OneDay,
}

impl CandleResolution {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::OneMinute => "1m",
            Self::FiveMinutes => "5m",
            Self::FifteenMinutes => "15m",
            Self::OneHour => "1h",
            Self::FourHours => "4h",
            Self::OneDay => "1d",
        }
    }

    pub fn interval_ms(self) -> i64 {
        match self {
            Self::OneMinute => 60_000,
            Self::FiveMinutes => 300_000,
            Self::FifteenMinutes => 900_000,
            Self::OneHour => 3_600_000,
            Self::FourHours => 14_400_000,
            Self::OneDay => 86_400_000,
        }
    }
}

impl fmt::Display for CandleResolution {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for CandleResolution {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "1m" => Ok(Self::OneMinute),
            "5m" => Ok(Self::FiveMinutes),
            "15m" => Ok(Self::FifteenMinutes),
            "1h" => Ok(Self::OneHour),
            "4h" => Ok(Self::FourHours),
            "1d" => Ok(Self::OneDay),
            _ => Err(format!("Unsupported candle resolution: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsOrderRequest {
    pub price: Decimal,
    pub size: Decimal,
    pub symbol: String,
    pub side: Side,
    pub tif: crate::TimeInForce,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsOrderMessage {
    pub order_id: Option<u64>,
    pub request: WsOrderRequest,
    pub status: crate::OrderUpdateStatus,
    pub timestamp: u64,
    pub reason: Option<String>,
    pub wallet_address: WalletAddress,
    #[serde(default = "default_instrument_type")]
    pub instrument_type: String,
}

fn default_instrument_type() -> String {
    "option".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WsMessage {
    /// Subscribe to a data channel
    Subscribe {
        channel: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        symbols: Option<Vec<String>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        expiry: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        option_type: Option<String>,
    },
    /// Unsubscribe from a data channel
    Unsubscribe {
        channel: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        symbols: Option<Vec<String>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        expiry: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        option_type: Option<String>,
    },
    /// Order status update (authenticated)
    OrderUpdate(WsOrderMessage),
    /// L2 orderbook snapshot/update
    OrderbookUpdate(WsOrderbookUpdate),
    /// Fill notification (authenticated)
    Fill(WsFillUpdate),
    /// Public trade event
    Trade(WsTradeUpdate),
    /// Underlying candle update
    CandleUpdate(WsCandleUpdate),
    /// Market listing change
    MarketUpdate(WsMarketUpdate),
    /// Incremental options chain update
    OptionsChainUpdate(WsOptionsChainUpdate),
    /// Real-time index/spot price update for all underlyings
    IndexPriceUpdate(WsIndexPriceUpdate),
    /// Portfolio update (authenticated)
    PortfolioUpdate(PortfolioUpdate),
    /// Position expiry notification (authenticated)
    PositionExpired(WsPositionExpired),
    /// Liquidation state change (authenticated)
    LiquidationStateChange(WsLiquidationStateChange),
    /// Competition leaderboard update for connected wallet (authenticated)
    CompetitionUpdate(WsCompetitionUpdate),
    /// Competition PnL summary for connected wallet (authenticated)
    CompetitionPnlSummary(WsCompetitionPnlSummary),
    /// Final competition stats notification for connected wallet (authenticated)
    CompetitionFinalStats(WsCompetitionFinalStats),
    /// Competition rank movement notification for connected wallet (authenticated)
    CompetitionRankChange(WsCompetitionRankChange),
    /// Competition gap-to-next notification for connected wallet (authenticated)
    CompetitionGapUpdate(WsCompetitionGapUpdate),
    /// Competition final standing notification for connected wallet (authenticated)
    CompetitionFinalStanding(WsCompetitionFinalStanding),
    /// Identify the connection with a wallet address (replaces query-param ?wallet=).
    Authenticate { wallet: String },
    /// Server confirms wallet identification
    Authenticated { wallet: String },
    /// Error message from server
    Error { message: String },
    /// Subscription confirmed
    Subscribed { channel: String },
    /// Unsubscription confirmed
    Unsubscribed { channel: String },
    /// Indicative market data from aggregated QP quotes (public)
    IndicativeMarketData(WsIndicativeMarketData),
    /// RFQ quotes available for taker (authenticated)
    RfqQuotes(WsRfqQuotes),
    /// RFQ status update (authenticated)
    RfqStatusUpdate(WsRfqStatusUpdate),
    /// Submit an RFQ request via WebSocket (authenticated)
    SubmitRfq {
        rfq_id: String,
        legs: Vec<WsRfqLegRequest>,
        wallet_address: String,
        nonce: u64,
        signature: String,
    },
    /// Submit an RFQ with auto-execute via WebSocket (authenticated).
    /// The taker pre-authorizes execution with a directional `limit_price`.
    SubmitAutoExecuteRfq {
        rfq_id: String,
        legs: Vec<WsRfqLegRequest>,
        wallet_address: String,
        /// Directional premium limit as a decimal string. Buy RFQs use this
        /// as a max debit. Sell RFQs use it as a min credit.
        limit_price: String,
        nonce: u64,
        signature: String,
    },
    /// Accept an RFQ quote via WebSocket (authenticated)
    AcceptRfqQuote {
        rfq_id: String,
        quote_id: String,
        wallet_address: String,
        nonce: u64,
        signature: String,
    },
    /// RFQ accept result pushed back to the client
    RfqAcceptResult {
        rfq_id: String,
        quote_id: String,
        status: String,
        fill_id: Option<String>,
        /// Used for wallet-based filtering in `publish_to_channel` so
        /// the result is only delivered to the taker, not all rfq
        /// subscribers. Excluded from the wire format.
        #[serde(skip, default)]
        taker_wallet: Option<WalletAddress>,
    },
    /// Place an order via WebSocket (authenticated)
    PlaceOrder {
        wallet: String,
        symbol: String,
        side: String,
        size: String,
        price: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tif: Option<String>,
        /// Optional order route. It is not required before July 4, 2026, but
        /// may become required later.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        route: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        client_id: Option<String>,
        nonce: u64,
        signature: String,
        #[serde(default)]
        mmp_enabled: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        builder_code_address: Option<String>,
    },
    /// Order placement result pushed back to the client
    OrderResult(WsOrderResult),
    #[serde(other)]
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsOrderResult {
    pub order_id: Option<u64>,
    pub status: String,
    pub symbol: String,
    pub side: String,
    pub price: String,
    pub size: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsIndicativeMarketData {
    pub instrument: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub best_bid: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bid_iv: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ask_iv: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub best_ask: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub indicative_bid_size: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub indicative_ask_size: Option<Decimal>,
    pub num_providers: u32,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsRfqQuotes {
    pub rfq_id: String,
    pub quotes: Vec<WsRfqQuoteEntry>,
    pub status: String,
    pub taker_wallet: WalletAddress,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsRfqQuoteEntry {
    pub quote_id: String,
    pub net_premium: Decimal,
    pub expires_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsRfqStatusUpdate {
    pub rfq_id: String,
    pub status: String,
    pub taker_wallet: WalletAddress,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsRfqLegRequest {
    pub instrument: String,
    pub side: Side,
    pub size: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsOrderbookUpdate {
    pub symbol: String,
    pub option_token_address: Option<WalletAddress>,
    pub bids: Vec<(Decimal, Decimal)>,
    pub asks: Vec<(Decimal, Decimal)>,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsFillUpdate {
    pub order_id: i64,
    pub fill_id: i64,
    pub symbol: String,
    pub side: String,
    pub price: Decimal,
    pub size: Decimal,
    pub timestamp: i64,
    pub wallet_address: WalletAddress,
    pub fee: Decimal,
    pub trade_id: i64,
    pub is_taker: bool,
    pub builder_code_address: Option<WalletAddress>,
    pub builder_code_fee: Option<Decimal>,
    #[serde(default = "default_ws_instrument_type")]
    pub instrument_type: String,
}

fn default_ws_instrument_type() -> String {
    "option".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsTradeUpdate {
    pub symbol: String,
    pub price: Decimal,
    pub size: Decimal,
    pub side: String,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsCandleUpdate {
    pub underlying: String,
    pub resolution: CandleResolution,
    pub start_time_ms: i64,
    pub end_time_ms: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum PortfolioUpdate {
    Initial {
        positions: Vec<PositionWithMetrics>,
        timestamp: i64,
    },
    PositionUpdate {
        position: PositionWithMetrics,
        timestamp: i64,
    },
    BalanceUpdate {
        total_margin_used: Decimal,
        timestamp: i64,
    },
    MarginUpdate {
        span_margin: SpanMarginSummary,
        total_margin_used: Decimal,
        available_balance: Decimal,
        timestamp: i64,
    },
    GreeksUpdate {
        per_leg: Vec<PositionGreeksLeg>,
        aggregate: Option<PortfolioGreeksAggregate>,
        timestamp: i64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsPositionExpired {
    pub wallet_address: WalletAddress,
    pub symbol: String,
    pub position_size: Decimal,
    pub settlement_price: Decimal,
    pub settlement_value: Decimal,
    pub settlement_entry_price: Option<Decimal>,
    pub cost_basis: Option<Decimal>,
    pub net_pnl: Option<Decimal>,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsLiquidationStateChange {
    pub wallet_address: WalletAddress,
    pub previous_state: String,
    pub new_state: String,
    pub liquidation_mode: Option<String>,
    pub margin_mode: String,
    pub equity: Decimal,
    pub mm_required: Decimal,
    pub maintenance_margin: Decimal,
    pub shortfall: Decimal,
    pub partial_liquidation: Option<WsPartialLiquidationState>,
    pub full_liquidation: Option<WsFullLiquidationState>,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsPartialLiquidationState {
    pub entered_at: i64,
    pub target_equity: Decimal,
    pub mm_shortfall: Decimal,
    pub escalation_deadline: i64,
    pub last_reprice_at: Option<i64>,
    pub active_order_request_ids: Vec<String>,
    pub active_order_client_ids: Vec<String>,
    pub bonus_bps: i32,
    pub pending_full_auction_id: Option<String>,
    pub pending_full_request_id: Option<String>,
    pub pending_full_tx_hash: Option<String>,
    pub pending_full_margin_needed: Option<Decimal>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsFullLiquidationState {
    pub auction_id: Option<String>,
    pub request_id: Option<String>,
    pub tx_hash: Option<String>,
    pub started_at: Option<i64>,
    pub chain_start_time: Option<i64>,
    pub margin_needed: Option<Decimal>,
    pub stop_request_id: Option<String>,
    pub stop_tx_hash: Option<String>,
    pub liquidated_at: Option<i64>,
    pub winner: Option<String>,
    pub bonus: Option<Decimal>,
    pub resolution_tx_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsCompetitionUpdate {
    pub wallet_address: WalletAddress,
    pub competition_id: i64,
    pub rank: i64,
    pub pnl: Decimal,
    pub volume: Decimal,
    pub efficiency: Decimal,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsCompetitionPnlStanding {
    pub competition_id: i64,
    pub competition_name: String,
    pub competition_state: String,
    pub rank: Option<usize>,
    pub pnl: Decimal,
    pub volume: Decimal,
    pub efficiency: Decimal,
    pub medal: Option<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsCompetitionPnlSummary {
    pub wallet_address: WalletAddress,
    pub lifetime_realized_pnl: Decimal,
    pub active_competition: Option<WsCompetitionPnlStanding>,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsCompetitionFinalStats {
    pub wallet_address: WalletAddress,
    pub competition_id: i64,
    pub rank: i64,
    pub pnl: Decimal,
    pub volume: Decimal,
    pub efficiency: Decimal,
    pub medal: Option<i64>,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexPriceEntry {
    pub underlying: String,
    pub price: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsIndexPriceUpdate {
    pub prices: Vec<IndexPriceEntry>,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsCompetitionRankChange {
    pub wallet_address: WalletAddress,
    pub competition_id: i64,
    pub from_rank: i64,
    pub to_rank: i64,
    pub delta_places: i64,
    pub pnl: Decimal,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsCompetitionGapUpdate {
    pub wallet_address: WalletAddress,
    pub competition_id: i64,
    pub rank: i64,
    pub next_rank: Option<i64>,
    pub gap_metric_value: Option<Decimal>,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsCompetitionFinalStanding {
    pub wallet_address: WalletAddress,
    pub competition_id: i64,
    pub rank: i64,
    pub pnl: Decimal,
    pub volume: Decimal,
    pub efficiency: Decimal,
    pub medal: Option<i64>,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum WsMarketUpdate {
    Created {
        symbol: String,
        strike: Decimal,
        is_call: bool,
        underlying: String,
        expiry: u32,
        timestamp: u64,
    },
    Deleted {
        symbol: String,
        timestamp: u64,
    },
    Expired {
        symbol: String,
        strike: Decimal,
        is_call: bool,
        underlying: String,
        expiry: u32,
        timestamp: u64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
#[allow(clippy::large_enum_variant)]
pub enum WsOptionsChainUpdate {
    Upsert {
        currency: String,
        expiry: u64,
        row: OptionsChainStrikeRow,
        timestamp: i64,
    },
    Remove {
        currency: String,
        expiry: u64,
        strike: f64,
        option_type: String,
        symbol: String,
        timestamp: i64,
    },
}
