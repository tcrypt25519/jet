# Jet: Engineering Philosophy & Patterns Manifesto

## 1. Context & Objective

Jet is a specialized compiler system designed to translate Ethereum Virtual Machine (EVM) bytecode into LLVM intermediate representation (LLVM IR), paired with a Just-In-Time (JIT) engine that lowers this IR to native machine code.

**The Mission:** Enable semantically identical Ethereum smart contracts to execute at native machine code speeds, primarily targeting high-throughput use cases like MEV simulation and large-scale state analysis.

**The State of the World:** This codebase is a reconstruction from partial backups. It is structurally sound but functionally incomplete. This manifesto serves as the architectural "north star" to guide the reconstruction and future development, ensuring that new code aligns with the original rigorous design intent.

---

## 2. Core Architectural Philosophies

These are the foundational beliefs that govern the Jet system. They represent inviolable principles that should guide all development decisions.

### 2.1 Semantic Preservation Above All

**The Principle:** Every transformation from EVM bytecode to native machine code must preserve the exact semantics of the original contract. A compiled contract must produce identical results to an interpreted execution for all possible inputs.

**Evidence in Codebase:**
- The `ReturnCode` enum carefully distinguishes between EVM-level successes, EVM-level failures, and Jet-level failures
- Error types explicitly differentiate between "unimplemented" (not yet built but planned) and "unexpected" (should never occur)
- Stack operations faithfully maintain EVM's 256-bit word semantics using `i256` types throughout

**Practical Implications:**
- Never optimize in ways that could change observable behavior
- When in doubt, preserve the slower but semantically correct implementation
- All edge cases in EVM specification must be handled identically
- While speed is the goal, correctness is the constraint. Match the yellow paper, then optimize

### 2.2 "EVM In a JIT" (Native Execution)

We do not interpret; we compile. Every EVM opcode is translated into a sequence of LLVM IR instructions or a call to a highly optimized runtime primitive. The goal is to strip away the interpretation overhead entirely.

### 2.3 Phase Isolation with Clean Boundaries

**The Principle:** The compilation pipeline is organized into distinct, isolated phases with well-defined interfaces between them.

**The Jet Pipeline:**

```
EVM Bytecode → Code Block Discovery → IR Generation → LLVM Module → JIT Compilation → Native Execution
```

**The system is strictly stratified into three distinct domains:**

1. **Builder (Compiler):** Pure transformation of EVM bytecode to LLVM IR. It knows nothing about the execution environment's specific memory address, only its structure.
   - `instructions.rs` - Pure bytecode parsing, no compilation logic
   - `builder/contract.rs` - IR generation phase, separate from execution
   - `builder/ops.rs` - Individual opcode implementations, isolated from control flow

2. **Engine (JIT):** Orchestrates the LLVM modules, optimization passes, and linking. It bridges the gap between the abstract IR and the physical CPU.
   - `engine/mod.rs` - JIT management, separate from IR construction

3. **Runtime (Execution):** A minimal, high-performance library (`jet_runtime`) providing the "syscalls" that the compiled code invokes (e.g., stack manipulation, memory access, keccak256).
   - `jet_runtime/` - Completely separate crate for runtime functions

**Practical Implications:**
- New opcodes should only touch `instructions.rs` (definition) and `builder/ops.rs` (implementation)
- Runtime modifications should not require builder changes
- Engine changes should not require understanding IR generation details

### 2.4 Explicit Control Flow Materialization

**The Principle:** EVM control flow must be materialized explicitly before code generation. Basic blocks are discovered before any IR is emitted.

**Evidence in Codebase:**
- `find_code_blocks()` performs a complete pass to identify all block boundaries
- `CodeBlocks` structure maintains the full control flow graph
- Jump tables are built as a dedicated final step
- `JUMPDEST` locations determine block boundaries definitively

**Block-Based Control Flow:**
- **Discovery:** `find_code_blocks` analyzes bytecode to find basic block boundaries (`JUMPDEST`, `STOP`, `RETURN`, `JUMP`, `JUMPI`)
- **Linking:** `build_contract_body` stitches these LLVM basic blocks together
- **Dynamic Jumps:** Handled via a switch statement (jump table) generated at the end of the function, mapping runtime PC values to LLVM block labels

**Practical Implications:**
- Never emit IR speculatively before understanding the full control flow
- Basic block discovery is a prerequisite, not an optimization
- Dynamic jump targets require explicit jump table construction

