use std::str::FromStr;

use inkwell::{
    context::Context,
    module::Module,
    values::{FunctionValue, GlobalValue},
};

use jet_ir::Types;
use jet_runtime;

/// Configuration options for the Jet compiler.
///
/// `Options` is passed when creating an [`Env`] and controls global compiler
/// behaviour such as the optimisation level and diagnostic output.
#[derive(serde::Serialize, Clone, Debug, Default)]
pub struct Options {
    mode: Mode,
    emit_llvm: bool,
    assert: bool,
}

impl Options {
    /// Creates a new `Options` value.
    ///
    /// # Parameters
    ///
    /// - `mode` — Compilation mode ([`Mode::Debug`] or [`Mode::Release`]).
    /// - `emit_llvm` — When `true`, the generated LLVM IR is printed to stdout
    ///   after each contract is compiled.
    /// - `assert` — When `true`, the LLVM IR is verified after each contract
    ///   is compiled, returning an error if verification fails.
    pub fn new(mode: Mode, emit_llvm: bool, assert: bool) -> Self {
        Self {
            mode,
            emit_llvm,
            assert,
        }
    }

    /// Returns the compilation mode.
    pub fn mode(&self) -> Mode {
        self.mode.clone()
    }

    /// Returns `true` if the generated LLVM IR should be printed to stdout.
    pub fn emit_llvm(&self) -> bool {
        self.emit_llvm
    }

    /// Returns `true` if LLVM IR verification is enabled after compilation.
    pub fn assert(&self) -> bool {
        self.assert
    }
}

/// Compilation mode controlling optimisation and diagnostic behaviour.
#[derive(clap::ValueEnum, serde::Serialize, Clone, Debug, Default, PartialEq, Eq)]
pub enum Mode {
    /// Debug mode: no optimisations, additional diagnostics.
    #[default]
    Debug = 0,
    /// Release mode: standard optimisations enabled.
    Release = 1,
}

impl FromStr for Mode {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "release" => Ok(Self::Release),
            "debug" => Ok(Self::Debug),
            _ => Err(()),
        }
    }
}

pub(crate) struct Symbols<'ctx> {
    jit_engine: GlobalValue<'ctx>,

    stack_push_word: FunctionValue<'ctx>,
    stack_push_ptr: FunctionValue<'ctx>,

    stack_pop: FunctionValue<'ctx>,
    stack_peek: FunctionValue<'ctx>,
    stack_swap: FunctionValue<'ctx>,

    mem_store: FunctionValue<'ctx>,
    mem_store_byte: FunctionValue<'ctx>,
    mem_load: FunctionValue<'ctx>,
    mem_expand: FunctionValue<'ctx>,

    contract_call: FunctionValue<'ctx>,
    contract_call_return_data_copy: FunctionValue<'ctx>,

    keccak256: FunctionValue<'ctx>,
    exp: FunctionValue<'ctx>,
    addmod: FunctionValue<'ctx>,
    mulmod: FunctionValue<'ctx>,
}

impl<'ctx> Symbols<'ctx> {
    pub fn new(module: &Module<'ctx>) -> Option<Self> {
        let jit_engine = module.get_global(jet_runtime::symbols::JIT_ENGINE)?;

        let stack_push_word = module.get_function(jet_runtime::symbols::FN_STACK_PUSH_WORD)?;
        let stack_push_ptr = module.get_function(jet_runtime::symbols::FN_STACK_PUSH_PTR)?;

        let stack_pop = module.get_function(jet_runtime::symbols::FN_STACK_POP)?;
        let stack_peek = module.get_function(jet_runtime::symbols::FN_STACK_PEEK)?;
        let stack_swap = module.get_function(jet_runtime::symbols::FN_STACK_SWAP)?;

        let mem_store = module.get_function(jet_runtime::symbols::FN_MEM_STORE_WORD)?;
        let mem_store_byte = module.get_function(jet_runtime::symbols::FN_MEM_STORE_BYTE)?;
        let mem_load = module.get_function(jet_runtime::symbols::FN_MEM_LOAD)?;
        let mem_expand = module.get_function(jet_runtime::symbols::FN_MEM_EXPAND)?;

        let contract_call = module.get_function(jet_runtime::symbols::FN_CONTRACT_CALL)?;
        let contract_call_return_data_copy =
            module.get_function(jet_runtime::symbols::FN_CONTRACT_CALL_RETURN_DATA_COPY)?;

        let keccak256 = module.get_function(jet_runtime::symbols::FN_KECCAK256)?;
        let exp = module.get_function(jet_runtime::symbols::FN_EXP)?;
        let addmod = module.get_function(jet_runtime::symbols::FN_ADDMOD)?;
        let mulmod = module.get_function(jet_runtime::symbols::FN_MULMOD)?;

        Some(Self {
            jit_engine,

            stack_push_ptr,
            stack_push_word,

            stack_pop,
            stack_peek,
            stack_swap,

            mem_store,
            mem_store_byte,
            mem_load,
            mem_expand,

            contract_call,
            contract_call_return_data_copy,

            keccak256,
            exp,
            addmod,
            mulmod,
        })
    }

