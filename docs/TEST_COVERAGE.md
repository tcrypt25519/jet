# Test Coverage Analysis

> **Last Updated**: 2026-02-13
> **Project**: jet EVM Compiler
> **Test Framework**: `/crates/jet/tests/test_roms.rs`

## Overview

This document provides a comprehensive analysis of test coverage for the jet EVM implementation, identifying testing gaps and providing actionable guidance for writing comprehensive tests.

### Current Coverage Summary

| Metric | Count |
|--------|------:|
| **Total Opcodes Defined** | 148 |
| **Implemented Opcodes** | ~80 (with meaningful implementation) |
| **Tested Opcodes** | 36 |
| **Untested (Implemented)** | ~44 |
| **Test Coverage Rate** | ~45% (36/80) |

### Coverage by Category

| Category | Implemented | Tested | Untested | Status |
|----------|------------|--------|----------|--------|
| **Arithmetic** | 11 | 10 | 1 | ✅ Well-tested |
| **Comparison & Bitwise** | 14 | 14 | 0 | ✅ Well-tested |
| **Cryptographic** | 1 | 0 | 1 | ❌ Broken implementation |
| **Stack Operations** | 48 | 0 | 48 | ❌ Indirect only |
| **Memory** | 3 | 3 | 0 | ✅ Good |
| **Control Flow** | 4 | 3 | 1 | ⚠️ Partial |
| **System** | 6 | 3 | 3 | ⚠️ Partial |
| **Call Data** | 2 | 2 | 0 | ⚠️ Under-tested |

**Legend**:
- ✅ **Well-tested**: Comprehensive tests with edge cases
- ⚠️ **Under-tested**: Basic tests exist, missing edge cases
- ❌ **No tests**: No explicit test coverage
- 🚫 **Not implemented**: Returns `UnimplementedInstruction` error

---

## Tested Opcodes (37 Total)

### Arithmetic Operations (10 tested) ✅

| Opcode | Hex | Description | Tests | Status |
|--------|-----|-------------|-------|--------|
| **ADD** | 0x01 | Addition | Basic tests | ✅ Tested |
| **MUL** | 0x02 | Multiplication | Basic + overflow tests | ✅ Tested |
| **SUB** | 0x03 | Subtraction | Basic + underflow tests | ✅ Tested |
| **DIV** | 0x04 | Division | Basic tests | ✅ Tested |
| **SDIV** | 0x05 | Signed division | 6 tests: basic, by zero, negative dividend/divisor, both negative, overflow (-2^255/-1) | ⚠️ Implementation bug: by zero returns 0 (correct) |
| **MOD** | 0x06 | Modulo | Basic tests | ✅ Tested |
| **SMOD** | 0x07 | Signed modulo | 4 tests: basic, by zero, negative dividend, negative divisor | ⚠️ Implementation bug: by zero returns dividend (should be 0) |
| **ADDMOD** | 0x08 | Addition modulo | 4 tests: basic, by zero, large values, no wrap | ⚠️ Implementation bug: by zero returns sum (should be 0) |
| **MULMOD** | 0x09 | Multiplication modulo | 4 tests: basic, by zero, large values, no wrap | ⚠️ Implementation bug: by zero returns product (should be 0) |
| **EXP** | 0x0A | Exponentiation | Basic tests | ✅ Tested |

### Comparison Operations (6 tested) ✅

| Opcode | Hex | Description | Tests | Status |
|--------|-----|-------------|-------|--------|
| **LT** | 0x10 | Less than (unsigned) | 4 tests: true, false, equal, zero boundary | ✅ Tested |
| **GT** | 0x11 | Greater than (unsigned) | 4 tests: true, false, equal, zero boundary | ✅ Tested |
| **SLT** | 0x12 | Signed less than | 4 tests: negative < zero, zero not < negative, positive, equal | ✅ Tested |
| **SGT** | 0x13 | Signed greater than | 4 tests: zero > negative, negative not > zero, positive, equal | ✅ Tested |
| **EQ** | 0x14 | Equality | 4 tests: true, false, zero, max value | ✅ Tested |
| **ISZERO** | 0x15 | Is zero | 3 tests: true, false, max value | ✅ Tested |

### Bitwise Operations (8 tested) ✅

| Opcode | Hex | Description | Tests | Status |
|--------|-----|-------------|-------|--------|
| **AND** | 0x16 | Bitwise AND | 3 tests: basic, all zeros, identity | ✅ Tested |
| **OR** | 0x17 | Bitwise OR | 3 tests: basic, identity with zero, all ones | ✅ Tested |
| **XOR** | 0x18 | Bitwise XOR | 3 tests: basic, identity with zero, self-cancel | ✅ Tested |
| **NOT** | 0x19 | Bitwise NOT | 3 tests: invert zeros, invert ones, single byte | ✅ Tested |
| **BYTE** | 0x1A | Extract byte | 3 tests: index 0 (MSB), out of range, from zero | ✅ Tested |
| **SHL** | 0x1B | Shift left | 3 tests: by 0, by 1, by 8 | ✅ Tested |
| **SHR** | 0x1C | Logical shift right | 3 tests: by 0, by 1, by 8 | ✅ Tested |
| **SAR** | 0x1D | Arithmetic shift right | 3 tests: positive by 0, positive by 1, negative preserves sign | ✅ Tested |

### Memory Operations (3 tested) ✅

| Opcode | Hex | Description | Tests | Status |
|--------|-----|-------------|-------|--------|
| **MLOAD** | 0x51 | Load from memory | Basic tests | ✅ Tested |
| **MSTORE** | 0x52 | Store to memory | Basic tests | ✅ Tested |
| **MSTORE8** | 0x53 | Store byte to memory | Basic tests | ✅ Tested |

