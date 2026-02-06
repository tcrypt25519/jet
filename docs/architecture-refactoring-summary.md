# Runtime Architecture

**Related:** ADR-001, ADR-002

## Overview

The Jet runtime uses a unified type system and programmatically generated LLVM IR to ensure type safety and consistency across all components.

## Architecture Components

### `jet_ir` Crate (Shared IR Layer)

The `jet_ir` crate provides shared LLVM IR types and constants used throughout the system:

```
crates/jet_ir/
├── src/
│   ├── lib.rs          # Module exports
│   ├── types.rs        # Unified Types<'ctx> struct
│   └── constants.rs    # EVM and runtime constants
└── Cargo.toml
```

**Purpose:**
- Single source of truth for LLVM types
- Shared between runtime builder and contract builder
- Ensures consistent memory layouts across the system

**Key Type:** `Types<'ctx>` - Contains all LLVM types (i8, i32, i256, exec_ctx, etc.)

### Runtime Builder (IR Generation)

The `RuntimeBuilder` in `jet_runtime` generates LLVM IR programmatically:

**File:** `crates/jet_runtime/src/runtime_builder.rs`

```rust
let runtime_builder = RuntimeBuilder::new(context, "JetVM Runtime");
let module = runtime_builder.build();
```

**Generated Functions:**
- Stack operations: `jet.stack.push.i256`, `jet.stack.push.ptr`, `jet.stack.pop`, `jet.stack.peek`, `jet.stack.swap`
- Memory operations: `jet.mem.load`, `jet.mem.store.word`, `jet.mem.store.byte`
- Uses `Types` from `jet_ir` for consistent type definitions

**External Functions:**
- Contract calls and return data handling (complex logic in Rust)
- Crypto operations like keccak256 (requires external dependencies)

### Memory Model

The execution context uses pointer-based memory representation as defined in ADR-002:

#### exec_ctx Structure

```
struct exec_ctx {
    stack_ptr: i32,         // Offset 0
    jump_ptr: i32,          // Offset 4
    return_offset: i32,     // Offset 8
    return_length: i32,     // Offset 12
    sub_call: ptr,          // Offset 16
    stack: [1024 x i256],   // Offset 24
    memory_ptr: ptr,        // Offset 32,792
    memory_len: i32,        // Offset 32,800
    memory_cap: i32,        // Offset 32,804
}
Total: 32,808 bytes
```

**Memory Management:**
- Heap-allocated buffer pointed to by `memory_ptr`
- `memory_len` tracks current usage
- `memory_cap` tracks allocated capacity
- Allows dynamic memory growth as per EVM semantics

### Crate Dependencies

```
┌─────────────┐
│ jet (main)  │
│ - builder   │───┐
│ - engine    │   │
└─────────────┘   │
       │          │
       ▼          ▼
┌──────────────────────┐
│   jet_ir (shared)    │
│   - Types<'ctx>      │
│   - Constants        │
└──────────────────────┘
       ▲          ▲
       │          │
       │          │
┌──────────────────────┐
│   jet_runtime        │
│   - exec (Context)   │
│   - builtins         │
│   - RuntimeBuilder   │
└──────────────────────┘
```

**Dependency Flow:**
- `jet` → `jet_ir` + `jet_runtime`
- `jet_runtime` → `jet_ir`
- All components use the same `Types` definition

### Constants

EVM and runtime constants are defined in `jet_ir`:

```rust
// System architecture (EVM-defined)
pub const WORD_SIZE_BYTES: u32 = 32;
pub const STACK_SIZE_WORDS: u32 = 1024;
pub const ADDRESS_SIZE_BYTES: usize = 20;
pub const BLOCK_HASH_HISTORY_SIZE: usize = 256;

// Runtime sizes (Jet-defined)
pub const MEMORY_INITIAL_SIZE_WORDS: u32 = 1024;
pub const STORAGE_INITIAL_SIZE_WORDS: u32 = 1024;
pub const SUB_CALL_RETURN_MAX_SIZE_WORDS: u32 = 1024;
```

## Implementation Details

### Memory Management in Context

The `Context` struct allocates memory on the heap:

```rust
impl Context {
    pub fn new() -> Self {
        let memory_size = (WORD_SIZE_BYTES * MEMORY_INITIAL_SIZE_WORDS) as usize;
        let memory_layout = Layout::from_size_align(memory_size, 32)?;
        let memory_ptr = unsafe { alloc_zeroed(memory_layout) };
        // ...
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        // Deallocate memory buffer
        unsafe { dealloc(self.memory_ptr, memory_layout); }
    }
}
```

### Runtime Function Generation

IR functions are generated using Inkwell:

```rust
fn build_stack_push_i256(&self) -> FunctionValue<'ctx> {
    let fn_type = self.context.bool_type().fn_type(
        &[self.types.exec_ctx.ptr_type(...).into(), self.types.i256.into()],
        false,
    );
    
    let function = self.module.add_function("jet.stack.push.i256", fn_type, None);
    // ... build IR instructions using self.builder ...
}
```

## Benefits

- **Type Safety**: IR generation checked at Rust compile time
- **Correctness**: Consistent memory layouts prevent corruption
- **Maintainability**: Type changes propagate automatically
- **Flexibility**: Easy to add new runtime functions
- **Performance**: LLVM optimizations apply to generated IR
- **ADR-001 Compliance**: Runtime defined via IR builder

## References

- [ADR-001](adrs/adr-001.md): EVM Word Representation and Runtime Implementation Strategy
- [ADR-002](adrs/adr-002.md): Pointer-Based Memory Representation
- [Architecture](architecture.md): System architecture overview
- Runtime Builder: `crates/jet_runtime/src/runtime_builder.rs`
- Unified Types: `crates/jet_ir/src/types.rs`

