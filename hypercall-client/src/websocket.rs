//! WebSocket client for real-time market data and account-scoped updates.
//!
//! Connects to the Hypercall WS endpoint (`/ws`) for streaming:
//! - **Public**: orderbook, trades, candles, index prices, options chain
//! - **Account-scoped**: order updates, fills, portfolio, position changes
//!
//! # Example
//!
//! ```rust,no_run
//! use hypercall_client::WsClient;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let ws = WsClient::new();
//! ws.connect("https://api.hypercall.xyz", None).await?;
//!
//! // Subscribe to fills (options + perps, distinguished by instrument_type)
//! ws.subscribe(vec!["fills", "order_updates"]).await?;
//!
//! // Wait for a message
//! let msg = ws.wait_for_message(|_| true, 5000).await;
//! println!("{:?}", msg);
//! # Ok(())
//! # }
//! ```
//!
//! # Channels
//!
//! | Channel | Auth | Description |
//! |---------|------|-------------|
//! | `orderbook` | No | L2 orderbook updates (filterable by symbols) |
//! | `trades` | No | Public trade events |
//! | `candles:BTC:1h` | No | Underlying candle updates |
//! | `index_prices` | No | Spot/index prices for all underlyings |
//! | `order_updates` | Yes | Order status changes (includes `instrument_type`) |
//! | `fills` | Yes | Fill notifications (includes `instrument_type`) |
//! | `portfolio` | Yes | Portfolio balance + position updates |

use std::sync::Arc;

use futures::{SinkExt, StreamExt};
use sonic_rs::JsonValueTrait;
use tokio::sync::{mpsc, Mutex};
use tokio::time::Duration;
use tokio_tungstenite::{connect_async, tungstenite};
use tracing::{debug, error, info};

use crate::error::{ClientError, Result};
use crate::wallet::AccountAddress;
use hypercall_sdk_types::ws_protocol::WsMessage;

const MAX_CONTROL_FRAME_PAYLOAD_LEN: usize = 125;

/// WebSocket client for Hypercall.
pub struct WsClient {
    messages: Arc<Mutex<Vec<sonic_rs::Value>>>,
    pongs: Arc<Mutex<Vec<Vec<u8>>>>,
    tx: Arc<Mutex<Option<mpsc::UnboundedSender<tungstenite::Message>>>>,
}

impl WsClient {
    /// Create a new WebSocket client (not yet connected).
    pub fn new() -> Self {
        Self {
            messages: Arc::new(Mutex::new(Vec::new())),
            pongs: Arc::new(Mutex::new(Vec::new())),
            tx: Arc::new(Mutex::new(None)),
        }
    }