### 2.5 The Shared Library Pattern (Dual-World Architecture)

**The Principle:** Jet operates across two worlds—compile-time (Rust/LLVM) and runtime (JIT-executed code)—with explicit interfaces between them. Contracts are not isolated scripts; they are functions in a shared symbol table.

**Evidence in Codebase:**
- `jet_runtime` is compiled as both `dylib` and `lib` (dual compilation modes)
- Runtime functions declared in LLVM IR, implemented in Rust, linked at JIT time
- `symbols.rs` defines the exact contract between compile-time and runtime
- Global `JIT_ENGINE` pointer bridges the two worlds

**Symbol Naming:**
- Contracts: `jet.contracts.<HASH>`
- Runtime functions: `jet.<domain>.<action>[.<type>]`

**Inter-Contract Calls:** Direct function calls via the JIT engine, bypassing the overhead of a full message passing loop where possible.

**Practical Implications:**
- Runtime functions must have stable ABI (`extern "C"`)
- Changes to `symbols.rs` require coordinated updates to both `jet.ll` and Rust implementations
- Memory layout in `exec::Context` must match the LLVM IR struct exactly

### 2.6 Explicit & Linear Memory Model

The execution context is a single, linear block of memory containing:
- **Registers/Metadata:** Stack pointers, return data offsets
- **EVM Stack:** Fixed-size array (1024 words)
- **EVM Memory:** Linear byte array
- **Sub-call Pointers:** For recursion

This structure allows LLVM to perform aggressive alias analysis and optimization, as the layout is predictable and contiguous.

### 2.7 Fail-Fast Compilation

**The Principle:** Invalid or unsupported bytecode should be rejected early with clear diagnostics, not discovered at runtime.

**Evidence in Codebase:**
- `Error::UnimplementedInstruction` - explicit about what's missing
- `Error::UnexpectedInstruction` - distinct from unimplemented
- `Error::InvariantViolation` - for internal consistency failures
- Verification step with `module.verify()` after construction

**Practical Implications:**
- Return errors, don't panic (except for true invariant violations)
- Error messages should identify the specific opcode or construct
- Compile-time failures are preferable to runtime failures

---

## 3. Validated Patterns

These patterns appear consistently throughout the codebase and represent the established idioms that should be emulated in new code.

### 3.1 The BuildContext Pattern

All IR generation operates through a `BuildCtx` structure that encapsulates the current compilation state.

```rust
pub(crate) struct BuildCtx<'ctx, 'b> {
    pub(crate) env: &'b Env<'ctx>,           // Global environment
    pub(crate) builder: &'b inkwell::builder::Builder<'ctx>,  // LLVM builder
    pub(crate) registers: Registers<'ctx>,    // Execution context pointers
    func: FunctionValue<'ctx>,                // Current function
}
```

**Usage Pattern:**
- Every opcode implementation receives `&BuildCtx`
- Environment access via `bctx.env`
- IR emission via `bctx.builder`
- Type access via `bctx.env.types()`
- Symbol access via `bctx.env.symbols()`

### 3.2 Opcode Implementation Structure

Each opcode follows a consistent implementation pattern:

```rust
pub(crate) fn opcode_name(bctx: &BuildCtx<'_, '_>) -> Result<(), Error> {
    // 1. Pop operands from stack (typed)
    let (a, b) = stack_pop_2(bctx)?;

    // 2. Load values from pointers
    let a = load_i256(bctx, a)?;
    let b = load_i256(bctx, b)?;

    // 3. Perform operation using LLVM builder
    let result = bctx.builder.build_int_add(a, b, "result_name")?;

    // 4. Push result back to stack
    stack_push_int(bctx, result)?;

    Ok(())
}
```

**Key Conventions:**
- Return `Result<(), Error>` - operations can fail
- Use `?` for error propagation
- Name LLVM values descriptively (`"add_result"`, not `""`)
- Pop before push (maintain stack discipline)

### 3.3 Stack Operation Helpers

Stack operations use typed helper functions:

