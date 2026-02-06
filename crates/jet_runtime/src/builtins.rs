use inkwell::execution_engine::ExecutionEngine;
use log::trace;

use crate::{
    exec::{jet_contract_fn_lookup, Context, ContractFunc, ReturnCode},
    ADDRESS_SIZE_BYTES,
};

// Contract calls
//

/// Error codes returned by jet_contract_call
const ERR_SUCCESS: i8 = 0;
const ERR_LOOKUP_FAILED: i8 = 1;
const ERR_INVOCATION_FAILED: i8 = 2;
const ERR_COPY_FAILED: i8 = 3;
const ERR_INVALID_JIT_ENGINE: i8 = -1;
const ERR_INVALID_CTX: i8 = -1;
const ERR_INVALID_POINTER: i8 = -3;
const ERR_SUB_CTX_CREATION_FAILED: i8 = -2;

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
/// - `-1`: Invalid JIT engine or context pointer
/// - `-2`: Sub-context creation failed
/// - `-3`: Invalid addr, ret_dest, or ret_len pointer
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
        None => return ERR_INVALID_JIT_ENGINE,
    };
    
    if addr.is_null() || ret_dest.is_null() || ret_len.is_null() {
        return ERR_INVALID_POINTER;
    }
    
    let addr_slice = unsafe { std::slice::from_raw_parts(addr, ADDRESS_SIZE_BYTES) };
    let fn_ptr = jet_contract_fn_lookup(jit_engine, addr_slice);
    if fn_ptr == 0 {
        return ERR_LOOKUP_FAILED;
    }

    // Instantiate a sub context
    let caller_ctx = match unsafe { ctx.as_mut() } {
        Some(ctx) => ctx,
        None => return ERR_INVALID_CTX,
    };
    
    let callee_ctx = match caller_ctx.init_sub_call() {
        Ok(ctx) => ctx,
        Err(e) => {
            log::error!("Failed to create sub-context: {}", e);
            return ERR_SUB_CTX_CREATION_FAILED;
        }
    };

    // Execute the contract function
    let contract_func: ContractFunc = unsafe { std::mem::transmute(fn_ptr) };
    let result = unsafe { contract_func(callee_ctx) };
    if result != ReturnCode::ExplicitReturn && result != ReturnCode::ImplicitReturn {
        return ERR_INVOCATION_FAILED;
    }

    // Copy return data
    if callee_ctx.return_len() == 0 {
        return ERR_SUCCESS;
    }

    let ret_dest = unsafe { *ret_dest };
    let ret_len = unsafe { *ret_len };
    let copy_result = jet_contract_call_return_data_copy(ctx, callee_ctx, ret_dest, 0, ret_len);
    
    // Map copy result to documented error codes
    match copy_result {
        0 => ERR_SUCCESS,
        _ => ERR_COPY_FAILED,
    }
}

/// Error codes returned by jet_contract_call_return_data_copy
const ERR_COPY_INVALID_PTR: u8 = 1;
const ERR_COPY_BOUNDS: u8 = 2;

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
pub unsafe extern "C" fn jet_contract_call_return_data_copy(
    ctx: *mut Context,
    sub_ctx: *const Context,
    dest_offset: u32,
    src_offset: u32,
    requested_ret_len: u32,
) -> u8 {
    let ctx = match unsafe { ctx.as_mut() } {
        Some(ctx) => ctx,
        None => return ERR_COPY_INVALID_PTR,
    };
    let sub_ctx = match unsafe { sub_ctx.as_ref() } {
        Some(ctx) => ctx,
        None => return ERR_COPY_INVALID_PTR,
    };

    // Get return and memory data from the callee
    let ret_offset = sub_ctx.return_off();
    let ret_len = sub_ctx.return_len();
    let mem_len = sub_ctx.memory_len();

    trace!("jet_contracts_call_return_data_copy:\ndest_offset: {}\nrequested_ret_len: {}\n\nret_offset: {}\nret_len: {}\nmem_len: {}", dest_offset, requested_ret_len, ret_offset, ret_len, mem_len);

    // Bounds checks for the memory and return data
    if src_offset + requested_ret_len > ret_len {
        return ERR_COPY_BOUNDS;
    }
    let ret_offset_end = ret_offset + requested_ret_len;
    if ret_offset_end > ret_len {
        return ERR_COPY_BOUNDS;
    }
    // TODO: Enable this check after adding memory len handling
    // if ret_offset_end > mem_len {
    //     return 3;
    // }

    // Copy the data
    let src_range = src_offset as usize..(src_offset + requested_ret_len) as usize;
    let dest_range = dest_offset as usize..(dest_offset + requested_ret_len) as usize;
    let dest = &mut ctx.memory_mut()[dest_range];
    dest.copy_from_slice(&sub_ctx.return_data()[src_range]);
    0
}

//  Utils
//

pub extern "C" fn jet_ops_keccak256(buffer: &mut [u8; 32]) -> u8 {
    // Hash the bytes
    use sha3::{Digest, Keccak256};
    let mut hasher = Keccak256::new();
    hasher.update(*buffer);
    let hash = hasher.finalize();

    // Write the hash back to the buffer
    for i in 0..32 {
        buffer[i] = hash[i];
    }
    0
}
