use inkwell::execution_engine::ExecutionEngine;
use log::trace;

use crate::{
    ADDRESS_SIZE_BYTES,
    exec::{Context, ContractFunc, ReturnCode, jet_contract_fn_lookup},
};

// Contract calls
//

/// Calls the contract at the given address.
///
/// # Safety
///
/// This function is unsafe because it dereferences the given pointers. The caller must ensure that
/// all the pointers are valid.
pub unsafe extern "C" fn jet_contract_call(
    ctx: *mut Context,
    jit_engine: *const ExecutionEngine,
    addr: *const u8,
    ret_dest: *const u32,
    ret_len: *const u32,
) -> i8 {
    // Look up the contract function
    let jit_engine = unsafe { jit_engine.as_ref() }.unwrap();
    let addr_slice = unsafe { std::slice::from_raw_parts(addr, ADDRESS_SIZE_BYTES) };
    let fn_ptr = jet_contract_fn_lookup(jit_engine, addr_slice);
    if fn_ptr == 0 {
        return 1; // Lookup failed
    }

    // Instantiate a sub context
    let caller_ctx = unsafe { ctx.as_mut() }.unwrap();
    let callee_ctx = caller_ctx.init_sub_call();

    // let callee_ctx = caller_ctx.sub_ctx_mut().unwrap();
    // let callee_ctx_ptr = callee_ctx as *mut Context;
    // caller_ctx.set_sub_call(callee_ctx_ptr as usize);

    // Execute the contract function
    let contract_func: ContractFunc = unsafe { std::mem::transmute(fn_ptr) };
    let result = unsafe { contract_func(callee_ctx) };
    if result != ReturnCode::ExplicitReturn && result != ReturnCode::ImplicitReturn {
        return 2; // Invocation failed
    }

    // Copy return data
    if callee_ctx.return_len() == 0 {
        return 0; // Success, but no return data
    }

    let ret_dest = unsafe { *ret_dest };
    let ret_len = unsafe { *ret_len };
    let copy_ret = jet_contract_call_return_data_copy(ctx, callee_ctx, ret_dest, 0, ret_len);
    copy_ret as i8
}

/// Copies return data from the sub context to the parent context.
///
/// # Safety
///
/// This function is unsafe because it dereferences the given pointers. The caller must ensure
/// that all the pointers are valid.
pub unsafe extern "C" fn jet_contract_call_return_data_copy(
    ctx: *mut Context,
    sub_ctx: *const Context,
    dest_offset: u32,
    src_offset: u32,
    requested_ret_len: u32,
) -> u8 {
    let ctx = unsafe { ctx.as_mut() }.unwrap();
    let sub_ctx = unsafe { sub_ctx.as_ref() }.unwrap();

    // Get return and memory data from the callee
    let ret_offset = sub_ctx.return_off();
    let ret_len = sub_ctx.return_len();
    let mem_len = sub_ctx.memory_len();

    trace!("jet_contracts_call_return_data_copy:\ndest_offset: {}\nrequested_ret_len: {}\n\nret_offset: {}\nret_len: {}\nmem_len: {}", dest_offset, requested_ret_len, ret_offset, ret_len, mem_len);

    // Bounds checks for the memory and return data
    if src_offset + requested_ret_len > ret_len {
        return 3;
    }
    let ret_offset_end = ret_offset + requested_ret_len;
    if ret_offset_end > ret_len {
        return 4;
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
