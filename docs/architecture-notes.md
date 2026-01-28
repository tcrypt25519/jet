---
created: 2026-01-25
updated: 2026-01-26
---

# JET Architecture Documentation

## Executive Summary

JET is an LLVM-based JIT compiler for the Ethereum Virtual Machine (EVM). It takes EVM bytecode and compiles it to LLVM IR, which is then lowered to native machine code via LLVM's ORC JIT infrastructure. The project is implemented in Rust and uses the `inkwell` crate for safe LLVM bindings.

**Technology Stack:**
- Language: Rust
- LLVM Version: 18
- LLVM Bindings: `inkwell` crate
- Hashing: `sha3` crate (for keccak256)

---

## Crate Structure

### `jet` (Main Compiler Crate)

- **Purpose**: Core compilation pipeline from EVM bytecode to LLVM IR
- **Dependencies**: `inkwell` (LLVM bindings), `jet_runtime` (runtime support)
- **Modules**:
  - `instructions` - EVM opcode definitions and bytecode iterator
  - `builder` - IR construction components (env, contract, ops, manager)
  - `engine` - ORC JIT wrapper

### `jet_runtime` (Runtime Support Crate)

- **Purpose**: Runtime support library for executing compiled contracts
- **Dependencies**: `inkwell`, `sha3` (keccak256)
- **Modules**:
  - `exec` - Execution context, return codes, contract function type
  - `builtins` - Runtime functions callable from compiled code
  - `symbols` - Symbol name constants for linking
  - `binding` - Display implementations for debugging

### `runtime-ir/jet.ll`

- **Purpose**: LLVM IR stub module with runtime declarations
- **Contents**:
  - External declarations for runtime functions
  - IR-based implementation of `jet.stack.push.i256`
  - Execution context layout in LLVM IR
  - Target triple: macOS x86_64 with corresponding datalayout

---

## Key Architectural Decisions

### 1. Stack Machine → Register Machine Translation

**The Core Challenge**: EVM is a stack-based virtual machine, while LLVM IR is register-based with SSA (Static Single Assignment) form.

**Solution - Hybrid Stack Model**:
The design uses a **real stack** in the execution context (`Context.stack`) as the source of truth, with an optional **virtual stack (vstack)** optimization that tracks values in LLVM SSA registers during basic block execution.

**Current Implementation**:
- Values are stored in a 1024-element stack of 32-byte words (`[Word; 1024]`)
- Stack operations (`push`, `pop`, `peek`, `swap`) are implemented as runtime function calls
- The `vstack` feature is scaffolded but currently disabled (all logic commented out)
- When enabled, `vstack` would keep recently pushed values in LLVM registers and only sync to the real stack at basic block boundaries

**Runtime Stack Builtins**:
- `stack_push_word` / `stack_push_ptr` - Push Word or pointer to Word
- `stack_pop` - Decrement stack_ptr, return &Word
- `stack_peek` - Read at (stack_ptr - peek_idx - 1) without popping
- `stack_swap` - Swap top with indexed value
- `__stack_push_int` - Zero-extend small integer widths to i256

**Why This Matters**:
- Pure stack simulation via memory is slow (function calls for every operation)
- Keeping values in SSA registers enables LLVM optimizations (constant folding, dead code elimination)
- The `__sync_vstack` function flushes the virtual stack to real stack at block boundaries
- Stack-machine semantics are preserved via runtime stack builtins instead of SSA register juggling

### 2. Basic Block Discovery and Control Flow

**Two-Pass Compilation**:

1. **Discovery Pass** (`find_code_blocks`)
   Splits bytecode into basic blocks using the instruction iterator:
   - **Terminators**: `STOP`, `RETURN`, `REVERT`, `JUMP` - End a block and mark it as terminating
   - **JUMPI**: Ends a block and creates a fallthrough block
   - **JUMPDEST**: Starts a new block and marks it as a jump destination
   - Creates LLVM `BasicBlock` for each code block