```rust
// Pop variants by count
fn stack_pop_1<'ctx>(bctx: &BuildCtx<'ctx, '_>) -> Result<StackPop1<'ctx>, Error>
fn stack_pop_2<'ctx>(bctx: &BuildCtx<'ctx, '_>) -> Result<StackPop2<'ctx>, Error>
fn stack_pop_3<'ctx>(bctx: &BuildCtx<'ctx, '_>) -> Result<StackPop3<'ctx>, Error>
fn stack_pop_7<'ctx>(bctx: &BuildCtx<'ctx, '_>) -> Result<StackPop7<'ctx>, Error>

// Load variants by type
fn load_i8<'a>(bctx: &BuildCtx<'a, '_>, ptr: PointerValue<'a>) -> Result<IntValue<'a>, Error>
fn load_i32<'a>(bctx: &BuildCtx<'a, '_>, ptr: PointerValue<'a>) -> Result<IntValue<'a>, Error>
fn load_i256<'a>(bctx: &BuildCtx<'a, '_>, ptr: PointerValue<'a>) -> Result<IntValue<'a>, Error>

// Push variants
fn __stack_push_int<'ctx>(bctx: &BuildCtx<'ctx, '_>, value: IntValue<'ctx>) -> Result<(), Error>
fn __stack_push_ptr<'ctx>(bctx: &BuildCtx<'ctx, '_>, value: PointerValue<'ctx>) -> Result<(), Error>
```

**Convention:** Functions prefixed with `__` are internal helpers, not public API.

### 3.4 Type Centralization

All LLVM types are defined once in `env::Types`:

```rust
pub struct Types<'ctx> {
    // Primitives
    pub i8: inkwell::types::IntType<'ctx>,
    pub i32: inkwell::types::IntType<'ctx>,
    pub i256: inkwell::types::IntType<'ctx>,
    pub ptr: inkwell::types::PointerType<'ctx>,

    // Composite types
    pub exec_ctx: inkwell::types::StructType<'ctx>,
    pub contract_fn: inkwell::types::FunctionType<'ctx>,
    // ...
}
```

**Access Pattern:** Always access via `bctx.env.types()`, never recreate types.

### 3.5 Symbol Management

Runtime function symbols are managed centrally in `symbols.rs`:

```rust
// Naming convention: operation domain + action + type
pub const FN_STACK_PUSH_WORD: &str = "jet.stack.push.i256";
pub const FN_STACK_PUSH_PTR: &str = "jet.stack.push.ptr";
pub const FN_STACK_POP: &str = "jet.stack.pop";
pub const FN_MEM_STORE_WORD: &str = "jet.mem.store.word";
pub const FN_CONTRACT_CALL: &str = "jet.contract.call";
```

**Naming Convention:** `jet.{domain}.{action}[.{type}]`

### 3.6 Runtime Bridging

We bridge Rust and LLVM via a dual-definition pattern:

1. **Rust Definition:** `extern "C"` functions in `crates/jet_runtime/src/builtins.rs`
2. **LLVM Declaration:** `runtime-ir/jet.ll` declares these functions
3. **Linkage:** `Engine::link_in_runtime` manually maps the LLVM symbols to the Rust function pointers

**Rule:** When adding a new runtime function, it must exist in all three places.

### 3.7 Status Code Propagation (ReturnCode)

The runtime communicates with the execution harness via the `ReturnCode` enum (repr `i8`):

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

**Pattern:** Compiled functions return `i8`. The harness wraps this in `ContractRun`.

This encoding allows quick success/failure checks via sign comparison.

### 3.8 Error Type Hierarchy

```rust
#[derive(Error, Debug)]
pub enum Error {
    // Transparent wrapper for library errors
    #[error(transparent)]
    Builder(#[from] BuilderError),

    // Domain-specific errors with context
    #[error("instruction is unimplemented: {}", .0)]
    UnimplementedInstruction(Instruction),

    // Invariant violations (internal bugs)
    #[error("invariant violation: {}", .0)]
    InvariantViolation(String),
}
```

**Classification:**
- `Unimplemented` - Known missing functionality
- `Unexpected` - Should not occur in valid input
- `InvariantViolation` - Internal bug in Jet itself

### 3.9 Test Structure with ROM Macros

```rust
rom_tests! {
    test_name: Test {
        roms: vec![vec![
            Instruction::PUSH1.opcode(),
            0x01,
            // ... bytecode
        ]],
        expected: TestContractRun {
            stack_ptr: 1,
            stack: vec![stack_word(&[0x03])],
            ..Default::default()
        },
    },
}
```

**Testing Philosophy:**
- Tests are written as EVM bytecode, not mocked IR
- Expected results specify full machine state
- Multiple contracts can be tested together (for CALL scenarios)
- Default values for unaffected state

