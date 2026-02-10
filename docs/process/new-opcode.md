# Adding New EVM Opcodes to Jet

Process documentation for implementing EVM opcodes in the Jet compiler.

## Checklist

### 1. Instruction Definition

**File**: `crates/jet/src/instructions.rs`

Add the opcode to the `Instruction` enum:
```rust
pub enum Instruction {
    // ...
    NEWOP = 0xNN,
}
```

Verify:
- Correct opcode value (match EVM spec)
- Entry in `opcode()` method
- Entry in `from_opcode()` method
- Entry in `name()` method for debugging

### 2. Opcode Handler Implementation

**File**: `crates/jet/src/builder/ops.rs`

Implement the opcode function:
```rust
pub(crate) fn newop(bctx: &BuildCtx<'_, '_>) -> Result<(), Error> {
    // Implementation
}
```

Requirements:
- Pop stack arguments using `__stack_pop_1()`, `__stack_pop_2()`, etc.
- Push results using `__stack_push_int()` or `__stack_push_ptr()`
- Call builtins via `bctx.env.symbols().builtin_name()`
- Handle errors with `Result<(), Error>`

### 3. Opcode Routing

**File**: `crates/jet/src/builder/mod.rs`

Add case to match statement in instruction compiler:
```rust
match instr {
    // ...
    Instruction::NEWOP => ops::newop(&bctx)?,
}
```

### 4. Data Representation (Critical)

**Endianness**: Stack words are stored **little-endian** internally.

- PUSH immediates are byte-reversed on load
- By the time data reaches builtins, it's already little-endian
- Use `from_le_bytes()` / `to_le_bytes()` in Rust builtins
- Use bnum's `from_digits()` with `u64::from_le_bytes()` for 256-bit values

Example (EXP opcode):
```rust
let read = |b: &[u8; 32]| {
    U256::from_digits([
        u64::from_le_bytes(b[0..8].try_into().unwrap()),
        u64::from_le_bytes(b[8..16].try_into().unwrap()),
        u64::from_le_bytes(b[16..24].try_into().unwrap()),
        u64::from_le_bytes(b[24..32].try_into().unwrap()),
    ])
};
```

### 5. Complex Operations (Builtins)

For operations requiring Rust implementation:

**File**: `crates/jet_runtime/src/builtins.rs`

```rust
pub extern "C" fn jet_ops_newop(arg: &mut [u8; 32]) -> i8 {
    // Implementation using little-endian representation
    0 // success
}
```

**File**: `crates/jet_runtime/src/symbols.rs`

```rust
pub const FN_NEWOP: &str = "jet.ops.newop";
```

**File**: `crates/jet_runtime/src/runtime_builder.rs`

Declare in `declare_external_builtins()`:
```rust
self.module.add_function(
    "jet.ops.newop",
    self.types.i8.fn_type(&[self.types.ptr.into()], false),
    None,
);
```

**File**: `crates/jet/src/builder/env.rs`

Add to `Symbols` struct:
```rust
pub(crate) struct Symbols<'ctx> {
    // ...
    newop: FunctionValue<'ctx>,
}
```

Initialize in `new()`:
```rust
let newop = module.get_function(jet_runtime::symbols::FN_NEWOP)?;
```

Add to struct initialization and accessor method.

**File**: `crates/jet/src/engine/mod.rs`

Link in JIT engine:
```rust
map_fn(sym.newop(), builtins::jet_ops_newop as *const () as usize);
```

### 6. Memory Operations

If the opcode reads/writes memory:

**Call memory expansion before access**:
```rust
let offset_i32 = load_i32(bctx, offset_ptr)?;
let size = bctx.env.types().i32.const_int(SIZE, false);
bctx.builder.build_call(
    bctx.env.symbols().mem_expand(),
    &[bctx.registers.exec_ctx.into(), offset_i32.into(), size.into()],
    "expand",
)?;
```

