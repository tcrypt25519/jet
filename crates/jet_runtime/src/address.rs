use std::{fmt, str::FromStr};

/// A 20-byte Ethereum address.
///
/// Wraps a `[u8; 20]` and provides a type-safe, semantically meaningful
/// representation for Ethereum addresses throughout the runtime.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct Address([u8; Self::LEN]);

impl Address {
    /// The zero address: `0x0000000000000000000000000000000000000000`.
    pub const ZERO: Self = Self([0u8; Self::LEN]);

    /// The number of bytes in an Ethereum address.
    pub const LEN: usize = 20;

    /// Number of hex characters needed to represent a full address (2 per byte).
    const HEX_LEN: usize = Self::LEN * 2;

    /// Creates an address from its raw bytes.
    pub fn new(bytes: [u8; Self::LEN]) -> Self {
        Self(bytes)
    }

    /// Returns the raw bytes of the address.
    pub fn as_bytes(&self) -> &[u8; Self::LEN] {
        &self.0
    }

    /// Returns a mutable reference to the raw bytes.
    pub fn as_bytes_mut(&mut self) -> &mut [u8; Self::LEN] {
        &mut self.0
    }
}

impl AsRef<[u8]> for Address {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl From<[u8; Address::LEN]> for Address {
    fn from(bytes: [u8; Address::LEN]) -> Self {
        Self(bytes)
    }
}

impl From<Address> for [u8; Address::LEN] {
    fn from(addr: Address) -> Self {
        addr.0
    }
}

impl fmt::Debug for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Address({})", self)
    }
}

impl fmt::Display for Address {
    /// Formats the address as a lowercase hex string with a `0x` prefix.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{}", hex::encode(self.0))
    }
}

/// Error returned when parsing an [`Address`] from a hex string fails.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddressParseError(String);

impl fmt::Display for AddressParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid Ethereum address: {}", self.0)
    }
}

impl std::error::Error for AddressParseError {}

impl FromStr for Address {
    type Err = AddressParseError;

    /// Parses an address from a hex string, optionally prefixed with `0x`.
    ///
    /// Short forms such as `"0x1"` are left-padded with zeros to 20 bytes,
    /// matching the EVM convention of right-aligned addresses.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let hex_str = s.strip_prefix("0x").unwrap_or(s);

        if hex_str.len() > Address::HEX_LEN {
            return Err(AddressParseError(format!(
                "expected at most {} hex chars, got {}",
                Address::HEX_LEN,
                hex_str.len()
            )));
        }

        // Left-pad with zeros to HEX_LEN chars using a stack buffer.
        let mut padded = [b'0'; Address::HEX_LEN];
        padded[Address::HEX_LEN - hex_str.len()..].copy_from_slice(hex_str.as_bytes());

        let mut arr = [0u8; Address::LEN];
        hex::decode_to_slice(padded, &mut arr).map_err(|e| AddressParseError(e.to_string()))?;
        Ok(Address(arr))
    }
}

impl<'a> TryFrom<&'a str> for Address {
    type Error = AddressParseError;

    fn try_from(s: &'a str) -> Result<Self, Self::Error> {
        s.parse()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_address() {
        assert_eq!(
            Address::ZERO.to_string(),
            "0x0000000000000000000000000000000000000000"
        );
    }

    #[test]
    fn from_bytes_roundtrip() {
        let bytes = [
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
        ];
        let addr = Address::new(bytes);
        assert_eq!(*addr.as_bytes(), bytes);
        assert_eq!(<[u8; Address::LEN]>::from(addr), bytes);
    }

    #[test]
    fn parse_full_address() {
        let s = "0x000102030405060708090a0b0c0d0e0f10111213";
        let addr: Address = s.parse().unwrap();
        assert_eq!(addr.to_string(), s);
    }

    #[test]
    fn parse_short_address_left_pads() {
        let addr: Address = "0x1234".parse().unwrap();
        assert_eq!(
            addr.to_string(),
            "0x0000000000000000000000000000000000001234"
        );
    }

    #[test]
    fn parse_without_prefix() {
        let addr: Address = "1234".parse().unwrap();
        assert_eq!(
            addr.to_string(),
            "0x0000000000000000000000000000000000001234"
        );
    }

    #[test]
    fn parse_too_long_fails() {
        let long = "0x".to_string() + &"a".repeat(Address::HEX_LEN + 2);
        assert!(long.parse::<Address>().is_err());
    }
}