    pub(crate) fn jit_engine(&self) -> GlobalValue<'ctx> {
        self.jit_engine
    }

    pub(crate) fn stack_push_ptr(&self) -> FunctionValue<'ctx> {
        self.stack_push_ptr
    }

    pub(crate) fn stack_push_word(&self) -> FunctionValue<'ctx> {
        self.stack_push_word
    }

    pub(crate) fn stack_pop(&self) -> FunctionValue<'ctx> {
        self.stack_pop
    }

    pub(crate) fn stack_peek(&self) -> FunctionValue<'ctx> {
        self.stack_peek
    }

    pub(crate) fn stack_swap(&self) -> FunctionValue<'ctx> {
        self.stack_swap
    }

    pub(crate) fn mem_store(&self) -> FunctionValue<'ctx> {
        self.mem_store
    }

    pub(crate) fn mem_store_byte(&self) -> FunctionValue<'ctx> {
        self.mem_store_byte
    }

    pub(crate) fn mem_load(&self) -> FunctionValue<'ctx> {
        self.mem_load
    }

    pub(crate) fn mem_expand(&self) -> FunctionValue<'ctx> {
        self.mem_expand
    }

    pub(crate) fn contract_call(&self) -> FunctionValue<'ctx> {
        self.contract_call
    }

    pub(crate) fn contract_call_return_data_copy(&self) -> FunctionValue<'ctx> {
        self.contract_call_return_data_copy
    }

    pub(crate) fn keccak256(&self) -> FunctionValue<'ctx> {
        self.keccak256
    }

    pub(crate) fn exp(&self) -> FunctionValue<'ctx> {
        self.exp
    }

    pub(crate) fn addmod(&self) -> FunctionValue<'ctx> {
        self.addmod
    }

    pub(crate) fn mulmod(&self) -> FunctionValue<'ctx> {
        self.mulmod
    }
}

/// LLVM build environment for a single compilation unit.
///
/// `Env` bundles an LLVM [`Context`], the LLVM [`Module`] being compiled,
/// the unified type registry, and resolved symbols for every runtime function
/// that has been declared in the module.
///
/// An `Env` is constructed once per compilation session and shared across all
/// contracts compiled into the same module.
pub struct Env<'ctx> {
    opts: Options,

    context: &'ctx Context,
    module: Module<'ctx>,

    types: Types<'ctx>,
    symbols: Symbols<'ctx>,
}

impl<'ctx> Env<'ctx> {
    /// Creates a new build environment from an existing LLVM module.
    ///
    /// The `module` must already have all runtime function declarations present
    /// (use [`jet_runtime::RuntimeBuilder`] to produce such a module). Returns
    /// an error if any expected runtime symbol cannot be found in the module.
    pub fn new(
        context: &'ctx Context,
        module: Module<'ctx>,
        opts: Options,
    ) -> Result<Self, super::Error> {
        let types = Types::new(context);
        let runtime_fns = Symbols::new(&module).ok_or_else(|| {
            super::Error::InvariantViolation("Failed to load all runtime functions".to_string())
        })?;

        Ok(Self {
            opts,
            context,
            module,
            types,
            symbols: runtime_fns,
        })
    }

    /// Returns the LLVM context.
    pub fn context(&self) -> &'ctx Context {
        self.context
    }

    /// Returns the LLVM module being compiled.
    pub fn module(&self) -> &Module<'ctx> {
        &self.module
    }

    /// Returns the compiler options used to create this environment.
    pub fn opts(&self) -> &Options {
        &self.opts
    }

    pub(crate) fn types(&self) -> &Types<'ctx> {
        &self.types
    }

    pub(crate) fn symbols(&self) -> &Symbols<'ctx> {
        &self.symbols
    }
}
