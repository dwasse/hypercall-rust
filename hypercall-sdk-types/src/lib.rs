pub mod api_models;
pub mod enums;
pub mod requests;
pub mod responses;
pub mod wallet_address;
pub mod ws_protocol;

pub use api_models::{
    ExchangeInfoResponse, Instrument, MarketInfo, MarketsResponse, SigningDomainInfo,
};
pub use enums::{
    FillSource, MarketAction, MarketUpdateStatus, OptionType, OrderAction, OrderRoute, OrderStatus,
    OrderUpdateStatus, QpStatus, RfqStatus, Side, TimeInForce, TradeSide, TradingModes,
    TransactionStatus,
};
pub use requests::{
    AcceptRfqRequest, ApproveAgentRequest, BulkCancelOrderRequest, BulkPlaceOrderRequest,
    CancelOrderByClientIdRequest, CancelOrderByCloidRequest, CancelOrderRequest, PlaceOrderRequest,
    ReplaceOrderRequest, RevokeAgentRequest, RfqLegRequest, SetMarginModeRequest,
    StandardMarginLiquidationOrderRequest, StandardMarginLiquidationPositionRequest,
    SubmitRfqRequest,
};
pub use responses::{
    ApiResponse, ApproveAgentResponse, AuthorizedAgentsResponse, BulkCancelOrderResponse,
    BulkOrderResult, BulkPlaceOrderResponse, CompetitionAccountPnl, CompetitionAccountResponse,
    CompetitionConnectedUserRank, CompetitionLeaderboardResponse, CompetitionLeaderboardRow,
    CompetitionPnlStanding, CompetitionPnlSummary, CompetitionPnlSummaryResponse, CursorPage, Fill,
    FullLiquidationStatusData, HistoricalPnlInterval, HistoricalPnlPoint, HistoricalPnlResponse,
    HistoricalTheoInterval, HistoricalTheoPoint, HistoricalTheoResponse, InstrumentResponse,
    InstrumentSpecResponse, JsonRpcError, JsonRpcResponse, L2Message, L2Update,
    LiquidationHistoryEntry, LiquidationStatusData, LiquidationStatusResponse, MarginSummary,
    Market, MarketResponse, MarketUpdateMessage, OptionGreeks, OptionSummary, OrderBookGreeks,
    OrderBookResponse, OrderBookStats, OrderInfo, OrderMessage, OrderUpdateMessage,
    OrderbookUpdate, OrdersResponse, Pagination, PartialLiquidationStatusData, PortfolioPosition,
    PortfolioResponse, PublicLiquidationsResponse, RevokeAgentResponse, RfqAcceptResponse,
    RfqHistoryResponse, RfqLegResponse, RfqQuoteLegResponse, RfqQuoteResponse, RfqStatusResponse,
    StandardMarginLiquidationOrderResponse, TickSizeStep, TradeMessage,
    HISTORICAL_PNL_INTERVAL_1D_MS, HISTORICAL_PNL_INTERVAL_1H_MS, HISTORICAL_PNL_INTERVAL_5M_MS,
    HISTORICAL_THEO_INTERVAL_1D_MS, HISTORICAL_THEO_INTERVAL_1H_MS, HISTORICAL_THEO_INTERVAL_5M_MS,
};
pub use wallet_address::WalletAddress;
pub use ws_protocol::*;

pub const RFQ_SELF_TRADE_REJECTION_REASON: &str =
    "Self-trade prevention: taker wallet equals quote provider wallet";

#[cfg(any(test, feature = "test-utils"))]
pub use wallet_address::test_wallet;
