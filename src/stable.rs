use crate::errors::Errors;
use crate::Bytes;
use anyhow::Result;

/// `StableStore` is used to provide stable storage
/// of key configurations to ensure safety.
pub trait StableStore {
    fn set(&self, key: Bytes, val: Bytes) -> Result<()>;

    /// `get` returns the value for key, or an empty byte slice if key was not found
    fn get(&self, key: Bytes) -> Result<Bytes, Errors>;

    fn set_u64(&self, key: Bytes, val: u64) -> Result<()>;

    /// `get_u64` returns the u64 value for key, or 0 if key was not found.
    fn get_u64(&self, key: Bytes) -> Result<u64, Errors>;
}