2. **Code Generation Pass** (`build_contract_body`)
   - Generates LLVM IR for each basic block
   - Emits blocks and wires fallthrough unless block terminates
   - For JUMP/JUMPI, routes to shared jump table
   - Tracks `jump_cases` for building the jump table
   - Adds jump table block at end with switch on jump_ptr

**Jump Table Implementation**:

```
Dynamic JUMP → jump_block (switch statement) → target JUMPDEST block
                                            → jump_failure_block (invalid jump)
```

The jump table is implemented as an LLVM switch statement that maps bytecode offsets to basic blocks. This enables dynamic jumps while maintaining type safety.

**PC Instruction**: Uses `block.offset + pc` to bake absolute program counter into IR.

### 3. Memory Model

**Execution Context Layout** (`Context` struct):

```rust
#[repr(C)]
struct Context {
    stack_ptr: u32,           // Current stack depth
    jump_ptr: u32,            // Target for dynamic jumps
    return_off: u32,          // Return data offset in memory
    return_len: u32,          // Return data length
    sub_call: Option<Box<Context>>,  // Nested call context
    stack: [Word; 1024],      // EVM stack (32-byte words)
    memory: [u8; 32KB],       // EVM memory (32 * MEMORY_INITIAL_SIZE_WORDS)
    memory_len: u32,          // Used memory length
    memory_cap: u32,          // Memory capacity
}
```

**LLVM IR exec_ctx Layout** (from env.rs):
```
Field 0: stack_ptr
Field 1: jump_ptr
Field 2: return_offset
Field 3: return_length
Field 4: sub_call (ptr)
Field 5: stack array
Field 6: mem struct { ptr, len, cap }
```

**Design Decisions**:
- Fixed-size allocations avoid dynamic memory management during execution
- Stack uses 32-byte words matching EVM word size
- Memory is a flat byte array with offset-based addressing
- Sub-calls create nested `Context` for isolation via `Option<Box<Context>>` with `init_sub_call()`
- Memory length/cap tracking exists but bounds checks are mostly TODOs

**Note**: Runtime IR exec_ctx layout differs slightly from Env::Types (memory struct vs inlined memory array) and from exec::Context (memory size).

### 4. Type System

**Core LLVM Types** (defined in `env.rs`):

```rust
i8, i32, i64, i160, i256     // Integer types
ptr                          // Generic pointer
word_bytes: [32 x i8]        // 32-byte array
stack: [1024 x i256]         // Stack array
exec_ctx: struct             // Execution context
block_info: struct           // Block metadata
contract_fn: fn(ptr, ptr) -> i8  // Contract function signature
```

**Key Insight**: LLVM's custom-width integers (`i256`) allow native 256-bit arithmetic without manual bigint libraries.

### 5. Symbol Management and Linking

**Symbol Naming Convention**:
- Runtime functions: `jet.stack.push.ptr`, `jet.mem.load`, etc.
- Contract functions: `jet.contracts.{address}` (prefixed with "jet.contracts.")
- Global state: `jet.jit_engine`
- Contract symbol derivation: Address bytes reversed before hex encoding (`jet_contract_fn_lookup`)

**Linking Strategy**:
1. Load precompiled LLVM IR module (`runtime-ir/jet.ll`) containing function declarations
2. Add compiled contract functions to the same module
3. Build contracts into module, optionally print IR via syntect, and verify
4. Create JIT execution engine
5. At JIT time, map symbol names to Rust function pointers via `ee.add_global_mapping`
6. Link runtime builtins and global jit_engine into JIT
7. Execute contracts by looking up function with `exec::mangle_contract_fn`

This enables:
- Cross-contract calls via symbol lookup
- Runtime function calls with minimal overhead
- Late binding of the JIT engine pointer for dynamic dispatch

