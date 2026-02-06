use std::str::FromStr;

use inkwell::{
    context::Context,
    module::Module,
    values::{FunctionValue, GlobalValue},
};

use jet_runtime;
use jet_ir::Types;

#[derive(serde::Serialize, Clone, Debug, Default)]
pub struct Options {
    mode: Mode,
    vstack: bool,
    emit_llvm: bool,
    assert: bool,
}

impl Options {
    pub fn new(mode: Mode, vstack: bool, emit_llvm: bool, assert: bool) -> Self {
        Self {
            mode,
            vstack,
            emit_llvm,
            assert,
        }
    }

    pub fn mode(&self) -> Mode {
        self.mode.clone()
    }

    pub fn vstack(&self) -> bool {
        self.vstack
    }

    pub fn emit_llvm(&self) -> bool {
        self.emit_llvm
    }

    pub fn assert(&self) -> bool {
        self.assert
    }
}

#[derive(clap::ValueEnum, serde::Serialize, Clone, Debug, Default, PartialEq, Eq)]
pub enum Mode {
    #[default]
    Debug = 0,
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

    contract_call: FunctionValue<'ctx>,
    contract_call_return_data_copy: FunctionValue<'ctx>,

    keccak256: FunctionValue<'ctx>,
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

        let contract_call = module.get_function(jet_runtime::symbols::FN_CONTRACT_CALL)?;
        let contract_call_return_data_copy =
            module.get_function(jet_runtime::symbols::FN_CONTRACT_CALL_RETURN_DATA_COPY)?;

        let keccak256 = module.get_function(jet_runtime::symbols::FN_KECCAK256)?;

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

            contract_call,
            contract_call_return_data_copy,

            keccak256,
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

    pub(crate) fn contract_call(&self) -> FunctionValue<'ctx> {
        self.contract_call
    }

    pub(crate) fn contract_call_return_data_copy(&self) -> FunctionValue<'ctx> {
        self.contract_call_return_data_copy
    }

    pub(crate) fn keccak256(&self) -> FunctionValue<'ctx> {
        self.keccak256
    }
}

pub struct Env<'ctx> {
    opts: Options,

    context: &'ctx Context,
    module: Module<'ctx>,

    types: Types<'ctx>,
    symbols: Symbols<'ctx>,
}

impl<'ctx> Env<'ctx> {
    pub fn new(context: &'ctx Context, module: Module<'ctx>, opts: Options) -> Self {
        let types = Types::new(context);
        let runtime_fns = Symbols::new(&module);

        if runtime_fns.is_none() {
            panic!("Failed to load all runtime functions");
        }

        Self {
            opts,

            context,
            module,

            types,
            symbols: runtime_fns.unwrap(),
        }
    }

    pub fn context(&self) -> &'ctx Context {
        self.context
    }

    pub fn module(&self) -> &Module<'ctx> {
        &self.module
    }

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
