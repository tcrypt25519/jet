# Troubleshooting Test Segfaults

## Overview

Test segmentation faults (SIGSEGV) are critical P0 issues that must be addressed immediately. Any segfault encountered during testing indicates a serious bug that could lead to crashes in production.

## Symptoms

```bash
cargo test  # Process didn't exit successfully (signal: 11, SIGSEGV: invalid memory reference)
```

## Debugging Steps

1. **Run with debug output:**
   ```bash
   RUST_LOG=debug cargo test
   ```

2. **Run single test to isolate the issue:**
   ```bash
   cargo test --test test_roms -- test_name --exact
   ```

3. **Use jetdbg binary for interactive debugging:**
   ```bash
   cargo run --bin jetdbg
   ```

4. **Check for common causes:**
   - Null pointer dereferences in runtime functions
   - Missing memory expansion before memory access (UAF/Use-After-Free)
   - Stack underflow not properly handled
   - Buffer overflows in builtin functions

5. **Verify memory expansion:**
   - All memory operations (MLOAD, MSTORE, MSTORE8, etc.) must call `mem_expand` before access
   - Memory expansion can reallocate, invalidating previous pointers

## Required Actions

**If you encounter a segfault:**

1. **Investigate immediately** - Do not continue with other work until the segfault is understood
2. **Fix if possible** - If you can identify and fix the root cause, do so
3. **Report if unable to fix** - Create a new issue with:
   - Full test name and reproduction steps
   - Debug output (RUST_LOG=debug)
   - Stack trace if available
   - Your analysis of the potential cause

**You MUST either:**
- Submit a pull request that fixes the segfault, OR
- Create a new issue that reports the segfault with detailed information

Do not leave segfaults unaddressed or undocumented.

## Common Root Causes

### Use-After-Free (UAF)
Memory expansion can reallocate the memory buffer, invalidating pointers obtained before expansion.

**Wrong:**
```rust
let ptr = mem_load(bctx, offset)?;  // Get pointer
mem_expand(bctx, offset, size)?;    // Reallocation invalidates ptr!
// Using ptr here is UAF
```

**Correct:**
```rust
mem_expand(bctx, offset, size)?;    // Expand first
let ptr = mem_load(bctx, offset)?;  // Then get pointer
```

### Null Pointer Dereference
Stack underflow or other error conditions can return null pointers.

**Check return values:**
```rust
let ptr = stack_pop(bctx)?;
// In builtin: always check ctx pointer is not null before dereferencing
if ctx.is_null() {
    return -1;  // Error code
}
```

### Buffer Overflow
Builtin functions must validate all buffer accesses.

```rust
// Always validate bounds
if offset + size > memory_len {
    return -1;  // Error
}
```

## Prevention

- Always call `mem_expand` before memory access
- Check return codes from runtime functions
- Validate all pointer arguments in builtins
- Use Rust's safety features (don't use `unsafe` unnecessarily)
- Write comprehensive tests including edge cases
- Run tests frequently during development