### 3.10 Runtime Function FFI

Runtime functions exposed to JIT-compiled code follow this pattern:

```rust
/// Documentation with safety requirements
///
/// # Safety
///
/// This function is unsafe because it dereferences the given pointers.
pub unsafe extern "C" fn function_name(
    ctx: *mut Context,
    param: *const SomeType,
) -> ReturnType {
    // 1. Convert raw pointers to references
    let ctx = unsafe { ctx.as_mut() }.unwrap();
    let param = unsafe { *param };

    // 2. Perform operation on Context
    ctx.some_operation(param)
}
```

---

## 4. Reconstruction Guidance

This section catalogs the current state of the codebase to guide reconstruction efforts.

### 4.1 Known Incomplete Areas

#### Unimplemented EVM Instructions

The following instructions are declared but return `UnimplementedInstruction`:

**Environment/Context Access:**
- `ADDRESS`, `BALANCE`, `ORIGIN`, `CALLER`, `CALLVALUE`
- `CALLDATALOAD`, `CALLDATASIZE`, `CALLDATACOPY`
- `CODESIZE`, `CODECOPY`, `EXTCODESIZE`, `EXTCODECOPY`, `EXTCODEHASH`
- `GASPRICE`

**Block Information:**
- `COINBASE`, `TIMESTAMP`, `NUMBER`, `DIFFICULTY`
- `GASLIMIT`, `CHAINID`, `SELFBALANCE`, `BASEFEE`
- `BLOBHASH`, `BLOBBASEFEE`

**Storage Operations:**
- `SLOAD`, `SSTORE` (critical for contract state)
- `TLOAD`, `TSTORE` (transient storage, EIP-1153)

**Memory Operations:**
- `MSIZE` (memory size tracking)
- `MCOPY` (memory copy, EIP-5656)
- `GAS` (remaining gas)

**Logging:**
- `LOG0`, `LOG1`, `LOG2`, `LOG3`, `LOG4`

**Contract Creation:**
- `CREATE`, `CREATE2`

**Call Variants:**
- `CALLCODE`, `DELEGATECALL`, `STATICCALL`
- `SELFDESTRUCT`

**Partially Implemented:**
- `EXP` - Stub exists, needs runtime power function
- `SIGNEXTEND` - Stub exists, needs implementation

#### Memory Length Tracking

Memory bounds checking is stubbed with `TODO: Handle this after we correctly handle memory_len`:

- `builtins::mem_store` - bounds check commented out
- `builtins::mem_store_byte` - bounds check commented out
- `builtins::mem_load` - bounds check commented out
- `builtins::jet_contract_call_return_data_copy` - bounds check commented out

**Status:** Memory works but without bounds validation.

**Reconstruction Priority:** High - correctness issue.

#### Gas Accounting

Gas accounting is designed but not implemented:

- No gas tracking per instruction
- No gas limit enforcement
- `GAS` opcode unimplemented

**Design Intent (from documentation):** Gas accounting should be amortized per basic block, with dynamic costs computed when needed.

**Reconstruction Priority:** Medium-High for production use.

### 4.2 The "Caution Zone" (Problematic Patterns)

#### Unsafe Runtime

The `jet_runtime` relies heavily on `unsafe` pointer arithmetic. This is intentional for speed but requires extreme discipline. New runtime functions must verify pointers derived from the JIT context.

#### Hardcoded Target Triple

**Location:** `runtime-ir/jet.ll`

```llvm
target triple = "x86_64-apple-macosx14.0.0"
```

**Issue:** This limits portability to macOS on x86_64.

**Recommendation:** Generate target-specific IR at build time or use target-agnostic IR where possible.

#### Runtime IR Location

**Location:**
- `/workspace/runtime-ir/jet.ll`

**Note:** This is the single source of truth for the runtime module layout.

#### Panic in Symbol Loading

**Location:** `env.rs`

```rust
if runtime_fns.is_none() {
    panic!("Failed to load all runtime functions");
}
```

**Issue:** Panics on startup rather than returning error.

**Recommendation:** Return `Result` from `Env::new()`.

#### Inconsistent Error Handling in Iterator

**Location:** `contract.rs`

```rust
IteratorItem::Invalid(pc) => {
    // TODO: return error
    panic!("Invalid instruction at PC {}", pc)
}
```

**Issue:** Panics instead of returning error as indicated by TODO.

