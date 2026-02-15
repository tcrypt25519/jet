use inkwell::execution_engine::ExecutionEngine;
use log::trace;

use crate::{
    ADDRESS_SIZE_BYTES,
    exec::{Context, ContractFunc, ReturnCode, jet_contract_fn_lookup},
};

// Contract calls
//

/// Error codes returned by jet_contract_call
#[repr(i8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ContractCallError {
    Success = 0,
    LookupFailed = 1,
    InvocationFailed = 2,
    CopyFailed = 3,
    InvalidJitEngine = -1,
    InvalidCtx = -2,
    InvalidPointer = -3,
    SubCtxCreationFailed = -4,
}

/// Calls the contract at the given address.
///
/// # Safety
///
/// This function is unsafe because it dereferences the given pointers. The caller must ensure that
/// all the pointers are valid.
///
/// # Returns
///
/// - `0`: Success
/// - `1`: Contract lookup failed
/// - `2`: Contract invocation failed  
/// - `3`: Return data copy failed
/// - `-1`: Invalid JIT engine pointer
/// - `-2`: Invalid context pointer
/// - `-3`: Invalid addr, ret_dest, or ret_len pointer
/// - `-4`: Sub-context creation failed
pub unsafe extern "C" fn jet_contract_call(
    ctx: *mut Context,
    jit_engine: *const ExecutionEngine,
    addr: *const u8,
    ret_dest: *const u32,
    ret_len: *const u32,
) -> i8 {
    // Validate all input pointers
    let jit_engine = match unsafe { jit_engine.as_ref() } {
        Some(engine) => engine,
        None => return ContractCallError::InvalidJitEngine as i8,
    };

    if addr.is_null() || ret_dest.is_null() || ret_len.is_null() {
        return ContractCallError::InvalidPointer as i8;
    }

    let addr_slice = unsafe { std::slice::from_raw_parts(addr, ADDRESS_SIZE_BYTES) };
    let fn_ptr = jet_contract_fn_lookup(jit_engine, addr_slice);
    if fn_ptr == 0 {
        return ContractCallError::LookupFailed as i8;
    }

    // Instantiate a sub context
    let caller_ctx = match unsafe { ctx.as_mut() } {
        Some(ctx) => ctx,
        None => return ContractCallError::InvalidCtx as i8,
    };

    let callee_ctx = match caller_ctx.init_sub_call() {
        Ok(ctx) => ctx,
        Err(e) => {
            log::error!("Failed to create sub-context: {}", e);
            return ContractCallError::SubCtxCreationFailed as i8;
        }
    };

    // Execute the contract function
    let contract_func: ContractFunc = unsafe { std::mem::transmute(fn_ptr) };
    let result = unsafe { contract_func(callee_ctx) };
    if result != ReturnCode::ExplicitReturn && result != ReturnCode::ImplicitReturn {
        return ContractCallError::InvocationFailed as i8;
    }

    // Copy return data
    if callee_ctx.return_len() == 0 {
        return ContractCallError::Success as i8;
    }

    let ret_dest = unsafe { *ret_dest };
    let ret_len = unsafe { *ret_len };
    let copy_result = unsafe { return_data_copy_impl(ctx, callee_ctx, ret_dest, 0, ret_len) };

    match copy_result {
        CopyError::Success => ContractCallError::Success as i8,
        err => {
            log::error!("Return data copy failed: {:?}", err);
            ContractCallError::CopyFailed as i8
        }
    }
}

/// Error codes returned by jet_contract_call_return_data_copy
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CopyError {
    Success = 0,
    InvalidPtr = 1,
    BoundsCheckFailed = 2,
    ArithmeticOverflow = 3,
    MemoryExpansionNeeded = 4,
}

/// Copies return data from the sub context to the parent context.
///
/// # Safety
///
/// This function is unsafe because it dereferences the given pointers. The caller must ensure
/// that all the pointers are valid.
///
/// # Returns
///
/// - `0`: Success
/// - `1`: Invalid context or sub-context pointer
/// - `2`: Bounds check failed
/// - `3`: Arithmetic overflow in offset calculations
/// - `4`: Memory expansion needed but not implemented
pub unsafe extern "C" fn jet_contract_call_return_data_copy(
    ctx: *mut Context,
    sub_ctx: *const Context,
    dest_offset: u32,
    src_offset: u32,
    requested_ret_len: u32,
) -> u8 {
    unsafe { return_data_copy_impl(ctx, sub_ctx, dest_offset, src_offset, requested_ret_len) as u8 }
}

