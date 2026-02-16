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
- **LLVM Version**: 18.0
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

### Three Cooperating Components

1. **Compiler (crates/jet)**: Parses bytecode, identifies basic blocks, and builds LLVM IR for each opcode.
2. **Runtime (crates/jet_runtime)**: Defines the execution context and provides builtin functions for stack, memory, and contract calls.
3. **Runtime IR module (runtime-ir/jet.ll)**: Declares runtime symbols and provides a small amount of IR-implemented functionality.

### Crate Structure

```
jet/
├── crates/
│   ├── jet/                    # Main compiler crate
│   │   ├── src/
│   │   │   ├── lib.rs          # Module exports
│   │   │   ├── instructions.rs # EVM opcode definitions
│   │   │   ├── builder/        # IR construction
│   │   │   │   ├── mod.rs      # Error types
│   │   │   │   ├── contract.rs # Core compilation logic
│   │   │   │   ├── env.rs      # LLVM environment setup
│   │   │   │   ├── manager.rs  # Build orchestration
│   │   │   │   └── ops.rs      # Opcode implementations
│   │   │   └── engine/         # JIT execution
│   │   │       └── mod.rs      # Engine wrapper
│   │   ├── bin/
│   │   │   └── jetdbg.rs       # Debug/testing utility
│   │   └── tests/              # Integration tests
│   │
│   └── jet_runtime/            # Runtime support crate
│       └── src/
│           ├── lib.rs          # Constants and config
│           ├── exec.rs         # Execution context
│           ├── builtins.rs     # Runtime functions
│           ├── symbols.rs      # Symbol name constants
│           └── binding/        # Display implementations
│
└── runtime-ir/
    └── jet.ll                  # LLVM IR runtime declarations
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

**Purpose**: Set up the LLVM compilation environment with types and symbols.

**Key Structures**:

```rust
struct Types<'ctx> {
    i8, i32, i64, i160, i256,     // Integer types
    ptr,                           // Pointer type
    word_bytes: [32 x i8],        // 32-byte array
    stack: [1024 x i256],         // EVM stack
    exec_ctx: struct,             // Execution context
    block_info: struct,           // Block metadata
    contract_fn: fn(ptr, ptr) -> i8,  // Contract signature
}

struct Symbols<'ctx> {
    jit_engine: GlobalValue,
    stack_push_word, stack_push_ptr, stack_pop, stack_peek, stack_swap,
    mem_store, mem_store_byte, mem_load,
    contract_call, contract_call_return_data_copy,
    keccak256,
}
```

### 3. Contract Builder (`contract.rs`)

**Purpose**: The core compilation logic that transforms EVM bytecode into LLVM IR.

**Key Types**:

```rust
struct Registers<'ctx> {
    exec_ctx: PointerValue,      // Pointer to execution context
    block_info: PointerValue,    // Pointer to block info
    jump_ptr: PointerValue,      // Pointer to jump target
    return_offset: PointerValue, // Return data offset
    return_length: PointerValue, // Return data length
    sub_call: PointerValue,      // Sub-call context pointer
}

struct BuildCtx<'ctx, 'b> {
    env: &Env,
    builder: &Builder,
    registers: Registers,
    func: FunctionValue,
}