#### Address Size Hardcoding

**Location:** `jet_runtime/src/lib.rs`

```rust
pub const ADDRESS_SIZE_BYTES: usize = 2;
```

**Issue:** EVM addresses are 20 bytes, not 2. This appears to be a testing shortcut.

**Recommendation:** Use proper 20-byte addresses before production.

#### Unchecked GEP Operations

**Various locations in `ops.rs`:**

```rust
let byte_ptr = unsafe { bctx.builder.build_in_bounds_gep(typ, word, &path, "byte") }?;
```

**Issue:** `build_in_bounds_gep` is unsafe and assumes valid indices. EVM byte access with index ≥ 32 should return 0, not undefined behavior.

**Recommendation:** Add bounds checking before GEP operations.

### 4.3 Integration Points

#### Runtime IR to Rust Binding

The `jet.ll` file declares external functions that must be implemented in Rust:

| IR Declaration | Rust Implementation | Location |
|---------------|---------------------|----------|
| `@jet.stack.push.ptr` | `builtins::stack_push_ptr` | `builtins.rs` |
| `@jet.stack.pop` | `builtins::stack_pop` | `builtins.rs` |
| `@jet.stack.peek` | `builtins::stack_peek` | `builtins.rs` |
| `@jet.stack.swap` | `builtins::stack_swap` | `builtins.rs` |
| `@jet.mem.store.word` | `builtins::mem_store` | `builtins.rs` |
| `@jet.mem.store.byte` | `builtins::mem_store_byte` | `builtins.rs` |
| `@jet.mem.load` | `builtins::mem_load` | `builtins.rs` |
| `@jet.contract.call` | `builtins::jet_contract_call` | `builtins.rs` |
| `@jet.contracts.call_return_data_copy` | `builtins::jet_contract_call_return_data_copy` | `builtins.rs` |
| `@jet.ops.keccak256` | `builtins::jet_ops_keccak256` | `builtins.rs` |

**Note:** `@jet.stack.push.i256` is implemented in LLVM IR directly for performance.

#### Type Structure Alignment

The `exec::Context` struct must exactly match `%jet.types.exec_ctx` in IR:

```rust
// Rust (exec.rs)
#[repr(C)]
pub struct Context {
    stack_ptr: u32,           // Field 0
    jump_ptr: u32,            // Field 1
    return_off: u32,          // Field 2
    return_len: u32,          // Field 3
    sub_call: Option<Box<Context>>,  // Field 4 (ptr)
    stack: [Word; 1024],      // Field 5
    memory: [u8; 32768],      // Field 6
    memory_len: u32,          // Field 7
    memory_cap: u32,          // Field 8
}
```

```llvm
; LLVM IR (jet.ll)
%jet.types.exec_ctx = type <{
  i32,                        ; Field 0: stack.ptr
  i32,                        ; Field 1: jump_ptr
  i32,                        ; Field 2: return offset
  i32,                        ; Field 3: return length
  ptr,                        ; Field 4: sub call ctx
  [1024 x %jet.types.word],   ; Field 5: stack
  [1024 x i8],                ; Field 6: memory (NOTE: Size mismatch!)
  i32,                        ; Field 7: mem length
  i32                         ; Field 8: mem capacity
}>
```

**Warning:** There is a size mismatch in memory allocation between Rust and IR. This needs investigation.

### 4.4 Build System Notes

- **LLVM 18:** The project is hard-pinned to LLVM 18. Do not attempt to upgrade without a full audit.
- **Runtime IR:** `runtime-ir/jet.ll` is the single source of truth for the runtime module layout and must be present at compile time (loaded by Engine)
- Target triple hardcoded to `x86_64-apple-macosx14.0.0` (needs platform abstraction)

### 4.5 Assumed Dependencies

- **LLVM 18.0** - Required, version-specific APIs used
- **Inkwell** - Rust bindings for LLVM (specific commit pinned)
- **sha3** - For KECCAK256 implementation
- **Rust nightly** - Uses `#![feature(allocator_api)]` and Edition 2024

---

## 5. Domain Lexicon

A guide to the naming conventions and domain language used throughout the codebase. To maintain cohesion, use these terms consistently.

### EVM Domain Terms