**Symbols**: Required symbols are fetched from the loaded runtime module and mapped to `jet_runtime::symbols`.

### 6. Runtime Function Architecture

**Categories of Builtins**:

1. **Stack Operations** (called from LLVM IR):
   - `stack_push_ptr(ctx, word_ptr)` - Push via pointer
   - `stack_pop(ctx) -> *Word` - Pop and return pointer
   - `stack_peek(ctx, index) -> *Word` - Read without pop
   - `stack_swap(ctx, index)` - Swap top with index

2. **Memory Operations**:
   - `mem_store(ctx, loc, val)` - Store 32-byte word
   - `mem_store_byte(ctx, loc, val)` - Store single byte
   - `mem_load(ctx, loc) -> *Word` - Load 32 bytes
   - Bounds checks are TODOs

3. **Contract Calls**:
   - `jet_contract_call(ctx, jit_engine, to, out_off, out_len, ...)` - Cross-contract call
     - Pops 7 args from stack
     - Looks up contract function pointer in ExecutionEngine
     - Creates sub-context, invokes callee, validates return code
     - Copies return data into caller if present
     - Pushes return code onto stack
   - `jet_contract_call_return_data_copy(...)` - Copy return data
     - Copies from sub_ctx return buffer into caller memory
     - Includes bounds checks on return data length

4. **Cryptographic**:
   - `jet_ops_keccak256(buffer)` - In-place SHA3 hash (hashes 32-byte buffer)

**Why Rust Functions?**:
- Complex logic (memory bounds checking, context management)
- Access to Rust ecosystem (sha3 crate)
- Can be replaced with inline LLVM IR for performance-critical paths

### 7. Return Code System

```rust
#[repr(i8)]
enum ReturnCode {
    // Jet-level failures (negative)
    InvalidJumpBlock = -1,

    // EVM-level successes (0-63)
    ImplicitReturn = 0,   // No explicit return
    ExplicitReturn = 1,   // RETURN opcode
    Stop = 2,             // STOP opcode

    // EVM-level failures (64+)
    Revert = 64,
    Invalid = 65,
    JumpFailure = 66,
}
```

This encoding allows quick success/failure checks via sign comparison. `ContractRun` wraps `ReturnCode` + `Context` snapshot.

---

## Compilation Pipeline

```
┌──────────────────┐
│  EVM Bytecode    │
│  [0x60, 0x01...] │
└────────┬─────────┘
         │
         ▼
┌──────────────────────────────────────────┐
│  Instruction Iterator                     │
│  - Parses opcodes                         │
│  - Extracts PUSH data (big→little endian) │
│  - Yields IteratorItem enum               │
│    • Instr(pc, Instruction)               │
│    • PushData(pc, [u8; 32])               │
│  - Push length: opcode - PUSH0            │
└────────┬─────────────────────────────────┘
         │
         ▼
┌──────────────────────────────────────────┐
│  Basic Block Discovery (find_code_blocks) │
│  - Identifies block boundaries            │
│  - Creates LLVM BasicBlocks               │
│  - Marks JUMPDEST locations               │
└────────┬─────────────────────────────────┘
         │
         ▼
┌──────────────────────────────────────────┐
│  IR Generation (build_contract_body)      │
│  - Iterates code blocks                   │
│  - Dispatches to ops::* functions         │
│  - Builds LLVM IR instructions            │
└────────┬─────────────────────────────────┘
         │
         ▼
┌──────────────────────────────────────────┐
│  Jump Table Construction                  │
│  - Creates switch statement               │
│  - Maps offsets to basic blocks           │
│  - Adds failure block for invalid jumps   │
└────────┬─────────────────────────────────┘
         │
         ▼
┌──────────────────────────────────────────┐
│  LLVM Module                              │
│  - Contains runtime declarations          │
│  - Contains compiled contract functions   │
│  - Ready for JIT compilation              │
└────────┬─────────────────────────────────┘
         │
         ▼
┌──────────────────────────────────────────┐
│  ORC JIT Execution Engine                 │
│  - Links runtime functions                │
│  - Compiles to native code                │
│  - Provides function lookup               │
│  - Executes and returns ContractRun       │
└──────────────────────────────────────────┘
```

