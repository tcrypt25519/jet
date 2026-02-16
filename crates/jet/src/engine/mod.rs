use inkwell::{
    OptimizationLevel,
    context::Context,
    execution_engine::{ExecutionEngine, FunctionLookupError, JitFunction},
    module::Module,
    support::LLVMString,
};
use log::{info, trace};
use thiserror::Error;

use jet_runtime::{
    Address, RuntimeBuilder, builtins, exec,
    exec::{BlockInfo, ContractFunc, ContractRun},
};

use crate::{
    builder,
    builder::{env, env::Env, manager::Manager},
};

/// Errors that can occur during contract compilation or execution.
#[derive(Error, Debug)]
#[error(transparent)]
pub enum Error {
    /// A compilation error occurred while translating EVM bytecode to LLVM IR.
    Build(#[from] builder::Error),
    /// A compiled contract function could not be found in the JIT engine.
    FunctionLookup(#[from] FunctionLookupError),
    /// A raw LLVM error string was returned by the LLVM C API.
    LLVM(#[from] LLVMString),
}

/// High-level JIT engine that compiles and executes EVM contracts.
///
/// `Engine` is the primary entry point for running EVM bytecode.  It manages
/// a single LLVM module and a LLVM JIT execution engine.  Contracts are added
/// with [`build_contract`][Engine::build_contract] and executed with
/// [`run_contract`][Engine::run_contract].
///
/// # Lifetime
///
/// The `'ctx` lifetime is tied to an inkwell [`Context`] that must outlive the
/// engine.  A typical usage pattern is to create the context on the stack and
/// pass a reference to [`Engine::new`].
pub struct Engine<'ctx> {
    build_manager: Manager<'ctx>,
}

impl<'ctx> Engine<'ctx> {
    /// Creates a new engine.
    ///
    /// Initialises the LLVM module with all runtime function declarations, then
    /// wraps it in a [`Manager`] that is ready to accept contracts.
    ///
    /// # Errors
    ///
    /// Returns [`Error`] if the runtime module cannot be built or if any
    /// required runtime symbol is missing.
    pub fn new(context: &'ctx Context, build_opts: env::Options) -> Result<Self, Error> {
        let runtime_module = load_runtime_module(context)?;
        let build_env = Env::new(context, runtime_module, build_opts)?;
        let build_manager = Manager::new(build_env);

        Ok(Engine { build_manager })
    }

    /// Compiles the EVM bytecode `rom` for the contract at `addr`.
    ///
    /// After a successful call the compiled function is available in the LLVM
    /// module and can be executed with [`run_contract`][Engine::run_contract].
    ///
    /// # Errors
    ///
    /// Returns [`Error::Build`] if compilation of the bytecode fails.
    pub fn build_contract(&mut self, addr: Address, rom: &[u8]) -> Result<(), Error> {
        self.build_manager.add_contract_function(addr, rom)?;
        Ok(())
    }

    /// Executes the previously compiled contract at `addr`.
    ///
    /// Creates a fresh JIT execution engine, links in all runtime builtins,
    /// allocates a new [`exec::Context`], and invokes the compiled contract
    /// function.  Returns a [`ContractRun`] containing the return code and the
    /// execution context (stack, memory, return data).
    ///
    /// # Errors
    ///
    /// Returns [`Error::FunctionLookup`] if no contract was compiled for
    /// `addr`, or [`Error::LLVM`] if the JIT engine cannot be created.
    pub fn run_contract(
        &self,
        addr: Address,
        _block_info: &BlockInfo,
    ) -> Result<ContractRun, Error> {
        // Create a JIT execution engine
        let jit = self
            .build_manager
            .env()
            .module()
            .create_jit_execution_engine(OptimizationLevel::None)?;
        self.link_in_runtime(&jit);

        // Load and run the contract function
        let contract_exec_fn = match self.get_contract_exec_fn(&jit, addr) {
            Ok(f) => f,
            Err(e) => {
                return Err(Error::FunctionLookup(e));
            }
        };

        trace!("Running function...");
        let ctx = exec::Context::new().map_err(|e| Error::Build(builder::Error::Runtime(e)))?;
        let result = unsafe { contract_exec_fn.call(&ctx as *const exec::Context) };
        trace!("Function returned");

        Ok(ContractRun::new(result, ctx))
    }

    fn link_in_runtime(&self, ee: &ExecutionEngine) {
        let sym = self.build_manager.env().symbols();
        let map_fn = |name, ptr| {
            ee.add_global_mapping(&name, ptr);
        };

        // Link in the JIT engine
        ee.add_global_mapping(&sym.jit_engine(), ee as *const ExecutionEngine as usize);

        // Link in external runtime functions (contract calls and crypto)
        // Stack and memory operations are now generated as IR, so they don't need linking
        map_fn(
            sym.contract_call(),
            builtins::jet_contract_call as *const () as usize,
        );
        map_fn(
            sym.contract_call_return_data_copy(),
            builtins::jet_contract_call_return_data_copy as *const () as usize,
        );
        map_fn(
            sym.keccak256(),
            builtins::jet_ops_keccak256 as *const () as usize,
        );
        map_fn(sym.exp(), builtins::jet_ops_exp as *const () as usize);
        map_fn(sym.addmod(), builtins::jet_ops_addmod as *const () as usize);
        map_fn(sym.mulmod(), builtins::jet_ops_mulmod as *const () as usize);
        map_fn(
            sym.mem_expand(),
            builtins::jet_mem_expand as *const () as usize,
        );
    }

    fn get_contract_exec_fn(
        &self,
        ee: &ExecutionEngine<'ctx>,
        addr: Address,
    ) -> Result<JitFunction<'_, ContractFunc>, FunctionLookupError> {
        let name = exec::mangle_contract_fn(&addr);
        info!("Looking up contract function {}", name);
        unsafe { ee.get_function(name.as_str()) }
    }
}

fn load_runtime_module(context: &Context) -> Result<Module<'_>, Error> {
    let runtime_builder = RuntimeBuilder::new(context, "JetVM Runtime");
    let module = runtime_builder.build();
    Ok(module)
}