struct CodeBlock<'ctx, 'b> {
    offset: usize,               // Bytecode offset
    rom: &[u8],                  // Bytecode slice
    basic_block: BasicBlock,     // LLVM basic block
    is_jumpdest: bool,           // Is a jump destination
    terminates: bool,            // Has terminator instruction
}
```

### 4. Operations (`ops.rs`)

**Purpose**: Implement each EVM opcode as LLVM IR generation.

**Pattern**: Each opcode function follows:
1. Pop operands from stack (as pointers)
2. Load values from pointers into SSA values
3. Perform LLVM operation
4. Push result back to stack

### 5. Engine (`engine/mod.rs`)

**Purpose**: Wrap the compilation and execution pipeline.

**Key Methods**:

```rust
impl Engine {
    fn new(context, opts) -> Self;           // Create with options
    fn build_contract(addr, rom) -> Result;  // Compile bytecode
    fn run_contract(addr, block_info) -> ContractRun;  // Execute
}
```

### 6. Execution Context (`exec.rs`)

**Purpose**: Runtime state for contract execution.

```rust
#[repr(C)]
struct Context {
    stack_ptr: u32,              // Stack depth (top-of-stack index)
    jump_ptr: u32,               // Dynamic jump target (temporary storage)
    return_off: u32,             // Return data offset (window in memory)
    return_len: u32,             // Return data length
    sub_call: Option<Box<Context>>,  // Nested call context (optional nested Context for CALL)
    stack: [[u8; 32]; 1024],     // The EVM stack (fixed array of 1024 EVM words)
    memory: [u8; 32768],         // EVM memory (linear memory buffer, initially 32KB)
    memory_len: u32,             // Used memory length
    memory_cap: u32,             // Memory capacity
}
```

**Important**: This struct is passed by pointer into JIT-compiled contract functions. The compiler assumes a specific field order when performing struct GEPs (getelementptr operations).

### 7. BlockInfo (`exec.rs`)

**Purpose**: Carries chain data exposed to opcodes like BLOCKHASH.

Fields include:
- number, difficulty, gas_limit, timestamp
- base_fee, blob_base_fee, chain_id
- hash (current block hash), hash_history (last 256), coinbase

The compiler currently only uses block hash access; additional opcodes are stubbed.

### 8. ReturnCode (`exec.rs`)

**Purpose**: Encode execution outcomes.

Return codes encode execution outcomes:
- **Negative values**: Jet-level failures (e.g., InvalidJumpBlock = -1)
- **0..63**: EVM-level success (ImplicitReturn = 0, ExplicitReturn = 1, Stop = 2)
- **64+**: EVM-level failure (Revert = 64, Invalid = 65, JumpFailure = 66)

Compiled functions always return one of these values.

### 9. Builtins (`builtins.rs`)

**Purpose**: Rust functions callable from compiled LLVM IR.

All functions use `extern "C"` ABI and are marked `unsafe`:

- `stack_push_ptr`, `stack_pop`, `stack_peek`, `stack_swap`
- `mem_store`, `mem_store_byte`, `mem_load`
- `jet_contract_call`, `jet_contract_call_return_data_copy`
- `jet_ops_keccak256`

These are declared in runtime IR and mapped at runtime using `ExecutionEngine::add_global_mapping`.

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

### JET's Solution: Real Stack Model

JET uses a **real stack** in the `Context` struct as the single source of truth. Every stack operation is a runtime function call:

```rust
pub fn add(bctx: &BuildCtx) -> Result<(), Error> {
    let (a, b) = stack_pop_2(bctx)?;     // Calls runtime `stack_pop`
    let a = load_i256(bctx, a)?;          // LLVM load from pointer
    let b = load_i256(bctx, b)?;
    let result = bctx.builder.build_int_add(a, b, "add_result")?;
    call_stack_push_i256(bctx, result)?;  // Calls runtime `stack_push`
    Ok(())
}
```

This preserves EVM stack semantics by keeping the canonical stack in runtime memory and operating on it via builtins. In LLVM IR:
- Stack values are handled as pointers to 32-byte words
- Arithmetic opcodes load i256 values from those pointers, compute in SSA, and then push the result back to the runtime stack

This avoids complex SSA stack simulation at the cost of runtime calls.

### Why This Design?

1. **Correctness First**: The real stack ensures correct semantics even with complex control flow

**Trade-offs**:
- **Pros**: Simplifies opcode lowering; avoids complex SSA stack modeling
- **Cons**: Frequent runtime calls and memory traffic; more JIT overhead

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

Gas accounting is designed but not yet implemented. The intended approach exploits LLVM's basic block structure: since a basic block either executes completely or not at all, gas costs can be amortized across the entire block. Many contracts have infrequent jumps, resulting in large basic blocks where gas accounting reduces to a single addition at the block's end. Instructions with dynamic gas costs require additional logic only when needed.

---

## Memory Model

### Execution Context Layout

```
┌─────────────────────────────────────────────────────────────────┐
│                        Context (repr(C))                         │
├─────────────────────────────────────────────────────────────────┤
│  stack_ptr: u32      │ Current stack depth (0-1023)             │
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
│  memory: [u8; 32768]  │ EVM memory (32KB default)               │
│  memory_len: u32      │ Used memory length                       │
│  memory_cap: u32      │ Memory capacity                          │
└─────────────────────────────────────────────────────────────────┘
```

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

All dynamic jumps go through a central `jump_block`. Dynamic jumps are handled by a late jump block:

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

```rust
fn jumpi(bctx, jump_block, jump_else_block) {
    let (pc, cond) = __stack_pop_2(bctx)?;

    // Store target for potential jump
    builder.build_store(registers.jump_ptr, pc);

    // Compare condition to zero
    let cmp = builder.build_int_compare(EQ, cond, zero, "jumpi_cmp");

    // Branch: if cond == 0, fall through; else jump
    builder.build_conditional_branch(cmp, jump_else_block, jump_block);
}
```

### Control Flow Opcodes

- `PC` is baked as a constant using `code_block.offset + pc` to yield the absolute bytecode index
- `JUMP`/`JUMPI` use the shared jump block as described above

---

## Runtime Function Architecture

### Why Runtime Functions?

Some operations are too complex for inline IR generation:

- Memory bounds checking
- Dynamic memory allocation
- Hash computation (keccak256)
- Cross-contract calls

### Function Declaration Pattern

**In `runtime-ir/jet.ll`**:

```llvm
declare ptr @jet.stack.pop (ptr)
declare i8 @jet.mem.store.word (ptr, ptr, ptr)
declare i8 @jet.contract.call(ptr, ptr, ptr, ptr, ptr)
```

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

### Special Case: IR-Defined Runtime Function

`jet.stack.push.i256` is defined directly in LLVM IR for efficiency:

```llvm
define i1 @jet.stack.push.i256 (%jet.types.exec_ctx*, i256) {
entry:
  ; Load stack pointer
  %stack.ptr.addr = getelementptr inbounds %jet.types.exec_ctx, ptr %0, i32 0, i32 0
  %stack.ptr = load i32, ptr %stack.ptr.addr
  %stack.top.addr = getelementptr inbounds %jet.types.exec_ctx, ptr %0, i32 0, i32 5, i32 %stack.ptr

  ; Store word directly (i256 to memory)
  store i256 %1, ptr %stack.top.addr

  ; Increment stack pointer
  %stack.ptr.next = add i32 %stack.ptr, 1
  store i32 %stack.ptr.next, ptr %stack.ptr.addr

  ret i1 true
}
```

This avoids a function call for the most common operation.

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

1. **Define opcode** in `instructions.rs` (if not present):

   ```rust
   instructions! {
       // ...
       NEWOP = 0xNN,
   }
   ```

2. **Implement operation** in `ops.rs`:

   ```rust
   pub(crate) fn newop(bctx: &BuildCtx<'_, '_>) -> Result<(), Error> {
       let a = __stack_pop_1(bctx)?;
       let a_val = load_i256(bctx, a)?;
       // ... perform operation ...
       __stack_push_int(bctx, result)?;
       Ok(())
   }
   ```

3. **Add dispatch** in `contract.rs`:

   ```rust
   Instruction::NEWOP => ops::newop(bctx),
   ```

### Adding a New Runtime Function

1. **Define symbol** in `symbols.rs`:

   ```rust
   pub const FN_NEW_FUNC: &str = "jet.category.newfunc";
   ```

2. **Declare in LLVM IR** (`runtime-ir/jet.ll`):

   ```llvm
   declare i8 @jet.category.newfunc (ptr, ptr)
   ```

3. **Implement in Rust** (`builtins.rs`):

   ```rust
   pub unsafe extern "C" fn new_func(ctx: *mut Context, arg: *const Word) -> i8 {
       // implementation
   }
   ```

4. **Add to Symbols struct** (`env.rs`):

   ```rust
   struct Symbols {
       // ...
       new_func: FunctionValue<'ctx>,
   }
   ```

5. **Link at runtime** (`engine/mod.rs`):

   ```rust
   map_fn(sym.new_func(), builtins::new_func as usize);
   ```

---

## File-by-File Summary

### `jet/src/lib.rs`
- Module structure declaration
- Enables `allocator_api` feature

### `jet/src/instructions.rs`
- Macro-based EVM opcode enum definition (`instruction!` macro)
- Implements `TryFrom<u8>`, `Display`, `opcode()` methods
- Custom `Iterator` that handles PUSH data bytes
- Converts PUSH data from big-endian to little-endian (bytes reversed)

### `jet/src/builder/mod.rs`
- Error enum for build failures
- Module declarations

### `jet/src/builder/contract.rs`
- **`Registers`**: Caches pointers into exec_ctx (jump_ptr, return_offset, return_length, sub_call)
- **`BuildCtx`**: Wraps Env, Builder, current function, and Registers
- **`CodeBlock`**: Represents a basic block with offset, ROM slice, flags
- **`CodeBlocks`**: Collection of CodeBlocks with helper methods
- **`build()`**: Main entry point - creates function, discovers blocks, generates IR
- **`find_code_blocks()`**: First pass - discovers basic block boundaries
- **`build_contract_body()`**: Second pass - generates IR for all blocks
- **`build_code_block()`**: Generates IR for a single block
- **`build_jump_table()`**: Creates the switch statement for dynamic jumps

### `jet/src/builder/env.rs`
- **`Options`**: Build configuration (mode Debug/Release, emit_llvm, assert)
- **`Types`**: All LLVM type definitions (i8/i32/i64/i160/i256, ptr, word_bytes, stack, mem, exec_ctx)
- **`Symbols`**: Runtime function lookups, mapped to `jet_runtime::symbols`
- **`Env`**: Wraps context, module, types, symbols

### `jet/src/builder/ops.rs`
- Implementations for each EVM opcode
- Helper functions for stack operations (`stack_pop_1/2/3/7`, `stack_push_int`, `call_stack_push_i256`)
- **Pattern**: Pop inputs → Load values → LLVM operation → Push result
- Many opcodes return `Error::UnimplementedInstruction`

### `jet/src/builder/manager.rs`
- **`Manager`**: Wraps Env and adds functions per contract address
- Builds contract, optionally prints IR via syntect, and verifies

### `jet/src/engine/mod.rs`
- **`Engine`**: Wraps Manager, handles compilation and execution
- Loads runtime IR module from `runtime-ir/jet.ll`
- Creates JIT execution engine
- Links Rust runtime functions at JIT time via `add_global_mapping`
- Executes contracts and returns `ContractRun`

### `jet_runtime/src/lib.rs`
- System constants (word size, stack size, memory size)

### `jet_runtime/src/exec.rs`
- **`Word`**: 32-byte array type alias (`[u8; 32]`)
- **`Context`**: Execution context struct with stack, memory, registers
- **`BlockInfo`**: EVM block metadata struct (hash, coinbase, etc)
- **`ReturnCode`**: Enum for execution results (encodes EVM and Jet-level success/failure)
- **`ContractRun`**: Wraps result and context
- **`ContractFunc`**: Function pointer type for compiled contracts
- Stack operations: push Word, pop/peek/swap logic

### `jet_runtime/src/builtins.rs`
- Unsafe `extern "C"` functions callable from LLVM IR
- Stack operations, memory operations, contract calls
- Keccak256 implementation using sha3 crate
- `mem_store`, `mem_load`, `mem_store_byte` operate on Context memory slice

### `jet_runtime/src/symbols.rs`
- String constants for all symbol names
- Used for consistent linking between Rust and LLVM
- Contract symbols prefixed with "jet.contracts."

### `jet_runtime/src/binding.rs`
- Display implementations for debugging

### `runtime-ir/jet.ll`
- LLVM IR file with type definitions and function declarations
- Contains `@jet.stack.push.i256` implementation (IR-based)
- Loaded at startup to provide runtime function signatures
- Uses macOS x86_64 target triple and datalayout (portability issue; see Known Limitations)
- Declares exec_ctx layout in LLVM IR

---

## Implementation Patterns

### Standard Opcode Implementation

```rust
// Binary operation pattern
pub(crate) fn binop(bctx: &BuildCtx<'_, '_>) -> Result<(), Error> {
    // 1. Pop operands (returns pointers)
    let (a, b) = __stack_pop_2(bctx)?;

    // 2. Load values from pointers
    let a = load_i256(bctx, a)?;
    let b = load_i256(bctx, b)?;

    // 3. Perform LLVM operation
    let result = bctx.builder.build_int_xxx(a, b, "binop_result")?;

    // 4. Push result
    __stack_push_int(bctx, result)?;

    Ok(())
}
```

### Runtime Call Pattern

```rust
pub(crate) fn runtime_op(bctx: &BuildCtx<'_, '_>) -> Result<(), Error> {
    let arg = __stack_pop_1(bctx)?;

    bctx.builder.build_call(
        bctx.env.symbols().runtime_function(),
        &[bctx.registers.exec_ctx.into(), arg.into()],
        "runtime_op_result",
    )?;

    Ok(())
}
```

### Control Flow Pattern

```rust
pub(crate) fn control_op(
    bctx: &BuildCtx<'_, '_>,
    target_block: BasicBlock,
) -> Result<(), Error> {
    // Build branch
    bctx.builder.build_unconditional_branch(target_block)?;

    Ok(())
}
```

---

## Testing Strategy

### Test Framework

The test framework uses a declarative macro. Tests under `crates/jet/tests` compile synthetic ROMs and assert on:

- Stack contents and pointer depth
- Jump pointer values
- Return offset/length
- Memory contents after MSTORE/MLOAD/RETURNDATACOPY

```rust
rom_tests! {
    test_name: Test {
        roms: vec![vec![
            Instruction::PUSH1.opcode(), 0x01,
            Instruction::PUSH1.opcode(), 0x02,
            Instruction::ADD.opcode(),
        ]],
        expected: TestContractRun {
            stack_ptr: 1,
            stack: vec![stack_word(&[0x03])],
            ..Default::default()
        },
    },
}
```

These tests serve as executable specs for the subset of opcodes currently implemented.

### Test Categories

1. **Arithmetic**: ADD, MUL, SUB, DIV, MOD
2. **Control Flow**: JUMP, JUMPI, PC
3. **Memory**: MLOAD, MSTORE, MSTORE8
4. **Contract Calls**: CALL, RETURNDATASIZE, RETURNDATACOPY
5. **Cryptographic**: KECCAK256

### Running Tests

```bash
cargo test -p jet
```

---

## Known Limitations and Future Work

### Currently Unimplemented Opcodes

Several opcode families are stubbed:

- **Storage**: SLOAD, SSTORE, TLOAD, TSTORE
- **Environment**: ADDRESS, BALANCE, CALLER, CALLVALUE, ORIGIN
- **Call data**: CALLDATALOAD, CALLDATASIZE, CALLDATACOPY
- **Block Info**: COINBASE, TIMESTAMP, NUMBER, DIFFICULTY, etc.
- **Logging**: LOG0-LOG4
- **Creation**: CREATE, CREATE2
- **Delegate Calls**: DELEGATECALL, STATICCALL, CALLCODE

### Known TODOs and Constraints

1. **Memory bounds checking**: Incomplete in runtime functions (bounds checks are TODOs)
2. **Gas accounting**: Not implemented
3. **Code eviction**: No memory management for compiled contracts
4. **Error handling**: Some panics need conversion to Results
5. **Runtime IR target triple**: macOS x86_64; host mismatch is likely
6. **Symbol naming inconsistencies**: `runtime-ir/jet.ll` declares `jet.stack.push.word`, but symbols expect `jet.stack.push.i256`
7. **Struct layout alignment**: Runtime IR exec_ctx layout differs from Env::Types (memory struct vs inlined memory array) and from exec::Context (memory size). Requires investigation.
8. **Address size**: Currently `ADDRESS_SIZE_BYTES = 2` in tests; EVM addresses are 20 bytes
9. **JUMPI type mismatch**: Condition uses `load_i64` but compares with i256 zero

### Design Decisions and Trade-offs

1. **Runtime stack as source of truth**
   - Pros: Simplifies opcode lowering; avoids complex SSA stack modeling
   - Cons: Frequent runtime calls and memory traffic; more JIT overhead

2. **Jump table for dynamic jumps**
   - Pros: Validates jump targets centrally; uses LLVM switch for clarity
   - Cons: Adds an extra block and indirect branch on every JUMP/JUMPI

3. **IR stub module for runtime symbols**
   - Pros: Keeps symbol discovery centralized; allows IR helpers like `jet.stack.push.i256` to be optimized by LLVM
   - Cons: Requires careful alignment between Rust structs and LLVM types

4. **Minimal opcode subset**
   - Pros: Enables rapid iteration on compiler correctness
   - Cons: Many opcodes are currently unimplemented

### Future Optimization Opportunities

1. **Inline runtime functions**: Convert Rust builtins to LLVM IR
2. **Gas amortization**: Compute gas per basic block, not per instruction
3. **Profile-guided optimization**: Use ORC's profiling for hot path optimization
4. **Shared library extraction**: Compile contracts to standalone `.so`/`.dll` files
5. **Add memory length/capacity tracking**: And bounds checks
6. **Expand opcode coverage**: With a test-first approach
7. **Shared IR layer (`jet_ir` crate)**: A planned refactoring would extract a shared `jet_ir` crate providing unified LLVM types and constants used by both the builder and runtime, eliminating layout drift between the two.

### Suggested Next Steps

1. Reconcile struct layouts and symbol names between `exec::Context`, `builder::env::Types`, and `runtime-ir/jet.ll`
2. Add memory length/capacity tracking and bounds checks
3. Expand opcode coverage with a test-first approach

---

## Quick Reference

### Key Files for Each Task

| Task | Primary Files |
|------|---------------|
| Add new opcode | `instructions.rs`, `ops.rs`, `contract.rs` |
| Add runtime function | `symbols.rs`, `jet.ll`, `builtins.rs`, `env.rs`, `engine/mod.rs` |
| Modify execution context | `exec.rs`, `jet.ll`, `env.rs` |
| Debug compilation | `jetdbg.rs`, enable `emit_llvm` option |
| Add tests | `tests/test_roms.rs`, `tests/roms/mod.rs` |

### Common Types

| Type | Size | Purpose |
|------|------|----|
| `Word` | 32 bytes | EVM stack word |
| `i256` | 256 bits | LLVM integer for arithmetic |
| `Context` | ~33KB | Execution state |
| `ReturnCode` | 1 byte | Execution result |

### Build Commands

```bash
# Build everything
make build

# Run debug tool
cargo run --bin jetdbg

# Run tests
cargo test -p jet

# Build with LLVM output
cargo run --bin jetdbg -- build --emit-llvm
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