### Control Flow (3 tested) ⚠️

| Opcode | Hex | Description | Tests | Status |
|--------|-----|-------------|-------|--------|
| **JUMP** | 0x56 | Unconditional jump | Basic jump test | ✅ Tested |
| **JUMPI** | 0x57 | Conditional jump | Not tested | ❌ Untested (implementation bug: type mismatch) |
| **JUMPDEST** | 0x5B | Jump destination | Used in JUMP tests | ✅ Tested |
| **PC** | 0x58 | Program counter | Basic test | ✅ Tested |

### System Operations (3 tested) ⚠️

| Opcode | Hex | Description | Tests | Status |
|--------|-----|-------------|-------|--------|
| **RETURN** | 0xF3 | Halt with return data | Basic tests | ✅ Tested |
| **RETURNDATASIZE** | 0x3D | Size of return data | Basic tests | ✅ Tested |
| **RETURNDATACOPY** | 0x3E | Copy return data | Basic tests | ✅ Tested |

### Cryptographic Operations (0 tested) ❌

| Opcode | Hex | Description | Tests | Status |
|--------|-----|-------------|-------|--------|
| **KECCAK256** | 0x20 | Keccak-256 hash | Test commented out | ❌ **BROKEN IMPLEMENTATION** - pops 1 value instead of 2 (offset, size); doesn't read from memory; hashes wrong data |

### Call Operations (2 tested) ⚠️

| Opcode | Hex | Description | Tests | Status |
|--------|-----|-------------|-------|--------|
| **CALL** | 0xF1 | Call contract | Basic test | ⚠️ Under-tested |
| **SIGNEXTEND** | 0x0B | Sign extend | Basic test | ⚠️ Under-tested |

---

## Untested Opcodes (11 Total)

### Arithmetic Operations (1 untested)

### Arithmetic Operations (1 untested)

| Opcode | Hex | Description | Key Edge Cases | Priority |
|--------|-----|-------------|----------------|----------|
| **SIGNEXTEND** | 0x0B | Sign extension | • Byte index validation<br>• Sign bit extraction<br>• Negative value extension<br>• Positive value truncation | **MEDIUM** |

**Why Medium Priority**: Sign extension is used in signed arithmetic operations. Already has basic test but needs edge cases.

---

### Comparison Operations ✅

All comparison operations (LT, GT, SLT, SGT, EQ, ISZERO) are now tested with comprehensive edge cases.

---

### Bitwise Operations ✅

All bitwise operations (AND, OR, XOR, NOT, BYTE, SHL, SHR, SAR) are now tested with edge cases.

**Note**: SHL, SHR, and SAR have implementation bugs (swapped arguments) documented in tests with FIXME comments.

---

### Control Flow (1 untested)

| Opcode | Hex | Description | Key Edge Cases | Priority |
|--------|-----|-------------|----------------|----------|
| **JUMPI** | 0x57 | Conditional jump | • Condition = 0 (no jump)<br>• Condition ≠ 0 (jump)<br>• Jump to invalid destination → error<br>• Jump to non-JUMPDEST → error<br>• Multiple conditional branches | **HIGH** |

**Why High Priority**: Conditional control flow is fundamental. JUMP is tested but JUMPI (conditional variant) has an implementation bug (type mismatch i64 vs i256) preventing testing.

---

### Stack Manipulation (48 untested)

#### DUP Operations (16 untested: DUP1-DUP16)

| Opcode Range | Hex Range | Description | Key Edge Cases | Priority |
|--------------|-----------|-------------|----------------|----------|
| **DUP1-DUP16** | 0x80-0x8F | Duplicate Nth stack item | • DUP1: duplicate top<br>• DUP16: duplicate 16th item<br>• Deep stack access<br>• Stack underflow prevention<br>• Verify correct depth | **MEDIUM** |

**Current Status**: Used extensively in tests but never explicitly tested in isolation.

#### SWAP Operations (16 untested: SWAP1-SWAP16)

| Opcode Range | Hex Range | Description | Key Edge Cases | Priority |
|--------------|-----------|-------------|----------------|----------|
| **SWAP1-SWAP16** | 0x90-0x9F | Swap top with Nth item | • SWAP1: swap top two items<br>• SWAP16: swap with 17th item<br>• Deep stack swap<br>• Verify correct swap depth<br>• Stack remains consistent | **MEDIUM** |

**Current Status**: Used in tests but never explicitly tested in isolation.

#### PUSH Operations (16 untested: PUSH1-PUSH16, partially tested)

| Opcode Range | Hex Range | Description | Key Edge Cases | Priority |
|--------------|-----------|-------------|----------------|----------|
| **PUSH0** | 0x5F | Push zero | • Pushes 0x00<br>• Single byte instruction | **LOW** |
| **PUSH1-PUSH32** | 0x60-0x7F | Push N bytes | • PUSH1: 1 byte immediate<br>• PUSH32: 32 bytes immediate<br>• Little-endian handling<br>• Partial push at code end | **LOW** |

**Current Status**: PUSH0 has basic test. PUSH1/PUSH2 used extensively. Other variants (PUSH3-PUSH32) untested.

**Why Medium/Low Priority**: Stack operations are used constantly in tests (implicit testing), but explicit tests ensure they work correctly in isolation and at boundaries.

---

### System Operations (3 untested)

