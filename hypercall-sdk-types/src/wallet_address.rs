//! Canonical Ethereum wallet address type for Hypercall.

use std::{cmp::Ordering, fmt, str::FromStr};

use alloy::hex::FromHexError;
use alloy::primitives::Address;
use serde::{de::Error as _, Deserialize, Deserializer, Serialize, Serializer};

/// Canonical Ethereum wallet address for Hypercall API requests and responses.
///
/// Serialized values are checksummed hex strings with a `0x` prefix.
#[derive(Clone, Copy, Eq, PartialEq, Hash, Debug, Default)]
pub struct WalletAddress(pub Address);

impl WalletAddress {
    /// Returns the inner alloy [`Address`].
    pub fn inner(&self) -> Address {
        self.0
    }

    /// Formats the address as a checksummed hex string with `0x` prefix.
    pub fn as_hex(&self) -> String {
        format!("{:#x}", self.0)
    }

    /// Returns the raw 20-byte slice.
    pub fn as_bytes(&self) -> &[u8; 20] {
        self.0.as_ref()
    }
}

impl fmt::Display for WalletAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:#x}", self.0)
    }
}

impl From<Address> for WalletAddress {
    fn from(addr: Address) -> Self {
        WalletAddress(addr)
    }
}

impl From<[u8; 20]> for WalletAddress {
    fn from(bytes: [u8; 20]) -> Self {
        WalletAddress(Address::from(bytes))
    }
}

impl FromStr for WalletAddress {
    type Err = FromHexError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let addr = Address::from_str(s)?;
        Ok(WalletAddress(addr))
    }
}

impl Serialize for WalletAddress {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.as_hex())
    }
}

impl<'de> Deserialize<'de> for WalletAddress {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        WalletAddress::from_str(&s).map_err(D::Error::custom)
    }
}

impl PartialOrd for WalletAddress {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for WalletAddress {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.as_slice().cmp(other.0.as_slice())
    }
}

// =============================================================================
// Test Helper - Single source of truth for test wallet addresses
// =============================================================================

/// Creates a deterministic test wallet address from an ID.
///
/// This is the recommended way to create wallet addresses in tests.
/// The address is constructed with zeros except for the last byte which is the ID.
///
/// # Example
/// ```
/// use hypercall_sdk_types::test_wallet;
///
/// let wallet = test_wallet(1);
/// assert_eq!(wallet.as_hex(), "0x0000000000000000000000000000000000000001");
/// ```
#[cfg(any(test, feature = "test-utils"))]
pub fn test_wallet(id: u8) -> WalletAddress {
    let mut bytes = [0u8; 20];
    bytes[19] = id;
    WalletAddress::from(bytes)
}

/// Macro for creating test wallet addresses.
///
/// This provides a convenient way to create test addresses inline.
///
/// # Example
/// ```ignore
/// let wallet = test_wallet!(42);
/// ```
#[cfg(any(test, feature = "test-utils"))]
#[macro_export]
macro_rules! test_wallet {
    ($id:expr) => {{
        let mut bytes = [0u8; 20];
        bytes[19] = $id;
        $crate::wallet_address::WalletAddress::from(alloy::primitives::Address::from(bytes))
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wallet_address_roundtrip() {
        let addr_str = "0x1234567890abcdef1234567890abcdef12345678";
        let wallet = WalletAddress::from_str(addr_str).unwrap();
        assert_eq!(wallet.as_hex().to_lowercase(), addr_str.to_lowercase());
    }

    #[test]
    fn test_wallet_address_ordering() {
        let a = WalletAddress::from_str("0x0000000000000000000000000000000000000001").unwrap();
        let b = WalletAddress::from_str("0x0000000000000000000000000000000000000002").unwrap();
        assert!(a < b);
    }

    #[test]
    fn test_serde_roundtrip() {
        let addr_str = "0x1234567890abcdef1234567890abcdef12345678";
        let wallet = WalletAddress::from_str(addr_str).unwrap();
        let json = sonic_rs::to_string(&wallet).unwrap();
        let parsed: WalletAddress = sonic_rs::from_str(&json).unwrap();
        assert_eq!(wallet, parsed);
    }

    #[test]
    fn test_test_wallet_helper() {
        let w1 = test_wallet(1);
        let w2 = test_wallet(2);
        let w1_again = test_wallet(1);

        assert_ne!(w1, w2);
        assert_eq!(w1, w1_again);
        assert!(w1 < w2);

        // Verify the address format
        assert_eq!(w1.as_hex(), "0x0000000000000000000000000000000000000001");
        assert_eq!(w2.as_hex(), "0x0000000000000000000000000000000000000002");
    }
}
