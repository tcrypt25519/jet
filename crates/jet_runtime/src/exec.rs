use inkwell::execution_engine::ExecutionEngine;
use log::error;

use crate::{symbols::FN_CONTRACT_PREFIX, *};

pub type Word = [u8; 32];
pub type Hash = [u8; 32];
type Address = [u8; ADDRESS_SIZE_BYTES];
pub type HashHistory = [Hash; BLOCK_HASH_HISTORY_SIZE];

pub type ContractFunc = unsafe extern "C" fn(*const Context) -> ReturnCode;

#[repr(C)]
pub struct Context {
    stack_ptr: u32,
    jump_ptr: u32,

    return_off: u32,
    return_len: u32,

    sub_call: Option<Box<Context>>,
    stack: [Word; STACK_SIZE_WORDS as usize],

    // Changed to pointer-based memory layout as per layout-mismatch-analysis.md
    pub(crate) memory_ptr: *mut u8,
    pub(crate) memory_len: u32,
    pub(crate) memory_cap: u32,
}

impl Context {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        // Allocate memory buffer on the heap
        let memory_size = (WORD_SIZE_BYTES * MEMORY_INITIAL_SIZE_WORDS) as usize;
        let memory_layout = std::alloc::Layout::from_size_align(memory_size, 32)
            .expect("Failed to create memory layout");
        let memory_ptr = unsafe { std::alloc::alloc_zeroed(memory_layout) };
        
        if memory_ptr.is_null() {
            panic!("Failed to allocate memory for EVM context");
        }

        Context {
            stack_ptr: 0,
            jump_ptr: 0,
            return_off: 0,
            return_len: 0,
            sub_call: None,
            stack: [[0; 32]; STACK_SIZE_WORDS as usize],
            memory_ptr,
            memory_len: 0,
            memory_cap: memory_size as u32,  // Use calculated memory_size
        }
    }

    pub fn stack_ptr(&self) -> u32 {
        self.stack_ptr
    }

    pub fn jump_ptr(&self) -> u32 {
        self.jump_ptr
    }

    pub fn return_off(&self) -> u32 {
        self.return_off
    }

    pub fn return_len(&self) -> u32 {
        self.return_len
    }

    pub fn return_data(&self) -> &[u8] {
        let offset = self.return_off as usize;
        let end = offset + self.return_len as usize;
        // TODO: Check bounds
        unsafe {
            std::slice::from_raw_parts(self.memory_ptr.add(offset), end - offset)
        }
    }

    pub fn stack(&self) -> &[Word] {
        &self.stack
    }

    pub fn memory(&self) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(self.memory_ptr, self.memory_len as usize)
        }
    }

    pub fn memory_mut(&mut self) -> &mut [u8] {
        unsafe {
            std::slice::from_raw_parts_mut(self.memory_ptr, self.memory_len as usize)
        }
    }

    pub fn memory_len(&self) -> u32 {
        self.memory_len
    }

    pub fn memory_cap(&self) -> u32 {
        self.memory_cap
    }

    pub fn sub_ctx(&self) -> Option<&Context> {
        self.sub_call.as_ref().map(|ctx| ctx.as_ref())
    }

    pub fn sub_ctx_mut(&mut self) -> Option<&mut Context> {
        self.sub_call.as_mut().map(|ctx| ctx.as_mut())
    }

    // Mutators; internal-only
    //
    // These functions are not meant to be exposed to the outside world. They are used internally
    // by builtins to manipulate the context.

    /// Puts the word into the stack and increments to the stack pointer.
    /// Returns false if the stack is full, true otherwise.
    pub(crate) fn stack_push(&mut self, word: Word) -> bool {
        if self.stack_ptr >= STACK_SIZE_WORDS {
            return false;
        }
        self.stack[self.stack_ptr as usize] = word;
        self.stack_ptr += 1;
        true
    }

    /// Pops a word from the stack and decrements the stack pointer.
    pub(crate) fn stack_pop(&mut self) -> &Word {
        // TODO: Handle bounds by making this function return a second value
        // if ctx.stack_ptr == 0 {
        //     return std::ptr::null();
        // }
        self.stack_ptr -= 1;
        &self.stack[self.stack_ptr as usize]
    }

    /// Peeks at a word in the stack without changing the stack pointer.
    pub(crate) fn stack_peek(&self, peek_idx: u32) -> &Word {
        // TODO: Handle bounds by making this function return a second value
        // if peek_idx >= ctx.stack_ptr {
        //     return std::ptr::null();
        // }
        let idx = (self.stack_ptr - peek_idx - 1) as usize;
        &self.stack[idx]
    }

    /// Swaps the top word of the stack with the word at the given index.
    /// Returns false if the given index is out of bounds, true otherwise.
    pub(crate) fn stack_swap(&mut self, swap_idx: u32) -> bool {
        if swap_idx >= self.stack_ptr - 1 {
            return false;
        }
        let top_idx = self.stack_ptr - 1;
        let swap_with_idx = self.stack_ptr - 2 - swap_idx;
        self.stack.swap(top_idx as usize, swap_with_idx as usize);
        true
    }

    /// Creates a new context and sets it as the sub context.
    pub(crate) fn init_sub_call(&mut self) -> &mut Context {
        self.sub_call = Some(Box::new(Context::new()));
        self.sub_call.as_mut().unwrap().as_mut()
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        // Deallocate memory buffer
        if !self.memory_ptr.is_null() {
            let memory_size = self.memory_cap as usize;
            let memory_layout = std::alloc::Layout::from_size_align(memory_size, 32)
                .expect("Failed to create memory layout");
            unsafe {
                std::alloc::dealloc(self.memory_ptr, memory_layout);
            }
        }
    }
}

