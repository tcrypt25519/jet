use inkwell::execution_engine::ExecutionEngine;
use log::error;

use crate::{
    address::Address,
    error::{Result, RuntimeError},
    symbols::FN_CONTRACT_PREFIX,
    *,
};

/// A 256-bit EVM stack word, stored as 32 bytes in big-endian order.
pub type Word = [u8; 32];

/// A 256-bit Keccak-256 hash value.
pub type Hash = [u8; 32];

/// A ring buffer of the [`BLOCK_HASH_HISTORY_SIZE`] most recent block hashes.
pub type HashHistory = [Hash; BLOCK_HASH_HISTORY_SIZE];

/// The C ABI of a JIT-compiled EVM contract function.
///
/// Every compiled contract is exposed as a function with this signature.
/// The `*const Context` argument points to the caller-allocated execution
/// context; the function returns a [`ReturnCode`] indicating how it stopped.
pub type ContractFunc = unsafe extern "C" fn(*const Context) -> ReturnCode;

/// EVM execution context passed to every JIT-compiled contract function.
///
/// `Context` holds all mutable state that persists for the lifetime of a
/// single contract invocation: the operand stack, memory, return-data
/// registers, and an optional sub-call context for nested `CALL` operations.
///
/// The layout of this struct is `#[repr(C)]` and must exactly match the
/// `exec_ctx` LLVM struct type defined in [`jet_ir::Types`].  The field order
/// is documented there.
///
/// # Memory management
///
/// The EVM memory region is heap-allocated in [`Context::new`] and freed in
/// the [`Drop`] implementation.  The capacity tracked by `memory_cap` is the
/// true allocation size, while `memory_len` tracks the portion currently
/// accessible to EVM instructions.
#[repr(C)]
pub struct Context {
    stack_ptr: u32,
    jump_ptr: u32,

    return_off: u32,
    return_len: u32,

    sub_call: Option<Box<Context>>,
    stack: [Word; STACK_SIZE_WORDS as usize],

    // Pointer-based memory layout as per ADR-002
    // u32 is safe for offsets/sizes, see ADR-004 (EVM gas costs prevent >4GB)
    pub(crate) memory_ptr: *mut u8,
    pub(crate) memory_len: u32,
    pub(crate) memory_cap: u32,
}

impl Context {
    /// Allocates and initialises a new execution context.
    ///
    /// Heap-allocates the EVM memory buffer (32-byte aligned) at an initial
    /// capacity of `MEMORY_INITIAL_SIZE_WORDS * WORD_SIZE_BYTES` bytes.
    ///
    /// # Errors
    ///
    /// Returns [`RuntimeError::MemoryLayout`] or [`RuntimeError::MemoryAllocation`]
    /// if the memory buffer cannot be allocated.
    pub fn new() -> Result<Self> {
        // Allocate memory buffer on the heap
        let memory_size = (WORD_SIZE_BYTES * MEMORY_INITIAL_SIZE_WORDS) as usize;
        let memory_layout = std::alloc::Layout::from_size_align(memory_size, 32)
            .map_err(|e| RuntimeError::MemoryLayout(e.to_string()))?;
        let memory_ptr = unsafe { std::alloc::alloc_zeroed(memory_layout) };

        if memory_ptr.is_null() {
            return Err(RuntimeError::MemoryAllocation(
                "Failed to allocate memory for EVM context".to_string(),
            ));
        }

        Ok(Context {
            stack_ptr: 0,
            jump_ptr: 0,
            return_off: 0,
            return_len: 0,
            sub_call: None,
            stack: [[0; 32]; STACK_SIZE_WORDS as usize],
            memory_ptr,
            memory_len: 0,
            memory_cap: memory_size as u32, // Use calculated memory_size
        })
    }

    /// Returns the current stack depth (number of items on the stack).
    pub fn stack_ptr(&self) -> u32 {
        self.stack_ptr
    }

    /// Returns the index of the current jump destination block.
    ///
    /// This register is written by `JUMP`/`JUMPI` and read by the jump
    /// dispatch table inserted by the compiler.
    pub fn jump_ptr(&self) -> u32 {
        self.jump_ptr
    }

    /// Returns the byte offset of the return data within EVM memory.
    pub fn return_off(&self) -> u32 {
        self.return_off
    }

    /// Returns the byte length of the return data within EVM memory.
    pub fn return_len(&self) -> u32 {
        self.return_len
    }

