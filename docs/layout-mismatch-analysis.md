# Execution Context Layout Mismatch Analysis

## Overview

This document analyzes the critical layout mismatches in the execution context structure across three different representations in the Jet codebase:
1. Rust runtime (`Context` struct in `crates/jet_runtime/src/exec.rs`)
2. LLVM IR (`exec_ctx` type in `runtime-ir/jet.ll` and `crates/jet/runtime-ir/jet.ll`)
3. Compiler type system (`Types::exec_ctx` in `crates/jet/src/builder/env.rs`)

These mismatches cause memory corruption and incorrect field access during execution because the compiler generates GEP (GetElementPtr) instructions based on one layout while the runtime uses a different layout.

## Constants (from `crates/jet_runtime/src/lib.rs`)

```rust
pub const WORD_SIZE_BYTES: u32 = 32;
pub const STACK_SIZE_WORDS: u32 = 1024;
pub const MEMORY_INITIAL_SIZE_WORDS: u32 = 1024;
```

Calculated values:
- Stack size in bytes: 32 * 1024 = 32,768 bytes
- Initial memory size in bytes: 32 * 1024 = 32,768 bytes

## Layout Comparison

### Layout 1: Rust Runtime Context

**File:** `crates/jet_runtime/src/exec.rs:14-27`

```rust
#[repr(C)]
pub struct Context {
    stack_ptr: u32,                    // Offset 0,  Size 4
    jump_ptr: u32,                     // Offset 4,  Size 4
    return_off: u32,                   // Offset 8,  Size 4
    return_len: u32,                   // Offset 12, Size 4
    sub_call: Option<Box<Context>>,    // Offset 16, Size 8 (64-bit pointer)
    stack: [Word; STACK_SIZE_WORDS as usize],  // Offset 24, Size 32,768
    memory: [u8; (WORD_SIZE_BYTES * MEMORY_INITIAL_SIZE_WORDS) as usize],  // Offset 32,792, Size 32,768
    memory_len: u32,                   // Offset 65,560, Size 4
    memory_cap: u32,                   // Offset 65,564, Size 4
}
// Total size: 65,568 bytes
```

**Memory representation:** Inline array of 32,768 bytes

### Layout 2: LLVM IR exec_ctx

**Files:** `runtime-ir/jet.ll:31-41`, `crates/jet/runtime-ir/jet.ll:31-41`

```llvm
%jet.types.word = type [32 x i8]

%jet.types.exec_ctx = type <{
  i32,                         // Offset 0,     Size 4   - stack_ptr
  i32,                         // Offset 4,     Size 4   - jump_ptr
  i32,                         // Offset 8,     Size 4   - return_offset
  i32,                         // Offset 12,    Size 4   - return_length
  ptr,                         // Offset 16,    Size 8   - sub_call
  [1024 x %jet.types.word],    // Offset 24,    Size 32,768 - stack
  [1024 x i8],                 // Offset 32,792, Size 1,024 - memory
  i32,                         // Offset 33,816, Size 4   - mem_length
  i32                          // Offset 33,820, Size 4   - mem_capacity
}>
// Total size: 33,824 bytes
```

**Memory representation:** Inline array of 1,024 bytes (INCORRECT - should be 32,768)

### Layout 3: Compiler Types::exec_ctx

**File:** `crates/jet/src/builder/env.rs:112-131`

```rust
let mem = context.struct_type(
    &[ptr.into(), mem_len.into(), mem_cap.into()],
    PACK_STRUCTS  // true
);

let exec_ctx = context.struct_type(
    &[
        stack_ptr.into(),      // Offset 0,     Size 4
        jump_ptr.into(),       // Offset 4,     Size 4
        return_offset.into(),  // Offset 8,     Size 4
        return_length.into(),  // Offset 12,    Size 4
        ptr.into(),            // Offset 16,    Size 8   - sub_call
        stack.into(),          // Offset 24,    Size 32,768
        mem.into(),            // Offset 32,792, Size 16
    ],
    PACK_STRUCTS,
);
```

Where `mem` is:
```
struct {
    ptr: ptr,       // 8 bytes - pointer to memory buffer
    i32,            // 4 bytes - length
    i32             // 4 bytes - capacity
}
// Total: 16 bytes (with packing)
```

**Memory representation:** Heap-allocated via pointer (only 16 bytes in struct: ptr + len + cap)

**Total exec_ctx size:** 32,808 bytes

## Critical Mismatches

### Mismatch 1: Memory Field Size

