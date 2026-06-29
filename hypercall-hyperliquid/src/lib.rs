//! Direct Hyperliquid perp venue implementation.
//!
//! This crate implements a direct perp venue using Hyperliquid's native
//! exchange API through `hypersdk`.

mod perp_venue;
mod registry;
mod venue;

pub use hypersdk::hypercore::Chain as HyperliquidChain;
pub use hypersdk::Address as HyperliquidAddress;

pub use perp_venue::{
    PerpVenue, PerpVenueCancelByClientIdRequest, PerpVenueCancelByOidRequest, PerpVenueFuture,
    PerpVenueOrderRequest, Tif,
};
pub use registry::{HyperliquidPerpAsset, HyperliquidPerpAssetRegistry};
pub use venue::{DirectHyperliquidPerpVenue, HyperliquidOrderResponseStatus, HyperliquidSigner};

fn hyperliquid_client_with_base_url(
    chain: HyperliquidChain,
    base_url: &str,
) -> hypercall_client::error::Result<hypersdk::hypercore::HttpClient> {
    let url = base_url.parse::<url::Url>().map_err(|error| {
        hypercall_client::ClientError::InvalidInput(format!(
            "invalid Hyperliquid base URL '{}': {}",
            base_url, error
        ))
    })?;
    Ok(hypersdk::hypercore::HttpClient::new(chain).with_url(url))
}
