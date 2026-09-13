use inkwell::context::Context;
use jet::{
    builder,
    builder::env::{Mode::Debug, Options, StackMode},
    engine,
    engine::Engine,
};
use jet_runtime::{self, Address, CallInfo, exec, exec::ReturnCode};
use log::trace;
use thiserror::Error;

#[derive(Error, Debug)]
#[error(transparent)]
pub(crate) enum Error {
    Build(#[from] builder::Error),
    Engine(#[from] engine::Error),
}

#[macro_export]
macro_rules! rom_tests {
    // Use the struct directly in the macro arguments
    ($($name:ident: $test:expr),* $(,)?) => {
        $(
            paste::item! {
                #[test]
                fn [<test_rom_with_real_stack_ $name>]() -> Result<(), Error> {
                    let t: Test = $test;
                    _test_rom_body(t, jet::builder::env::StackMode::RuntimeOnly)
                }

                #[test]
                fn [<test_rom_with_symbolic_stack_ $name>]() -> Result<(), Error> {
                    let t: Test = $test;
                    _test_rom_body(t, jet::builder::env::StackMode::SymbolicPreferred)
                }
            }
        )*
    };
}

macro_rules! assert_eq_named {
    ($name:expr, $left:expr, $right:expr) => {
        assert_eq!($left, $right, concat!("Checking ", $name, " want={:?}, got={:?}"), $right, $left);
    };
}

pub(crate) struct Test {
    pub(crate) roms:     Vec<Vec<u8>>,
    pub(crate) expected: TestContractRun,
}

#[derive(Default)]
pub(crate) struct TestContractRun {
    pub(crate) result:        ReturnCode,
    pub(crate) stack_ptr:     u32,
    pub(crate) jump_ptr:      u32,
    pub(crate) return_offset: u32,
    pub(crate) return_length: u32,
    pub(crate) stack:         Vec<[u8; 32]>,
    pub(crate) memory:        Option<Vec<u8>>,
    pub(crate) memory_len:    Option<u32>,
}

impl TestContractRun {
    fn assert_eq(&self, run: &exec::ContractRun) {
        assert_eq!(run.result(), self.result);

        let ctx = run.ctx();
        assert_eq_named!("stack_ptr", ctx.stack_ptr(), self.stack_ptr);
        assert_eq_named!("jump_ptr", ctx.jump_ptr(), self.jump_ptr);
        assert_eq_named!("return_off", ctx.return_off(), self.return_offset);
        assert_eq_named!("return_len", ctx.return_len(), self.return_length);
        assert_eq_named!("stack_len", ctx.stack_ptr(), self.stack.len() as u32);

        assert_eq_named!("stack", &ctx.stack()[..self.stack.len()], self.stack.as_slice());

        if let Some(expected_memory) = &self.memory {
            assert_eq_named!("memory", &ctx.memory()[..expected_memory.len()], expected_memory.as_slice());
        }

        if let Some(expected_memory_len) = self.memory_len {
            assert_eq_named!("memory_len", ctx.memory_len(), expected_memory_len);
        }
    }
}

pub(crate) fn _test_rom_body(t: Test, stack_mode: StackMode) -> Result<(), Error> {
    let llvm_ctx = Context::create();
    let opts = Options::new(Debug, false, true).with_stack_mode(stack_mode);
    let block_info = new_test_block_info();

    let mut engine = Engine::new(&llvm_ctx, opts)?;

    assert_ne!(t.roms.len(), 0);
    for (i, rom) in t.roms.iter().enumerate() {
        let mut addr = Address::ZERO;
        addr.as_bytes_mut()[Address::LEN - 1] = i as u8;
        trace!("Building contract at address {}", addr);
        engine.build_contract(addr, rom.as_slice())?;
    }

    let call_info = new_test_call_info();
    let run = engine.run_contract(call_info, &block_info)?;
    t.expected.assert_eq(&run);

    Ok(())
}

pub(crate) fn _test_contracts_with_call_info(
    make_call_info: &impl Fn() -> CallInfo,
    contracts: &[(Address, Vec<u8>)],
    expected: &TestContractRun,
    stack_mode: StackMode,
) -> Result<(), Error> {
    let call_info = make_call_info();
    let llvm_ctx = Context::create();
    let opts = Options::new(Debug, false, true).with_stack_mode(stack_mode);
    let block_info = new_test_block_info();

    let mut engine = Engine::new(&llvm_ctx, opts)?;
    for (addr, rom) in contracts {
        trace!("Building contract at address {}", addr);
        engine.build_contract(*addr, rom.as_slice())?;
    }

    let run = engine.run_contract(call_info, &block_info)?;
    expected.assert_eq(&run);

    Ok(())
}

pub(crate) fn _test_rom_with_call_info(call_info: CallInfo, t: Test, stack_mode: StackMode) -> Result<(), Error> {
    let llvm_ctx = Context::create();
    let opts = Options::new(Debug, false, true).with_stack_mode(stack_mode);
    let block_info = new_test_block_info();

    let mut engine = Engine::new(&llvm_ctx, opts)?;

    assert_ne!(t.roms.len(), 0);
    for (i, rom) in t.roms.iter().enumerate() {
        let mut addr = Address::ZERO;
        addr.as_bytes_mut()[Address::LEN - 1] = i as u8;
        trace!("Building contract at address {}", addr);
        engine.build_contract(addr, rom.as_slice())?;
    }

    let run = engine.run_contract(call_info, &block_info)?;
    t.expected.assert_eq(&run);

    Ok(())
}

pub(crate) fn stack_word(bytes: &[u8]) -> [u8; 32] {
    let mut word = [0; 32];
    word[..bytes.len()].copy_from_slice(bytes);
    word
}

pub(crate) fn new_test_block_info() -> exec::BlockInfo {
    let hash = [
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31,
    ];
    let hash_history = new_test_block_info_hash_history();
    let coinbase = Address::new([1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    exec::BlockInfo::new(42, 100, 100, 1717354173, 5_000_000, 1_000_000, 1, hash, hash_history, coinbase)
}

pub(crate) fn new_test_call_info() -> CallInfo {
    CallInfo::new(Address::ZERO, Address::ZERO, Address::ZERO, [0u8; 32], &[]).unwrap()
}

fn new_test_block_info_hash_history() -> exec::HashHistory {
    let mut hash_history = [[0; 32]; jet_runtime::BLOCK_HASH_HISTORY_SIZE];

    hash_history
        .iter_mut()
        .enumerate()
        .take(jet_runtime::BLOCK_HASH_HISTORY_SIZE)
        .for_each(|(i, hash)| {
            hash[31] = i as u8;
        });

    hash_history
}
