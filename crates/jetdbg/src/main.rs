use clap::{Parser, Subcommand};
use inkwell::context::Context;
use log::info;
use simple_logger::SimpleLogger;
use thiserror::Error;

use jet::instructions::Instruction;
use jet_runtime::{Address, exec};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    cmd: Option<Commands>,

    #[arg(short, long)]
    log_level: Option<log::LevelFilter>,

    #[arg(short, long)]
    mode: Option<jet::builder::env::Mode>,

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
    emit_llvm: Option<bool>,

    #[arg(short, long, action)]
    assert: Option<bool>,
}

#[derive(Error, Debug)]
enum Error {
    #[error(transparent)]
    Clap(#[from] clap::Error),
    #[error(transparent)]
    Build(#[from] jet::builder::Error),
    #[error(transparent)]
    Engine(#[from] jet::engine::Error),
    #[error("Failed to initialize logger: {0}")]
    Logger(#[from] log::SetLoggerError),
}

fn build_cmd(args: BuildArgs) -> Result<(), Error> {
    let build_opts = jet::builder::env::Options::new(
        args.mode.unwrap_or(jet::builder::env::Mode::Debug),
        args.emit_llvm.unwrap_or(true),
        args.assert.unwrap_or(true),
    );

    // Alice calls Bob and copies the return data.
    // CALL stack (top-to-bottom at call time): gas, addr, value, argsOffset, argsLen, retOffset, retLen
    let alice_rom = [
        Instruction::PUSH1.opcode(), // retLen: output len = 10 bytes
        0x0A,
        Instruction::PUSH1.opcode(), // retOffset: output offset = 0
        0x00,
        Instruction::PUSH1.opcode(), // argsLen: input len = 0
        0x00,
        Instruction::PUSH1.opcode(), // argsOffset: input offset = 0
        0x00,
        Instruction::PUSH1.opcode(), // value = 0
        0x00,
        Instruction::PUSH20.opcode(), // addr: Bob's 20-byte address
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,
        0x01,
        Instruction::PUSH1.opcode(), // gas = 0
        0x00,
        Instruction::CALL.opcode(),
        Instruction::RETURNDATASIZE.opcode(),
        Instruction::PUSH1.opcode(), // len = 2
        0x02,
        Instruction::PUSH1.opcode(), // src offset = 0
        0x00,
        Instruction::PUSH1.opcode(), // dest offset = 2
        0x02,
        Instruction::RETURNDATACOPY.opcode(),
    ];

    let bob_rom = [
        Instruction::PUSH1.opcode(),
        0xFF,
        Instruction::PUSH1.opcode(),
        0x01,
        Instruction::MSTORE.opcode(), // Mem[0x01] = 0xFF
        Instruction::PUSH1.opcode(),
        0xFF,
        Instruction::PUSH1.opcode(),
        0x0A,
        Instruction::MSTORE.opcode(), // Mem[0x0A] = 0xFF
        Instruction::PUSH1.opcode(),
        0x0A,
        Instruction::PUSH1.opcode(),
        0x00,
        Instruction::RETURN.opcode(), // return mem[0x00..0x0A]
    ];

    // Create the LLVM JIT engine
    let context = Context::create();
    let mut engine = jet::engine::Engine::new(&context, build_opts)?;

    // Build the contracts
    let alice_addr: Address = "0x1234".parse().expect("valid address");
    let bob_addr: Address = "0x0000000000000000000000000000000000000001"
        .parse()
        .expect("valid address");
    engine.build_contract(alice_addr, alice_rom.as_slice())?;
    engine.build_contract(bob_addr, bob_rom.as_slice())?;

    // Run Alice's contract with a test block
    let block_info = new_test_block_info();
    let run = engine.run_contract(alice_addr, &block_info)?;
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
    logger.init()?;

    // Dispatch command, forwarding top-level flags when no subcommand is given.
    match cli.cmd {
        Some(Commands::Build(args)) => build_cmd(args),
        None => build_cmd(BuildArgs {
            mode: cli.mode,
            emit_llvm: cli.emit_llvm,
            assert: cli.assert,
        }),
    }?;

    Ok(())
}

fn new_test_block_info() -> exec::BlockInfo {
    let hash = [
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
        25, 26, 27, 28, 29, 30, 31,
    ];
    let hash_history = new_test_block_info_hash_history();
    let coinbase = Address::new([1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);

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

    for (i, hash) in hash_history.iter_mut().enumerate() {
        hash[31] = i as u8;
    }

    hash_history
}