unsafe fn return_data_copy_impl(
    ctx: *mut Context,
    sub_ctx: *const Context,
    dest_offset: u32,
    src_offset: u32,
    requested_ret_len: u32,
) -> CopyError {
    let ctx = match unsafe { ctx.as_mut() } {
        Some(ctx) => ctx,
        None => return CopyError::InvalidPtr,
    };
    let sub_ctx = match unsafe { sub_ctx.as_ref() } {
        Some(ctx) => ctx,
        None => return CopyError::InvalidPtr,
    };

    // Get return data from the callee
    let ret_len = sub_ctx.return_len();

    trace!(
        "jet_contracts_call_return_data_copy:\ndest_offset: {}\nrequested_ret_len: {}\n\nret_len: {}",
        dest_offset, requested_ret_len, ret_len
    );

    // TODO: Validate ret_offset + ret_len <= memory_len once memory_len is tracked by MSTORE

    // Bounds check: validate src_offset + requested_ret_len doesn't overflow and is within ret_len
    let src_end = match src_offset.checked_add(requested_ret_len) {
        Some(end) => end,
        None => return CopyError::ArithmeticOverflow,
    };
    if src_end > ret_len {
        return CopyError::BoundsCheckFailed;
    }

    // Validate destination range doesn't overflow
    let required_memory_len = match dest_offset.checked_add(requested_ret_len) {
        Some(len) => len,
        None => return CopyError::ArithmeticOverflow,
    };

    // Ensure memory is large enough for the write
    if ctx.memory_len() < required_memory_len {
        if required_memory_len > ctx.memory_cap() {
            // TODO: Expand memory capacity
            return CopyError::MemoryExpansionNeeded;
        }
        // Expand memory length to accommodate the write
        ctx.memory_len = required_memory_len;
    }

    // Copy the data - all bounds have been validated
    let src_start = src_offset as usize;
    let src_range = src_start..src_end as usize;
    let dest_start = dest_offset as usize;
    let dest_range = dest_start..required_memory_len as usize;
    let dest = &mut ctx.memory_mut()[dest_range];
    dest.copy_from_slice(&sub_ctx.return_data()[src_range]);
    CopyError::Success
}

//  Utils
//

pub extern "C" fn jet_ops_exp(base: &mut [u8; 32], exp: &[u8; 32]) -> i8 {
    // Stack words are stored little-endian (PUSH immediates are reversed on load).
    use bnum::types::U256;
    let read = |b: &[u8; 32]| {
        U256::from_digits([
            u64::from_le_bytes(b[0..8].try_into().unwrap()),
            u64::from_le_bytes(b[8..16].try_into().unwrap()),
            u64::from_le_bytes(b[16..24].try_into().unwrap()),
            u64::from_le_bytes(b[24..32].try_into().unwrap()),
        ])
    };

    let mut b = read(base);
    let mut e = read(exp);
    let mut result = U256::ONE;

    while e != U256::ZERO {
        if e & U256::ONE != U256::ZERO {
            result = result.wrapping_mul(b);
        }
        b = b.wrapping_mul(b);
        e >>= 1u32;
    }

    let d = result.digits();
    base[0..8].copy_from_slice(&d[0].to_le_bytes());
    base[8..16].copy_from_slice(&d[1].to_le_bytes());
    base[16..24].copy_from_slice(&d[2].to_le_bytes());
    base[24..32].copy_from_slice(&d[3].to_le_bytes());
    0
}

// Helper functions for U256/U512 conversions shared by ADDMOD and MULMOD
fn read_u256(bytes: &[u8; 32]) -> bnum::types::U256 {
    bnum::types::U256::from_digits([
        u64::from_le_bytes(bytes[0..8].try_into().unwrap()),
        u64::from_le_bytes(bytes[8..16].try_into().unwrap()),
        u64::from_le_bytes(bytes[16..24].try_into().unwrap()),
        u64::from_le_bytes(bytes[24..32].try_into().unwrap()),
    ])
}

