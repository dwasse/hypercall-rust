//! # Hypercall Client
//!
//! Rust SDK for trading on Hypercall public HTTP and websocket APIs.
//!
//! ## Quick Start: Place an Options Order
//!
//! ```rust,no_run
//! use hypercall_client::{HypercallClient, HypercallWallet};
//! use hypercall_sdk_types::{Side, TimeInForce};
//! use rust_decimal::Decimal;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let api = HypercallClient::new("https://api.hypercall.xyz");
//! let wallet = HypercallWallet::from_private_key("0xYOUR_PRIVATE_KEY", 999)?;
//!
//! // Place a BTC call buy (options use typed args)
//! let resp = api.place_order(&wallet, "BTC-20260501-76000-C", Side::Buy, Decimal::new(2000, 0), Decimal::new(5, 0), TimeInForce::IOC).await?;
//! println!("Order ID: {:?}", resp);
//! # Ok(())
//! # }
//! ```
//!
//! ## Crate Features
//!
pub mod api;
pub mod error;
pub mod qp_client;
pub mod rfq;
pub mod wallet;
pub mod websocket;

pub use api::{
    BulkOrderParams, BulkReplaceOrderParams, HypercallClient, OrderDecimalInput, OrderOptions,
    PlaceOrderParams, PublicLiquidationsQuery, ReplaceOrderParams, StandardMarginLiquidationParams,
};
pub use error::ClientError;
pub use hypercall_sdk_types::ws_protocol::WsMessage;
pub use hypercall_sdk_types::{CursorPage, LiquidationHistoryEntry, PublicLiquidationsResponse};
pub use qp_client::{
    NoopCallbacks, QpClientCallbacks, QpClientConfig, QpDisconnectReason, QpWriteFailure,
    QpWriteOperation,
};
pub use wallet::{
    AccountAddress, AtomicNonceProvider, CancelOrderSignature, HypercallSigner, HypercallWallet,
    NonceProvider, PlaceOrderSignature, ReplaceOrderSignature, StandardMarginLiquidationSignature,
};
pub use websocket::WsClient;

// Re-export commonly needed types from hypercall-sdk-types
pub use hypercall_sdk_types::{
    AcceptRfqRequest, ApiResponse, BulkCancelOrderResponse, BulkPlaceOrderResponse,
    CancelOrderRequest, ExchangeInfoResponse, Fill, HistoricalPnlInterval, HistoricalPnlPoint,
    HistoricalPnlResponse, HistoricalTheoInterval, HistoricalTheoPoint, HistoricalTheoResponse,
    InstrumentSpecResponse, MarginSummary, Market, OptionType, OrderInfo, OrderMessage,
    OrderStatus, PlaceOrderRequest, PortfolioPosition, PortfolioResponse, RfqLegRequest, Side,
    StandardMarginLiquidationOrderResponse, StandardMarginLiquidationPositionRequest,
    SubmitRfqRequest, TimeInForce,
};