| Opcode | Hex | Description | Key Edge Cases | Priority |
|--------|-----|-------------|----------------|----------|
| **STOP** | 0x00 | Halt execution | • Sets return code to Stop<br>• Empty return data<br>• Stack/memory state preserved | **LOW** |
| **POP** | 0x50 | Remove top stack item | • Non-empty stack<br>• Stack underflow prevention<br>• Stack pointer decrement | **LOW** |
| **REVERT** | 0xFD | Revert state changes | • Return code set to Revert<br>• Return data offset/length<br>• Memory preserved for return | **MEDIUM** |
| **INVALID** | 0xFE | Invalid opcode | • Consumes all gas<br>• Return code set to Invalid<br>• Halts execution | **MEDIUM** |
| **SELFDESTRUCT** | 0xFF | Contract destruction | • Transfers balance<br>• Marks contract for deletion<br>• Return code handling | **MEDIUM** |

**Current Status**: RETURN is tested. REVERT, INVALID, SELFDESTRUCT implemented but untested.

---

### Block/Environment (1 untested)

| Opcode | Hex | Description | Key Edge Cases | Priority |
|--------|-----|-------------|----------------|----------|
| **BLOCKHASH** | 0x40 | Get block hash | • Current implementation is stub<br>• Returns zero for all inputs<br>• Block number validation | **LOW** |

**Why Low Priority**: Currently a stub implementation. Test when actual implementation is added.

---

## Under-tested Opcodes

### KECCAK256 (0x20) - ⚠️ HIGH PRIORITY

**Current Coverage**:
```rust
keccak256_empty_hash: Test {
    // Tests: PUSH0 + KECCAK256 with zero offset/size
    // Verifies: Empty input hash = 0xc5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470
}
```

**Missing Test Cases**:
1. **Various input lengths**:
   - 1 byte input
   - 31 bytes (one word minus 1)
   - 32 bytes (exactly one word)
   - 33 bytes (crosses word boundary)
   - 64 bytes (exactly two words)
   - 100+ bytes (multiple words)

2. **Memory expansion edge cases**:
   - Hash at offset 0
   - Hash at offset 31 (word boundary)
   - Hash at offset 32 (second word)
   - Hash at high offset (test memory expansion)

3. **Known test vectors** (from Ethereum test suite):
   - `keccak256("") = 0xc5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470`
   - `keccak256("hello") = 0x1c8aff950685c2ed4bc3174f3472287b56d9517b9c948127319a09a7a36deac8`
   - `keccak256("Hello, World!") = ...`

4. **Zero-length at non-zero offset**:
   - PUSH 0 (size), PUSH 100 (offset), KECCAK256 → should hash zero bytes

**Priority Justification**: Cryptographic operations are security-critical and must be thoroughly tested against known vectors.

---

### CALL (0xF1) - ⚠️ HIGH PRIORITY

**Current Coverage**:
```rust
basic_call_with_return_data: Test {
    // Tests: Basic call to contract 0x01 with return data
    // Verifies: Return data size and RETURNDATACOPY
}
```

**Missing Test Cases**:
1. **Call result handling**:
   - Successful call → 1 on stack
   - Failed call → 0 on stack
   - Gas exhaustion handling

2. **Address validation**:
   - Call to non-existent contract
   - Call to empty contract
   - Call to contract 0x00

3. **Value transfer** (if implemented):
   - Call with value = 0
   - Call with value > 0

4. **Gas forwarding**:
   - Forward all gas
   - Forward limited gas
   - Insufficient gas for call

5. **Call depth limits**:
   - Call depth = 1
   - Call depth = 1024 (maximum)
   - Call depth > 1024 → error

6. **Return data edge cases**:
   - Call with no return data
   - Call with return data > requested size
   - Call with return data < requested size

**Priority Justification**: Cross-contract calls are complex and security-critical. Many attack vectors involve call behavior.

---

### RETURNDATASIZE (0x3D) / RETURNDATACOPY (0x3E) - ⚠️ MEDIUM PRIORITY

**Current Coverage**:
```rust
// Only tested indirectly via basic_call_with_return_data
```

**Missing Test Cases**:
1. **RETURNDATASIZE standalone**:
   - Before any call → 0
   - After successful call → return data size
   - After failed call → 0 or preserved?
   - Multiple calls → updates correctly

2. **RETURNDATACOPY edge cases**:
   - Copy zero bytes
   - Copy partial return data
   - Copy all return data
   - Copy with offset beyond return data size → error or zero pad?
   - Out-of-bounds copy behavior

**Priority Justification**: These opcodes have subtle behavior around call failures and out-of-bounds access.

---

### PC (0x58) - LOW PRIORITY

**Current Coverage**:
```rust
program_counter: Test {
    // Tests: PC at offsets 0, 1, 2, and after JUMP (offset 7)
    // Verifies: PC increments correctly and updates after jump
}
```

**Missing Test Cases**:
1. **PC at various bytecode positions**:
   - After PUSH1 (PC should skip immediate byte)
   - After PUSH32 (PC should skip 32 immediate bytes)
   - At very high offsets (64+, 100+, 255+)

2. **PC with multiple jumps**:
   - PC after first jump
   - PC after second jump
   - PC in nested jump scenarios

**Priority Justification**: PC has basic coverage. Additional tests are nice-to-have but not critical.

---

### PUSH0 (0x5F) - LOW PRIORITY

**Current Coverage**:
```rust
// Used as helper in keccak256_empty_hash test
```

**Missing Test Cases**:
1. **Explicit PUSH0 test**:
   - Verify stack contains exactly 0x00 (32 zero bytes)
   - Verify stack pointer increments by 1
   - Distinguish from PUSH1 with 0x00 byte

