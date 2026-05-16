//! Stack backend trait and concrete implementations.
//!
//! [`StackBackend`] is the only interface between EVM opcode semantics and the
//! underlying stack representation.  There are two concrete implementations:
//!
//! * [`RuntimeStackBackend`] – emits LLVM `call` instructions to the
//!   `jet_runtime` C-ABI builtins, keeping the EVM stack in the `Context`
//!   struct at runtime.  This is a direct replacement for the `call_stack_*`
//!   helpers currently in `ops.rs`.
//!
//! * [`SymbolicStackBackend`] – tracks values as LLVM [`IntValue`]s at
//!   compile time, eliminating all runtime stack traffic for supported ops.
//!
//! Neither implementation is wired into [`BuildCtx`] yet; that migration is a
//! separate step.
//!
//! [`BuildCtx`]: crate::builder::contract::BuildCtx

use std::cell::RefCell;

use inkwell::{
    builder::Builder,
    values::{AsValueRef, FunctionValue, IntValue, PointerValue},
};
use jet_runtime::exec::ReturnCode;

use crate::builder::{Error, env::Env, symbolic::SymbolicStack};

// ---------------------------------------------------------------------------
// Trait
// ---------------------------------------------------------------------------

/// Abstraction over EVM stack mechanics used during LLVM IR construction.
///
/// All methods speak exclusively in `IntValue<'ctx>` (256-bit EVM words).
/// Each implementation is responsible for any internal normalisation, loads,
/// stores, or compile-time state management needed to satisfy that contract.
///
/// # Index convention
/// `dup` and `swap` take a 1-based index matching the EVM opcode family:
/// `DUP1`/`SWAP1` pass `1`, `DUP16`/`SWAP16` pass `16`.
pub(crate) trait StackBackend<'ctx> {
    /// Push a 256-bit word onto the stack.
    ///
    /// Values narrower than 256 bits must be zero-extended by the
    /// implementation before being stored.
    fn push_word(&self, value: IntValue<'ctx>) -> Result<(), Error>;

    /// Pop and return the top 256-bit word.
    ///
    /// Returns an error (or emits a terminal IR path) on underflow.
    fn pop_word(&self) -> Result<IntValue<'ctx>, Error>;

    /// Duplicate the word at depth `index` (1 = top) and push the copy.
    fn dup(&self, index: u8) -> Result<(), Error>;

    /// Swap the top word with the word at depth `index + 1` (1 = second from top).
    fn swap(&self, index: u8) -> Result<(), Error>;

    /// Pop two words. Returns `(top, second)` matching EVM stack order.
    fn pop_2(&self) -> Result<(IntValue<'ctx>, IntValue<'ctx>), Error> {
        let a = self.pop_word()?;
        let b = self.pop_word()?;
        Ok((a, b))
    }

    /// Pop three words. Returns `(top, second, third)` matching EVM stack order.
    fn pop_3(
        &self,
    ) -> Result<(IntValue<'ctx>, IntValue<'ctx>, IntValue<'ctx>), Error> {
        let a = self.pop_word()?;
        let b = self.pop_word()?;
        let c = self.pop_word()?;
        Ok((a, b, c))
    }
}

// ---------------------------------------------------------------------------
// RuntimeStackBackend
// ---------------------------------------------------------------------------

/// Stack backend that emits `jet_runtime` C-ABI builtin calls.
///
/// Stack state lives in the `Context` struct at runtime.  `pop_word` emits a
/// null-pointer check and branches to a `StackUnderflow` return block on
/// failure, preserving the existing IR contract from `call_stack_pop` in
/// `ops.rs`.
pub(crate) struct RuntimeStackBackend<'ctx, 'b> {
    builder: &'b Builder<'ctx>,
    env: &'b Env<'ctx>,
    func: FunctionValue<'ctx>,
    exec_ctx: PointerValue<'ctx>,
}

impl<'ctx, 'b> RuntimeStackBackend<'ctx, 'b> {
    pub(crate) fn new(
        builder: &'b Builder<'ctx>,
        env: &'b Env<'ctx>,
        func: FunctionValue<'ctx>,
        exec_ctx: PointerValue<'ctx>,
    ) -> Self {
        Self {
            builder,
            env,
            func,
            exec_ctx,
        }
    }

    /// Zero-extend a value narrower than 256 bits to i256.
    fn normalize(&self, value: IntValue<'ctx>) -> Result<IntValue<'ctx>, Error> {
        let bit_width = value.get_type().get_bit_width();
        match bit_width {
            1 | 8 | 32 => Ok(self.builder.build_int_z_extend(
                value,
                self.env.types().i256,
                "int_to_word",
            )?),
            256 => Ok(value),
            _ => Err(Error::InvalidBitWidth(bit_width)),
        }
    }
}

impl<'ctx, 'b> StackBackend<'ctx> for RuntimeStackBackend<'ctx, 'b> {
    fn push_word(&self, value: IntValue<'ctx>) -> Result<(), Error> {
        let value = self.normalize(value)?;
        self.builder.build_call(
            self.env.symbols().stack_push_word(),
            &[self.exec_ctx.into(), value.into()],
            "stack_push_i256",
        )?;
        Ok(())
    }

