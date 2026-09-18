# Adding New EVM Opcodes to Jet

Process documentation for implementing EVM opcodes in the Jet compiler.

Before starting, read the opcode's reference page in
`.agents/skills/evm-opcodes/references/docs/<HEX>.md` for its stack inputs and
outputs, gas cost and edge cases. `OPERATION_STATUS.md` lists which opcodes are
still unimplemented.

## Checklist

### 1. Instruction Definition

**File**: `crates/jet/src/instructions.rs`

Every opcode in the current EVM is already in the `Instruction` enum. If a new
one is needed, add it with its opcode byte:
```rust
NEWOP = 0xNN,
```

### 2. Opcode Emitter

**File**: `crates/jet/src/builder/ops.rs`

Emitters are generic over the stack backend, so the same code serves the
runtime stack and the symbolic stack:
```rust
pub(crate) fn newop<'ctx, S: StackBackend<'ctx>>(bctx: &BuildCtx<'ctx, '_, S>) -> Result<(), Error> {
    let (a, b) = bctx.stack.pop_2(bctx)?;
    let result = bctx.builder.build_int_add(a, b, "newop_result")?;
    bctx.stack.push_word(bctx, result)
}
```

Requirements:
- Pop operands with `bctx.stack.pop_word(bctx)`, `pop_2`, `pop_3` or `pop_7`. Words are `i256` `IntValue`s.
- Push results with `bctx.stack.push_word(bctx, value)`. Narrower integers are zero-extended to `i256`. Use `push_word_with_known_u64` when the pushed value is a compile-time constant that a later `JUMP` may consume.
- Call builtins through `bctx.env.symbols().<name>()`.
- Never touch `Context.stack` directly.

### 3. Routing and Stack Effect

**File**: `crates/jet/src/builder/contract.rs`

Two match statements need the opcode:

1. `build_non_jump_instruction` dispatches to the emitter. Replace the
   `Err(Error::UnimplementedInstruction(...))` arm:
   ```rust
   Instruction::NEWOP => ops::newop(bctx),
   ```
2. `apply_abstract_instruction` tells the symbolic planner the stack effect.
   Move the opcode out of the unimplemented group into the arm matching its
   pops and pushes, for example:
   ```rust
   Instruction::NEWOP => {
       stack.pop_n(2)?;
       stack.push_unknown()
   }
   ```
   If the pushed value is a known constant, use `push_known(Some(value))` so
   static jump targets survive.

Emission asserts in debug builds that the planned and emitted stack heights
agree, so a mismatch fails at the block that diverged.

### 4. Gas

**File**: `crates/jet/src/builder/gas.rs`

Add the static cost to `static_cost`. If the cost depends on operands (memory
expansion, data length), add a `DynamicGas` kind, return it from
`dynamic_kind`, and implement its cost next to the existing kinds. Dynamic
charges run before the operation has any side effect, so an unaffordable
memory expansion leaves memory unchanged.

### 5. Data Representation (Critical)

**Endianness**: Stack words are stored **little-endian** internally.

- PUSH immediates are byte-reversed when the bytecode is decoded
- By the time data reaches builtins, it is already little-endian
- Use `from_le_bytes()` / `to_le_bytes()` in Rust builtins; `builtins.rs` has
  `read_u256` and `write_u256` for whole words

### 6. Complex Operations (Builtins)

For operations that need Rust, follow
[`new-runtime-function.md`](new-runtime-function.md): symbol constant in
`symbols.rs`, declaration in `RuntimeBuilder::declare_external_builtins`,
`unsafe extern "C"` implementation in `builtins.rs`, `Symbols` field and
accessor in `env.rs`, and `map_fn` linking in `engine/mod.rs`.

### 7. Memory Operations

Memory helpers take a `MemoryRegion`, and only `expand_memory_region` can
construct one (ADR 006):
```rust
let (loc, val) = bctx.stack.pop_2(bctx)?;
let loc_i32 = truncate_to_i32(bctx, loc, "newop_loc")?;
let size = bctx.env.types().i32.const_int(32, false);
let region = expand_memory_region(bctx, loc_i32, size, "newop")?;
build_mem_store_value(bctx, &region, val)
```

- `truncate_to_i32` maps values above `u32::MAX` to a sentinel that expansion rejects (ADR 004)
- Expansion rounds to 32-byte boundaries, updates `memory_len` monotonically and reallocates if needed
- Expansion gas is charged before memory changes

### 8. Tests

**File**: `crates/jet/tests/test_roms.rs`