    /// Returns the return data slice from memory.
    ///
    /// # Safety
    ///
    /// The caller must ensure that `return_off + return_len` does not exceed `memory_len`
    /// and that the memory pointer is valid. Use caution when calling this method, as it
    /// creates an unsafe slice without bounds checking.
    ///
    /// This validation should be performed before setting `return_off` and `return_len`,
    /// or at call sites before using this method.
    pub fn return_data(&self) -> &[u8] {
        let offset = self.return_off as usize;
        let len = self.return_len as usize;
        unsafe { std::slice::from_raw_parts(self.memory_ptr.add(offset), len) }
    }

    /// Returns the full stack array.
    ///
    /// Only entries at indices `0..stack_ptr()` contain live values.
    pub fn stack(&self) -> &[Word] {
        &self.stack
    }

    /// Returns the currently accessible EVM memory as a byte slice.
    pub fn memory(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.memory_ptr, self.memory_len as usize) }
    }

    /// Returns the currently accessible EVM memory as a mutable byte slice.
    pub fn memory_mut(&mut self) -> &mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self.memory_ptr, self.memory_len as usize) }
    }

    /// Returns the number of accessible bytes in EVM memory.
    pub fn memory_len(&self) -> u32 {
        self.memory_len
    }

    /// Returns the capacity of the underlying EVM memory buffer in bytes.
    pub fn memory_cap(&self) -> u32 {
        self.memory_cap
    }

    /// Returns a reference to the sub-call context, if one exists.
    ///
    /// A sub-call context is created when the contract executes a `CALL`-family
    /// instruction.  The sub-call's return data is then available via
    /// `RETURNDATASIZE` and `RETURNDATACOPY`.
    pub fn sub_ctx(&self) -> Option<&Context> {
        self.sub_call.as_ref().map(|ctx| ctx.as_ref())
    }

    /// Returns a mutable reference to the sub-call context, if one exists.
    pub fn sub_ctx_mut(&mut self) -> Option<&mut Context> {
        self.sub_call.as_mut().map(|ctx| ctx.as_mut())
    }

    // Mutators; internal-only
    //
    // These functions are not meant to be exposed to the outside world. They are used internally
    // by builtins to manipulate the context.

    /// Puts the word into the stack and increments to the stack pointer.
    /// Returns false if the stack is full, true otherwise.
    #[allow(dead_code)]
    pub(crate) fn stack_push(&mut self, word: Word) -> bool {
        if self.stack_ptr >= STACK_SIZE_WORDS {
            return false;
        }
        self.stack[self.stack_ptr as usize] = word;
        self.stack_ptr += 1;
        true
    }

    /// Pops a word from the stack and decrements the stack pointer.
    #[allow(dead_code)]
    pub(crate) fn stack_pop(&mut self) -> &Word {
        // TODO: Handle bounds by making this function return a second value
        // if ctx.stack_ptr == 0 {
        //     return std::ptr::null();
        // }
        self.stack_ptr -= 1;
        &self.stack[self.stack_ptr as usize]
    }

    /// Peeks at a word in the stack without changing the stack pointer.
    #[allow(dead_code)]
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
    #[allow(dead_code)]
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
    ///
    /// Returns a mutable reference to the newly created sub-context.
    ///
    /// # Errors
    ///
    /// Returns an error if memory allocation for the sub-context fails.
    pub(crate) fn init_sub_call(&mut self) -> Result<&mut Context> {
        self.sub_call = Some(Box::new(Context::new()?));
        Ok(self.sub_call.as_deref_mut().expect("just assigned"))
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        // Deallocate memory buffer
        if !self.memory_ptr.is_null() {
            let memory_size = self.memory_cap as usize;
            match std::alloc::Layout::from_size_align(memory_size, 32) {
                Ok(memory_layout) => unsafe {
                    std::alloc::dealloc(self.memory_ptr, memory_layout);
                },
                Err(e) => {
                    // Log the error but don't panic in drop
                    log::error!("Failed to create memory layout during dealloc: {}", e);
                }
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
    /// Creates a new `ContractRun` from a return code and execution context.
    pub fn new(result: ReturnCode, ctx: Context) -> Self {
        ContractRun { result, ctx }
    }

    /// Returns the [`ReturnCode`] produced by the contract.
    pub fn result(&self) -> ReturnCode {
        self.result.clone()
    }

    /// Returns a reference to the execution context after the contract ran.
    ///
    /// The context contains the final stack state, EVM memory contents, and
    /// the return data written by a `RETURN` or `REVERT` instruction.
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
    /// Creates a new `BlockInfo` describing the current block.
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

    /// Returns the block number.
    pub fn number(&self) -> u64 {
        self.number
    }

    /// Returns the block difficulty (or prevrandao after the Merge).
    pub fn difficulty(&self) -> u64 {
        self.difficulty
    }

    /// Returns the block gas limit.
    pub fn gas_limit(&self) -> u64 {
        self.gas_limit
    }

    /// Returns the block timestamp (Unix seconds).
    pub fn timestamp(&self) -> u64 {
        self.timestamp
    }

    /// Returns the EIP-1559 base fee in wei.
    pub fn base_fee(&self) -> u64 {
        self.base_fee
    }

    /// Returns the EIP-4844 blob base fee.
    pub fn blob_base_fee(&self) -> u64 {
        self.blob_base_fee
    }

    /// Returns the chain ID.
    pub fn chain_id(&self) -> u64 {
        self.chain_id
    }

    /// Returns the hash of this block.
    pub fn hash(&self) -> &Hash {
        &self.hash
    }

    /// Returns the ring buffer of the [`BLOCK_HASH_HISTORY_SIZE`] most recent block hashes.
    pub fn hash_history(&self) -> &HashHistory {
        &self.hash_history
    }

    /// Returns the beneficiary (miner/validator) address.
    pub fn coinbase(&self) -> &Address {
        &self.coinbase
    }
}

/// Status code returned by a JIT-compiled contract function.
///
/// Negative values indicate a Jet-level (internal) failure. Non-negative
/// values indicate that the EVM instruction stream was followed to a defined
/// stopping point: values below 64 are EVM successes, values at or above 64
/// are EVM failures.
///
/// # Examples
///
/// ```
/// use jet_runtime::exec::ReturnCode;
///
/// let code = ReturnCode::default();
/// assert_eq!(code, ReturnCode::ImplicitReturn);
/// ```
#[derive(Clone, Debug, PartialEq, Default)]
#[repr(i8)]
pub enum ReturnCode {
    // Jet-level failures

    /// The jump-dispatch table was given a block index with no corresponding `JUMPDEST`.
    InvalidJumpBlock = -1,
    /// A `POP`-style instruction was executed on an empty stack.
    StackUnderflow = -2,

    // EVM-level successes

    /// The contract ran to the end of its bytecode without a `RETURN` or `STOP`.
    #[default]
    ImplicitReturn = 0,
    /// The contract executed a `RETURN` instruction.
    ExplicitReturn = 1,
    /// The contract executed a `STOP` instruction.
    Stop = 2,

    // EVM-level failures

    /// The contract executed a `REVERT` instruction.
    Revert = 64,
    /// The contract executed an `INVALID` instruction.
    Invalid = 65,
    /// A `JUMP` or `JUMPI` targeted a byte that is not a `JUMPDEST`.
    JumpFailure = 66,
}

/// Mangles the given address into a contract function name.
pub fn mangle_contract_fn(address: &Address) -> String {
    format!("{}{}", FN_CONTRACT_PREFIX, address)
}

/// Finds the pointer to the compiled contract function for the given address.
///
/// `addr_slice` contains the 20 address bytes in the little-endian order used
/// by the JIT stack (least-significant byte first). They are reversed here to
/// recover the canonical big-endian address before the function-name lookup.
pub fn jet_contract_fn_lookup(jit_engine: &ExecutionEngine, addr_slice: &[u8]) -> usize {
    // The stack stores values little-endian; reverse to get the canonical
    // big-endian address bytes.
    // EVM stack words are 32 bytes but addresses are 20; take only the last
    // ADDRESS_SIZE_BYTES bytes (the address occupies the low bytes of the word)
    // after reversing from little-endian to big-endian order.
    let mut bytes = [0u8; ADDRESS_SIZE_BYTES];
    for (i, b) in addr_slice.iter().rev().take(ADDRESS_SIZE_BYTES).enumerate() {
        bytes[i] = *b;
    }
    let address = Address::new(bytes);
    let fn_name = mangle_contract_fn(&address);

    // Look up the function pointer.
    match jit_engine.get_function_address(fn_name.as_str()) {
        Ok(ptr) => ptr,
        Err(e) => {
            error!("Error looking up contract function {}: {}", fn_name, e);
            0
        }
    }
}