| Term | Meaning in Jet | Codebase Usage |
|------|---------------|----------------|
| **ROM / Bytecode** | Read-only memory containing bytecode | `rom: &[u8]` parameters |
| **Word** | 256-bit (32-byte) value | `type Word = [u8; 32]` |
| **PC** | Program counter / bytecode offset | `pc: usize` |
| **Opcode** | Single EVM instruction byte | `Instruction` enum |
| **Stack** | EVM execution stack (max 1024 words) | `stack: [Word; 1024]` |
| **Memory** | Byte-addressable contract memory | `memory: [u8; ...]` |
| **Storage** | Persistent key-value store | Not yet implemented |
| **JUMPDEST** | Valid jump target marker | Identifies block boundaries |
| **Return Data** | Data returned from sub-calls | `return_off`, `return_len` |

### Compiler Domain Terms

| Term | Meaning in Jet | Codebase Usage |
|------|---------------|----------------|
| **Builder** | IR construction subsystem | `builder/` module |
| **Env** | Compilation environment | `Env<'ctx>` struct |
| **BuildCtx** | Per-function build context | `BuildCtx<'ctx, 'b>` |
| **CodeBlock** | Basic block during discovery | `CodeBlock<'ctx, 'b>` |
| **Manager** | Contract compilation coordinator | `Manager<'ctx>` |
| **Types** | LLVM type definitions | `Types<'ctx>` |
| **Symbols** | Runtime function references | `Symbols<'ctx>` |

### Execution Domain Terms

| Term | Meaning in Jet | Codebase Usage |
|------|---------------|----------------|
| **Engine** | JIT execution environment | `Engine<'ctx>` |
| **Context (`exec_ctx`)** | Runtime execution state | `exec::Context` |
| **ContractRun** | Result of contract execution | `ContractRun` struct |
| **ReturnCode** | Execution result status | `ReturnCode` enum |
| **BlockInfo** | EVM block environment data | `BlockInfo` struct |
| **Builtins** | Runtime helper functions | `builtins.rs` |

### Naming Conventions

#### Functions

| Pattern | Usage | Example |
|---------|-------|---------|
| `build_*` | IR construction functions | `build_contract_body` |
| `find_*` | Discovery/analysis functions | `find_code_blocks` |
| `load_*` | LLVM load operations | `load_i256` |
| `__*` | Internal helpers (not public) | `__stack_pop_2` |
| `jet_*` | External API (FFI) | `jet_contract_call` |

#### Types

| Pattern | Usage | Example |
|---------|-------|---------|
| `*Ctx` | Context/state structures | `BuildCtx`, `Context` |
| `*Info` | Read-only data containers | `BlockInfo` |
| `*Run` | Execution results | `ContractRun` |
| `*Value` | LLVM value wrappers | `IntValue`, `PointerValue` |

#### LLVM IR Names

| Pattern | Usage | Example |
|---------|-------|---------|
| `*_result` | Operation outputs | `"add_result"` |
| `*_ptr` | Pointer values | `"word_ptr"` |
| `*_value` | Loaded values | `"jump_value"` |
| `block` | Basic blocks | `"jump_block"` |

### Symbol Naming in IR

Runtime functions follow the pattern: `jet.{domain}.{action}[.{type}]`

```rust
jet.stack.push.i256     - Push 256-bit integer to stack
jet.stack.push.ptr      - Push pointer to stack
jet.stack.pop           - Pop from stack
jet.mem.store.word      - Store 32-byte word to memory
jet.contract.call       - Call another contract
jet.ops.keccak256       - Compute keccak256 hash
```

### Test Naming

Test names describe the behavior being verified:

```rust
one_plus_two                          // Simple arithmetic
basic_jump                            // Control flow
basic_mem_ops                         // Memory operations
vstack_accesses_real_stack_after_jump // Edge case behavior
return_sets_offset_and_length         // Return value semantics
basic_call_with_return_data           // Cross-contract calls
```

---

## 6. Guidance for Coding Agents

This section provides specific guidance for AI coding agents assisting with Jet development.

### 6.1 Preservation Priorities

**NEVER break these invariants:**

1. **Semantic Equivalence:** If an optimization changes the result of an EVM calculation (even a "bug-compatible" one), it is wrong. Compiled contracts must behave identically to interpreted execution.
2. **Stack discipline** - Pop before push, maintain correct stack pointer
3. **Return code semantics** - The meaning of each `ReturnCode` variant
4. **Type alignment** - Rust struct layouts must match LLVM IR definitions exactly
5. **Symbol contracts** - Function signatures in IR must match Rust implementations
6. **Safety:** Never compromise the `unsafe` boundaries. If you touch `builtins.rs`, you must double-check pointer validity.

