# Jet Architecture Documentation

## Table of Contents

1. [Project Overview](#project-overview)
2. [History and Genesis](#history-and-genesis)
3. [Design Motivation: The MEV Use Case](#design-motivation-the-mev-use-case)
4. [System Architecture](#system-architecture)
5. [Core Components](#core-components)
6. [The Stack Machine to Register Machine Translation](#the-stack-machine-to-register-machine-translation)
7. [Compilation Pipeline](#compilation-pipeline)
8. [Memory Model](#memory-model)
9. [Control Flow and Jump Tables](#control-flow-and-jump-tables)
10. [Runtime Function Architecture](#runtime-function-architecture)
11. [Symbol Management and Linking](#symbol-management-and-linking)
12. [Code Organization](#code-organization)
13. [File-by-File Summary](#file-by-file-summary)
14. [Implementation Patterns](#implementation-patterns)
15. [Testing Strategy](#testing-strategy)
16. [Known Limitations and Future Work](#known-limitations-and-future-work)
17. [Quick Reference](#quick-reference)

---

## Project Overview

### What is JET?

JET (JIT for EVM Transactions) is an LLVM-based JIT compiler for the Ethereum Virtual Machine. Instead of interpreting EVM bytecode instruction-by-instruction, JET compiles contracts to native machine code via LLVM, enabling significant performance improvements for compute-intensive operations.

The system compiles Ethereum Virtual Machine (EVM) bytecode into LLVM IR and then executes that IR using LLVM's JIT infrastructure (via inkwell). The compiler emits one LLVM function per contract. Execution runs that function with a pointer to a runtime `Context`, returning a `ReturnCode`.

### Key Value Proposition

1. **Performance**: Native code execution vs. interpretation
2. **Optimization**: LLVM's optimization passes (constant folding, dead code elimination, etc.)
3. **Portability**: LLVM IR is architecture-independent; can target x86_64, ARM, etc.
4. **MEV Use Case**: Ideal for scenarios where the same contract is executed thousands of times with warm data

### Technology Stack

- **Language**: Rust (edition 2024)
- **LLVM Version**: 22.1
- **LLVM Bindings**: `inkwell` crate
- **Target**: ORC (On-Request Compilation) JIT

---

## History and Genesis

The project originated in 2020 at Ava Labs, where the initial concept was to build a native-machine smart contract platform that went beyond EVM optimization to rethink the execution substrate entirely.

After the internal project was discontinued due to organizational changes, the concept was reimplemented from scratch in Rust. This clean-room rewrite served multiple purposes: learning Rust, ensuring complete IP provenance clarity, and signaling a fresh implementation with no connection to prior internal work. The Rust implementation also proved well-suited to the problem domain, with explicit ownership semantics for JIT lifetimes and intentional use of unsafe code around executable memory.

The project was renamed to "Jet," a name that naturally captures the "EVM in a JIT" concept while suggesting speed and providing short, composable naming for components like JetBuilder (IR construction) and JetEngine (ORC instantiation and execution).

---

## Design Motivation: The MEV Use Case

A key insight driving the project came from observing MEV (Maximal Extractable Value) operations. MEV searchers commonly instantiate a local EVM to simulate contract executions—for example, calculating Uniswap trade outcomes by crafting transactions that call relevant pool functions and executing them locally.

The standard objection to EVM JIT compilation—that I/O bottlenecks in state management dominate execution time—doesn't apply in this context. MEV searchers load all relevant state data into memory once, then execute the same functions thousands of times over in-memory data. This scenario is the ideal use case for JIT compilation: amortizing compilation costs over many executions with warm data.

This extends to a broader architectural pattern: contracts could be lowered directly to shared libraries that any program could link against. For instance, Uniswap utility contracts could be compiled to native code libraries, allowing direct programmatic access to their functionality without EVM overhead.

---

## System Architecture

### High-Level Data Flow

```
┌─────────────────────────────────────────────────────────────────────────┐
│                           JET Architecture                               │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────────────────┐   │
│  │ EVM Bytecode │───▶│  JetBuilder  │───▶│     LLVM Module          │   │
│  │ (contract)   │    │  (compiler)  │    │ (IR + declarations)      │   │
│  └──────────────┘    └──────────────┘    └──────────┬───────────────┘   │
│                                                      │                   │
│                                                      ▼                   │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────────────────┐   │
│  │   Result     │◀───│  JetEngine   │◀───│    ORC JIT Engine        │   │
│  │ (ReturnCode) │    │  (executor)  │    │ (native compilation)     │   │
│  └──────────────┘    └──────────────┘    └──────────────────────────┘   │
│                              │                                           │
│                              ▼                                           │
│                      ┌──────────────┐                                    │
│                      │ jet_runtime  │                                    │
│                      │ (builtins)   │                                    │
│                      └──────────────┘                                    │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

### Five Cooperating Crates

1. **Compiler (`crates/jet`)**: Parses bytecode, discovers basic blocks, plans the symbolic stack, emits LLVM IR for each opcode with per-block gas charges, and drives the JIT through `Engine`.
2. **Runtime (`crates/jet_runtime`)**: Defines the execution `Context`, `CallInfo` and `BlockInfo`, provides builtin functions for memory expansion, calls, calldata and wide arithmetic, and generates the runtime IR module via `RuntimeBuilder`.
3. **Shared types (`crates/jet_ir`)**: Unified LLVM type registry (`jet_ir::Types`) and constants shared by both compiler and runtime to prevent layout drift.
4. **Push macros (`crates/jet_push_macros`)**: Proc-macro crate generating `PUSH0`..`PUSH32` bytecode helper macros for tests.
5. **Debug CLI (`crates/jetdbg`)**: Compiles two sample contracts, runs a CALL between them, and prints the generated IR.

### Crate Structure

```
jet/
├── crates/
│   ├── jet/                    # Main compiler crate
│   │   ├── src/
│   │   │   ├── lib.rs          # Module exports
│   │   │   ├── instructions.rs # EVM opcode definitions and bytecode iterator
│   │   │   ├── builder/        # IR construction
│   │   │   │   ├── mod.rs      # Error types
│   │   │   │   ├── contract.rs # Block discovery, symbolic planning, dispatch, jumps
│   │   │   │   ├── env.rs      # Options, Mode, StackMode, Env, Symbols
│   │   │   │   ├── gas.rs      # Static costs and dynamic gas kinds
│   │   │   │   ├── manager.rs  # Build orchestration
│   │   │   │   ├── ops.rs      # Opcode emitters, memory regions, gas charges
│   │   │   │   ├── stack.rs    # StackBackend trait, runtime and symbolic backends
│   │   │   │   └── symbolic.rs # SymbolicStack of LLVM values
│   │   │   └── engine/         # JIT execution
│   │   │       └── mod.rs      # Engine wrapper
│   │   └── tests/              # Integration tests (test_roms.rs, roms/mod.rs harness)
│   │
│   ├── jet_ir/                 # Shared IR types and constants
│   │   └── src/
│   │       ├── lib.rs          # Re-exports
│   │       ├── constants.rs    # EVM + Jet runtime constants
│   │       └── types.rs        # Unified LLVM type registry
│   │
│   ├── jet_push_macros/        # Proc-macro crate for PUSH opcodes
│   │   └── src/
│   │       └── lib.rs          # generate_push_macros! proc-macro
│   │
│   ├── jet_runtime/            # Runtime support crate
│   │   └── src/
│   │       ├── lib.rs          # Re-exports (including jet_ir::*)
│   │       ├── address.rs      # Address newtype ([u8; 20])
│   │       ├── call_info.rs    # Per-frame CallInfo
│   │       ├── exec.rs         # Execution context, BlockInfo, ReturnCode
│   │       ├── builtins.rs     # Extern "C" runtime functions
│   │       ├── runtime_builder.rs  # Programmatic IR generation
│   │       ├── symbols.rs      # Symbol name constants
│   │       ├── layout_tests.rs # Rust/LLVM layout checks
│   │       └── binding/        # Display implementations
│   │
│   └── jetdbg/                 # Debug CLI
│       └── src/
│           └── main.rs
```

### Tiered Compilation Strategy

Jet employs compilation at two levels:

**EVM to LLVM IR Phase**: A mixture of eager and lazy compilation. The system can analyze contract execution frequency to determine compilation priorities. Contracts can be identified as frequently-executed by examining their deployment code—Solidity's optimizer makes size-versus-execution-frequency tradeoffs that signal expected usage patterns. Popular contracts can be pre-compiled during initialization.

**IR to Native Machine Code Phase**: ORC not only performs initial compilation but can actively analyze executing code and recompile with different optimizations. The database stores LLVM IR rather than machine code, making it portable across architectures—the same compiled IR can be moved between systems and will lower to the appropriate machine code at runtime.

---

## Core Components

### 1. Instruction Module (`instructions.rs`)

**Purpose**: Define EVM opcodes and provide bytecode iteration.

**Key Types**:

```rust
enum Instruction {
    STOP = 0x00,
    ADD = 0x01,
    // ... all 150+ EVM opcodes
}

enum IteratorItem {
    Instr(usize, Instruction),    // (pc, opcode)
    PushData(usize, [u8; 32]),    // (pc, data in little-endian)
    Invalid(usize),               // Invalid opcode at pc
}
```

**Critical Behavior**: The iterator converts PUSH data from big-endian (EVM native) to little-endian (x86/ARM native) during parsing. This is important for subsequent LLVM operations.

**Bytecode Decoding**: `instructions::Iterator` walks raw bytecode and emits:
- `Instr(pc, Instruction)` for standard opcodes
- `PushData(pc, [u8; 32])` for PUSH0..PUSH32, with bytes reversed to convert big-endian immediates into Jet's little-endian internal word

### 2. Environment (`env.rs`)

**Purpose**: Build-time configuration and the LLVM compilation environment.

**Key Structures**:

```rust
pub struct Options { mode: Mode, emit_llvm: bool, assert: bool, stack_mode: StackMode }
pub enum Mode { Debug, Release }
pub enum StackMode { RuntimeOnly, SymbolicPreferred }   // RuntimeOnly is the default

struct Symbols<'ctx> {
    jit_engine: GlobalValue,
    stack_push_word, stack_push_ptr, stack_pop, stack_peek, stack_swap,
    mem_expand, gas_failure_static,
    contract_call, contract_call_values, contract_call_return_data_copy,
    call_data_load, keccak256, exp, addmod, mulmod,
}
```

`Env` wraps the LLVM `Context`, `Module`, the `jet_ir::Types` registry and `Symbols`. `Options::with_stack_mode` selects the stack backend for a compilation session.

### 3. Contract Builder (`contract.rs`)

**Purpose**: The core compilation logic that transforms EVM bytecode into LLVM IR.

**Key Types**:

```rust
struct Registers<'ctx> {
    exec_ctx: PointerValue,      // Pointer to execution context (function parameter)
    block_info: PointerValue,    // Pointer to block info (function parameter)
    jump_ptr: PointerValue,      // Pointer to jump target
    return_offset: PointerValue, // Return data offset
    return_length: PointerValue, // Return data length
    sub_call: PointerValue,      // Sub-call context pointer
    call_info: PointerValue,     // Per-frame call info pointer
    gas_remaining: PointerValue, // Gas remaining slot
}

struct BuildCtx<'ctx, 'b, S: StackBackend<'ctx>> {
    env: &Env,
    builder: &Builder,
    registers: Registers,
    func: FunctionValue,
    stack: S,                    // RuntimeStackBackend or SymbolicStackBackend
    gas_remaining: Cell<Option<IntValue>>,  // Current block's SSA gas value
}

struct CodeBlock<'ctx, 'b> {
    offset: usize,               // Bytecode offset
    rom: &[u8],                  // Bytecode slice
    basic_block: BasicBlock,     // LLVM basic block
    is_jumpdest: bool,           // Is a jump destination
    terminates: bool,            // Has terminator instruction
}
```

`find_code_blocks` partitions the bytecode, `build_non_jump_instruction` dispatches every non-jump opcode to its emitter for both backends, and `apply_abstract_instruction` models each opcode's stack effect for the symbolic planner.

### 4. Operations (`ops.rs`)

**Purpose**: Implement each EVM opcode as LLVM IR generation.

**Pattern**: Each emitter is generic over `S: StackBackend` and follows:
1. Pop operands as `i256` SSA values through `bctx.stack`
2. Perform the LLVM operation, or call a runtime builtin
3. Push the result through `bctx.stack`

Memory-touching emitters first obtain a `MemoryRegion` from `expand_memory_region` (ADR 006), and dynamic gas is charged before any side effect.

### 5. Engine (`engine/mod.rs`)

**Purpose**: Wrap the compilation and execution pipeline.

**Key Methods**:

```rust
impl Engine {
    fn new(context, opts) -> Result<Self>;                          // Create with options
    fn build_contract(addr, rom) -> Result<()>;                     // Compile bytecode
    fn run_contract(call_info, block_info) -> Result<ContractRun>;  // Execute the contract at call_info.address
}
```

### 6. Execution Context (`exec.rs`)

**Purpose**: Runtime state for contract execution.

```rust
#[repr(C)]
pub struct Context {
    stack_ptr: u32,              // Stack depth (next free slot)
    jump_ptr: u32,               // Dynamic jump target (runtime backend)
    return_off: u32,             // Return data offset (window in memory)
    return_len: u32,             // Return data length
    sub_call: Option<Box<Context>>,  // Nested call context for CALL
    stack: [Word; 1024],         // The EVM stack
    memory_ptr: *mut u8,         // Heap-allocated EVM memory (ADR-002)
    memory_len: u32,             // Used memory length
    memory_cap: u32,             // Allocated capacity
    call_info: *mut CallInfo,    // Per-frame call context
    gas_remaining: u64,          // Gas left for this frame
    gas_failure: Option<Box<GasFailure>>,  // Out-of-gas diagnostics
}
```

**Important**: This struct is passed by pointer into JIT-compiled contract functions. The compiler assumes a specific field order when performing struct GEPs (getelementptr operations), and `layout_tests.rs` checks that Rust and `jet_ir::Types` agree.

`CallInfo` (`call_info.rs`) carries the calldata pointer and length, the frame's address, origin, caller and value, and the gas limit. It is required by `Engine::run_contract` and is created for the callee by the CALL builtin.

### 7. BlockInfo (`exec.rs`)

**Purpose**: Carries chain data exposed to the block information opcodes.

Fields: number, difficulty, gas_limit, timestamp, base_fee, blob_base_fee, chain_id, hash (current block hash), hash_history (last 256 hashes), coinbase.

`COINBASE`, `TIMESTAMP`, `NUMBER`, `DIFFICULTY`, `GASLIMIT`, `CHAINID`, `BASEFEE` and `BLOBBASEFEE` load their field directly through the `block_info` register; `BLOCKHASH` consumes its block number and reads `hash_history`, pushing zero outside the window.

### 8. ReturnCode (`exec.rs`)

**Purpose**: Encode execution outcomes.

- **Negative values**: Jet-level failures (`InvalidJumpBlock` = -1, `StackUnderflow` = -2, `StackOverflow` = -3)
- **0..63**: EVM-level success (`ImplicitReturn` = 0, `ExplicitReturn` = 1, `Stop` = 2)
- **64+**: EVM-level failure (`Revert` = 64, `Invalid` = 65, `JumpFailure` = 66, `OutOfGas` = 67)

Compiled functions always return one of these values.

### 9. Builtins (`builtins.rs`)

**Purpose**: Rust functions callable from compiled LLVM IR.

All functions use the `extern "C"` ABI and are marked `unsafe`. By symbol name:

- `jet.mem.expand`: memory expansion with overflow checks (ADR 005)
- `jet.contract.call`, `jet.contract.call.values`, `jet.contracts.call_return_data_copy`: JIT-to-JIT calls and return data
- `jet.call.dataload`: `CALLDATALOAD` word reads
- `jet.ops.keccak256`, `jet.ops.exp`, `jet.ops.addmod`, `jet.ops.mulmod`: hashing and wide arithmetic
- `jet.gas.failure.static`: records out-of-gas diagnostics

These are declared in the runtime IR module and mapped at JIT creation with `ExecutionEngine::add_global_mapping`. Stack helpers (`jet.stack.*`) and simple memory helpers (`jet.mem.store.*`, `jet.mem.load`) are defined in IR by `RuntimeBuilder` rather than in Rust.

---

## The Stack Machine to Register Machine Translation

### The Fundamental Challenge

The EVM is a **stack machine**: operations implicitly pop operands from a stack and push results back. Example:

```
PUSH1 0x01    ; stack: [1]
PUSH1 0x02    ; stack: [1, 2]
ADD           ; stack: [3]
```

LLVM IR is a **register machine** with SSA (Static Single Assignment): every value is assigned exactly once to a virtual register.

```llvm
%a = i256 1
%b = i256 2
%c = add i256 %a, %b
```

### JET's Solution: Two Stack Backends

Opcode emitters are written once against the `StackBackend` trait in `stack.rs`, and the backend is chosen per compilation with `Options::with_stack_mode`:

- **`RuntimeStackBackend`** (default): the EVM stack lives in `Context.stack` and every push, pop, peek and swap is a call to an IR-defined runtime helper (`jet.stack.*`). The helpers bounds-check, so underflow and overflow return `StackUnderflow` or `StackOverflow`.
- **`SymbolicStackBackend`**: stack slots are LLVM SSA values held in a `SymbolicStack` during IR construction. A fixed-point abstract interpretation over the code blocks plans the control flow first, creating entry phis per block variant and specialising blocks by incoming stack height. The stack is materialized into `Context.stack` only at contract exits, so the observable context matches the runtime backend. Planning is bounded (64 entry states per block, 4096 total) and falls back to the runtime backend when exceeded.

```rust
pub(crate) fn add<'ctx, S: StackBackend<'ctx>>(bctx: &BuildCtx<'ctx, '_, S>) -> Result<(), Error> {
    let (a, b) = bctx.stack.pop_2(bctx)?;
    let result = bctx.builder.build_int_add(a, b, "add_result")?;
    bctx.stack.push_word(bctx, result)
}
```

Only `JUMP` and `JUMPI` lowering differs between backends. See [`symbolic-stack.md`](symbolic-stack.md) and ADR 007 for the planner, fault exits and limits.

### Why This Design?

The runtime backend keeps correctness simple and serves as the differential oracle: every rom test runs under both backends. The symbolic backend removes the per-opcode runtime calls and memory traffic on the paths it can plan, which is where the performance is.

---

## Compilation Pipeline

### Phase 1: Bytecode Parsing

```rust
// instructions.rs - Iterator yields parsed opcodes and data
for item in instructions::Iterator::new(bytecode) {
    match item {
        IteratorItem::Instr(pc, Instruction::ADD) => { /* handle ADD */ }
        IteratorItem::PushData(pc, data) => { /* handle PUSH data */ }
        IteratorItem::Invalid(pc) => { /* error */ }
    }
}
```

### Phase 2: Basic Block Discovery

```rust
// contract.rs - find_code_blocks()
fn find_code_blocks(env, func, bytecode) -> CodeBlocks {
    // Creates LLVM basic blocks using a single linear scan:
    for item in instructions::Iterator::new(bytecode) {
        match instr {
            STOP | RETURN | REVERT | JUMP => {
                // Terminates current block
                current_block.set_terminates();
            }
            JUMPI => {
                // Conditional terminator: ends block, creates new block for fall-through
                current_block = blocks.add(pc + 1, create_bb());
            }
            JUMPDEST => {
                // Ends previous block, starts new jump target block
                // Marks the start of a jump target block
                current_block = blocks.add(pc + 1, create_bb());
                current_block.set_is_jumpdest();
            }
        }
    }
}
```

Each block captures the slice of ROM it covers and whether it terminates.

### Phase 3: IR Generation

```rust
// contract.rs - build_contract_body()
fn build_contract_body(bctx, code_blocks) {
    // Iterates discovered blocks and emits instructions via builder::ops
    for code_block in code_blocks.iter() {
        if code_block.is_jumpdest() {
            jump_cases.push((offset, basic_block));
        }

        build_code_block(bctx, code_block, jump_block, following_block)?;

        if !code_block.terminates() {
            // Add implicit branch to next block
            // Wires fallthrough to the next block when a block does not terminate
            builder.build_unconditional_branch(next_block);
        }
    }

    // Emits a shared jump-table block if any JUMPDEST exists
    build_jump_table(bctx, jump_block, jump_cases);
}
```

### Phase 4: Jump Table Construction

```rust
// contract.rs - build_jump_table()
fn build_jump_table(bctx, jump_block, jump_cases) {
    // Create failure block for invalid jumps
    builder.position_at_end(jump_failure_block);
    builder.build_return(ReturnCode::JumpFailure);

    // Build switch statement
    builder.position_at_end(jump_block);
    let jump_value = builder.build_load(jump_ptr);
    builder.build_switch(jump_value, jump_failure_block, jump_cases);
}
```

### Phase 5: JIT Compilation and Execution

```rust
// engine/mod.rs
fn run_contract(&self, addr, block_info) -> ContractRun {
    // Create JIT engine
    let jit = module.create_jit_execution_engine(OptimizationLevel::None);

    // Link runtime functions
    self.link_in_runtime(&jit);

    // Look up and call contract function
    let contract_fn = jit.get_function(mangle_contract_fn(addr));
    let ctx = Context::new();
    let result = contract_fn.call(&ctx);

    ContractRun::new(result, ctx)
}
```

### Gas Accounting

Gas is charged per basic block. `gas.rs` holds the Osaka static cost of every opcode and classifies the ones with dynamic costs (`DynamicGas`). For each block, the static costs of the instructions up to the next dynamic charge are summed at compile time and emitted as one affordability check and one subtraction. Gas values flow in SSA across block edges with phis at joins and backedges. Dynamic costs for memory expansion, `KECCAK256`, `EXP`, `RETURNDATACOPY` and `CALL` memory are computed in IR and charged before the operation mutates anything. Out-of-gas returns `ReturnCode::OutOfGas`, zeroes `gas_remaining`, and stores a `GasFailure` (pc, available, required) in the context. `GAS` pushes the remaining gas after its own charge.

Gas is not yet forwarded to callees; `CALL` runs the callee with an unbounded limit.

---

## Memory Model

### Execution Context Layout

The `Context` struct uses **pointer-based memory** (ADR-002): memory is heap-allocated and referenced by pointer, not stored inline. This matches EVM semantics (unbounded growth) and eliminates layout drift between Rust and generated IR.

```
┌─────────────────────────────────────────────────────────────────┐
│                        Context (repr(C))                         │
├─────────────────────────────────────────────────────────────────┤
│  stack_ptr: u32      │ Current stack depth (0-1024)             │
│  jump_ptr: u32       │ Target offset for dynamic JUMP           │
│  return_off: u32     │ Return data start offset in memory       │
│  return_len: u32     │ Return data length in bytes              │
│  sub_call: Option<Box<Context>>  │ Nested call context          │
├─────────────────────────────────────────────────────────────────┤
│  stack: [[u8; 32]; 1024]                                        │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │ Word 0   │ Word 1   │ Word 2   │ ... │ Word 1023            ││
│  │ [32 bytes each, little-endian]                              ││
│  └─────────────────────────────────────────────────────────────┘│
├─────────────────────────────────────────────────────────────────┤
│  memory_ptr: *mut u8  │ Pointer to heap-allocated memory buffer │
│  memory_len: u32      │ Used memory length                       │
│  memory_cap: u32      │ Allocated capacity                       │
├─────────────────────────────────────────────────────────────────┤
│  call_info: *mut CallInfo │ Frame address, origin, caller,       │
│                           │ value, calldata and gas limit        │
│  gas_remaining: u64       │ Gas left for this frame              │
│  gas_failure: Option<Box<GasFailure>> │ Out-of-gas diagnostics   │
└─────────────────────────────────────────────────────────────────┘

LLVM field indices (for GEP operations):
  0: stack_ptr, 1: jump_ptr, 2: return_off, 3: return_len,
  4: sub_call, 5: stack, 6: memory_ptr, 7: memory_len, 8: memory_cap,
  9: call_info, 10: gas_remaining, 11: gas_failure
```

Memory is initially allocated as `WORD_SIZE_BYTES * MEMORY_INITIAL_SIZE_WORDS` bytes (32 KB) with 32-byte alignment, and freed in `Context::drop`. The `jet_ir::Types` struct defines an identical layout in LLVM IR so that generated code and Rust agree on every field offset.

### Word Representation

- **Size**: 32 bytes (256 bits)
- **Endianness**: Little-endian storage (converted from EVM big-endian during parsing)
- **LLVM Type**: `i256` for arithmetic, `[32 x i8]` for byte access

EVM immediates are big-endian, but Jet stores words as little-endian in memory. The byte iterator reverses PUSH data, and the BYTE opcode reverses its index (`31 - idx`) to match this internal representation.

### Stack Operations

```
Stack Pointer (stack_ptr) points to next free slot:

stack_ptr = 3

  ┌─────┬─────┬─────┬─────┬─────┬─────┐
  │  A  │  B  │  C  │     │     │ ... │
  └─────┴─────┴─────┴─────┴─────┴─────┘
    [0]   [1]   [2]   [3]
                       ↑
                  stack_ptr (next write position)

PUSH D: stack[3] = D; stack_ptr = 4
POP:    stack_ptr = 3; return stack[2] (C)
DUP2:   push(stack[stack_ptr - 2])  // Copy B to top
SWAP1:  swap(stack[stack_ptr-1], stack[stack_ptr-2])
```

---

## Control Flow and Jump Tables

### Static Vs Dynamic Jumps

**Static Jumps** (JUMPI fall-through): The compiler knows both possible destinations at compile time.

**Dynamic Jumps** (JUMP, JUMPI taken branch): The target is a runtime value on the stack.

### Jump Table Implementation

In runtime mode all dynamic jumps go through a central `jump_block` (symbolic mode is described under JUMPI below):

1. `JUMP` / `JUMPI` store the target into `exec_ctx.jump_ptr`
2. Control branches to the shared jump block
3. The jump block switches on `jump_ptr` to the target block
4. If no case matches, the function returns `ReturnCode::JumpFailure`

This keeps target validation centralized and avoids indirect branches.

```
                    ┌─────────────────┐
                    │   JUMP opcode   │
                    │ 1. Pop target   │
                    │ 2. Store to     │
                    │    jump_ptr     │
                    │ 3. Branch to    │
                    │    jump_block   │
                    └────────┬────────┘
                             │
                             ▼
┌────────────────────────────────────────────────────────────┐
│                       jump_block                            │
│  ┌──────────────────────────────────────────────────────┐  │
│  │  %target = load i32, ptr %jump_ptr                   │  │
│  │  switch i32 %target, label %jump_failure [           │  │
│  │    i32 0x05, label %block_at_0x05                    │  │
│  │    i32 0x10, label %block_at_0x10                    │  │
│  │    i32 0x2A, label %block_at_0x2A                    │  │
│  │  ]                                                    │  │
│  └──────────────────────────────────────────────────────┘  │
└────────────────────────────────────────────────────────────┘
         │                    │                    │
         ▼                    ▼                    ▼
┌─────────────┐      ┌─────────────┐      ┌─────────────┐
│ JUMPDEST@5  │      │ JUMPDEST@16 │      │ JUMPDEST@42 │
└─────────────┘      └─────────────┘      └─────────────┘
```

### JUMPI (Conditional Jump) Implementation

In runtime mode:

```rust
let (pc, cond) = bctx.stack.pop_2(bctx)?;
let target = /* narrow i256 to i32; values above u32::MAX become a sentinel no jumpdest can match */;
bctx.builder.build_store(bctx.registers.jump_ptr, target)?;
let is_zero = bctx.builder.build_int_compare(EQ, cond, zero, "jumpi_cmp")?;
bctx.builder.build_conditional_branch(is_zero, fallthrough_block, jump_block)?;
```

In symbolic mode a `JUMPI` whose target slot carries `known_u64` metadata branches directly to the planned block variant; otherwise the taken edge is a site-local `switch` over every `JUMPDEST` variant at the current stack height. Both backends narrow targets through the same helper, so wide targets fail the jump rather than wrap onto a real jumpdest.

### Control Flow Opcodes

- `PC` is baked as a constant using `code_block.offset + pc` to yield the absolute bytecode index
- `JUMP`/`JUMPI` use the shared jump block in runtime mode and planned edges or site-local switches in symbolic mode
- Bytes after `STOP`, `RETURN`, `REVERT`, `INVALID` or `JUMP` and before the next `JUMPDEST` belong to no block; they are still scanned so push data cannot fake a `JUMPDEST`

---

## Runtime Function Architecture

### Why Runtime Functions?

Some operations are too complex for inline IR generation:

- Memory bounds checking
- Dynamic memory allocation
- Hash computation (keccak256)
- Cross-contract calls

### RuntimeBuilder: Programmatic IR Generation

The static `runtime-ir/jet.ll` file has been replaced by `jet_runtime::RuntimeBuilder`, a Rust struct that generates the runtime LLVM module programmatically using `inkwell`. This eliminates the host target triple mismatch that the hand-written `.ll` file suffered from and lets the runtime IR evolve alongside Rust types without keeping two representations in sync.

`RuntimeBuilder::build()` generates the following IR functions:

| Function | Kind | Description |
|---|---|---|
| `jet.stack.push.i256` | IR-defined | Push i256 value onto stack |
| `jet.stack.push.ptr` | IR-defined | Push word from pointer onto stack |
| `jet.stack.pop` | IR-defined | Pop word pointer from stack (null on underflow) |
| `jet.stack.peek` | IR-defined | Peek at word at index without popping |
| `jet.stack.swap` | IR-defined | Swap top word with word at index |
| `jet.mem.load` | IR-defined | Load i256 from memory (returns value, not pointer) |
| `jet.mem.store.word` | IR-defined | Store 32-byte word to memory |
| `jet.mem.store.byte` | IR-defined | Store single byte to memory |
| `jet.contract.call` | Declared | Cross-contract call (implemented in `builtins.rs`) |
| `jet.ops.keccak256` | Declared | Keccak256 hash (implemented in `builtins.rs`) |
| `jet.ops.exp` | Declared | Modular exponentiation (implemented in `builtins.rs`) |
| `jet.ops.addmod` | Declared | 512-bit ADDMOD (implemented in `builtins.rs`) |
| `jet.ops.mulmod` | Declared | 512-bit MULMOD (implemented in `builtins.rs`) |
| `jet.mem.expand` | Declared | Dynamic memory expansion (implemented in `builtins.rs`) |

IR-defined functions are compiled by LLVM and benefit from standard optimization passes. Declared functions are `extern "C"` Rust functions linked via `add_global_mapping` at JIT startup.

**In `symbols.rs`**:

```rust
pub const FN_STACK_POP: &str = "jet.stack.pop";
pub const FN_MEM_STORE_WORD: &str = "jet.mem.store.word";
pub const FN_CONTRACT_CALL: &str = "jet.contract.call";
```

**In `builtins.rs`**:

```rust
pub unsafe extern "C" fn stack_pop(ctx: *mut Context) -> *const Word {
    let ctx = unsafe { ctx.as_mut() }.unwrap();
    ctx.stack_pop() as *const Word
}
```

**Linking at JIT time**:

```rust
fn link_in_runtime(&self, ee: &ExecutionEngine) {
    ee.add_global_mapping(&sym.stack_pop(), builtins::stack_pop as usize);
}
```

### Calls and Sub-contexts

`CALL` lowers to a runtime builtin that:

1. Looks up the callee function pointer via the JIT engine
2. Creates a new sub-context (`Context::init_sub_call`)
3. Executes the callee JIT function
4. Copies return data into the caller memory if requested

The call returns a small status code pushed onto the stack.

### Returns, Reverts, and Invalid

- `RETURN` writes return offset/length in the context, then returns `ReturnCode::ExplicitReturn`
- `REVERT` and `INVALID` return their respective codes

---

## Symbol Management and Linking

### Naming Convention

| Type | Pattern | Example |
|------|---------|----|
| Runtime functions | `jet.{category}.{operation}` | `jet.stack.push.i256` |
| Contract functions | `jet.contracts.{address}` | `jet.contracts.0x1234` |
| Globals | `jet.{name}` | `jet.jit_engine` |

Runtime symbols are defined in `jet_runtime::symbols`.

### Contract Address Mangling

```rust
pub fn mangle_contract_fn(address: &str) -> String {
    format!("{}{}", FN_CONTRACT_PREFIX, address)
    // "jet.contracts." + "0x1234" = "jet.contracts.0x1234"
}
```

Contract symbols are mangled with `jet.contracts.` prefix and the address string. At execution time, `jet_contract_fn_lookup` reverses address bytes and reconstructs the mangled name to lookup the function pointer.

### Cross-Contract Call Flow

```
┌──────────────────────────────────────────────────────────────────────────┐
│                        Cross-Contract Call                                │
├──────────────────────────────────────────────────────────────────────────┤
│                                                                           │
│  Contract A                         Contract B                            │
│  ┌─────────────────────┐           ┌─────────────────────┐               │
│  │ ...                 │           │ jet.contracts.0xB   │               │
│  │ CALL to 0xB ──────────────┐     │ ┌─────────────────┐ │               │
│  │                     │     │     │ │ Function body   │ │               │
│  │                     │     │     │ │ ...             │ │               │
│  │                     │     │     │ │ RETURN          │ │               │
│  └─────────────────────┘     │     │ └─────────────────┘ │               │
│                              │     └─────────────────────┘               │
│                              │                                            │
│                              ▼                                            │
│  ┌───────────────────────────────────────────────────────────────────┐   │
│  │                    jet_contract_call()                             │   │
│  │  1. Look up jet.contracts.0xB in JIT engine                       │   │
│  │  2. Create sub-call Context                                        │   │
│  │  3. Execute contract B function                                    │   │
│  │  4. Copy return data to caller's memory                           │   │
│  │  5. Return status code                                             │   │
│  └───────────────────────────────────────────────────────────────────┘   │
│                                                                           │
└──────────────────────────────────────────────────────────────────────────┘
```

---

## Code Organization

### Adding a New Opcode

See [`docs/process/new-opcode.md`](../process/new-opcode.md) for the full checklist. In short:

1. **Implement the emitter** in `ops.rs`:

   ```rust
   pub(crate) fn newop<'ctx, S: StackBackend<'ctx>>(bctx: &BuildCtx<'ctx, '_, S>) -> Result<(), Error> {
       let (a, b) = bctx.stack.pop_2(bctx)?;
       let result = bctx.builder.build_int_add(a, b, "newop_result")?;
       bctx.stack.push_word(bctx, result)
   }
   ```

2. **Dispatch it** in `build_non_jump_instruction` in `contract.rs`:

   ```rust
   Instruction::NEWOP => ops::newop(bctx),
   ```

3. **Model its stack effect** in `apply_abstract_instruction` in `contract.rs` so the symbolic planner can track heights.

4. **Add its gas cost** to `static_cost` in `gas.rs`, plus a `DynamicGas` kind if the cost depends on operands.

### Adding a New Runtime Function

See [`docs/process/new-runtime-function.md`](../process/new-runtime-function.md) for the full checklist.

---

## File-by-File Summary

### `jet/src/lib.rs`
- Module structure declaration and crate docs

### `jet/src/instructions.rs`
- Macro-based EVM opcode enum definition (`instruction!` macro)
- Implements `TryFrom<u8>`, `Display`, `opcode()` methods
- Custom `Iterator` that handles PUSH data bytes
- Converts PUSH data from big-endian to little-endian (bytes reversed)

### `jet/src/builder/mod.rs`
- Error enum for build failures
- Module declarations

### `jet/src/builder/contract.rs`
- **`Registers`**: Caches pointers into exec_ctx and block_info
- **`BuildCtx<S>`**: Wraps Env, Builder, current function, Registers, the stack backend and the block's gas value
- **`CodeBlock`** / **`CodeBlocks`**: Basic blocks with offset, ROM slice, flags, and a `jumpdest_pc -> block_index` map
- **`build()`**: Main entry point - creates function, discovers blocks, plans (symbolic mode), generates IR
- **`find_code_blocks()`**: First pass - discovers basic block boundaries, skips dead code
- **`apply_abstract_instruction()`**: Stack effect of every opcode for the symbolic planner
- **`build_non_jump_instruction()`**: Shared opcode dispatch for both backends, with static and dynamic gas charges
- **`build_jump_table()`**: The runtime-mode switch for dynamic jumps

### `jet/src/builder/env.rs`
- **`Options`**: Build configuration (mode Debug/Release, emit_llvm, assert, stack_mode)
- **`Mode`** and **`StackMode`** enums
- **`Symbols`**: Runtime function lookups, mapped to `jet_runtime::symbols`
- **`Env`**: Wraps context, module, `jet_ir::Types`, symbols

### `jet/src/builder/ops.rs`
- Emitters for each implemented EVM opcode, generic over `StackBackend`
- **`MemoryRegion`** and `expand_memory_region`: type-level guarantee that memory is expanded before access (ADR 006)
- `charge_static_gas` and the dynamic gas charge helpers
- `build_zero_guarded_value` for division-like opcodes, `truncate_to_i32` for offsets and sizes

### `jet/src/builder/stack.rs`
- **`StackBackend`** trait: `push_word`, `push_word_with_known_u64`, `pop_word`, `pop_2`, `pop_3`, `pop_7`, `peek_word`, `dup`, `swap`, `materialize_for_return`
- **`RuntimeStackBackend`**: calls the `jet.stack.*` runtime helpers
- **`SymbolicStackBackend`**: operates on a `SymbolicStack` and materializes it at exits

### `jet/src/builder/symbolic.rs`
- **`SymbolicStack`**: stack slots as LLVM `IntValue`s with optional `known_u64` metadata

### `jet/src/builder/gas.rs`
- `static_cost` table and `DynamicGas` classification per opcode

### `jet/src/builder/manager.rs`
- **`Manager`**: Wraps Env and adds functions per contract address
- Builds contract, optionally prints IR via syntect, and verifies

### `jet/src/engine/mod.rs`
- **`Engine`**: Wraps Manager, handles compilation and execution
- Calls `RuntimeBuilder::build()` to generate the runtime LLVM module
- Creates JIT execution engine
- Links `extern "C"` builtins at JIT time via `add_global_mapping`
- Executes contracts with a `CallInfo` and `BlockInfo`, returning `ContractRun`

### `jetdbg/src/main.rs`
- Debug CLI: compiles two sample contracts, runs a CALL between them, prints IR
- `--mode`, `--emit-llvm`, `--assert` and `--log-level` flags

### `jet_ir/src/constants.rs`
- Canonical constants: `WORD_SIZE_BYTES`, `STACK_SIZE_WORDS`, `ADDRESS_SIZE_BYTES`, `MEMORY_INITIAL_SIZE_WORDS`, etc.
- Single source of truth for sizes shared by compiler and runtime

### `jet_ir/src/types.rs`
- **`Types<'ctx>`**: Unified LLVM type registry built from an inkwell `Context`
- Defines all primitive types (`i8`, `i32`, `i64`, `i160`, `i256`, `ptr`)
- Defines the `exec_ctx` (12 fields, packed), `call_info` and `block_info` struct layouts, the single authoritative definitions
- Re-used by both `RuntimeBuilder` and the compiler's `env.rs` to guarantee layout consistency

### `jet_push_macros/src/lib.rs`
- **`generate_push_macros!(0..=32)`** proc-macro
- Generates `PUSH0!`, `PUSH1!(b)`, ..., `PUSH32!(b0, b1, ...)` bytecode helper macros
- Each macro takes exactly N byte arguments and emits the correct opcode + data bytes

### `jet_runtime/src/lib.rs`
- Module declarations; re-exports `jet_ir::*` (constants flow from `jet_ir`)
- Public surface: `Address`, `CallInfo`, `Result`, `RuntimeError`, `RuntimeBuilder`

### `jet_runtime/src/address.rs`
- **`Address([u8; 20])`** newtype with `#[repr(transparent)]`
- Derives `Clone`, `Copy`, `PartialEq`, `Eq`, `Hash`, `Default`
- `Display`/`Debug` emit lowercase `0x`-prefixed hex
- `FromStr`/`TryFrom<&str>` parse hex strings with optional `0x` prefix
- `From<[u8; 20]>`, `Into<[u8; 20]>`, `AsRef<[u8]>` for zero-cost interop

### `jet_runtime/src/call_info.rs`
- **`CallInfo`**: calldata pointer and length, address, origin, caller, value, gas limit
- `#[repr(C)]`, matched by `jet_ir::Types::call_info`

### `jet_runtime/src/exec.rs`
- **`Word`**: 32-byte array type alias (`[u8; 32]`)
- **`Context`**: Execution context with pointer-based memory (ADR-002), call info, gas remaining and gas failure diagnostics
- **`BlockInfo`**: EVM block metadata struct
- **`ReturnCode`**: Enum for execution results (EVM and Jet-level success/failure)
- **`ContractRun`**: Wraps result and context
- **`ContractFunc`**: Function pointer type for compiled contracts

### `jet_runtime/src/builtins.rs`
- Unsafe `extern "C"` functions for operations that need Rust stdlib/deps
- Contract calls, calldata loads, keccak256, EXP, ADDMOD, MULMOD, memory expansion, gas failure recording
- These are declared in the runtime IR module and linked via `add_global_mapping`

### `jet_runtime/src/runtime_builder.rs`
- **`RuntimeBuilder`**: Generates the runtime LLVM module programmatically
- `build()` returns a `Module<'ctx>` containing all IR-defined runtime functions
- Uses `jet_ir::Types` for consistent struct layouts
- IR-defined functions: bounds-checked stack push, pop, peek and swap, and basic memory operations
- Declared-only functions: contract calls, crypto, arithmetic ops, memory expansion, gas failure

### `jet_runtime/src/layout_tests.rs`
- Asserts `Context`, `CallInfo` and `BlockInfo` sizes, field counts and field types match `jet_ir::Types`

### `jet_runtime/src/symbols.rs`
- String constants for all symbol names
- Used for consistent linking between Rust and LLVM
- Contract symbols prefixed with `"jet.contracts."`

### `jet_runtime/src/binding/`
- Display implementations for debugging

---

## Implementation Patterns

### Standard Opcode Implementation

```rust
// Binary operation pattern
pub(crate) fn binop<'ctx, S: StackBackend<'ctx>>(bctx: &BuildCtx<'ctx, '_, S>) -> Result<(), Error> {
    // 1. Pop operands as i256 SSA values (top first)
    let (a, b) = bctx.stack.pop_2(bctx)?;

    // 2. Perform LLVM operation
    let result = bctx.builder.build_int_xxx(a, b, "binop_result")?;

    // 3. Push result
    bctx.stack.push_word(bctx, result)
}
```

### Runtime Call Pattern

```rust
pub(crate) fn runtime_op<'ctx, S: StackBackend<'ctx>>(bctx: &BuildCtx<'ctx, '_, S>) -> Result<(), Error> {
    let arg = bctx.stack.pop_word(bctx)?;

    let ret = bctx.builder.build_call(
        bctx.env.symbols().runtime_function(),
        &[bctx.registers.exec_ctx.into(), arg.into()],
        "runtime_op_result",
    )?;
    let value = ret.try_as_basic_value().unwrap_basic().into_int_value();

    bctx.stack.push_word(bctx, value)
}
```

### Memory Access Pattern

```rust
pub(crate) fn mstore<'ctx, S: StackBackend<'ctx>>(bctx: &BuildCtx<'ctx, '_, S>) -> Result<(), Error> {
    let (loc, val) = bctx.stack.pop_2(bctx)?;
    let loc_i32 = truncate_to_i32(bctx, loc, "mstore_loc")?;
    let size = bctx.env.types().i32.const_int(32, false);
    let region = expand_memory_region(bctx, loc_i32, size, "mstore")?;
    build_mem_store_value(bctx, &region, val)
}
```

### Zero-Guarded Division Pattern

```rust
pub(crate) fn div<'ctx, S: StackBackend<'ctx>>(bctx: &BuildCtx<'ctx, '_, S>) -> Result<(), Error> {
    let (a, b) = bctx.stack.pop_2(bctx)?;
    let result = build_zero_guarded_value(bctx, b, "div", |bctx| {
        bctx.builder.build_int_unsigned_div(a, b, "div_result")
    })?;
    bctx.stack.push_word(bctx, result)
}
```

---

## Testing Strategy

### Test Framework

The test framework uses a declarative macro. Tests under `crates/jet/tests` compile synthetic ROMs and assert on:

- Return code, stack contents and pointer depth
- Jump pointer values
- Return offset/length
- Memory contents and length after memory opcodes
- Gas remaining and out-of-gas diagnostics

```rust
rom_tests! {
    test_name: Test {
        roms: vec![bytecode![
            PUSH1!(0x01),
            PUSH1!(0x02),
            ADD!(),
        ]],
        expected: TestContractRun {
            stack_ptr: 1,
            stack: vec![stack_word(&[0x03])],
            ..Default::default()
        },
    },
}
```

Every case expands to two tests, one per `StackMode`, so the runtime backend acts as a differential oracle for the symbolic backend. Tests that need a specific `CallInfo` are plain `#[test]` functions using `run_both_modes`.

### Test Categories

1. **Arithmetic, comparison, bitwise**: every opcode from `ADD` to `SAR`, including division by zero and signed edge cases
2. **Control flow**: static and dynamic `JUMP`/`JUMPI`, joins, backedges, wide targets, dead code, `PC`
3. **Stack faults**: underflow and overflow in both backends, planned symbolic fault exits, runtime fallback
4. **Memory**: expansion rounding, monotonicity, overflow rejection, `MSIZE`
5. **Calls and context**: `CALL`, `RETURNDATASIZE`, `RETURNDATACOPY`, `ADDRESS`, `ORIGIN`, `CALLER`, `CALLVALUE`, `CALLDATALOAD`, `CALLDATASIZE`, block info propagation
6. **Gas**: static block charges, dynamic costs, out-of-gas diagnostics
7. **Runtime unit tests**: memory expansion builtin, runtime IR generation, struct layouts, address parsing

See [`docs/test_coverage.md`](../test_coverage.md) for the per-opcode table.

### Running Tests

```bash
make test           # cargo nextest, all crates
make test-all       # plus doctests
```

---

## Known Limitations and Future Work

### Currently Unimplemented Opcodes

See [`OPERATION_STATUS.md`](../../OPERATION_STATUS.md) for the authoritative list. The missing families are:

- **Account state and code**: BALANCE, SELFBALANCE, EXTCODESIZE, EXTCODECOPY, EXTCODEHASH, CODESIZE, CODECOPY
- **Storage**: SLOAD, SSTORE, TLOAD, TSTORE
- **Data copies**: CALLDATACOPY, MCOPY
- **Transaction data**: GASPRICE, BLOBHASH
- **Logging**: LOG0-LOG4
- **Creation**: CREATE, CREATE2
- **Other calls**: CALLCODE, DELEGATECALL, STATICCALL
- **SELFDESTRUCT**

### Known TODOs and Constraints

1. **Gas forwarding**: `CALL` discards its gas operand and runs the callee with an unbounded limit; the 63/64 rule and refunds are not modelled.
2. **CALL data and value**: the input offset and length are discarded, so the callee sees empty calldata, and no balance moves.
3. **Code eviction**: No memory management for compiled contracts; the JIT cache grows unbounded.
4. **Dynamic jump precision**: a dynamic jump dispatches over every `JUMPDEST` (shared jump block in runtime mode, per-site switch in symbolic mode).

**Previously resolved limitations** (no longer issues):
- ~~No gas accounting~~: per-block static charges and IR-computed dynamic costs, see Gas Accounting
- ~~Stack overflow unchecked~~: the runtime push, peek and swap helpers bounds-check, and the symbolic planner proves faults ahead of time
- ~~Memory bounds checking incomplete~~: `MemoryRegion` makes expansion a precondition of every memory helper (ADR 006)
- ~~Runtime IR target triple mismatch~~: eliminated when `runtime-ir/jet.ll` was replaced by `RuntimeBuilder`
- ~~Struct layout mismatches between Rust and LLVM IR~~: resolved by `jet_ir::Types` as the single source of truth (ADR-002)

### Design Decisions and Trade-offs

1. **Runtime stack by default, symbolic stack on request**
   - Pros: the runtime backend is simple and serves as the oracle; the symbolic backend removes runtime calls where it can plan
   - Cons: two lowering paths for `JUMP`/`JUMPI`, and bounded planning falls back to the runtime backend on hostile bytecode

2. **Switch dispatch for dynamic jumps**
   - Pros: validates jump targets centrally; uses LLVM switch for clarity
   - Cons: adds a switch on every dynamic `JUMP`/`JUMPI`

3. **IR-defined runtime helpers**
   - Pros: keeps symbol discovery centralized; lets LLVM inline and optimize stack and memory helpers
   - Cons: requires careful alignment between Rust structs and LLVM types, enforced by layout tests

4. **JIT-to-JIT calls only**
   - Pros: enables rapid iteration on the compiler without an account model
   - Cons: no external state, so most state opcodes remain unimplemented

### Future Optimization Opportunities

1. **Constant folding** through arithmetic so more jump targets are static in symbolic mode
2. **Shared switches** between dynamic jump sites of the same stack height
3. **Symbolic mode by default** once the opcode surface is complete
4. **Inline more builtins**: convert remaining Rust builtins (e.g., ADDMOD/MULMOD) to IR-defined functions
5. **Profile-guided optimization**: use ORC's profiling for hot path optimization
6. **Shared library extraction**: compile contracts to standalone `.so`/`.dll` files

### Suggested Next Steps

1. Forward gas and calldata through `CALL`
2. Add an account and storage model behind the state opcodes
3. Expand opcode coverage with a test-first approach

---

## Quick Reference

### Key Files for Each Task

| Task | Primary Files |
|------|---------------|
| Add new opcode | `ops.rs`, `contract.rs` (dispatch and stack effect), `gas.rs`, `test_roms.rs` |
| Add IR-defined runtime function | `runtime_builder.rs`, `symbols.rs`, `env.rs` |
| Add extern "C" runtime function | `builtins.rs`, `runtime_builder.rs` (declare), `symbols.rs`, `env.rs`, `engine/mod.rs` (link) |
| Change stack lowering | `stack.rs`, `symbolic.rs`, `contract.rs` (planner) |
| Change gas costs | `gas.rs`, `ops.rs` (dynamic charges) |
| Modify execution context layout | `exec.rs`, `call_info.rs`, `jet_ir/types.rs`, `layout_tests.rs` (must stay in sync) |
| Modify shared constants | `jet_ir/constants.rs` |
| Debug compilation | `jetdbg`, `--emit-llvm` |
| Add tests | `tests/test_roms.rs`, `tests/roms/mod.rs` |

### Common Types

| Type | Size | Purpose |
|------|------|----|
| `Word` | 32 bytes | EVM stack word |
| `i256` | 256 bits | LLVM integer for arithmetic |
| `Context` | ~33KB | Execution state |
| `CallInfo` | 1 frame | Address, origin, caller, value, calldata, gas limit |
| `ReturnCode` | 1 byte | Execution result |

### Build Commands

```bash
# Build everything
make build

# Run debug tool
cargo run -p jetdbg

# Run tests
make test

# Compile the samples in release mode
cargo run -p jetdbg -- --mode release
```

---

## Glossary

| Term | Definition |
|------|------------|
| **Basic Block** | A sequence of instructions with one entry point and one exit point |
| **SSA** | Static Single Assignment - each variable assigned exactly once |
| **ORC** | On-Request Compilation - LLVM's modern JIT framework |
| **JUMPDEST** | EVM opcode marking valid jump destinations |
| **Word** | 256-bit (32-byte) value, the fundamental unit in EVM |
| **ROM** | Read-only memory containing bytecode |
| **GEP** | GetElementPtr - LLVM instruction for computing addresses |