**Priority Justification**: PUSH0 is simple and already used in tests. Explicit test is nice-to-have.

---

## Testing Priority Recommendations

### 🔴 High Priority (Security-Critical)

Complete these tests **first** to ensure correctness of security-critical operations:

1. **Arithmetic with edge cases** (SDIV, SMOD, ADDMOD, MULMOD)
   - **Why**: Overflow/underflow bugs can lead to financial loss
   - **Focus**: Division by zero, signed overflow, large value handling
   - **Estimated effort**: 4-6 tests per opcode = 16-24 tests

2. **Comparison operations** (LT, GT, SLT, SGT, EQ, ISZERO)
   - **Why**: Fundamental to all conditional logic and control flow
   - **Focus**: Boundary values (0, max), signed vs unsigned
   - **Estimated effort**: 3-4 tests per opcode = 18-24 tests

3. **KECCAK256 expansion**
   - **Why**: Cryptographic correctness is non-negotiable
   - **Focus**: Known test vectors, various lengths
   - **Estimated effort**: 8-10 tests

4. **CALL expansion**
   - **Why**: Cross-contract interaction is complex and security-critical
   - **Focus**: Failure cases, gas handling, call depth
   - **Estimated effort**: 10-15 tests

5. **JUMPI conditional control flow**
   - **Why**: Conditional jumps are fundamental to contract logic
   - **Focus**: Jump/no-jump conditions, invalid destinations
   - **Estimated effort**: 5-8 tests

**Total High Priority**: ~57-81 additional tests

---

### 🟡 Medium Priority (Common Operations)

Complete these tests **second** to ensure correctness of frequently-used operations:

1. **Bitwise operations** (AND, OR, XOR, NOT, BYTE)
   - **Why**: Common in optimized contracts
   - **Focus**: Identity operations, all-zeros, all-ones
   - **Estimated effort**: 3-4 tests per opcode = 15-20 tests

2. **Shift operations** (SHL, SHR, SAR)
   - **Why**: Used in packing/unpacking and bit manipulation
   - **Focus**: Shift by 0, 1, 255, 256+; sign extension for SAR
   - **Estimated effort**: 4-5 tests per opcode = 12-15 tests

3. **Stack manipulation explicit tests** (DUP1-16, SWAP1-16)
   - **Why**: Ensure correctness at stack boundaries
   - **Focus**: DUP1, DUP16, SWAP1, SWAP16, mid-range variants
   - **Estimated effort**: 2-3 tests per category = 10-15 tests

4. **System operations** (REVERT, INVALID, SELFDESTRUCT)
   - **Why**: Error handling and contract lifecycle
   - **Focus**: Return codes, state preservation
   - **Estimated effort**: 2-3 tests per opcode = 6-9 tests

5. **RETURNDATASIZE/RETURNDATACOPY standalone**
   - **Why**: Subtle edge cases around call failures
   - **Focus**: Out-of-bounds, zero-length, multiple calls
   - **Estimated effort**: 5-7 tests

**Total Medium Priority**: ~48-66 additional tests

---

### 🟢 Low Priority (Simple or Stub Operations)

Complete these tests **last** or when time permits:

1. **STOP, POP**
   - **Why**: Simple operations with obvious behavior
   - **Estimated effort**: 1-2 tests each = 2-4 tests

2. **PUSH1-PUSH32 variants**
   - **Why**: Already tested indirectly, low complexity
   - **Estimated effort**: Sample tests for PUSH3, PUSH16, PUSH32 = 3-5 tests

3. **PC expansion**
   - **Why**: Already has basic coverage
   - **Estimated effort**: 2-3 additional tests

4. **PUSH0 explicit test**
   - **Why**: Already used in other tests
   - **Estimated effort**: 1 test

5. **BLOCKHASH**
   - **Why**: Stub implementation
   - **Estimated effort**: 2-3 tests when implemented

**Total Low Priority**: ~10-16 additional tests

---

### Overall Testing Roadmap

| Phase | Priority | Tests | Estimated Effort |
|-------|----------|-------|-----------------|
| **Phase 1** | High | 57-81 | 2-3 days |
| **Phase 2** | Medium | 48-66 | 1-2 days |
| **Phase 3** | Low | 10-16 | 0.5-1 day |
| **Total** | All | 115-163 | 3.5-6 days |

**Note**: Effort estimates assume familiarity with test framework and opcode specifications.

---

## Edge Cases Reference

This section documents common patterns that should be tested across applicable opcodes.

### Arithmetic Operations

| Edge Case | Description | Example |
|-----------|-------------|---------|
| **Division by zero** | EVM spec: `x / 0 = 0`, `x % 0 = 0` | `DIV`, `SDIV`, `MOD`, `SMOD`, `ADDMOD`, `MULMOD` |
| **Overflow** | Result wraps modulo 2^256 | `ADD(2^256-1, 2) = 1` |
| **Underflow** | Result wraps modulo 2^256 | `SUB(0, 1) = 2^256-1` |
| **Maximum values** | Operands = 2^256-1 | All arithmetic operations |
| **Zero operands** | One or both operands = 0 | All arithmetic operations |
| **Signed overflow** | `-2^255 / -1 = -2^255` (special case) | `SDIV` |
| **Sign preservation** | Result sign matches operands | `SMOD` sign = dividend sign |
| **No intermediate modulo** | Intermediate result not subject to 2^256 | `ADDMOD`, `MULMOD` |