Memory expansion:
- Rounds to 32-byte boundaries
- Updates `memory_len` monotonically
- Reallocates if needed
- Must be called BEFORE memory access

### 7. Tests

**File**: `crates/jet/tests/test_roms.rs`

Add test cases to `rom_tests!` macro:

```rust
opcode_basic_case: Test {
    roms: vec![vec![
        Instruction::PUSH1.opcode(), 0x42,
        Instruction::NEWOP.opcode(),
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
- Memory expansion (if applicable)

**Endianness test pattern**:
```rust
// Test that catches big-endian bugs
opcode_endian_test: Test {
    roms: vec![vec![
        Instruction::PUSH2.opcode(), 0x01, 0x00, // 0x0100 big-endian = 256
        Instruction::NEWOP.opcode(),
    ]],
    expected: TestContractRun {
        stack_ptr: 1,
        stack: vec![{
            let mut w = [0u8; 32];
            w[0] = 0x00;  // Little-endian LSB
            w[1] = 0x01;  // Little-endian MSB
            w
        }],
        ..Default::default()
    },
},
```

### 8. Gas Accounting (Future)

Placeholder for when gas is implemented:
- Check EVM Yellow Paper for gas costs
- Add metering calls before expensive operations
- Include memory expansion costs

### 9. EVM Spec Verification

Confirm implementation matches Ethereum spec:
- Stack arguments (number and order)
- Stack results
- Side effects (memory, storage, logs)
- Error conditions (stack underflow, invalid jumps)
- Edge cases in Yellow Paper

Reference: `docs/ext/evm/opcodes.json`

### 10. Build and CI

Run locally:
```bash
make test
cargo clippy --all-targets --all-features
```

CI will:
- Run clippy (enforces Rust idioms)
- Run all tests with nextest
- Check formatting with rustfmt

Common clippy issues:
- `manual_div_ceil`: Use `.div_ceil()` instead of `((x + n - 1) / n)`
- Unused variables: Remove or prefix with `_`
- Unsafe code: Document safety requirements

### 11. Documentation

Add inline documentation for complex logic:
```rust
/// Implements the NEWOP opcode (0xNN).
///
/// Pops two values from stack, performs operation, pushes result.
/// Uses little-endian representation internally.
pub(crate) fn newop(bctx: &BuildCtx<'_, '_>) -> Result<(), Error> {
```

Update docs if behavior differs from standard EVM:
- `docs/ext/evm/evm.md` for EVM semantics
- ADR document if architectural decision made

## Common Pitfalls

### Endianness Confusion
Stack words are little-endian after PUSH. Test with multi-byte values.

### Stack Management
Use helper functions consistently:
- `__stack_pop_1()`, `__stack_pop_2()`, etc.
- `__stack_push_int()`, `__stack_push_ptr()`

Never manipulate stack directly.

### Memory Expansion
Always call `mem_expand` BEFORE accessing memory. Expansion must:
- Check offset + size overflow
- Round to 32-byte boundary
- Update `memory_len` before use

### Type Conversions
- Stack pointers point to 32-byte values
- Use `load_i32()` / `load_i256()` to convert
- Check if arithmetic ops need i256 or can use smaller types

### Builtin Function Signatures
Match Rust function signature to LLVM IR declaration exactly:
- Parameter types (ptr, i32, i256)
- Return type (usually i8 for error code)
- Calling convention (extern "C")

## Reference Implementations

Good examples to reference:

- **Pure LLVM IR**: `SIGNEXTEND` in `ops.rs` (shifts and selects)
- **Rust builtin**: `EXP` in `builtins.rs` (external function)
- **Memory ops**: `MSTORE` / `MSTORE8` (memory expansion)
- **Simple arithmetic**: `ADD` / `MUL` (basic stack operations)

## Commit Message

Follow conventional commits:
```
feat: implement [OPCODE] (0xNN)

[Brief description of what the opcode does]

Implementation:
- [Key technical details]
- [Approach taken]

Tests:
- [Test coverage summary]

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>
```