### 6.2 Testing Philosophy

**All new code should include tests that:**

1. Express behavior as EVM bytecode, not as mocked IR
2. Verify complete machine state (stack, memory, return values)
3. Test edge cases explicitly
4. Use the `rom_tests!` macro for consistency

**Test structure:**

```rust
test_name: Test {
    roms: vec![vec![
        // EVM bytecode as instruction opcodes
    ]],
    expected: TestContractRun {
        stack_ptr: N,
        stack: vec![expected_values],
        // ... other expected state
        ..Default::default()
    },
}
```

**Testing Levels:**
- **Unit Tests:** Verify individual `ops` function correctly in isolation
- **Integration Tests:** Use `tests/roms/` to run full contract bytecode and verify the `ContractRun` result
- **Golden Tests:** Future work should compare Jet execution results against a reference EVM (e.g., Geth or Reth) trace

### 6.3 Extension Points

#### Adding a New EVM Opcode

1. Add variant to `Instruction` enum in `instructions.rs`
2. Add match arm in `build_code_block()` in `contract.rs`
3. Implement operation function in `ops.rs` following the pattern:

   ```rust
   pub(crate) fn opcode_name(bctx: &BuildCtx<'_, '_>) -> Result<(), Error> {
       // Pop operands
       // Load values
       // Build IR operation
       // Push result
   }
   ```

4. Add test in `test_roms.rs`
5. If complex, add `builtins::<opname>` and expose via `Symbols`

#### Adding a New Runtime Function

1. Add symbol constant to `symbols.rs`
2. Add IR declaration to `jet.ll`
3. Implement function in `builtins.rs` with `extern "C"` ABI
4. Add to `Symbols` struct in `env.rs`
5. Add linking in `Engine::link_in_runtime()`

#### Adding a New Return Code

1. Add variant to `ReturnCode` enum in `exec.rs`
2. Ensure value follows the convention:
   - Negative: Jet-level failures
   - 0-63: EVM-level successes
   - 64+: EVM-level failures

### 6.4 Documentation Expectations

#### Inline Documentation

- All public functions require doc comments
- Unsafe functions require `# Safety` sections
- Complex algorithms require step-by-step comments
- Magic numbers require explanation

#### File-Level Documentation

- Each module should have a header comment explaining its purpose
- Dependencies between modules should be documented

### 6.5 Code Quality Standards

1. **Error handling:** Return `Result`, don't panic (except for true invariant violations)
2. **Naming:** Follow established conventions (see Lexicon section)
3. **LLVM values:** Always provide meaningful names for IR values
4. **Lifetimes:** Use `'ctx` for LLVM context lifetime, `'b` for builder lifetime
5. **Formatting:** Use `cargo fmt` before committing
6. **Linting:** Address all `cargo clippy` warnings

### 6.6 Common Pitfalls

1. **Endianness:** EVM is big-endian, most machines are little-endian. The `Iterator` in `instructions.rs` handles byte reversal for PUSH data.
2. **Stack indexing:** EVM stack grows upward. `peek(0)` is top of stack, `peek(1)` is one below.
3. **Memory vs Storage:** Memory is volatile (per-call), storage is persistent. They have different semantics and implementations.
4. **Context lifetimes:** The `Context` struct is passed as a raw pointer to JIT code. Lifetime tracking is manual.
5. **Jump targets:** Only `JUMPDEST` opcodes are valid jump targets. All other destinations are invalid.

### 6.7 Reconstruction Priorities

When prioritizing reconstruction work:

1. **Critical (blocks basic functionality):**
   - Memory length tracking and bounds checking
   - Storage operations (SLOAD/SSTORE)
   - Call data access (CALLDATALOAD, etc.)

2. **Important (blocks production use):**
   - Gas accounting
   - All environment opcodes (ADDRESS, CALLER, etc.)
   - Logging operations

3. **Enhancement (improves performance/completeness):**
   - Virtual stack optimization
   - Platform-agnostic IR
   - Extended call variants (DELEGATECALL, etc.)

---

## Appendix A: Architecture Diagrams

### Compilation Pipeline