Add the opcode to `define_ops!` at the top of the file, then add cases to a
`rom_tests!` block. Each case runs under both stack backends.

```rust
// Tests NEWOP basic case: <constraint from the reference page>
newop_basic_case: Test {
    roms: vec![bytecode![
        PUSH1!(0x42),
        NEWOP!(),
    ]],
    expected: TestContractRun {
        stack_ptr: 1,
        stack: vec![stack_word(&[0x42])],
        ..Default::default()
    },
},
```

**Required test coverage**:
- Basic functionality (happy path)
- Edge cases (zero, max values, boundaries)
- Endianness-sensitive tests (multi-byte values)
- Error conditions
- Memory expansion and gas where applicable, using `memory_len`,
  `gas_remaining` and `gas_failure` in `TestContractRun`

**Endianness test pattern**:
```rust
// Tests NEWOP with a multi-byte value; catches big-endian bugs
newop_endian_test: Test {
    roms: vec![bytecode![
        PUSH2!(0x01, 0x00), // 0x0100 big-endian = 256
        NEWOP!(),
    ]],
    expected: TestContractRun {
        stack_ptr: 1,
        stack: vec![stack_word(&[0x00, 0x01])], // little-endian bytes
        ..Default::default()
    },
},
```

Tests that need a specific `CallInfo` are plain `#[test]` functions using
`run_both_modes`; see the ADDRESS and CALLDATALOAD tests.

### 9. EVM Spec Verification

Confirm the implementation matches the reference page:
- Stack arguments (number and order)
- Stack results
- Side effects (memory, storage, logs)
- Error conditions (stack underflow, invalid jumps)
- Gas

### 10. Build and CI

Run locally:
```bash
make commit-check   # fmt-check, check, clippy, test-all
```

CI runs clippy with `-D warnings`, all tests with nextest, and doctests, and
auto-commits `cargo +nightly fmt` changes.

Common clippy issues:
- `manual_div_ceil`: Use `.div_ceil()` instead of `((x + n - 1) / n)`
- Unused variables: Remove or prefix with `_`
- Unsafe code: Document safety requirements

### 11. Documentation

Document the emitter when the logic is not obvious:
```rust
/// Implements NEWOP (0xNN): pops two words, ..., pushes the result.
pub(crate) fn newop<'ctx, S: StackBackend<'ctx>>(bctx: &BuildCtx<'ctx, '_, S>) -> Result<(), Error> {
```

Then update:
- `OPERATION_STATUS.md`: remove the opcode from the unimplemented table
- `docs/test_coverage.md`: mark it implemented and tested
- An ADR if an architectural decision was made

## Common Pitfalls

### Endianness Confusion
Stack words are little-endian after PUSH. Test with multi-byte values.

### Stack Management
Use the backend consistently:
- `bctx.stack.pop_word()`, `pop_2()`, `pop_3()`, `pop_7()`
- `bctx.stack.push_word()`, `push_word_with_known_u64()`

Never manipulate `Context.stack` directly, and keep the planner's stack effect
in `apply_abstract_instruction` in step with the emitter.

### Memory Expansion
Always obtain a `MemoryRegion` before accessing memory. Expansion:
- Rejects offset + size overflow
- Rounds to a 32-byte boundary
- Updates `memory_len` before use

### Type Conversions
- Stack words are `i256` values
- Use `truncate_to_i32()` for offsets and sizes; values above `u32::MAX` become a rejected sentinel
- Check if arithmetic ops need i256 or can use smaller types

### Builtin Function Signatures
Match the Rust function signature to the LLVM IR declaration exactly:
- Parameter types (ptr, i32, i256)
- Return type (usually i8 for error code)
- Calling convention (extern "C")

## Critical Mistakes to Avoid

### CRITICAL: Understanding Stack Operand Order

**The most common and serious mistake**: Misunderstanding EVM stack operand semantics.

#### How EVM Stack Actually Works

When you execute:
```
PUSH1 0x03
PUSH1 0x0A
SUB
```

The stack state is:
- Stack after first PUSH: `[0x03]`
- Stack after second PUSH: `[0x0A, 0x03]` (0x0A on top)

When `bctx.stack.pop_2(bctx)` is called:
1. First pop gets **top** of stack (0x0A) → returned as tuple element `a`
2. Second pop gets **second** from top (0x03) → returned as tuple element `b`
3. Returns `(a, b)` = `(0x0A, 0x03)`

For SUB, the operation computes: `a - b` = `0x0A - 0x03` = `7` ✓

#### Common Misunderstandings