**Example Test Cases**:
```rust
// SDIV signed overflow special case
sdiv_overflow: Test {
    roms: vec![bytecode![
        // Push -1 (0xFFFF...FFFF)
        PUSH32!(...all 0xFF bytes...),
        // Push -2^255 (0x8000...0000)
        PUSH32!(0x80, 0x00, ...),
        SDIV!(),
    ]],
    expected: TestContractRun {
        stack_ptr: 1,
        stack: vec![stack_word(&[0x00, 0x00, ..., 0x80])], // -2^255
        ..Default::default()
    },
}

// ADDMOD no intermediate modulo
addmod_large_values: Test {
    roms: vec![bytecode![
        PUSH1!(0x02),              // N = 2
        PUSH32!(...all 0xFF...),   // b = 2^256-1
        PUSH1!(0x02),              // a = 2
        ADDMOD!(),                 // (2 + 2^256-1) % 2 = 1
    ]],
    expected: TestContractRun {
        stack_ptr: 1,
        stack: vec![stack_word(&[0x01])],
        ..Default::default()
    },
}
```

---

### Bitwise and Shift Operations

| Edge Case | Description | Example |
|-----------|-------------|---------|
| **All zero bits** | Operand = 0x00...00 | All bitwise operations |
| **All one bits** | Operand = 0xFF...FF (2^256-1) | All bitwise operations |
| **Identity operations** | `x & x = x`, `x \| 0 = x`, `x ^ 0 = x` | `AND`, `OR`, `XOR` |
| **Self-cancellation** | `x ^ x = 0` | `XOR` |
| **Double negation** | `~~x = x` | `NOT` |
| **Single bit patterns** | 0x80...00, 0x00...01 | Shift operations |
| **Shift by 0** | Identity (no change) | `SHL`, `SHR`, `SAR` |
| **Shift by 1** | Basic shift | `SHL`, `SHR`, `SAR` |
| **Shift by 255** | Near-maximum shift | `SHL`, `SHR`, `SAR` |
| **Shift by 256+** | Result = 0 (or -1 for SAR with negative) | `SHL`, `SHR`, `SAR` |
| **Byte extraction boundaries** | Index 0, 31, 32+ | `BYTE` |

**Example Test Cases**:
```rust
// AND identity
and_identity: Test {
    roms: vec![bytecode![
        PUSH1!(0x5A),
        PUSH1!(0x5A),
        AND!(),
    ]],
    expected: TestContractRun {
        stack_ptr: 1,
        stack: vec![stack_word(&[0x5A])],
        ..Default::default()
    },
}

// SHL shift by 256 (overflow)
shl_overflow: Test {
    roms: vec![bytecode![
        PUSH2!(0x01, 0x00),  // shift = 256
        PUSH1!(0xFF),        // value = 0xFF
        SHL!(),
    ]],
    expected: TestContractRun {
        stack_ptr: 1,
        stack: vec![stack_word(&[0x00])], // Result is 0
        ..Default::default()
    },
}

// BYTE out of range
byte_out_of_range: Test {
    roms: vec![bytecode![
        PUSH1!(32),          // index = 32 (out of range)
        PUSH1!(0xFF),        // value = 0xFF
        BYTE!(),
    ]],
    expected: TestContractRun {
        stack_ptr: 1,
        stack: vec![stack_word(&[0x00])], // Result is 0
        ..Default::default()
    },
}
```

---

### Memory Operations

| Edge Case | Description | Example |
|-----------|-------------|---------|
| **Out-of-bounds read** | Read beyond allocated memory → zero padding | `MLOAD` |
| **Memory expansion** | Write extends memory in 32-byte chunks | `MSTORE`, `MSTORE8` |
| **Expansion boundaries** | Write at offsets 0, 31, 32, 63, 64, 96 | Memory tests |
| **Zero-length operations** | Size = 0 for copy/hash operations | `KECCAK256`, `RETURNDATACOPY` |
| **Maximum offset values** | High offsets (test memory expansion cost) | Memory operations |
| **Monotonic expansion** | Memory only grows, never shrinks | All memory writes |

**Example Test Cases** (already well-covered in current tests):
```rust
// See existing tests:
// - mstore_at_zero_expands_to_32
// - mstore_at_31_expands_to_64
// - mstore_at_32_expands_to_64
// - memory_expansion_is_monotonic
```

---

### Stack Operations

| Edge Case | Description | Example |
|-----------|-------------|---------|
| **Stack underflow prevention** | Ensure opcodes fail gracefully with insufficient stack | All opcodes |
| **Deep stack access** | DUP16, SWAP16 (maximum depth) | `DUP16`, `SWAP16` |
| **Empty stack operations** | Operations on empty stack | `POP` on empty stack |
| **Stack pointer consistency** | Stack pointer matches actual stack depth | All stack operations |

**Example Test Cases**:
```rust
// DUP16 deep stack access
dup16_deep_stack: Test {
    roms: vec![bytecode![
        PUSH1!(0x01), PUSH1!(0x02), PUSH1!(0x03), PUSH1!(0x04),
        PUSH1!(0x05), PUSH1!(0x06), PUSH1!(0x07), PUSH1!(0x08),
        PUSH1!(0x09), PUSH1!(0x0A), PUSH1!(0x0B), PUSH1!(0x0C),
        PUSH1!(0x0D), PUSH1!(0x0E), PUSH1!(0x0F), PUSH1!(0x10),
        DUP16!(),  // Duplicate 16th item (0x01)
    ]],
    expected: TestContractRun {
        stack_ptr: 17,
        stack: vec![
            stack_word(&[0x01]), stack_word(&[0x02]), ...,
            stack_word(&[0x10]), stack_word(&[0x01]), // Duplicated
        ],
        ..Default::default()
    },
}

// SWAP1 basic swap
swap1_basic: Test {
    roms: vec![bytecode![
        PUSH1!(0x42),
        PUSH1!(0x24),
        SWAP1!(),
    ]],
    expected: TestContractRun {
        stack_ptr: 2,
        stack: vec![stack_word(&[0x24]), stack_word(&[0x42])],
        ..Default::default()
    },
}
```