/// Represents the result of a contract execution.
pub struct ContractRun {
    result: ReturnCode,
    ctx: Context,
}

impl ContractRun {
    pub fn new(result: ReturnCode, ctx: Context) -> Self {
        ContractRun { result, ctx }
    }

    pub fn result(&self) -> ReturnCode {
        self.result.clone()
    }

    pub fn ctx(&self) -> &Context {
        &self.ctx
    }
}

/// Information about the current block that gets exposed to the EVM.
#[repr(C)]
pub struct BlockInfo {
    number: u64,
    difficulty: u64,
    gas_limit: u64,
    timestamp: u64,
    base_fee: u64,
    blob_base_fee: u64,
    chain_id: u64,
    hash: Hash,
    hash_history: HashHistory,
    coinbase: Address,
}

impl BlockInfo {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        number: u64,
        difficulty: u64,
        gas_limit: u64,
        timestamp: u64,
        base_fee: u64,
        blob_base_fee: u64,
        chain_id: u64,
        hash: Hash,
        hash_history: HashHistory,
        coinbase: Address,
    ) -> Self {
        BlockInfo {
            number,
            difficulty,
            gas_limit,
            timestamp,
            base_fee,
            blob_base_fee,
            chain_id,
            hash,
            hash_history,
            coinbase,
        }
    }

    pub fn number(&self) -> u64 {
        self.number
    }

    pub fn difficulty(&self) -> u64 {
        self.difficulty
    }

    pub fn gas_limit(&self) -> u64 {
        self.gas_limit
    }

    pub fn timestamp(&self) -> u64 {
        self.timestamp
    }

    pub fn base_fee(&self) -> u64 {
        self.base_fee
    }

    pub fn blob_base_fee(&self) -> u64 {
        self.blob_base_fee
    }

    pub fn chain_id(&self) -> u64 {
        self.chain_id
    }

    pub fn hash(&self) -> &Hash {
        &self.hash
    }

    pub fn hash_history(&self) -> &HashHistory {
        &self.hash_history
    }

    pub fn coinbase(&self) -> &Address {
        &self.coinbase
    }
}

/// Return codes returned by contract function calls.
/// - Negative values are Jet-level failures.
/// - Positive values are successfully captured EVM-returns.
/// - Positive values below 64 are EVM-level successes.
/// - Positive values above 64 are EVM-level failures.
#[derive(Clone, Debug, PartialEq, Default)]
#[repr(i8)]
pub enum ReturnCode {
    // Jet-level failures
    InvalidJumpBlock = -1,

    // EVM-level successes
    #[default]
    ImplicitReturn = 0,
    ExplicitReturn = 1,
    Stop = 2,

    // EVM-level failures
    Revert = 64,
    Invalid = 65,
    JumpFailure = 66,
}

/// Mangles the given address into a contract function name.
pub fn mangle_contract_fn(address: &str) -> String {
    format!("{}{}", FN_CONTRACT_PREFIX, address)
}

/// Finds the pointer to the compiled contract function for the given address.
pub fn jet_contract_fn_lookup(jit_engine: &ExecutionEngine, addr_slice: &[u8]) -> usize {
    // Convert the address to a function name
    let reversed_addr = addr_slice.iter().rev().cloned().collect::<Vec<u8>>();
    let mut addr_str = "0x".to_owned();
    addr_str.push_str(&hex::encode(reversed_addr.as_slice()));
    let fn_name = mangle_contract_fn(addr_str.as_str());

    // Look up the function pointer
    match jit_engine.get_function_address(fn_name.as_str()) {
        Ok(ptr) => ptr,
        Err(e) => {
            error!("Error looking up contract function {}: {}", fn_name, e);
            0
        }
    }
}