---

## Instruction Implementation Patterns

### Arithmetic Operations (Example: ADD)

```rust
pub(crate) fn add(bctx: &BuildCtx<'_, '_>) -> Result<(), Error> {
    let (a, b) = __stack_pop_2(bctx)?;          // Pop pointers
    let a = load_i256(bctx, a)?;                 // Load as i256
    let b = load_i256(bctx, b)?;
    let result = bctx.builder.build_int_add(a, b, "add_result")?;  // LLVM add
    __call_stack_push_i256(bctx, result)?;       // Push result
    Ok(())
}
```

**General Pattern**: Pop pointers to words → Load i256 → Compute → Push i256

### Control Flow (Example: JUMP)

```rust
pub(crate) fn jump(bctx: &BuildCtx<'_, '_>, jump_block: BasicBlock) -> Result<(), Error> {
    let pc = __stack_pop_1(bctx)?;
    let pc_i32 = load_i32(bctx, pc)?;
    bctx.builder.build_store(bctx.registers.jump_ptr, pc_i32)?;  // Store target
    bctx.builder.build_unconditional_branch(jump_block)?;        // Branch to jump table
    Ok(())
}
```

**JUMPI**: Compares condition to zero and branches to jump table or fallthrough. Currently uses `load_i64` but compares with i256 zero.

**RETURN**: Writes `return_offset` and `return_length` into exec_ctx and returns.

**REVERT/INVALID**: Return with appropriate ReturnCode.

### Memory Operations (Example: MSTORE)

```rust
pub(crate) fn mstore(bctx: &BuildCtx<'_, '_>) -> Result<(), Error> {
    let (loc, val) = __stack_pop_2(bctx)?;
    bctx.builder.build_call(
        bctx.env.symbols().mem_store(),
        &[bctx.registers.exec_ctx.into(), loc.into(), val.into()],
        "mstore",
    )?;
    Ok(())
}
```

### Special Instructions

**BYTE**: Index is reversed (`31 - idx`) to adapt little-endian word storage.

**CALL**: Pops 7 arguments, passes (ctx, jit_engine, to, out_off, out_len) to runtime, pushes return code.

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
- Includes PUSH0..PUSH32, DUP/SWAP, memory, call, etc.

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
- **`Options`**: Build configuration (mode Debug/Release, vstack flag, emit_llvm, assert)
- **`Types`**: All LLVM type definitions (i8/i32/i64/i160/i256, ptr, word_bytes, stack, mem, exec_ctx)
- **`Symbols`**: Runtime function lookups, mapped to `jet_runtime::symbols`
- **`Env`**: Wraps context, module, types, symbols

### `jet/src/builder/ops.rs`
- Implementations for each EVM opcode
- Helper functions for stack operations (`__stack_pop_1`, `__stack_pop_2`, `__stack_push_int`, `__call_stack_push_i256`)
- **Pattern**: Pop inputs → Load values → LLVM operation → Push result
- Handles arithmetic, comparison, bitwise, memory, control flow
- Many opcodes return `Error::UnimplementedInstruction`

### `jet/src/builder/manager.rs`
- **`Manager`**: Wraps Env and adds functions per contract address
- Builds contract, optionally prints IR via syntect, and verifies

### `jet/src/engine/mod.rs`
- **`Engine`**: Wraps Manager, handles compilation and execution
- Loads runtime IR module from `runtime-ir/jet.ll`
- Builds contracts into the module
- Creates JIT execution engine
- Links Rust runtime functions at JIT time
- Looks up contract function with `exec::mangle_contract_fn`
- Executes contracts and returns `ContractRun`
- Context is created on host and passed into JIT function

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
- Unsafe extern "C" functions callable from LLVM IR
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
- Contains `@jet.stack.push.i256` implementation (IR-based, pushes i256 into stack)
- Loaded at startup to provide runtime function signatures
- Uses macOS x86_64 target triple and datalayout
- Declares exec_ctx layout in LLVM IR