---

### Control Flow

| Edge Case | Description | Example |
|-----------|-------------|---------|
| **Invalid jump destinations** | Jump to non-JUMPDEST → error | `JUMP`, `JUMPI` |
| **Jump to invalid PC** | Jump beyond code length → error | `JUMP`, `JUMPI` |
| **Conditional jump conditions** | JUMPI with zero/non-zero conditions | `JUMPI` |
| **Call depth limits** | Maximum 1024 call depth | `CALL` |
| **Gas exhaustion** | Insufficient gas for operation | All opcodes |

**Example Test Cases**:
```rust
// JUMPI with zero (no jump)
jumpi_no_jump: Test {
    roms: vec![bytecode![
        PUSH1!(0x00),      // condition = 0 (false)
        PUSH1!(0x06),      // destination
        JUMPI!(),          // Should NOT jump
        PUSH1!(42),        // Should execute this
        STOP!(),
        JUMPDEST!(),       // Jump destination (not reached)
        PUSH1!(99),
    ]],
    expected: TestContractRun {
        stack_ptr: 1,
        stack: vec![stack_word(&[42])],
        ..Default::default()
    },
}

// JUMPI with non-zero (jump)
jumpi_jump: Test {
    roms: vec![bytecode![
        PUSH1!(0x01),      // condition = 1 (true)
        PUSH1!(0x07),      // destination
        JUMPI!(),          // Should jump
        PUSH1!(42),        // Should NOT execute this
        STOP!(),
        JUMPDEST!(),       // Jump destination (reached)
        PUSH1!(99),
    ]],
    expected: TestContractRun {
        stack_ptr: 1,
        jump_ptr: 7,
        stack: vec![stack_word(&[99])],
        ..Default::default()
    },
}
```

---

## Test Framework Quick Reference

### Basic Test Structure

```rust
rom_tests! {
    test_name: Test {
        roms: vec![bytecode![
            PUSH1!(value),
            OPCODE!(),
        ]],
        expected: TestContractRun {
            result: ReturnCode::Stop,
            stack_ptr: 1,
            stack: vec![stack_word(&[expected_value])],
            memory: Some(vec![...]),
            memory_len: Some(32),
            ..Default::default()
        },
    },
}
```

### Helper Macros Available

Located in `/crates/jet/tests/test_roms.rs`:

```rust
// Construct bytecode from instruction sequences
bytecode![PUSH1!(0x01), ADD!(), PUSH1!(0x02)]

// Push operations
PUSH1!(byte)           // Push 1 byte: vec![0x60, byte]
PUSH2!(b1, b2)         // Push 2 bytes: vec![0x61, b1, b2]
// Note: PUSH3-PUSH32 not yet defined in test macros

// Opcode macros (defined in lines 39-59)
PUSH0!()              ADD!()               MUL!()
SUB!()                DIV!()               MOD!()
EXP!()                SIGNEXTEND!()        KECCAK256!()
JUMP!()               JUMPDEST!()          PC!()
MLOAD!()              MSTORE!()            MSTORE8!()
RETURN!()             CALL!()
RETURNDATASIZE!()     RETURNDATACOPY!()

// Construct 256-bit stack word (little-endian)
stack_word(&[0x42])                    // 0x42 at byte 0, rest zeros
stack_word(&[0x01, 0x02, 0x03])       // Little-endian: 0x030201...
```

### Adding New Opcode Test Macros

To add support for testing a new opcode:

1. **Add macro definition** (after line 59):
```rust
define_ops!(
    // ... existing opcodes ...
    SDIV,      // Add your opcode here
    SMOD,
    // ...
);
```

2. **Use in test**:
```rust
sdiv_basic: Test {
    roms: vec![bytecode![
        PUSH1!(0x02),
        PUSH1!(0x0A),
        SDIV!(),
    ]],
    expected: TestContractRun {
        stack_ptr: 1,
        stack: vec![stack_word(&[0x05])],
        ..Default::default()
    },
}
```

### TestContractRun Fields

```rust
pub struct TestContractRun {
    pub result: ReturnCode,         // Stop, ExplicitReturn, InvalidJumpBlock, etc.
    pub stack_ptr: usize,           // Number of items on stack
    pub stack: Vec<[u8; 32]>,       // Stack contents (little-endian 256-bit words)
    pub memory: Option<Vec<u8>>,    // Memory contents (byte array)
    pub memory_len: Option<usize>,  // Total allocated memory length
    pub jump_ptr: usize,            // Current instruction pointer (PC)
    pub return_offset: usize,       // Return data offset (for RETURN)
    pub return_length: usize,       // Return data length (for RETURN)
}
```

**Default values**:
- `result`: `ReturnCode::Stop`
- `stack_ptr`: 0
- `stack`: empty vec
- `memory`: None
- `memory_len`: None
- `jump_ptr`: 0
- `return_offset`: 0
- `return_length`: 0

Use `..Default::default()` to fill in defaults for fields you don't need to check.

### Stack Word Construction