**WRONG**: "The EVM spec says `a - b` where `a` is Stack Index 0 and `b` is Stack Index 1, so I need to do `b - a` in the implementation."

**CORRECT**: The EVM spec's "Stack Index 0" is the top of the stack (the value pushed last), and `pop_2` returns `(top, second)`, so the implementation uses `a - b` directly.

#### Verification Method

**Always verify with a simple manual trace:**
1. Write down the PUSH sequence
2. Draw the stack state after each PUSH (top of stack on left)
3. Check what `pop_2` will return (first pop = top)
4. Verify the operation produces the expected result

#### Test Coverage Required

For **every** non-commutative operation (SUB, DIV, MOD, etc.):
- Test basic case with clear expected result
- Test the "reversed" case to catch operand order bugs
- Example for SUB:
  - `10 - 3 = 7` (basic test)
  - `3 - 10 = -7` (underflow test catches order bugs)

### CRITICAL: Zero Divisor Handling

**EVM spec requirement**: Division and modulo by zero must return 0, NOT trigger undefined behavior.

LLVM's `build_int_unsigned_div` and `build_int_unsigned_rem` are undefined for
a zero divisor, and a `select` does not help because both arms are evaluated.
Use `build_zero_guarded_value`, which branches around the operation and merges
the result with a phi:

```rust
pub(crate) fn div<'ctx, S: StackBackend<'ctx>>(bctx: &BuildCtx<'ctx, '_, S>) -> Result<(), Error> {
    let (a, b) = bctx.stack.pop_2(bctx)?;
    let result = build_zero_guarded_value(bctx, b, "div", |bctx| {
        bctx.builder.build_int_unsigned_div(a, b, "div_result")
    })?;
    bctx.stack.push_word(bctx, result)
}
```

Same pattern applies to MOD, SDIV, SMOD, and any other division-like operations.

### CRITICAL: Read Opcode Documentation

**Before writing ANY test**, read the corresponding file in
`.agents/skills/evm-opcodes/references/docs/<HEX>.md`.

For **each** opcode you test:

1. **Read** the reference page completely
2. **Check** the "Stack input" section for operand order
3. **Review** all examples in the documentation
4. **Identify** all edge cases mentioned (zero values, overflow, special conditions)
5. **Write tests** for EVERY constraint and edge case listed
6. **Add comments** above each test explaining which constraint it verifies

Example:
```rust
// Tests MOD by zero: the reference page for 0x06 requires a % 0 = 0
mod_by_zero: Test {
    // ...
}
```

### CRITICAL: Macro Naming Conventions

**Rust lints enforce `snake_case` for macro names.**

Defining macros with ALL_CAPS names triggers `non_snake_case` warnings and
fails CI with `-D warnings`. The opcode macros are generated by `define_ops!`
with the necessary `#[allow(non_snake_case)]`; add new opcodes there instead
of writing macros by hand.

### Test Comments Are Mandatory

**Every test must have a comment explaining**:
1. What constraint from the spec it's testing
2. What the expected behavior is
3. If it's an edge case, why it matters

Bad:
```rust
mul_test: Test { ... }
```

Good:
```rust
// Tests MUL overflow: EVM spec requires result modulo 2^256
// Verifies that 2 * (2^256 - 1) = 2^256 - 2 (wrapped)
mul_overflow: Test { ... }
```

### Incremental Compilation Can Hide Bugs

**Symptom**: Tests fail even after fixing the implementation.

**Cause**: Cargo's incremental compilation cache doesn't always detect changes in proc macros or LLVM IR generation.

**Solution**: When in doubt, do a complete clean rebuild:
```bash
rm -rf $CARGO_TARGET_DIR
cargo test
```

On Termux specifically, the target directory is at `/data/data/com.termux/files/home/.cargo/jet-target`.

## Reference Implementations

Good examples to reference:

- **Pure LLVM IR**: `SIGNEXTEND` in `ops.rs` (shifts and selects)
- **Rust builtin**: `EXP` in `builtins.rs` (external function)
- **Memory ops**: `MSTORE` / `MSTORE8` / `RETURNDATACOPY` (memory regions)
- **Simple arithmetic**: `ADD` / `MUL` (basic stack operations)
- **Struct field reads**: `TIMESTAMP` and the other block info opcodes
- **Builtin-backed context reads**: `CALLDATALOAD`

## Commit Message

Follow conventional commits, without trailers:
```
feat: implement NEWOP (0xNN)

Brief description of what the opcode does and how it is lowered.
```