fn write_u256(bytes: &mut [u8; 32], val: bnum::types::U256) {
    let d = val.digits();
    bytes[0..8].copy_from_slice(&d[0].to_le_bytes());
    bytes[8..16].copy_from_slice(&d[1].to_le_bytes());
    bytes[16..24].copy_from_slice(&d[2].to_le_bytes());
    bytes[24..32].copy_from_slice(&d[3].to_le_bytes());
}

fn u256_to_u512(val: bnum::types::U256) -> bnum::types::U512 {
    let d = val.digits();
    bnum::types::U512::from_digits([d[0], d[1], d[2], d[3], 0, 0, 0, 0])
}

/// ADDMOD with 512-bit precision as required by EVM spec.
/// Computes (a + b) % n using 512-bit intermediate arithmetic to prevent overflow.
/// Note: result and a may point to the same buffer, but this is safe because
/// all inputs are read into local variables before result is written.
pub extern "C" fn jet_ops_addmod(
    result: &mut [u8; 32],
    a: &[u8; 32],
    b: &[u8; 32],
    n: &[u8; 32],
) -> i8 {
    // Read all inputs into local variables before writing to result
    let a_u256 = read_u256(a);
    let b_u256 = read_u256(b);
    let n_u256 = read_u256(n);

    // EVM spec: if n == 0, return 0
    if n_u256 == bnum::types::U256::ZERO {
        write_u256(result, bnum::types::U256::ZERO);
        return 0;
    }

    // Convert to 512-bit for addition without overflow
    let a_u512 = u256_to_u512(a_u256);
    let b_u512 = u256_to_u512(b_u256);
    let n_u512 = u256_to_u512(n_u256);

    // Perform addition in 512-bit
    let sum = a_u512 + b_u512;

    // Modulo operation
    let mod_result = sum % n_u512;

    // Convert back to 256-bit by taking lower 256 bits
    // This is safe because mod_result < n < 2^256
    let digits = mod_result.digits();
    let result_u256 = bnum::types::U256::from_digits([digits[0], digits[1], digits[2], digits[3]]);
    write_u256(result, result_u256);

    0
}

/// MULMOD with 512-bit precision as required by EVM spec.
/// Computes (a * b) % n using 512-bit intermediate arithmetic to prevent overflow.
/// Note: result and a may point to the same buffer, but this is safe because
/// all inputs are read into local variables before result is written.
pub extern "C" fn jet_ops_mulmod(
    result: &mut [u8; 32],
    a: &[u8; 32],
    b: &[u8; 32],
    n: &[u8; 32],
) -> i8 {
    // Read all inputs into local variables before writing to result
    let a_u256 = read_u256(a);
    let b_u256 = read_u256(b);
    let n_u256 = read_u256(n);

    // EVM spec: if n == 0, return 0
    if n_u256 == bnum::types::U256::ZERO {
        write_u256(result, bnum::types::U256::ZERO);
        return 0;
    }

    // Convert to 512-bit for multiplication without overflow
    let a_u512 = u256_to_u512(a_u256);
    let b_u512 = u256_to_u512(b_u256);
    let n_u512 = u256_to_u512(n_u256);

    // Perform multiplication in 512-bit
    let product = a_u512 * b_u512;

    // Modulo operation
    let mod_result = product % n_u512;

    // Convert back to 256-bit by taking lower 256 bits
    // This is safe because mod_result < n < 2^256
    let digits = mod_result.digits();
    let result_u256 = bnum::types::U256::from_digits([digits[0], digits[1], digits[2], digits[3]]);
    write_u256(result, result_u256);

    0
}

