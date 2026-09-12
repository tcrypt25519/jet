use std::alloc::{Layout, alloc, dealloc};

use crate::{
    Address,
    error::{Result, RuntimeError},
    exec::Word,
};

/// Immutable EVM call data associated with one execution frame.
///
/// The field order is chosen to avoid padding with `#[repr(C)]` so that the
/// in-memory layout matches the packed LLVM `call_info` type generated in
/// [`jet_ir::Types`].  The `Context` stores a pointer to this structure.
#[repr(C)]
pub struct CallInfo {
    calldata_ptr: *mut u8,
    calldata_len: u32,
    address: Address,
    origin: Address,
    caller: Address,
    value: Word,
}

impl CallInfo {
    /// Creates call information and copies the supplied calldata.
    pub fn new(
        address: Address,
        origin: Address,
        caller: Address,
        value: Word,
        calldata: &[u8],
    ) -> Result<Self> {
        let calldata_len = u32::try_from(calldata.len())
            .map_err(|_| RuntimeError::MemoryLayout("calldata exceeds u32::MAX".to_string()))?;
        let calldata_ptr = if calldata.is_empty() {
            std::ptr::null_mut()
        } else {
            let layout = Layout::array::<u8>(calldata.len())
                .map_err(|e| RuntimeError::MemoryLayout(e.to_string()))?;
            let ptr = unsafe { alloc(layout) };
            if ptr.is_null() {
                return Err(RuntimeError::MemoryAllocation(
                    "Failed to allocate calldata".to_string(),
                ));
            }
            unsafe { std::ptr::copy_nonoverlapping(calldata.as_ptr(), ptr, calldata.len()) };
            ptr
        };
        Ok(Self {
            calldata_ptr,
            calldata_len,
            address,
            origin,
            caller,
            value,
        })
    }

    /// Returns the executing contract address.
    pub fn address(&self) -> Address {
        self.address
    }
    /// Returns the transaction origin.
    pub fn origin(&self) -> Address {
        self.origin
    }
    /// Returns the immediate caller.
    pub fn caller(&self) -> Address {
        self.caller
    }
    /// Returns the call value.
    pub fn value(&self) -> &Word {
        &self.value
    }
    /// Returns the calldata length.
    pub fn calldata_len(&self) -> u32 {
        self.calldata_len
    }
    /// Returns the calldata.
    pub fn calldata(&self) -> &[u8] {
        if self.calldata_len == 0 {
            return &[];
        }
        unsafe { std::slice::from_raw_parts(self.calldata_ptr, self.calldata_len as usize) }
    }
}

impl Drop for CallInfo {
    fn drop(&mut self) {
        if self.calldata_len == 0 {
            return;
        }
        if let Ok(layout) = Layout::array::<u8>(self.calldata_len as usize) {
            unsafe { dealloc(self.calldata_ptr, layout) };
        }
    }
}
