//! Quote Provider WebSocket protocol for `/ws/quotes`.

use serde::{Deserialize, Serialize};

/// Gateway-authenticated reconnect request for an already-authenticated QP.
///
/// Direct public QP connections use `ConnectQuoteProvider` and strict nonce
/// checks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GatewayResumeQuoteProvider {
    pub wallet: String,
    pub timestamp: String,
    pub nonce: u64,
    pub signature: String,
}

/// Messages sent by a QP client.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum QpClientMessage {
    /// First frame for direct QP connections.
    ConnectQuoteProvider {
        wallet: String,
        timestamp: String,
        nonce: u64,
        signature: String,
    },
    /// First frame for gateway-managed reconnects.
    ///
    /// Public QP clients must not send it directly.
    GatewayResumeQuoteProvider {
        wallet: String,
        timestamp: String,
        nonce: u64,
        signature: String,
    },
    /// Periodic indicative quote update.
    IndicativeQuoteUpdate { quotes: Vec<IndicativeQuote> },
    /// Firm quote response to an RFQ.
    RfqResponse {
        rfq_id: String,
        /// "quote" or "decline".
        action: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        legs: Option<Vec<QpResponseLeg>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        net_premium: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        valid_for_ms: Option<u64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        signature: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        nonce: Option<u64>,
    },
}

/// Messages sent to a QP client.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum QpServerMessage {
    Authenticated {
        wallet: String,
    },
    AuthFailed {
        reason: String,
    },
    RfqRequest {
        rfq_id: String,
        legs: Vec<QpRfqLeg>,
        taker_wallet: String,
        request_timestamp: u64,
        response_deadline_ms: u64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        auto_accept_limit: Option<String>,
        #[serde(default)]
        auto_execute: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        taker_limit_price: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        reference_price: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        min_improvement_tick: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        auction_deadline_ms: Option<u64>,
        #[serde(default)]
        requires_price_improvement: bool,
    },
    QpMarginRejection {
        rfq_id: String,
        quote_id: String,
        reason: String,
    },
    RfqAlreadyFilled {
        rfq_id: String,
        filled_by_quote_id: String,
    },
}

/// Backwards-compatible names for API-side code.
pub type QpInboundMessage = QpClientMessage;
pub type QpOutboundMessage = QpServerMessage;

/// A single indicative quote for an instrument.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndicativeQuote {
    pub instrument: String,
    pub bid_price: String,
    pub ask_price: String,
    pub max_bid_size: String,
    pub max_ask_size: String,
}

/// A leg in a QP's firm quote response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QpResponseLeg {
    pub instrument: String,
    pub side: String,
    pub price: String,
    pub size: String,
}

/// A leg in an RFQ request sent to QPs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QpRfqLeg {
    pub instrument: String,
    pub side: String,
    pub size: String,
}

#[cfg(test)]
mod tests {
    use super::{QpClientMessage, QpRfqLeg, QpServerMessage};

    #[test]
    fn rfq_request_deserializes_without_auto_accept_limit() {
        let json = r#"{"type":"rfq_request","rfq_id":"abc","legs":[],"taker_wallet":"0x123","request_timestamp":1,"response_deadline_ms":5000,"auto_execute":false}"#;
        let msg: QpServerMessage = serde_json::from_str(json).unwrap();

        match msg {
            QpServerMessage::RfqRequest {
                auto_accept_limit,
                auto_execute,
                taker_limit_price,
                reference_price,
                min_improvement_tick,
                auction_deadline_ms,
                requires_price_improvement,
                ..
            } => {
                assert_eq!(auto_accept_limit, None);
                assert!(!auto_execute);
                assert_eq!(taker_limit_price, None);
                assert_eq!(reference_price, None);
                assert_eq!(min_improvement_tick, None);
                assert_eq!(auction_deadline_ms, None);
                assert!(!requires_price_improvement);
            }
            _ => panic!("expected rfq_request"),
        }
    }

    #[test]
    fn rfq_request_serializes_auto_accept_limit_when_present() {
        let msg = QpServerMessage::RfqRequest {
            rfq_id: "abc".to_string(),
            legs: vec![QpRfqLeg {
                instrument: "BTC-20260501-90000-C".to_string(),
                side: "buy".to_string(),
                size: "1".to_string(),
            }],
            taker_wallet: "0x123".to_string(),
            request_timestamp: 1,
            response_deadline_ms: 5000,
            auto_accept_limit: Some("3999".to_string()),
            auto_execute: true,
            taker_limit_price: Some("3999".to_string()),
            reference_price: Some("3999".to_string()),
            min_improvement_tick: Some("0.0001".to_string()),
            auction_deadline_ms: Some(2000),
            requires_price_improvement: true,
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains(r#""auto_accept_limit":"3999""#));
        assert!(json.contains(r#""auto_execute":true"#));
        assert!(json.contains(r#""taker_limit_price":"3999""#));
        assert!(json.contains(r#""reference_price":"3999""#));
        assert!(json.contains(r#""min_improvement_tick":"0.0001""#));
        assert!(json.contains(r#""auction_deadline_ms":2000"#));
        assert!(json.contains(r#""requires_price_improvement":true"#));
    }

    #[test]
    fn gateway_resume_has_distinct_wire_type() {
        let msg = QpClientMessage::GatewayResumeQuoteProvider {
            wallet: "0x123".to_string(),
            timestamp: "42".to_string(),
            nonce: 7,
            signature: "0xsig".to_string(),
        };

        assert_eq!(
            serde_json::to_string(&msg).unwrap(),
            r#"{"type":"gateway_resume_quote_provider","wallet":"0x123","timestamp":"42","nonce":7,"signature":"0xsig"}"#
        );
    }
}
