//! Stable WebSocket wire DTOs for Hypercall clients.
//!
//! This crate intentionally contains protocol-shaped data only.

pub mod client;
pub mod qp;

pub use client::{
    ClientControlMessage, GatewayErrorCode, GatewayStatus, GatewayStatusMessage,
    UnsupportedWriteKind,
};
pub use qp::{
    GatewayResumeQuoteProvider, IndicativeQuote, QpClientMessage, QpInboundMessage,
    QpOutboundMessage, QpResponseLeg, QpRfqLeg, QpServerMessage,
};
