use inkwell::context::Context;
use log::trace;
use thiserror::Error;

use jet::{
    builder,
    builder::env::{Mode::Debug, Options},
    engine,
    engine::Engine,
};
use jet_runtime::{self, exec, exec::ReturnCode};

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
                // #[test]
                // fn [<test_rom_with_vstack_ $name>]() -> Result<(), Error> {
                //     let t: Test = $test;
                //     _test_rom_body(t, true)
                // }

                #[test]
                fn [<test_rom_with_real_stack_ $name>]() -> Result<(), Error> {
                    let t: Test = $test;
                    _test_rom_body(t, false)
                }
            }
        )*
    };
}

macro_rules! assert_eq_named {
    ($name:expr, $left:expr, $right:expr) => {
        assert_eq!(
            $left, $right,
            concat!("Checking ", $name, " want={:?}, got={:?}"),
            $right, $left
        );
    };
}

pub(crate) struct Test {
    pub(crate) roms: Vec<Vec<u8>>,
    pub(crate) expected: TestContractRun,
}

#[derive(Default)]
pub(crate) struct TestContractRun {
    pub(crate) result: ReturnCode,
    pub(crate) stack_ptr: u32,
    pub(crate) jump_ptr: u32,
    pub(crate) return_offset: u32,
    pub(crate) return_length: u32,
    pub(crate) stack: Vec<[u8; 32]>,
    pub(crate) memory: Option<Vec<u8>>,
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

        // let actual_stack = ctx.stack();
        assert_eq_named!(
            "stack",
            &ctx.stack()[..self.stack.len()],
            self.stack.as_slice()
        );
        // for (i, expected_word) in self.stack.iter().enumerate() {
        //     // let idx = i * 32;
        //     // let mut actual_word: Word = [0; 32];
        //     // actual_word[..].copy_from_slice(actual_stack[i]);
        //     // assert_eq!(actual_stack[i], *expected_word);
        //     let actual_word = actual_stack[i];
        //     assert_eq_named!("stack word", actual_word, *expected_word);
        // }

        if let Some(expected_memory) = &self.memory {
            assert_eq_named!(
                "memory",
                &ctx.memory()[..expected_memory.len()],
                expected_memory.as_slice()
            );
        }
    }
}

pub(crate) fn _test_rom_body(t: Test, use_vstack: bool) -> Result<(), Error> {
    let llvm_ctx = Context::create();
    let opts = Options::new(Debug, use_vstack, false, true);
    let block_info = new_test_block_info();

    let mut engine = Engine::new(&llvm_ctx, opts)?;

    assert_ne!(t.roms.len(), 0);
    for (i, rom) in t.roms.iter().enumerate() {
        let addr = hex::encode(vec![0, i as u8]);
        let prefixed_addr = format!("0x{}", addr);
        trace!("Building contract at address {}", prefixed_addr);
        engine.build_contract(prefixed_addr.as_str(), rom.as_slice())?;
    }

    let run = engine.run_contract("0x0000", &block_info)?;
    t.expected.assert_eq(&run);

    Ok(())
}

pub(crate) fn stack_word(bytes: &[u8]) -> [u8; 32] {
    let mut word = [0; 32];
    word[..bytes.len()].copy_from_slice(bytes);
    word
}

fn new_test_block_info() -> exec::BlockInfo {
    let hash = [
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
        25, 26, 27, 28, 29, 30, 31,
    ];
    let hash_history = new_test_block_info_hash_history();
    let coinbase = [1, 0];
    exec::BlockInfo::new(
        42,
        100,
        100,
        1717354173,
        5_000_000,
        1_000_000,
        1,
        hash,
        hash_history,
        coinbase,
    )
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