/// Compute Keccak-256 of `size` bytes at `offset` in the execution context's memory.
///
/// The result is written to `result`.  Memory must already be expanded to cover
/// `[offset, offset + size)` by the caller (i.e. `jet_mem_expand` must have been
/// called before this function).
///
/// Returns 0 on success, -1 if `ctx` is null or if the memory range is invalid.
///
/// # Safety
///
/// `ctx` must be a valid pointer if non-null (null check is performed and handled).
/// `result` must point to a valid, non-null 32-byte aligned output buffer.
pub unsafe extern "C" fn jet_ops_keccak256(
    ctx: *mut Context,
    offset: u32,
    size: u32,
    result: *mut [u8; 32],
) -> i8 {
    use sha3::{Digest, Keccak256};
    
    // Check for null context pointer
    if ctx.is_null() {
        return -1;
    }

    // SAFETY: ctx is non-null after check above
    let (memory_ptr, memory_len) = unsafe {
        let ctx_ref = &*ctx;
        (ctx_ref.memory_ptr, ctx_ref.memory_len as usize)
    };

    let start = offset as usize;
    let end = start.saturating_add(size as usize);

    // Guard: memory must cover the requested range (caller is responsible).
    if end > memory_len {
        return -1;
    }

    // SAFETY: memory range [start, end) is validated above
    let data = unsafe { std::slice::from_raw_parts(memory_ptr.add(start), size as usize) };

    let mut hasher = Keccak256::new();
    hasher.update(data);
    let hash = hasher.finalize();

    // SAFETY: result is a valid pointer per function contract
    let result_ref = unsafe { &mut *result };
    result_ref.copy_from_slice(&hash);
    0
}

/// Error codes for memory expansion
#[repr(i8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MemoryExpansionError {
    Success = 0,
    InvalidPointer = -1,
    ArithmeticOverflow = -2,
    AllocationFailed = -3,
}

/// Expands memory to accommodate an access at offset with given size.
/// Follows EVM semantics: rounds up to 32-byte boundaries and updates memory_len.
///
/// # Safety
///
/// This function is unsafe because it dereferences the given pointer and may reallocate memory.
/// The caller must ensure that the context pointer is valid.
///
/// # Returns
///
/// - `0`: Success
/// - `-1`: Invalid context pointer
/// - `-2`: Arithmetic overflow in offset + size
/// - `-3`: Memory allocation failed
pub unsafe extern "C" fn jet_mem_expand(ctx: *mut Context, offset: u32, size: u32) -> i8 {
    let ctx = match unsafe { ctx.as_mut() } {
        Some(ctx) => ctx,
        None => return MemoryExpansionError::InvalidPointer as i8,
    };

    // If size is 0, no expansion needed
    if size == 0 {
        return MemoryExpansionError::Success as i8;
    }

    // Check for arithmetic overflow
    let end_offset = match offset.checked_add(size) {
        Some(end) => end,
        None => return MemoryExpansionError::ArithmeticOverflow as i8,
    };

    // Round up to 32-byte boundary
    let required_len = end_offset.div_ceil(32) * 32;

    // If already large enough, we're done
    if required_len <= ctx.memory_len {
        return MemoryExpansionError::Success as i8;
    }

    // If we need to expand beyond capacity, reallocate
    if required_len > ctx.memory_cap {
        // Allocate new memory with 32-byte alignment
        let new_layout = match std::alloc::Layout::from_size_align(required_len as usize, 32) {
            Ok(layout) => layout,
            Err(_) => return MemoryExpansionError::AllocationFailed as i8,
        };

        let new_ptr = unsafe { std::alloc::alloc_zeroed(new_layout) };
        if new_ptr.is_null() {
            return MemoryExpansionError::AllocationFailed as i8;
        }

        // Copy old data to new allocation
        if ctx.memory_len > 0 {
            unsafe {
                std::ptr::copy_nonoverlapping(ctx.memory_ptr, new_ptr, ctx.memory_len as usize);
            }
        }

        // Free old allocation
        if !ctx.memory_ptr.is_null() && ctx.memory_cap > 0 {
            let old_layout = std::alloc::Layout::from_size_align(ctx.memory_cap as usize, 32)
                .expect("old layout should be valid");
            unsafe {
                std::alloc::dealloc(ctx.memory_ptr, old_layout);
            }
        }

        // Update context
        ctx.memory_ptr = new_ptr;
        ctx.memory_cap = required_len;
    }

    // Update memory length (expansion is always monotonic)
    ctx.memory_len = required_len;

    MemoryExpansionError::Success as i8
}
