# Adding a New Runtime Function to Jet

Process documentation for adding an `extern "C"` builtin function callable from JIT-compiled EVM code.

## When to Add a Runtime Function

Add a runtime function when an EVM opcode requires logic that is too complex for inline LLVM IR:

- Crypto operations (e.g., KECCAK256)
- Arithmetic requiring extended precision (e.g., EXP, ADDMOD, MULMOD)
- Dynamic memory management (e.g., `jet.mem.expand`)
- Cross-contract calls

For simple arithmetic and stack/memory operations, prefer IR-defined functions in `RuntimeBuilder` instead.

## Checklist

### 1. Define the Symbol Name

**File**: `crates/jet_runtime/src/symbols.rs`

```rust
pub const FN_NEW_FUNC: &str = "jet.category.newfunc";
```

Follow the existing naming scheme: `jet.{category}.{operation}`.

### 2. Declare in RuntimeBuilder

**File**: `crates/jet_runtime/src/runtime_builder.rs`

Inside `declare_external_builtins()`, add a forward declaration:

```rust
self.module.add_function(
    "jet.category.newfunc",
    self.types.i8.fn_type(
        &[self.types.ptr.into(), self.types.ptr.into()],
        false,
    ),
    None,
);
```

The declaration makes the symbol visible in the IR module so the compiler can emit call instructions.

### 3. Implement in Rust

**File**: `crates/jet_runtime/src/builtins.rs`

```rust
/// # Safety
///
/// `ctx` must be a valid, non-null pointer to a `Context` that outlives this call.
/// `arg` must be a valid, non-null pointer to a `Word`.
pub unsafe extern "C" fn new_func(ctx: *mut Context, arg: *const Word) -> i8 {
    // SAFETY: caller guarantees ctx and arg are valid and aligned
    let ctx = unsafe { &mut *ctx };
    let arg = unsafe { &*arg };
    // implementation
    0 // success
}
```

Rules:
- Use `extern "C"` — JIT code calls via C ABI.
- Mark `unsafe` and document the safety contract.
- Return `i8`: `0` for success, non-zero for error.
- Input words are **little-endian**; use `from_le_bytes` / `to_le_bytes`.

### 4. Add to the Symbols Struct

**File**: `crates/jet/src/builder/env.rs`

Add a field to `Symbols`:

```rust
pub(crate) struct Symbols<'ctx> {
    // ...
    new_func: FunctionValue<'ctx>,
}
```

Initialize it in `Symbols::new()` by looking up the declared function:

```rust
let new_func = module
    .get_function(jet_runtime::symbols::FN_NEW_FUNC)
    .ok_or(Error::MissingSymbol(jet_runtime::symbols::FN_NEW_FUNC))?;
```

Add a public accessor:

```rust
pub(crate) fn new_func(&self) -> FunctionValue<'ctx> {
    self.new_func
}
```

### 5. Link at JIT Startup

**File**: `crates/jet/src/engine/mod.rs`

Inside the function that maps builtins:

```rust
map_fn(sym.new_func(), builtins::new_func as usize);
```

This binds the symbol name to the Rust function pointer at the time the JIT engine is created.

### 6. Call from an Opcode

**File**: `crates/jet/src/builder/ops.rs`

```rust
pub(crate) fn newop(bctx: &BuildCtx<'_, '_>) -> Result<(), Error> {
    let arg = __stack_pop_1(bctx)?;

    bctx.builder.build_call(
        bctx.env.symbols().new_func(),
        &[bctx.registers.exec_ctx.into(), arg.into()],
        "new_func_result",
    )?;

    Ok(())
}
```

### 7. Test

Add test cases to `crates/jet/tests/test_roms.rs` covering:
- Happy path
- Edge cases (zero, max values)
- Error conditions

See `docs/process/new-opcode.md` for the full test checklist.

## Common Pitfalls

**Little-endian inputs**: Stack words arrive little-endian. Use `from_le_bytes` in Rust, not `from_be_bytes`.

**Null pointer checks**: Validate pointer arguments before dereferencing; `stack_pop` can return null on underflow.

**Signature mismatch**: The Rust `extern "C"` signature must exactly match the LLVM IR declaration in `RuntimeBuilder`. Parameter count, types, and return type must all agree.

**Memory expansion before access**: If the builtin reads or writes EVM memory, call `jet.mem.expand` from the opcode handler in `ops.rs` before invoking the builtin.