```rust
// Single byte (little-endian, zero-padded)
stack_word(&[0x42])
// Result: [0x42, 0x00, 0x00, ..., 0x00] (32 bytes)

// Multiple bytes (little-endian)
stack_word(&[0x01, 0x02, 0x03])
// Result: [0x01, 0x02, 0x03, 0x00, ..., 0x00] (32 bytes)

// Full 32-byte value (little-endian)
stack_word(&[
    0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07,
    0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F,
    0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17,
    0x18, 0x19, 0x1A, 0x1B, 0x1C, 0x1D, 0x1E, 0x1F,
])

// All ones (2^256 - 1)
{
    let mut w = [0xFF_u8; 32];
    w  // Use inline block for complex construction
}

// Signed negative value (-1)
{
    let mut w = [0xFF_u8; 32];
    w
}

// -2^255 (most negative signed value)
{
    let mut w = [0x00_u8; 32];
    w[31] = 0x80;  // Set sign bit (most significant bit)
    w
}
```

**Important**: Stack words are stored in **little-endian** format in jet. This means:
- `stack_word(&[0x01, 0x02])` represents `0x0201` (not `0x0102`)
- Least significant byte at index 0
- Most significant byte at index 31

### Multi-ROM Tests (with CALL)

```rust
basic_call_with_return_data: Test {
    roms: vec![
        // ROM 0: Main contract
        bytecode![
            // ... setup CALL parameters ...
            PUSH2!(0x00, 0x01),  // address = contract 1
            PUSH1!(0x00),
            CALL!(),
            RETURNDATASIZE!(),
            // ...
        ],
        // ROM 1: Called contract
        bytecode![
            PUSH1!(0xFF),
            PUSH1!(0x00),
            MSTORE!(),
            PUSH1!(0x20),
            PUSH1!(0x00),
            RETURN!(),
        ]
    ],
    expected: TestContractRun {
        // ... expectations ...
    },
}
```

### Running Tests

```bash
# Run all tests
cargo test --test test_roms

# Run specific test
cargo test --test test_roms -- basic_jump

# Run with output
cargo test --test test_roms -- --nocapture

# Run with Makefile (sets CARGO_TARGET_DIR for Termux)
make test
```

### Common Patterns

#### Testing arithmetic overflow
```rust
mul_overflow: Test {
    roms: vec![vec![
        Instruction::PUSH1.opcode(), 0x02,
        Instruction::PUSH32.opcode(),
        0xFF, 0xFF, /* ... 30 more 0xFF bytes ... */, 0xFF,
        Instruction::MUL.opcode(),
    ]],
    expected: TestContractRun {
        stack_ptr: 1,
        stack: vec![{
            let mut w = [0xFF_u8; 32];
            w[0] = 0xFE;  // (2^256-1) * 2 = 2^257 - 2, mod 2^256 = 2^256 - 2
            w
        }],
        ..Default::default()
    },
}
```

#### Testing memory expansion
```rust
mstore_at_32_expands_to_64: Test {
    roms: vec![bytecode![
        PUSH1!(0x42),
        PUSH1!(0x20),  // offset = 32
        MSTORE!(),
    ]],
    expected: TestContractRun {
        stack_ptr: 0,
        memory_len: Some(64),  // ceil((32 + 32) / 32) * 32 = 64
        ..Default::default()
    },
}
```

#### Testing control flow
```rust
basic_jump: Test {
    roms: vec![bytecode![
        PUSH1!(0x03),
        JUMP!(),
        JUMPDEST!(),
        PUSH1!(42),
    ]],
    expected: TestContractRun {
        stack_ptr: 1,
        jump_ptr: 3,           // PC after jump
        stack: vec![stack_word(&[42])],
        ..Default::default()
    },
}
```

---

## Opcode Documentation Reference

Detailed opcode specifications are available in `/storage/emulated/0/code/jet/docs/opcodes/*.mdx` (170 files).

### File Naming Convention

- Hex opcode → filename: `0x05` → `05.mdx`, `0x1B` → `1B.mdx`
- Each file contains:
  - YAML front matter (fork, group)
  - Stack input/output specifications
  - Examples with input/output tables
  - Error cases
  - Special notes

### Example: Reading Opcode Documentation

For SDIV (0x05):
```bash
cat /storage/emulated/0/code/jet/docs/opcodes/05.mdx
```

Key sections:
- **Stack input**: `a` (numerator), `b` (denominator)
- **Stack output**: `a // b` (signed division result, 0 if b = 0)
- **Examples**: `10 / 10 = 1`, `-2 / -1 = 2` (special overflow case)
- **Notes**: Two's complement signed integers, overflow semantics

### Quick Reference: Opcode Categories

| Category | Hex Range | File Pattern |
|----------|-----------|--------------|
| Arithmetic | 0x00-0x0B | `00.mdx`-`0B.mdx` |
| Comparison & Bitwise | 0x10-0x1D | `10.mdx`-`1D.mdx` |
| Cryptographic | 0x20 | `20.mdx` |
| Environment | 0x30-0x3F | `30.mdx`-`3F.mdx` |
| Block Information | 0x40-0x4A | `40.mdx`-`4A.mdx` |
| Stack/Memory/Storage | 0x50-0x5E | `50.mdx`-`5E.mdx` |
| Push | 0x5F-0x7F | `5F.mdx`-`7F.mdx` |
| Dup | 0x80-0x8F | `80.mdx`-`8F.mdx` |
| Swap | 0x90-0x9F | `90.mdx`-`9F.mdx` |
| Log | 0xA0-0xA4 | `A0.mdx`-`A4.mdx` |
| System | 0xF0-0xFF | `F0.mdx`-`FF.mdx` |

---

## Maintenance

### Updating This Document

As tests are added:

1. **Mark opcodes as tested**: Move from "Untested" to "Under-tested" or remove entirely
2. **Update statistics**: Increment tested count, decrement untested count
3. **Add new edge cases**: Document any new patterns discovered during testing
4. **Update priority**: Adjust priorities based on implementation progress
5. **Track progress**: Use git history to see test coverage improvements over time

### Tracking Progress with Git

```bash
# See test coverage improvements
git log --oneline -- docs/TEST_COVERAGE.md crates/jet/tests/test_roms.rs

# Compare test count
git diff HEAD~5 -- crates/jet/tests/test_roms.rs | grep "^+.*: Test {"

# Generate test coverage report
grep -c ": Test {" crates/jet/tests/test_roms.rs
```

---

## Appendix: Quick Opcode Lookup

### Implemented Opcodes (48 total)

| Hex | Opcode | Tested | Category |
|-----|--------|--------|----------|
| 0x00 | STOP | ❌ | System |
| 0x01 | ADD | ✅ | Arithmetic |
| 0x02 | MUL | ✅ | Arithmetic |
| 0x03 | SUB | ✅ | Arithmetic |
| 0x04 | DIV | ✅ | Arithmetic |
| 0x05 | SDIV | ❌ | Arithmetic |
| 0x06 | MOD | ✅ | Arithmetic |
| 0x07 | SMOD | ❌ | Arithmetic |
| 0x08 | ADDMOD | ❌ | Arithmetic |
| 0x09 | MULMOD | ❌ | Arithmetic |
| 0x0A | EXP | ✅ | Arithmetic |
| 0x0B | SIGNEXTEND | ✅ | Arithmetic |
| 0x10 | LT | ❌ | Comparison |
| 0x11 | GT | ❌ | Comparison |
| 0x12 | SLT | ❌ | Comparison |
| 0x13 | SGT | ❌ | Comparison |
| 0x14 | EQ | ❌ | Comparison |
| 0x15 | ISZERO | ❌ | Comparison |
| 0x16 | AND | ❌ | Bitwise |
| 0x17 | OR | ❌ | Bitwise |
| 0x18 | XOR | ❌ | Bitwise |
| 0x19 | NOT | ❌ | Bitwise |
| 0x1A | BYTE | ❌ | Bitwise |
| 0x1B | SHL | ❌ | Bitwise |
| 0x1C | SHR | ❌ | Bitwise |
| 0x1D | SAR | ❌ | Bitwise |
| 0x20 | KECCAK256 | ⚠️ | Crypto |
| 0x3D | RETURNDATASIZE | ⚠️ | Call Data |
| 0x3E | RETURNDATACOPY | ⚠️ | Call Data |
| 0x40 | BLOCKHASH | ❌ | Block Info |
| 0x50 | POP | ❌ | Stack |
| 0x51 | MLOAD | ✅ | Memory |
| 0x52 | MSTORE | ✅ | Memory |
| 0x53 | MSTORE8 | ✅ | Memory |
| 0x56 | JUMP | ✅ | Control Flow |
| 0x57 | JUMPI | ❌ | Control Flow |
| 0x58 | PC | ⚠️ | Control Flow |
| 0x5B | JUMPDEST | ✅ | Control Flow |
| 0x5F | PUSH0 | ⚠️ | Stack |
| 0x60-0x7F | PUSH1-PUSH32 | ⚠️ | Stack (indirect) |
| 0x80-0x8F | DUP1-DUP16 | ❌ | Stack (indirect) |
| 0x90-0x9F | SWAP1-SWAP16 | ❌ | Stack (indirect) |
| 0xF1 | CALL | ⚠️ | System |
| 0xF3 | RETURN | ✅ | System |
| 0xFD | REVERT | ❌ | System |
| 0xFE | INVALID | ❌ | System |
| 0xFF | SELFDESTRUCT | ❌ | System |

**Legend**:
- ✅ **Well-tested**: Comprehensive tests with edge cases
- ⚠️ **Under-tested**: Basic tests exist, missing edge cases
- ❌ **No tests**: No explicit test coverage

---

## Contributing Tests

### Before Writing Tests

1. **Read this document**: Understand priorities and edge cases
2. **Check opcode documentation**: Read `/docs/opcodes/<hex>.mdx` for specification
3. **Review existing tests**: See patterns in `/crates/jet/tests/test_roms.rs`
4. **Identify edge cases**: Use "Edge Cases Reference" section above

### Writing Tests

1. **Add opcode macro** (if not exists): Update `define_ops!` macro
2. **Write test case**: Follow structure in "Test Framework Quick Reference"
3. **Test edge cases**: Include at least 2-3 edge cases per opcode
4. **Verify against spec**: Cross-reference with opcode documentation
5. **Run tests**: `make test` or `cargo test --test test_roms`

### After Writing Tests

1. **Update this document**: Mark opcodes as tested
2. **Update statistics**: Increment tested count
3. **Submit PR**: Include test results and coverage improvements

---

## References

- **Test Framework**: `/crates/jet/tests/test_roms.rs`
- **Opcode Implementations**: `/crates/jet/src/builder/ops.rs`
- **Opcode Dispatch**: `/crates/jet/src/builder/contract.rs` (lines 389-655)
- **Opcode Definitions**: `/crates/jet/src/instructions.rs`
- **Opcode Documentation**: `/docs/opcodes/*.mdx` (170 files)
- **EVM Specification**: [ethereum.org/en/developers/docs/evm/opcodes](https://ethereum.org/en/developers/docs/evm/opcodes/)

---

**Document Version**: 1.0
**Last Updated**: 2026-02-12
**Maintained By**: jet development team
