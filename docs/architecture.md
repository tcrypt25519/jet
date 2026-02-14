# Jet Architecture Documentation

## Table of Contents

1. [Project Overview](#project-overview)
2. [System Architecture](#system-architecture)
3. [Core Components](#core-components)
4. [The Stack Machine to Register Machine Translation](#the-stack-machine-to-register-machine-translation)
5. [Compilation Pipeline](#compilation-pipeline)
6. [Memory Model](#memory-model)
7. [Control Flow and Jump Tables](#control-flow-and-jump-tables)
8. [Runtime Function Architecture](#runtime-function-architecture)
9. [Symbol Management and Linking](#symbol-management-and-linking)
10. [Code Organization](#code-organization)
11. [Implementation Patterns](#implementation-patterns)
12. [Testing Strategy](#testing-strategy)
13. [Known Limitations and Future Work](#known-limitations-and-future-work)
14. [Quick Reference](#quick-reference)

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
3. **Gas accounting**: Not implemented
4. **Code eviction**: No memory management for compiled contracts
5. **Error handling**: Some panics need conversion to Results
6. **Runtime IR target triple**: macOS x86_64; host mismatch is likely
7. **Symbol naming inconsistencies**: Between runtime IR and symbols (e.g., push.word vs push.i256)
8. **Address size**: Currently 2 bytes, not 20 bytes
9. **Struct layout alignment**: Requires careful alignment between Rust structs and LLVM types

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

### Suggested Next Steps

1. Reconcile struct layouts and symbol names between:
   - `exec::Context`
   - `builder::env::Types`
   - `runtime-ir/jet.ll`
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
