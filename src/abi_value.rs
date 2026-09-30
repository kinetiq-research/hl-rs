use alloy::{
    dyn_abi::{DynSolType, DynSolValue},
    primitives::{keccak256, Address, U256},
};
use rust_decimal::Decimal;

use crate::Error;

/// Result type for ABI value conversions.
pub type AbiResult = Result<DynSolValue, Error>;

// ============================================================================
// ToAbiValue trait - converts Rust types to EIP-712 ABI values
// ============================================================================

/// Trait for converting Rust types to EIP-712 ABI values.
///
/// The `abi_type` parameter is a [`DynSolType`] that determines how the value
/// should be encoded for EIP-712 struct hashing.
///
/// This allows a single Rust type to support multiple EIP-712 encodings. For example,
/// `Address` can encode as both `DynSolType::Address` (20-byte address) and
/// `DynSolType::String` (keccak of lowercase hex representation).
///
/// # Errors
/// Returns an error if the Rust type cannot be encoded as the requested ABI type.
pub trait ToAbiValue {
    fn to_abi_value(&self, abi_type: &DynSolType) -> AbiResult;
}

// --- Address ---

impl ToAbiValue for Address {
    fn to_abi_value(&self, abi_type: &DynSolType) -> AbiResult {
        match abi_type {
            DynSolType::Address => Ok(DynSolValue::Address(*self)),
            // EIP-712 string encoding: keccak256 of lowercase hex representation
            DynSolType::String => Ok(DynSolValue::FixedBytes(
                keccak256(self.to_string().to_lowercase()),
                32,
            )),
            _ => Err(Error::AbiEncode {
                rust_type: "Address",
                abi_type: format!("{:?}", abi_type),
            }),
        }
    }
}

// --- String types ---

impl ToAbiValue for String {
    fn to_abi_value(&self, abi_type: &DynSolType) -> AbiResult {
        match abi_type {
            // EIP-712 string encoding: keccak256 of the string bytes
            DynSolType::String => Ok(DynSolValue::FixedBytes(keccak256(self.as_bytes()), 32)),
            _ => Err(Error::AbiEncode {
                rust_type: "String",
                abi_type: format!("{:?}", abi_type),
            }),
        }
    }
}

impl ToAbiValue for str {
    fn to_abi_value(&self, abi_type: &DynSolType) -> AbiResult {
        match abi_type {
            DynSolType::String => Ok(DynSolValue::FixedBytes(keccak256(self.as_bytes()), 32)),
            _ => Err(Error::AbiEncode {
                rust_type: "str",
                abi_type: format!("{:?}", abi_type),
            }),
        }
    }
}

// --- Decimal (encoded as string) ---

impl ToAbiValue for Decimal {
    fn to_abi_value(&self, abi_type: &DynSolType) -> AbiResult {
        match abi_type {
            // Normalize the decimal string so equivalent values with different
            // scales (for example 1.0 vs 1) hash identically for EIP-712.
            DynSolType::String => Ok(DynSolValue::FixedBytes(keccak256(self.normalize().to_string()), 32)),
            _ => Err(Error::AbiEncode {
                rust_type: "Decimal",
                abi_type: format!("{:?}", abi_type),
            }),
        }
    }
}

// --- Boolean ---

impl ToAbiValue for bool {
    fn to_abi_value(&self, abi_type: &DynSolType) -> AbiResult {
        match abi_type {
            DynSolType::Bool => Ok(DynSolValue::Bool(*self)),
            _ => Err(Error::AbiEncode {
                rust_type: "bool",
                abi_type: format!("{:?}", abi_type),
            }),
        }
    }
}

// --- Numeric types ---

impl ToAbiValue for u64 {
    fn to_abi_value(&self, abi_type: &DynSolType) -> AbiResult {
        match abi_type {
            DynSolType::Uint(bits) => Ok(DynSolValue::Uint(U256::from(*self), *bits)),
            _ => Err(Error::AbiEncode {
                rust_type: "u64",
                abi_type: format!("{:?}", abi_type),
            }),
        }
    }
}

impl ToAbiValue for u32 {
    fn to_abi_value(&self, abi_type: &DynSolType) -> AbiResult {
        match abi_type {
            DynSolType::Uint(bits) => Ok(DynSolValue::Uint(U256::from(*self), *bits)),
            _ => Err(Error::AbiEncode {
                rust_type: "u32",
                abi_type: format!("{:?}", abi_type),
            }),
        }
    }
}

impl ToAbiValue for u128 {
    fn to_abi_value(&self, abi_type: &DynSolType) -> AbiResult {
        match abi_type {
            DynSolType::Uint(bits) => Ok(DynSolValue::Uint(U256::from(*self), *bits)),
            _ => Err(Error::AbiEncode {
                rust_type: "u128",
                abi_type: format!("{:?}", abi_type),
            }),
        }
    }
}

impl ToAbiValue for U256 {
    fn to_abi_value(&self, abi_type: &DynSolType) -> AbiResult {
        match abi_type {
            DynSolType::Uint(bits) => Ok(DynSolValue::Uint(*self, *bits)),
            _ => Err(Error::AbiEncode {
                rust_type: "U256",
                abi_type: format!("{:?}", abi_type),
            }),
        }
    }
}

// --- Option<T> ---
// For optional fields, None is encoded using a type-specific default:
// - string: empty string (keccak256(""))
// - address: zero address
// - uint*: 0
// - bool: false

impl<T: ToAbiValue + Default> ToAbiValue for Option<T> {
    fn to_abi_value(&self, abi_type: &DynSolType) -> AbiResult {
        match self {
            Some(inner) => inner.to_abi_value(abi_type),
            None => T::default().to_abi_value(abi_type),
        }
    }
}
