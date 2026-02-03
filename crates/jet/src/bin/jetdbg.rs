use clap::{Parser, Subcommand};
use inkwell::context::Context;
use log::info;
use simple_logger::SimpleLogger;
use thiserror::Error;

use jet::instructions::Instruction;
use jet_runtime::{self, exec};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    cmd: Option<Commands>,

    #[arg(short, long)]
    log_level: Option<log::LevelFilter>,

    #[arg(short, long)]
    mode: Option<jet::builder::env::Mode>,

    #[arg(short, long, action)]
    use_vstack: Option<bool>,

    #[arg(short, long)]
    emit_llvm: Option<bool>,

    #[arg(short, long, action)]
    assert: Option<bool>,
}

#[derive(Parser, Debug, Default, Clone)]
struct BuildArgs {
    #[arg(short, long)]
    mode: Option<jet::builder::env::Mode>,

    #[arg(short, long, action)]
    use_vstack: Option<bool>,

    #[arg(short, long, action)]
    emit_llvm: Option<bool>,

    #[arg(short, long, action)]
    assert: Option<bool>,
}

#[derive(Error, Debug)]
#[error(transparent)]
enum Error {
    Clap(#[from] clap::Error),
    Build(#[from] jet::builder::Error),
    Engine(#[from] jet::engine::Error),
}

fn build_cmd(args: BuildArgs) -> Result<(), Error> {
    let build_opts = jet::builder::env::Options::new(
        args.mode.unwrap_or(jet::builder::env::Mode::Debug),
        args.use_vstack.unwrap_or(false),
        args.emit_llvm.unwrap_or(true),
        args.assert.unwrap_or(true),
    );

    // let alice_rom = [
    //     Instruction::PUSH0.opcode(),
    //     Instruction::KECCAK256.opcode(),
    //     Instruction::PUSH1.opcode(),
    //     0x01,
    //     Instruction::PUSH1.opcode(),
    //     0x02,
    //     Instruction::ADD.opcode(),
    // ];

    // let alice_rom = [
    //     Instruction::PUSH1.opcode(),
    //     0xFF,
    //     Instruction::PUSH1.opcode(),
    //     0x02,
    //     Instruction::MSTORE.opcode(),
    //     Instruction::PUSH1.opcode(),
    //     0x00,
    //     Instruction::MLOAD.opcode(),
    //     Instruction::PUSH2.opcode(),
    //     0xFF,
    //     0xFF,
    //     Instruction::PUSH1.opcode(),
    //     0x00,
    //     Instruction::MSTORE8.opcode(),
    //     Instruction::PUSH1.opcode(),
    //     0x00,
    //     Instruction::MLOAD.opcode(),
    // ];

    // let alice_rom = [
    //     Instruction::PUSH2.opcode(),
    //     0xFF,
    //     0x00,
    //     Instruction::PUSH2.opcode(),
    //     0x00,
    //     0xFF,
    //     Instruction::ADD.opcode(),
    //     Instruction::PUSH1.opcode(),
    //     1,
    //     Instruction::ADD.opcode(),
    //     Instruction::DUP1.opcode(),
    //     Instruction::PUSH1.opcode(),
    //     2,
    //     Instruction::MUL.opcode(),
    //     Instruction::PC.opcode(),
    //     // Instruction::BYTE.opcode(),
    // ];

    // let alice_rom = [
    //     Instruction::PUSH2.opcode(),
    //     0xFF,
    //     0x00,
    //     Instruction::PUSH2.opcode(),
    //     0x00,
    //     0xFF,
    //     Instruction::ADD.opcode(),
    //     Instruction::PUSH1.opcode(),
    //     1,
    //     Instruction::ADD.opcode(),
    //     // Instruction::BYTE.opcode(),
    // ];

    let alice_rom = [
        Instruction::PUSH1.opcode(), // Output len
        0x0A,
        Instruction::PUSH1.opcode(), // Output offset
        0x00,
        Instruction::PUSH1.opcode(), // Input len
        0x00,
        Instruction::PUSH1.opcode(), // Input offset
        0x00,
        Instruction::PUSH1.opcode(), // Value
        0x00,
        Instruction::PUSH2.opcode(), // Address
        0x00,
        0x01,
        Instruction::PUSH1.opcode(), // Gas
        0x00,
        Instruction::CALL.opcode(), // Mem: 0x00FF
        Instruction::RETURNDATASIZE.opcode(),
        Instruction::PUSH1.opcode(), // Len
        0x02,
        Instruction::PUSH1.opcode(), // Src offset
        0x00,
        Instruction::PUSH1.opcode(), // Dest offset
        0x02,
        Instruction::RETURNDATACOPY.opcode(), // Mem: 0x00FF00FF0000000000000000
    ];

    let bob_rom = [
        Instruction::PUSH1.opcode(),
        0xFF,
        Instruction::PUSH1.opcode(),
        0x01,
        Instruction::MSTORE.opcode(), // Mem: 0x00FF
        Instruction::PUSH1.opcode(),
        0xFF,
        Instruction::PUSH1.opcode(),
        0x0A,
        Instruction::MSTORE.opcode(), // Mem: 0x00FF0000000000000000FF
        Instruction::PUSH1.opcode(),
        0x0A,
        Instruction::PUSH1.opcode(),
        0x00,
        Instruction::RETURN.opcode(), // Return 0x00FF0000000000000000
    ];

    // Create the LLVM JIT engine
    let context = Context::create();
    let mut engine = jet::engine::Engine::new(&context, build_opts)?;

    // Build the contract
    engine.build_contract("0x1234", alice_rom.as_slice())?;
    engine.build_contract("0x0001", bob_rom.as_slice())?;

    // Run the contract with a test block
    let block_info = new_test_block_info();
    let run = engine.run_contract("0x1234", &block_info)?;
    info!("{}", run);

    Ok(())
}

#[derive(Subcommand, Debug, Clone)]
enum Commands {
    Build(BuildArgs),
}

fn main() -> Result<(), Error> {
    let cli = Cli::parse();

    // Configure logger
    let logger = match cli.log_level {
        Some(level) => SimpleLogger::new().with_level(level),
        None => SimpleLogger::new().with_level(log::LevelFilter::Trace),
    };
    logger.init().unwrap();

    // Dispatch command
    match cli.cmd {
        Some(Commands::Build(args)) => build_cmd(args),
        None => build_cmd(BuildArgs::default()),
    }?;

    Ok(())
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