    /// Connect to the WebSocket endpoint.
    ///
    /// If a wallet is provided, sends an `Authenticate` message after connecting
    /// and waits for the `Authenticated` confirmation before returning.
    pub async fn connect(&self, base_url: &str, wallet: Option<&AccountAddress>) -> Result<()> {
        let ws_url = format!("{}/ws", base_url.replace("http", "ws"));

        debug!("Connecting to WebSocket: {}", ws_url);

        let (ws_stream, response) = connect_async(&ws_url)
            .await
            .map_err(|e| ClientError::WebSocket(format!("Failed to connect: {}", e)))?;

        info!("WebSocket connected, status: {}", response.status());

        let (mut write, mut read) = ws_stream.split();
        let (tx, mut rx) = mpsc::unbounded_channel();

        // Store the sender
        *self.tx.lock().await = Some(tx.clone());

        let messages = self.messages.clone();
        let pongs = self.pongs.clone();
        let pong_tx = tx.clone();
        // Spawn task to handle incoming messages
        tokio::spawn(async move {
            while let Some(msg) = read.next().await {
                match msg {
                    Ok(tungstenite::Message::Text(text)) => {
                        debug!("WS received: {}", text);
                        if let Ok(json) = sonic_rs::from_str::<sonic_rs::Value>(&text) {
                            messages.lock().await.push(json);
                        }
                    }
                    Ok(tungstenite::Message::Close(_)) => {
                        info!("WebSocket closed");
                        break;
                    }
                    Ok(tungstenite::Message::Ping(payload)) => {
                        debug!("WS received ping");
                        if pong_tx.send(tungstenite::Message::Pong(payload)).is_err() {
                            break;
                        }
                    }
                    Ok(tungstenite::Message::Pong(payload)) => {
                        debug!("WS received pong");
                        pongs.lock().await.push(payload);
                    }
                    Err(e) => {
                        error!("WebSocket error: {}", e);
                        break;
                    }
                    _ => {}
                }
            }
        });

        // Spawn task to handle outgoing messages
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                if write.send(msg).await.is_err() {
                    break;
                }
            }
        });

        // Send Authenticate message if wallet is provided
        if let Some(w) = wallet {
            let auth_msg = WsMessage::Authenticate {
                wallet: w.to_string(),
            };
            let json = sonic_rs::to_string(&auth_msg)?;
            let tx_guard = self.tx.lock().await;
            if let Some(tx) = tx_guard.as_ref() {
                tx.send(tungstenite::Message::Text(json))
                    .map_err(|e| ClientError::WebSocket(format!("Failed to send auth: {}", e)))?;
            }
            drop(tx_guard);

            // Wait for Authenticated response
            let auth_result = self
                .wait_for_message(
                    |m| {
                        m.get("type")
                            .and_then(|t| t.as_str())
                            .is_some_and(|t| t == "Authenticated" || t == "Error")
                    },
                    5000,
                )
                .await;

            match auth_result {
                Some(msg) => {
                    let msg_type = msg.get("type").and_then(|t| t.as_str()).unwrap_or("");
                    if msg_type == "Error" {
                        let err_msg = msg
                            .get("message")
                            .and_then(|m| m.as_str())
                            .unwrap_or("Unknown error");
                        return Err(ClientError::WebSocket(format!(
                            "Authentication failed: {}",
                            err_msg
                        )));
                    }
                    info!("WebSocket authenticated as wallet: {}", w);
                }
                None => {
                    return Err(ClientError::WebSocket(
                        "Timed out waiting for authentication response".to_string(),
                    ));
                }
            }
        }

        Ok(())
    }

    fn validate_control_payload(payload: &[u8]) -> Result<()> {
        if payload.len() > MAX_CONTROL_FRAME_PAYLOAD_LEN {
            return Err(ClientError::WebSocket(format!(
                "WebSocket control frame payload exceeds {} bytes",
                MAX_CONTROL_FRAME_PAYLOAD_LEN
            )));
        }

        Ok(())
    }

    async fn send_control_frame(&self, msg: tungstenite::Message) -> Result<()> {
        let tx = self.tx.lock().await;
        let tx = tx
            .as_ref()
            .ok_or(ClientError::WebSocket("Not connected".to_string()))?;

        tx.send(msg)
            .map_err(|e| ClientError::WebSocket(format!("Failed to send: {}", e)))
    }

    /// Send an empty WebSocket ping frame.
    pub async fn ping(&self) -> Result<()> {
        self.ping_with_payload(Vec::new()).await
    }

    /// Send a WebSocket ping frame with a payload.
    ///
    /// WebSocket control frame payloads are limited to 125 bytes by the protocol.
    pub async fn ping_with_payload(&self, payload: impl Into<Vec<u8>>) -> Result<()> {
        let payload = payload.into();
        Self::validate_control_payload(&payload)?;
        self.send_control_frame(tungstenite::Message::Ping(payload))
            .await
    }

    /// Send a WebSocket pong frame with a payload.
    ///
    /// This is rarely needed directly because `WsClient` automatically responds
    /// to server ping frames.
    pub async fn pong_with_payload(&self, payload: impl Into<Vec<u8>>) -> Result<()> {
        let payload = payload.into();
        Self::validate_control_payload(&payload)?;
        self.send_control_frame(tungstenite::Message::Pong(payload))
            .await
    }

    /// Subscribe to channels.
    pub async fn subscribe(&self, channels: Vec<&str>) -> Result<()> {
        let tx = self.tx.lock().await;
        let tx = tx
            .as_ref()
            .ok_or(ClientError::WebSocket("Not connected".to_string()))?;

        for channel in channels {
            let msg = WsMessage::Subscribe {
                channel: channel.to_string(),
                symbols: None,
                expiry: None,
                option_type: None,
            };
            let json = sonic_rs::to_string(&msg)?;
            debug!("Subscribing to: {}", channel);
            tx.send(tungstenite::Message::Text(json))
                .map_err(|e| ClientError::WebSocket(format!("Failed to send: {}", e)))?;

            // Wait a bit for subscription confirmation
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        Ok(())
    }

    /// Get all received WebSocket pong payloads.
    pub async fn get_pongs(&self) -> Vec<Vec<u8>> {
        self.pongs.lock().await.clone()
    }

    /// Clear all received WebSocket pong payloads.
    pub async fn clear_pongs(&self) {
        self.pongs.lock().await.clear();
    }

    /// Wait for a pong payload matching a predicate.
    pub async fn wait_for_pong<F>(&self, check: F, timeout_ms: u64) -> Option<Vec<u8>>
    where
        F: Fn(&[u8]) -> bool,
    {
        let start = std::time::Instant::now();
        let timeout = Duration::from_millis(timeout_ms);

        while start.elapsed() < timeout {
            let pongs = self.get_pongs().await;
            if let Some(pong) = pongs.iter().find(|payload| check(payload)) {
                return Some(pong.clone());
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        None
    }

    /// Get all received messages.
    pub async fn get_messages(&self) -> Vec<sonic_rs::Value> {
        self.messages.lock().await.clone()
    }

    /// Clear all received messages.
    pub async fn clear_messages(&self) {
        self.messages.lock().await.clear();
    }

    /// Wait for a message matching a predicate.
    pub async fn wait_for_message<F>(&self, check: F, timeout_ms: u64) -> Option<sonic_rs::Value>
    where
        F: Fn(&sonic_rs::Value) -> bool,
    {
        let start = std::time::Instant::now();
        let timeout = Duration::from_millis(timeout_ms);

        while start.elapsed() < timeout {
            let messages = self.get_messages().await;
            if let Some(msg) = messages.iter().find(|m| check(m)) {
                return Some(msg.clone());
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        None
    }

    /// Count messages matching a predicate.
    pub async fn count_messages<F>(&self, check: F) -> usize
    where
        F: Fn(&sonic_rs::Value) -> bool,
    {
        self.get_messages()
            .await
            .iter()
            .filter(|msg| check(msg))
            .count()
    }
}

impl Default for WsClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sonic_rs::JsonValueTrait;

    #[test]
    fn test_ws_client_new() {
        let client = WsClient::new();
        // Client should be created successfully
        assert!(client.messages.try_lock().is_ok());
        assert!(client.pongs.try_lock().is_ok());
        assert!(client.tx.try_lock().is_ok());
    }

    #[test]
    fn test_ws_client_default() {
        let client = WsClient::default();
        assert!(client.messages.try_lock().is_ok());
    }

    #[tokio::test]
    async fn test_ws_client_get_messages_empty() {
        let client = WsClient::new();
        let messages = client.get_messages().await;
        assert!(messages.is_empty());
    }

    #[tokio::test]
    async fn test_ws_client_clear_messages() {
        let client = WsClient::new();
        // Add a message manually
        client
            .messages
            .lock()
            .await
            .push(sonic_rs::json!({"test": "value"}));
        assert_eq!(client.get_messages().await.len(), 1);

        // Clear messages
        client.clear_messages().await;
        assert!(client.get_messages().await.is_empty());
    }

    #[tokio::test]
    async fn test_ws_client_count_messages() {
        let client = WsClient::new();
        // Add some messages
        {
            let mut messages = client.messages.lock().await;
            messages.push(sonic_rs::json!({"type": "order", "id": 1}));
            messages.push(sonic_rs::json!({"type": "trade", "id": 2}));
            messages.push(sonic_rs::json!({"type": "order", "id": 3}));
        }

        let order_count = client
            .count_messages(|m| m.get("type").and_then(|t| t.as_str()) == Some("order"))
            .await;
        assert_eq!(order_count, 2);

        let trade_count = client
            .count_messages(|m| m.get("type").and_then(|t| t.as_str()) == Some("trade"))
            .await;
        assert_eq!(trade_count, 1);
    }

    #[tokio::test]
    async fn test_ws_client_subscribe_not_connected() {
        let client = WsClient::new();
        let result = client.subscribe(vec!["orders"]).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            crate::error::ClientError::WebSocket(_)
        ));
    }

    #[tokio::test]
    async fn test_ws_client_ping_not_connected() {
        let client = WsClient::new();
        let result = client.ping().await;
        assert!(matches!(
            result.unwrap_err(),
            crate::error::ClientError::WebSocket(_)
        ));
    }

    #[tokio::test]
    async fn test_ws_client_sends_ping_with_payload() {
        let client = WsClient::new();
        let (tx, mut rx) = mpsc::unbounded_channel();
        *client.tx.lock().await = Some(tx);

        client.ping_with_payload(b"health".to_vec()).await.unwrap();

        match rx.recv().await.unwrap() {
            tungstenite::Message::Ping(payload) => assert_eq!(payload, b"health"),
            other => panic!("Expected ping frame, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_ws_client_rejects_oversized_ping_payload() {
        let client = WsClient::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        *client.tx.lock().await = Some(tx);

        let result = client.ping_with_payload(vec![0; 126]).await;
        assert!(matches!(
            result.unwrap_err(),
            crate::error::ClientError::WebSocket(_)
        ));
    }

    #[tokio::test]
    async fn test_ws_client_tracks_received_pongs() {
        let client = WsClient::new();
        client.pongs.lock().await.push(b"health".to_vec());

        let pong = client
            .wait_for_pong(|payload| payload == b"health", 100)
            .await;
        assert_eq!(pong, Some(b"health".to_vec()));

        client.clear_pongs().await;
        assert!(client.get_pongs().await.is_empty());
    }

    #[tokio::test]
    async fn test_ws_client_responds_to_server_ping() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut ws = tokio_tungstenite::accept_async(stream).await.unwrap();
            ws.send(tungstenite::Message::Ping(b"server".to_vec()))
                .await
                .unwrap();

            match tokio::time::timeout(Duration::from_secs(1), ws.next())
                .await
                .unwrap()
                .unwrap()
                .unwrap()
            {
                tungstenite::Message::Pong(payload) => assert_eq!(payload, b"server"),
                other => panic!("Expected pong frame, got {other:?}"),
            }
        });

        let client = WsClient::new();
        client
            .connect(&format!("http://{}", addr), None)
            .await
            .unwrap();

        server.await.unwrap();
    }

    #[tokio::test]
    async fn test_ws_client_wait_for_message_timeout() {
        let client = WsClient::new();

        // Wait for a message that doesn't exist with short timeout
        let result = client
            .wait_for_message(|m| m.get("never").is_some(), 100)
            .await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_ws_client_wait_for_message_found() {
        let client = WsClient::new();

        // Add a message in a background task after a delay
        let messages = client.messages.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(50)).await;
            messages.lock().await.push(sonic_rs::json!({"found": true}));
        });

        let result = client
            .wait_for_message(|m| m.get("found").is_some(), 500)
            .await;
        assert!(result.is_some());
        assert_eq!(result.unwrap()["found"], true);
    }

    #[test]
    fn test_ws_message_subscribe_serialize() {
        let msg = WsMessage::Subscribe {
            channel: "orders".to_string(),
            symbols: None,
            expiry: None,
            option_type: None,
        };
        let json = sonic_rs::to_string(&msg).unwrap();
        assert!(json.contains("Subscribe"));
        assert!(json.contains("orders"));
    }

    #[test]
    fn test_ws_message_unsubscribe_serialize() {
        let msg = WsMessage::Unsubscribe {
            channel: "trades".to_string(),
            symbols: None,
            expiry: None,
            option_type: None,
        };
        let json = sonic_rs::to_string(&msg).unwrap();
        assert!(json.contains("Unsubscribe"));
        assert!(json.contains("trades"));
    }

    #[test]
    fn test_ws_message_subscribed_deserialize() {
        let json = r#"{"type": "Subscribed", "channel": "orders"}"#;
        let msg: WsMessage = sonic_rs::from_str(json).unwrap();
        match msg {
            WsMessage::Subscribed { channel } => assert_eq!(channel, "orders"),
            _ => panic!("Expected Subscribed variant"),
        }
    }

    #[test]
    fn test_ws_message_unsubscribed_deserialize() {
        let json = r#"{"type": "Unsubscribed", "channel": "trades"}"#;
        let msg: WsMessage = sonic_rs::from_str(json).unwrap();
        match msg {
            WsMessage::Unsubscribed { channel } => assert_eq!(channel, "trades"),
            _ => panic!("Expected Unsubscribed variant"),
        }
    }

    #[test]
    fn test_ws_message_error_deserialize() {
        let json = r#"{"type": "Error", "message": "invalid channel"}"#;
        let msg: WsMessage = sonic_rs::from_str(json).unwrap();
        match msg {
            WsMessage::Error { message } => assert_eq!(message, "invalid channel"),
            _ => panic!("Expected Error variant"),
        }
    }

    #[test]
    fn test_ws_message_unknown_type_deserialize() {
        let json = r#"{"type": "unknown_type", "data": "something"}"#;
        let msg: WsMessage = sonic_rs::from_str(json).unwrap();
        match msg {
            WsMessage::Other => {}
            _ => panic!("Expected Other variant"),
        }
    }

    #[test]
    fn test_ws_message_debug() {
        let msg = WsMessage::Subscribe {
            channel: "orders".to_string(),
            symbols: None,
            expiry: None,
            option_type: None,
        };
        let debug = format!("{:?}", msg);
        assert!(debug.contains("Subscribe"));
        assert!(debug.contains("orders"));
    }

    #[test]
    fn test_ws_message_clone() {
        let msg = WsMessage::Error {
            message: "test error".to_string(),
        };
        let cloned = msg.clone();
        match cloned {
            WsMessage::Error { message } => assert_eq!(message, "test error"),
            _ => panic!("Clone failed"),
        }
    }
}