| Layout | Memory Representation | Size in Struct | Location |
|--------|----------------------|----------------|----------|
| Rust Runtime | Inline `[u8; 32768]` | 32,768 bytes | Offset 32,792 |
| LLVM IR | Inline `[1024 x i8]` | 1,024 bytes | Offset 32,792 |
| Compiler Types | Pointer `{ptr, i32, i32}` | 16 bytes | Offset 32,792 |

### Mismatch 2: Total Structure Size

| Layout | Total Size |
|--------|-----------|
| Rust Runtime | 65,568 bytes |
| LLVM IR | 33,824 bytes |
| Compiler Types | 32,808 bytes |

### Mismatch 3: Field Offsets After Memory

The `memory_len` and `memory_cap` fields have different offsets:

| Layout | memory_len Offset | memory_cap Offset |
|--------|------------------|------------------|
| Rust Runtime | 65,560 | 65,564 |
| LLVM IR | 33,816 | 33,820 |
| Compiler Types | 32,792 (inside mem struct) | 32,796 (inside mem struct) |

## Impact Analysis

### Memory Corruption Scenarios

1. **GEP to memory field (index 6)**
   - Compiler generates: GEP to offset 32,792, expecting 16-byte struct
   - Runtime has: 32,768-byte inline array (Rust) or 1,024-byte inline array (LLVM IR)
   - Result: Accessing ptr/len/cap will read garbage data from memory array

2. **GEP to memory_len field (index 7 in Compiler Types, part of index 6)**
   - Compiler generates: GEP to mem.len (inside the 16-byte struct at offset 32,792)
   - Runtime has: Either at offset 65,560 (Rust) or 33,816 (LLVM IR)
   - Result: Reading/writing completely wrong memory location

3. **Stack allocation size**
   - When allocating Context on stack, size mismatch causes stack corruption
   - Different sizes: 65,568 vs 33,824 vs 32,808 bytes

## Pros and Cons Analysis

### Approach 1: Inline Memory Array (Rust Current)

**Pros:**
- Simple, contiguous memory layout
- No heap allocation needed for memory buffer
- No pointer indirection for memory access
- Cache-friendly (all data in one block)
- Matches Rust ownership semantics naturally
- Size is compile-time constant

**Cons:**
- Large structure size (65,568 bytes)
- Must be allocated on heap (too large for stack in most cases)
- Fixed size, cannot grow beyond initial capacity
- Wastes space if memory usage is small
- Expensive to copy or move

### Approach 2: Pointer-Based Memory (Compiler Types Current)

**Pros:**
- Small structure size (32,808 bytes)
- Can grow memory dynamically by reallocating
- Only allocates what's needed
- More flexible for varying memory requirements
- Standard Vec-like representation
- Easier to pass around (smaller structure)

**Cons:**
- Requires heap allocation for memory buffer
- Extra pointer indirection on every memory access
- Two separate allocations (Context + memory buffer)
- More complex lifetime management
- Potential cache misses (memory buffer separate from Context)
- Need to manage memory allocation/deallocation

### Approach 3: Hybrid (Inline with Growth Capability)

**Pros:**
- Start with inline array for common case
- Can switch to heap allocation if growth needed
- Optimizes for common path (small memory usage)
- Single allocation for common case

**Cons:**
- Most complex implementation
- Need to track whether using inline or heap memory
- Requires careful handling of growth transition
- May have both inline buffer AND pointer (wasted space)

## Recommendation: Pointer-Based Memory

**Winner: Approach 2 (Pointer-Based Memory)**

The pointer-based approach (currently in Compiler Types) should be adopted across all three representations.

### Rationale

1. **Flexibility:** EVM memory can grow during execution. The inline array approach is fundamentally incompatible with this requirement.