---

## Testing Strategy

The test framework (`tests/roms/mod.rs`) provides:
- `Test` struct with ROMs and expected outcomes
- `TestContractRun` for expected state assertions
- `rom_tests!` macro for declarative test definitions
- Support for multi-contract tests (cross-contract calls)

**Test Coverage**:
- ROM-based tests compile small bytecode snippets and assert stack/memory
- Arithmetic operations (add, etc.)
- Control flow (jump/jumpdest, jumpi)
- Memory operations (mstore, mload)
- Contract calls and return data (call, returndatacopy)
- Keccak256 hashing
- Program counter tracking (PC)
- vstack-specific test exists (but vstack behavior is disabled)

---

## Known Limitations and TODOs

### Implementation Gaps

1. **vstack disabled**: Virtual stack optimization is scaffolded but commented out; current behavior always uses runtime stack
2. **Memory bounds checking**: TODO comments indicate incomplete bounds validation
3. **Gas accounting**: Mentioned in docs/jet-description.md but not yet implemented
4. **Many opcodes unimplemented**: SLOAD/SSTORE, LOG*, CREATE/CREATE2, DELEGATECALL, etc.
5. **Code eviction**: No memory management for compiled contract cache
6. **Error handling**: Some panics instead of proper error propagation

### Observed Inconsistencies

1. **Symbol naming**: runtime-ir/jet.ll declares `jet.stack.push.word`, but symbols expect `jet.stack.push.i256`. Only `push.i256` is defined in IR.
2. **Layout mismatches**: Runtime IR exec_ctx layout differs from Env::Types (memory struct vs inlined memory array) and from exec::Context (memory size).
3. **Test configuration**: `ADDRESS_SIZE_BYTES` is 2 for tests; EVM addresses are 20 bytes.
4. **JUMPI type mismatch**: JUMPI condition uses `load_i64` but compares with i256 zero.
5. **Target triple**: Runtime IR uses macOS; host may differ.

---

## Design Trade-offs

### 1. Safety Vs Performance

- Using Rust for runtime functions provides memory safety
- Could inline more operations in LLVM IR for performance
- Current design prioritizes correctness and debuggability over raw performance

### 2. Compilation Granularity

- Currently compiles entire contracts as single functions
- Could support function-level or hot-path compilation
- Basic block structure enables future optimization passes

### 3. Stack Implementation

- Real stack with optional vstack is more conservative
- Pure register allocation would be faster but more complex
- Current approach is easier to verify correctness
- Stack-machine semantics preserved via runtime builtins instead of SSA register juggling

### 4. Symbol Management

- Single shared symbol table simplifies cross-contract calls
- Could cause issues at scale (symbol collisions, eviction)
- Mangled names with bytecode hash ensure uniqueness
- Runtime and compiler are strongly coupled by struct layout assumptions

---

## Documentation Reference

- **README**: Describes Jet as LLVM-based EVM execution environment; uses LLVM 18
- **docs/jet-description.md**: Narrative overview with tiered compilation strategy, gas amortization with basic blocks, control flow via jump table, and memory layout. This doc describes a broader vision than the current implementation.

---

## Summary

JET demonstrates a practical approach to JIT compilation of stack-based bytecode, balancing correctness, performance, and implementation complexity. The hybrid stack model, two-pass compilation, and Rust-LLVM integration create a foundation for incremental optimization while maintaining EVM semantics.

The current implementation focuses on a minimal but functional core, with many optimization opportunities (vstack, inlined operations, gas accounting) clearly marked for future development.