```
┌─────────────────────────────────────────────────────────────────────┐
│                          JET COMPILER                               │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  ┌──────────────┐     ┌──────────────┐     ┌──────────────────────┐│
│  │  EVM         │     │   Code       │     │      LLVM IR         ││
│  │  Bytecode    │────▶│   Block      │────▶│      Module          ││
│  │  (rom: &[u8])│     │   Discovery  │     │                      ││
│  └──────────────┘     └──────────────┘     └──────────────────────┘│
│                                                     │               │
│         instructions.rs    contract.rs          ops.rs              │
│                                                     │               │
│                                                     ▼               │
│                                            ┌──────────────────────┐│
│                                            │    JIT Engine        ││
│                                            │    (LLVM ORC)        ││
│                                            └──────────────────────┘│
│                                                     │               │
│                                                     ▼               │
│                                            ┌──────────────────────┐│
│                                            │  Native Machine      ││
│                                            │  Code                ││
│                                            └──────────────────────┘│
└─────────────────────────────────────────────────────────────────────┘
```

### Runtime Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                        JIT EXECUTION                                │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  ┌──────────────────────┐        ┌────────────────────────────────┐│
│  │   Compiled Contract  │        │        Runtime Context         ││
│  │   (Native Code)      │◀──────▶│        (exec::Context)         ││
│  └──────────────────────┘        └────────────────────────────────┘│
│            │                                    │                   │
│            │    ┌───────────────────┐           │                   │
│            └───▶│  Runtime Builtins │◀──────────┘                   │
│                 │  (Rust FFI)       │                               │
│                 └───────────────────┘                               │
│                          │                                          │
│                          ▼                                          │
│                 ┌───────────────────┐                               │
│                 │  Symbol Table     │                               │
│                 │  (jet.* symbols)  │                               │
│                 └───────────────────┘                               │
└─────────────────────────────────────────────────────────────────────┘
```

### Module Dependencies

```
                    ┌───────────────┐
                    │   jet (lib)   │
                    └───────────────┘
                           │
          ┌────────────────┼────────────────┐
          ▼                ▼                ▼
   ┌────────────┐   ┌────────────┐   ┌────────────┐
   │  builder   │   │   engine   │   │instructions│
   └────────────┘   └────────────┘   └────────────┘
          │                │
          │                ▼
          │        ┌─────────────────┐
          └───────▶│  jet_runtime    │
                   └─────────────────┘
                          │
            ┌─────────────┼─────────────┐
            ▼             ▼             ▼
      ┌─────────┐   ┌─────────┐   ┌─────────┐
      │builtins │   │  exec   │   │ symbols │
      └─────────┘   └─────────┘   └─────────┘
```

---

## Appendix B: Quick Reference

### File Locations

| Purpose | Location |
|---------|----------|
| EVM opcodes | `crates/jet/src/instructions.rs` |
| IR generation | `crates/jet/src/builder/` |
| Opcode implementations | `crates/jet/src/builder/ops.rs` |
| JIT engine | `crates/jet/src/engine/mod.rs` |
| Runtime context | `crates/jet_runtime/src/exec.rs` |
| FFI builtins | `crates/jet_runtime/src/builtins.rs` |
| Symbol names | `crates/jet_runtime/src/symbols.rs` |
| LLVM IR runtime | `runtime-ir/jet.ll` |
| Tests | `crates/jet/tests/` |

### Common Operations

| Task | Location | Function |
|------|----------|----------|
| Add opcode | `instructions.rs` | `instructions!` macro |
| Implement opcode | `ops.rs` | `pub(crate) fn name()` |
| Pop from stack | `ops.rs` | `__stack_pop_N()` |
| Push to stack | `ops.rs` | `__stack_push_int/ptr()` |
| Load value | `ops.rs` | `load_iN()` |
| Build IR | `contract.rs` | `BuildCtx.builder.*` |
| Add runtime fn | `builtins.rs` | `extern "C" fn` |

### Return Codes

| Code | Meaning |
|------|---------|
| -1 | `InvalidJumpBlock` - Jet error |
| 0 | `ImplicitReturn` - Normal completion |
| 1 | `ExplicitReturn` - RETURN opcode |
| 2 | `Stop` - STOP opcode |
| 64 | `Revert` - REVERT opcode |
| 65 | `Invalid` - INVALID opcode |
| 66 | `JumpFailure` - Invalid jump target |

---

*This manifesto is a living document. Update it as the codebase evolves.*

*Version: 2.0.0*
*Last Updated: January 2026*
*Reconstruction Phase: Initial Discovery Complete*
