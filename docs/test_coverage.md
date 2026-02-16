# Test Coverage

> **Last Updated**: 2026-02-15
> **Test Framework**: `crates/jet/tests/test_roms.rs`

## Summary

| Metric | Count |
|--------|------:|
| **Total Opcodes Defined** | 148 |
| **Implemented Opcodes** | ~80 (with meaningful implementation) |
| **Tested Opcodes** | 36 |
| **Untested (Implemented)** | ~44 |
| **Test Coverage Rate** | ~45% (36/80) |

## Coverage by Category

| Category | Implemented | Tested | Status |
|----------|------------|--------|--------|
| **Arithmetic** | 11 | 10 | ✅ Well-tested |
| **Comparison & Bitwise** | 14 | 14 | ✅ Well-tested |
| **Cryptographic** | 1 | 0 | ❌ Broken (in-memory hashing not yet implemented) |
| **Stack Operations** | 48 | 0 | ❌ Indirect only |
| **Memory** | 3 | 3 | ✅ Good |
| **Control Flow** | 4 | 3 | ⚠️ Partial (JUMPI has type mismatch) |
| **System** | 6 | 3 | ⚠️ Partial |
| **Call Data** | 2 | 2 | ⚠️ Under-tested |

## Quick Opcode Lookup

| Hex | Opcode | Tested | Notes |
|-----|--------|--------|-------|
| 0x00 | STOP | ❌ | |
| 0x01 | ADD | ✅ | |
| 0x02 | MUL | ✅ | |
| 0x03 | SUB | ✅ | |
| 0x04 | DIV | ✅ | |
| 0x05 | SDIV | ✅ | |
| 0x06 | MOD | ✅ | |
| 0x07 | SMOD | ✅ | |
| 0x08 | ADDMOD | ✅ | 512-bit intermediate arithmetic for EVM compliance |
| 0x09 | MULMOD | ✅ | 512-bit intermediate arithmetic for EVM compliance |
| 0x0A | EXP | ✅ | |
| 0x0B | SIGNEXTEND | ⚠️ | Basic test only |
| 0x10 | LT | ✅ | |
| 0x11 | GT | ✅ | |
| 0x12 | SLT | ✅ | |
| 0x13 | SGT | ✅ | |
| 0x14 | EQ | ✅ | |
| 0x15 | ISZERO | ✅ | |
| 0x16 | AND | ✅ | |
| 0x17 | OR | ✅ | |
| 0x18 | XOR | ✅ | |
| 0x19 | NOT | ✅ | |
| 0x1A | BYTE | ✅ | |
| 0x1B | SHL | ✅ | |
| 0x1C | SHR | ✅ | |
| 0x1D | SAR | ✅ | |
| 0x20 | KECCAK256 | ❌ | **BROKEN**: pops offset+size but hashes a stack slot, not memory bytes |
| 0x3D | RETURNDATASIZE | ⚠️ | Indirect only |
| 0x3E | RETURNDATACOPY | ⚠️ | Indirect only |
| 0x40 | BLOCKHASH | ❌ | Stub |
| 0x50 | POP | ❌ | |
| 0x51 | MLOAD | ✅ | |
| 0x52 | MSTORE | ✅ | |
| 0x53 | MSTORE8 | ✅ | |
| 0x56 | JUMP | ✅ | |
| 0x57 | JUMPI | ❌ | Bug: condition uses `load_i64` vs `i256` zero |
| 0x58 | PC | ⚠️ | Basic only |
| 0x5B | JUMPDEST | ✅ | Used in JUMP tests |
| 0x5F | PUSH0 | ⚠️ | Indirect only |
| 0x60–0x7F | PUSH1–PUSH32 | ⚠️ | PUSH1/PUSH2 used extensively; others untested |
| 0x80–0x8F | DUP1–DUP16 | ❌ | Used in tests but never isolated |
| 0x90–0x9F | SWAP1–SWAP16 | ❌ | Used in tests but never isolated |
| 0xF1 | CALL | ⚠️ | Basic test only |
| 0xF3 | RETURN | ✅ | |
| 0xFD | REVERT | ❌ | |
| 0xFE | INVALID | ❌ | |
| 0xFF | SELFDESTRUCT | ❌ | |

**Legend**: ✅ Well-tested · ⚠️ Under-tested · ❌ No tests / broken

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

### Helper Macros

```rust
// Construct bytecode
bytecode![PUSH1!(0x01), ADD!(), PUSH1!(0x02)]

// Push operations
PUSH1!(byte)           // vec![0x60, byte]
PUSH2!(b1, b2)         // vec![0x61, b1, b2]

// Available opcode macros
PUSH0!()  ADD!()    MUL!()    SUB!()      DIV!()    MOD!()
EXP!()    SDIV!()   SMOD!()   ADDMOD!()   MULMOD!() SIGNEXTEND!()
SHL!()    SHR!()    SAR!()    AND!()      OR!()     XOR!()
NOT!()    BYTE!()   LT!()     GT!()       SLT!()    SGT!()
EQ!()     ISZERO!() KECCAK256!()
JUMP!()   JUMPDEST!() PC!()
MLOAD!()  MSTORE!() MSTORE8!()
RETURN!() REVERT!() CALL!()
RETURNDATASIZE!() RETURNDATACOPY!()

// Construct 256-bit stack word (little-endian)
stack_word(&[0x42])                   // 0x42 at byte 0, rest zeros
stack_word(&[0x01, 0x02, 0x03])      // little-endian: 0x030201...
```

### TestContractRun Fields

```rust
pub struct TestContractRun {
    pub result: ReturnCode,         // Stop, ExplicitReturn, etc.
    pub stack_ptr: usize,           // Number of items on stack
    pub stack: Vec<[u8; 32]>,       // Stack contents (little-endian)
    pub memory: Option<Vec<u8>>,    // Memory contents
    pub memory_len: Option<usize>,  // Allocated memory length
    pub jump_ptr: usize,            // Current PC value
    pub return_offset: usize,       // Return data offset
    pub return_length: usize,       // Return data length
}
```

Use `..Default::default()` to fill unneeded fields (all zero/None/Stop).

**Stack words are little-endian**: `stack_word(&[0x01, 0x02])` represents `0x0201`.

### Multi-ROM Tests (CALL)

```rust
basic_call_with_return_data: Test {
    roms: vec![
        bytecode![/* ROM 0: caller */],
        bytecode![/* ROM 1: callee */],
    ],
    expected: TestContractRun { ... },
}
```

### Running Tests

```bash
make test                                         # Termux (sets CARGO_TARGET_DIR)
cargo test --test test_roms                       # All tests
cargo test --test test_roms -- basic_jump         # Single test
cargo test --test test_roms -- --nocapture        # With output
```

---

## References

- **Test source**: `crates/jet/tests/test_roms.rs`
- **Opcode implementations**: `crates/jet/src/builder/ops.rs`
- **Opcode dispatch**: `crates/jet/src/builder/contract.rs`
- **Opcode definitions**: `crates/jet/src/instructions.rs`
- **EVM opcode specs**: `docs/reference/evm/`