2. **Standard pattern:** The `{ptr, len, cap}` pattern is well-established (like Rust's `Vec`) and understood.

3. **Reasonable size:** 32,808 bytes is still large but more manageable than 65,568 bytes. The Context can be allocated on heap if needed.

4. **Memory efficiency:** Most contracts use far less than 32KB of memory. Allocating only what's needed saves resources.

5. **Alignment with EVM:** The EVM spec allows memory to grow unbounded (with gas costs). A fixed-size inline array doesn't match this model.

6. **GEP correctness:** Pointer-based representation makes field offsets predictable and consistent across all three layers.

## Recommended Changes

### Updated Layout Definition

All three representations should use:

```
exec_ctx = {
    stack_ptr: i32,        // 4 bytes
    jump_ptr: i32,         // 4 bytes
    return_offset: i32,    // 4 bytes
    return_length: i32,    // 4 bytes
    sub_call: ptr,         // 8 bytes
    stack: [1024 x word],  // 32,768 bytes (1024 * 32)
    memory_ptr: ptr,       // 8 bytes
    memory_len: i32,       // 4 bytes
    memory_cap: i32        // 4 bytes
}
Total: 32,808 bytes
```

### Changes Required

1. **`crates/jet_runtime/src/exec.rs`:**
   ```rust
   #[repr(C)]
   pub struct Context {
       stack_ptr: u32,
       jump_ptr: u32,
       return_off: u32,
       return_len: u32,
       sub_call: Option<Box<Context>>,
       stack: [Word; STACK_SIZE_WORDS as usize],
       memory_ptr: *mut u8,  // Changed from inline array
       memory_len: u32,
       memory_cap: u32,
   }
   ```

   Update `Context::new()` to allocate memory buffer and set pointer.

2. **`runtime-ir/jet.ll` and `crates/jet/runtime-ir/jet.ll`:**
   ```llvm
   %jet.types.exec_ctx = type <{
     i32,                      // stack_ptr
     i32,                      // jump_ptr
     i32,                      // return_offset
     i32,                      // return_length
     ptr,                      // sub_call
     [1024 x %jet.types.word], // stack
     ptr,                      // memory_ptr (changed from [1024 x i8])
     i32,                      // memory_len
     i32                       // memory_cap
   }>
   ```

3. **`crates/jet/src/builder/env.rs`:**
   ```rust
   let exec_ctx = context.struct_type(
       &[
           stack_ptr.into(),      // i32
           jump_ptr.into(),       // i32
           return_offset.into(),  // i32
           return_length.into(),  // i32
           ptr.into(),            // ptr - sub_call
           stack.into(),          // [1024 x i256]
           ptr.into(),            // ptr - memory_ptr (changed from mem struct)
           mem_len.into(),        // i32
           mem_cap.into(),        // i32
       ],
       PACK_STRUCTS,
   );
   ```

   Remove the `mem` struct definition, use individual fields instead.

## Improvements to Winning Layout

While the pointer-based approach is the winner, we should consider these refinements:

### 1. Small Buffer Optimization (SBO)

Add a small inline buffer (e.g., 256 bytes) for tiny allocations:

```rust
const INLINE_MEMORY_SIZE: usize = 256;

#[repr(C)]
pub struct Context {
    // ... other fields ...
    memory_inline: [u8; INLINE_MEMORY_SIZE],
    memory_ptr: *mut u8,
    memory_len: u32,
    memory_cap: u32,
    memory_uses_inline: bool,  // flag to track which buffer is active
}
```

**Benefits:**
- No heap allocation for very small contracts
- Better cache locality for common case
- Minimal complexity increase

**Trade-offs:**
- Increases struct size by 256 bytes
- Adds conditional logic to memory access

**Recommendation:** Consider this as a future optimization after establishing correctness with pure pointer-based approach.

### 2. Alignment Considerations

Ensure memory pointer is properly aligned for SIMD operations:

```rust
memory_ptr: *mut u8,  // Should be aligned to at least 32 bytes for AVX2
```

### 3. Memory Growth Strategy

Document and implement a consistent growth strategy (e.g., double capacity when growing), similar to `Vec`.

### 4. Lifetime and Safety

Consider using safer abstractions in Rust:
- Use `Box<[u8]>` or `Vec<u8>` instead of raw pointer where possible
- Document ownership and lifetime rules clearly
- Add safety invariants in comments

## Migration Path

1. **Phase 1:** Update all three layouts to use pointer-based representation
2. **Phase 2:** Update all memory access code to use pointer indirection
3. **Phase 3:** Add tests to verify layout consistency
4. **Phase 4:** Consider SBO optimization if profiling shows benefit

## Testing Strategy

1. **Layout verification:**
   - Static assert that all three representations have same size
   - Static assert that field offsets match across representations

2. **Runtime verification:**
   - Create Context in Rust, pass to LLVM, verify field access
   - Test memory allocation, growth, and deallocation
   - Test GEP operations generated by compiler

3. **Integration tests:**
   - Run existing EVM test suite
   - Add tests specifically for memory operations
   - Test edge cases (zero memory, large memory, growth)

## Conclusion

The pointer-based memory representation should be adopted universally across the Rust runtime, LLVM IR, and compiler type system. This provides:

1. **Correctness:** Consistent layout eliminates memory corruption bugs
2. **Flexibility:** Supports dynamic memory growth as required by EVM
3. **Efficiency:** Only allocates memory as needed
4. **Maintainability:** Standard `{ptr, len, cap}` pattern is well-understood

The current mismatch is a critical bug that will cause memory corruption and incorrect execution. Fixing this should be high priority.