    fn pop_word(&self) -> Result<IntValue<'ctx>, Error> {
        let ret = self.builder.build_call(
            self.env.symbols().stack_pop(),
            &[self.exec_ctx.into()],
            "word_ptr",
        )?;
        let ptr = unsafe { PointerValue::new(ret.as_value_ref()) };

        // Null pointer signals stack underflow; branch to a terminal block.
        let is_null = self.builder.build_is_null(ptr, "is_stack_underflow")?;
        let underflow_block = self
            .env
            .context()
            .append_basic_block(self.func, "stack_underflow");
        let valid_block = self
            .env
            .context()
            .append_basic_block(self.func, "stack_valid");
        self.builder
            .build_conditional_branch(is_null, underflow_block, valid_block)?;

        self.builder.position_at_end(underflow_block);
        let underflow_code = self
            .env
            .types()
            .i8
            .const_int(ReturnCode::StackUnderflow as u64, false);
        self.builder.build_return(Some(&underflow_code))?;

        self.builder.position_at_end(valid_block);
        // Load the i256 word from the pointer returned by the runtime.
        let loaded = self
            .builder
            .build_load(self.env.types().i256, ptr, "load_int")?;
        let word = unsafe { IntValue::new(loaded.as_value_ref()) };
        Ok(word)
    }

    fn dup(&self, index: u8) -> Result<(), Error> {
        let index_value = self.env.types().i8.const_int(index as u64, false);
        let ret = self.builder.build_call(
            self.env.symbols().stack_peek(),
            &[self.exec_ctx.into(), index_value.into()],
            "stack_peek_word_result",
        )?;
        let ptr = unsafe { PointerValue::new(ret.as_value_ref()) };
        self.builder.build_call(
            self.env.symbols().stack_push_ptr(),
            &[self.exec_ctx.into(), ptr.into()],
            "stack_push_ptr",
        )?;
        Ok(())
    }

    fn swap(&self, index: u8) -> Result<(), Error> {
        let index_value = self.env.types().i8.const_int(index as u64, false);
        self.builder.build_call(
            self.env.symbols().stack_swap(),
            &[self.exec_ctx.into(), index_value.into()],
            "stack_swap_ret",
        )?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// SymbolicStackBackend
// ---------------------------------------------------------------------------

/// Stack backend that tracks values at compile time as LLVM `IntValue`s.
///
/// No runtime stack traffic is emitted for push/pop/dup/swap.  All operations
/// manipulate an in-memory [`SymbolicStack`] that exists only during
/// compilation.  Values narrower than 256 bits are zero-extended on push,
/// exactly matching the [`RuntimeStackBackend`] contract.
pub(crate) struct SymbolicStackBackend<'ctx, 'b> {
    builder: &'b Builder<'ctx>,
    env: &'b Env<'ctx>,
    stack: RefCell<SymbolicStack<'ctx>>,
}

impl<'ctx, 'b> SymbolicStackBackend<'ctx, 'b> {
    pub(crate) fn new(builder: &'b Builder<'ctx>, env: &'b Env<'ctx>) -> Self {
        Self {
            builder,
            env,
            stack: RefCell::new(SymbolicStack::new()),
        }
    }

    /// Zero-extend a value narrower than 256 bits to i256.
    fn normalize(&self, value: IntValue<'ctx>) -> Result<IntValue<'ctx>, Error> {
        let bit_width = value.get_type().get_bit_width();
        match bit_width {
            1 | 8 | 32 => Ok(self.builder.build_int_z_extend(
                value,
                self.env.types().i256,
                "int_to_word",
            )?),
            256 => Ok(value),
            _ => Err(Error::InvalidBitWidth(bit_width)),
        }
    }
}

impl<'ctx, 'b> StackBackend<'ctx> for SymbolicStackBackend<'ctx, 'b> {
    fn push_word(&self, value: IntValue<'ctx>) -> Result<(), Error> {
        let value = self.normalize(value)?;
        self.stack.borrow_mut().push_word(value);
        Ok(())
    }

    fn pop_word(&self) -> Result<IntValue<'ctx>, Error> {
        self.stack.borrow_mut().pop_word()
    }

    fn dup(&self, index: u8) -> Result<(), Error> {
        self.stack.borrow_mut().dup(index)
    }

    fn swap(&self, index: u8) -> Result<(), Error> {
        self.stack.borrow_mut().swap(index)
    }
}
