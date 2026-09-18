# Test Coverage

Last updated: 2026-09-18

This inventory reflects the current tree. "Implemented" means the opcode compiles
through `crates/jet/src/builder/contract.rs` without returning
`Error::UnimplementedInstruction`. "Tested" means a rom test in
`crates/jet/tests/test_roms.rs` executes the opcode.

## Summary

| Metric | Count |
|---|---:|
| Opcodes defined in `Instruction` | 148 |
| Implemented | 122 |
| Unimplemented | 26 |
| Implemented and executed by a rom test | 66 |
| Implemented but not executed by a rom test | 56 |

Every `rom_tests!` case runs twice, once per stack backend
(`StackMode::RuntimeOnly` and `StackMode::SymbolicPreferred`), so the runtime
backend acts as a differential oracle for the symbolic one.

## Test Suites

| Suite | Location | Cases |
|---|---|---:|
| Rom tests, generated for both stack modes | `crates/jet/tests/test_roms.rs` | 144 x 2 |
| Call context, calldata, nested CALL and gas | `crates/jet/tests/test_roms.rs` (plain `#[test]` functions) | 22 |
| Invalid opcode decoding | `crates/jet/tests/invalid_opcode.rs` | 1 |
| Symbolic planner limits and static gas regions | `crates/jet/src/builder/contract.rs` | 3 |
| Memory expansion builtin | `crates/jet_runtime/src/builtins.rs` | 7 |
| Runtime IR module generation | `crates/jet_runtime/src/runtime_builder.rs` | 2 |
| Rust and LLVM struct layouts | `crates/jet_runtime/src/layout_tests.rs` | 10 |
| Address parsing | `crates/jet_runtime/src/address.rs` | 6 |
| PUSH macros | `crates/jet_push_macros/tests/macro_tests.rs` | 7 |

## Coverage by Category

| Category | Defined | Implemented | Tested |
|---|---:|---:|---:|
| Stop and arithmetic | 12 | 12 | 12 |
| Comparison and bitwise | 14 | 14 | 14 |
| KECCAK256 | 1 | 1 | 1 |
| Environment | 16 | 8 | 8 |
| Block information | 10 | 8 | 8 |
| Stack, memory, storage and flow | 15 | 10 | 10 |
| PUSH | 33 | 33 | 7 |
| DUP | 16 | 16 | 1 |
| SWAP | 16 | 16 | 1 |
| LOG | 5 | 0 | 0 |
| System | 10 | 4 | 4 |

## Opcode Table

Yes means implemented or tested, no means not, and a dash under Tested means
the opcode is not implemented.

### Stop and arithmetic

| Hex | Opcode | Implemented | Tested |
|---|---|---|---|
| 0x00 | STOP | yes | yes |
| 0x01 | ADD | yes | yes |
| 0x02 | MUL | yes | yes |
| 0x03 | SUB | yes | yes |
| 0x04 | DIV | yes | yes |
| 0x05 | SDIV | yes | yes |
| 0x06 | MOD | yes | yes |
| 0x07 | SMOD | yes | yes |
| 0x08 | ADDMOD | yes | yes |
| 0x09 | MULMOD | yes | yes |
| 0x0A | EXP | yes | yes |
| 0x0B | SIGNEXTEND | yes | yes |

### Comparison and bitwise

| Hex | Opcode | Implemented | Tested |
|---|---|---|---|
| 0x10 | LT | yes | yes |
| 0x11 | GT | yes | yes |
| 0x12 | SLT | yes | yes |
| 0x13 | SGT | yes | yes |
| 0x14 | EQ | yes | yes |
| 0x15 | ISZERO | yes | yes |
| 0x16 | AND | yes | yes |
| 0x17 | OR | yes | yes |
| 0x18 | XOR | yes | yes |
| 0x19 | NOT | yes | yes |
| 0x1A | BYTE | yes | yes |
| 0x1B | SHL | yes | yes |
| 0x1C | SHR | yes | yes |
| 0x1D | SAR | yes | yes |

### KECCAK256

| Hex | Opcode | Implemented | Tested |
|---|---|---|---|
| 0x20 | KECCAK256 | yes | yes |

### Environment

| Hex | Opcode | Implemented | Tested |
|---|---|---|---|
| 0x30 | ADDRESS | yes | yes |
| 0x31 | BALANCE | no | - |
| 0x32 | ORIGIN | yes | yes |
| 0x33 | CALLER | yes | yes |
| 0x34 | CALLVALUE | yes | yes |
| 0x35 | CALLDATALOAD | yes | yes |
| 0x36 | CALLDATASIZE | yes | yes |
| 0x37 | CALLDATACOPY | no | - |
| 0x38 | CODESIZE | no | - |
| 0x39 | CODECOPY | no | - |
| 0x3A | GASPRICE | no | - |
| 0x3B | EXTCODESIZE | no | - |
| 0x3C | EXTCODECOPY | no | - |
| 0x3D | RETURNDATASIZE | yes | yes |
| 0x3E | RETURNDATACOPY | yes | yes |
| 0x3F | EXTCODEHASH | no | - |

### Block information

| Hex | Opcode | Implemented | Tested |
|---|---|---|---|
| 0x40 | BLOCKHASH | yes | yes |
| 0x41 | COINBASE | yes | yes |
| 0x42 | TIMESTAMP | yes | yes |
| 0x43 | NUMBER | yes | yes |
| 0x44 | DIFFICULTY | yes | yes |
| 0x45 | GASLIMIT | yes | yes |
| 0x46 | CHAINID | yes | yes |
| 0x47 | SELFBALANCE | no | - |
| 0x49 | BLOBHASH | no | - |
| 0x4A | BLOBBASEFEE | yes | yes |

### Stack, memory, storage and flow

| Hex | Opcode | Implemented | Tested |
|---|---|---|---|
| 0x50 | POP | yes | yes |
| 0x51 | MLOAD | yes | yes |
| 0x52 | MSTORE | yes | yes |
| 0x53 | MSTORE8 | yes | yes |
| 0x54 | SLOAD | no | - |
| 0x55 | SSTORE | no | - |
| 0x56 | JUMP | yes | yes |
| 0x57 | JUMPI | yes | yes |
| 0x58 | PC | yes | yes |
| 0x59 | MSIZE | yes | yes |
| 0x5A | GAS | yes | yes |
| 0x5B | JUMPDEST | yes | yes |
| 0x5C | TLOAD | no | - |
| 0x5D | TSTORE | no | - |
| 0x5E | MCOPY | no | - |

### PUSH

| Hex | Opcode | Implemented | Tested |
|---|---|---|---|
| 0x5F | PUSH0 | yes | yes |
| 0x60 | PUSH1 | yes | yes |
| 0x61 | PUSH2 | yes | yes |
| 0x62 | PUSH3 | yes | no |
| 0x63 | PUSH4 | yes | yes |
| 0x64 | PUSH5 | yes | yes |
| 0x65 | PUSH6 | yes | no |
| 0x66 | PUSH7 | yes | no |
| 0x67 | PUSH8 | yes | no |
| 0x68 | PUSH9 | yes | no |
| 0x69 | PUSH10 | yes | no |
| 0x6A | PUSH11 | yes | no |
| 0x6B | PUSH12 | yes | no |
| 0x6C | PUSH13 | yes | no |
| 0x6D | PUSH14 | yes | no |
| 0x6E | PUSH15 | yes | no |
| 0x6F | PUSH16 | yes | no |
| 0x70 | PUSH17 | yes | no |
| 0x71 | PUSH18 | yes | no |
| 0x72 | PUSH19 | yes | no |
| 0x73 | PUSH20 | yes | yes |
| 0x74 | PUSH21 | yes | no |
| 0x75 | PUSH22 | yes | no |
| 0x76 | PUSH23 | yes | no |
| 0x77 | PUSH24 | yes | no |
| 0x78 | PUSH25 | yes | no |
| 0x79 | PUSH26 | yes | no |
| 0x7A | PUSH27 | yes | no |
| 0x7B | PUSH28 | yes | no |
| 0x7C | PUSH29 | yes | no |
| 0x7D | PUSH30 | yes | no |
| 0x7E | PUSH31 | yes | no |
| 0x7F | PUSH32 | yes | yes |

### DUP

| Hex | Opcode | Implemented | Tested |
|---|---|---|---|
| 0x80 | DUP1 | yes | yes |
| 0x81 | DUP2 | yes | no |
| 0x82 | DUP3 | yes | no |
| 0x83 | DUP4 | yes | no |
| 0x84 | DUP5 | yes | no |
| 0x85 | DUP6 | yes | no |
| 0x86 | DUP7 | yes | no |
| 0x87 | DUP8 | yes | no |
| 0x88 | DUP9 | yes | no |
| 0x89 | DUP10 | yes | no |
| 0x8A | DUP11 | yes | no |
| 0x8B | DUP12 | yes | no |
| 0x8C | DUP13 | yes | no |
| 0x8D | DUP14 | yes | no |
| 0x8E | DUP15 | yes | no |
| 0x8F | DUP16 | yes | no |

### SWAP

| Hex | Opcode | Implemented | Tested |
|---|---|---|---|
| 0x90 | SWAP1 | yes | yes |
| 0x91 | SWAP2 | yes | no |
| 0x92 | SWAP3 | yes | no |
| 0x93 | SWAP4 | yes | no |
| 0x94 | SWAP5 | yes | no |
| 0x95 | SWAP6 | yes | no |
| 0x96 | SWAP7 | yes | no |
| 0x97 | SWAP8 | yes | no |
| 0x98 | SWAP9 | yes | no |
| 0x99 | SWAP10 | yes | no |
| 0x9A | SWAP11 | yes | no |
| 0x9B | SWAP12 | yes | no |
| 0x9C | SWAP13 | yes | no |
| 0x9D | SWAP14 | yes | no |
| 0x9E | SWAP15 | yes | no |
| 0x9F | SWAP16 | yes | no |

### LOG

| Hex | Opcode | Implemented | Tested |
|---|---|---|---|
| 0xA0 | LOG0 | no | - |
| 0xA1 | LOG1 | no | - |
| 0xA2 | LOG2 | no | - |
| 0xA3 | LOG3 | no | - |
| 0xA4 | LOG4 | no | - |

### System

| Hex | Opcode | Implemented | Tested |
|---|---|---|---|
| 0xF0 | CREATE | no | - |
| 0xF1 | CALL | yes | yes |
| 0xF2 | CALLCODE | no | - |
| 0xF3 | RETURN | yes | yes |
| 0xF4 | DELEGATECALL | no | - |
| 0xF5 | CREATE2 | no | - |
| 0xFA | STATICCALL | no | - |
| 0xFD | REVERT | yes | yes |
| 0xFE | INVALID | yes | yes |
| 0xFF | SELFDESTRUCT | no | - |

## Behaviours Covered Beyond Single Opcodes

- Stack faults: underflow through POP, ADD, JUMP, RETURN, DUP and SWAP,
  overflow from straight-line pushes, and faults at dynamically reached
  jumpdests. Both backends return `StackUnderflow` or `StackOverflow` with
  the pre-fault stack and memory left visible.
- Control flow: static and dynamic JUMP and JUMPI, joins with equal and
  differing stack heights, loop backedges, targets above `u32::MAX`, and
  failed jumps that keep the remaining stack.
- Dead code: bytes after STOP, RETURN, REVERT, INVALID or JUMP and before
  the next JUMPDEST are skipped, and push data cannot fake a JUMPDEST.
- Memory: expansion rounding and monotonicity, rejection of offset plus
  size overflow for MLOAD, MSTORE, KECCAK256 and RETURN, zero-size RETURN
  at the maximum offset, and CALL output expansion.
- Calls: return data copy, RETURNDATASIZE and RETURNDATACOPY without a
  prior call, out-of-bounds copies, and block info and call info
  propagation into a sub-call.
- Gas: exact static block charges, out-of-gas diagnostics (pc, available,
  required), GAS after its own charge, memory expansion charged before the
  store, EXP per-byte cost, and planned symbolic stack faults charging the
  instructions before the fault.
- Symbolic planner: net-growth loops exceed the plan limits and fall back
  to the runtime backend, and bounded roms stay within the limits.

## Test Framework Quick Reference

### Rom Tests

`rom_tests!` in `crates/jet/tests/roms/mod.rs` expands each case into two
tests, `test_rom_with_real_stack_<name>` and
`test_rom_with_symbolic_stack_<name>`.

```rust
rom_tests! {
    test_name: Test {
        roms: vec![bytecode![
            PUSH1!(0x01),
            PUSH1!(0x02),
            ADD!(),
        ]],
        expected: TestContractRun {
            stack_ptr: 1,
            stack: vec![stack_word(&[0x03])],
            ..Default::default()
        },
    },
}
```

Opcode macros come from `define_ops!` at the top of `test_roms.rs`; add an
opcode there before using it in `bytecode!`. `PUSH0!` through `PUSH32!`
come from `jet_push_macros`. `stack_word(&[..])` builds a 32-byte word from
little-endian bytes, so `stack_word(&[0x01, 0x02])` is `0x0201`.

Several roms in one test compile at addresses ending in `00`, `01`, and so
on, which is how CALL tests reach a callee.

### TestContractRun

```rust
pub(crate) struct TestContractRun {
    pub(crate) result: ReturnCode,
    pub(crate) stack_ptr: u32,
    pub(crate) jump_ptr: u32,
    pub(crate) return_offset: u32,
    pub(crate) return_length: u32,
    pub(crate) stack: Vec<[u8; 32]>,
    pub(crate) memory: Option<Vec<u8>>,
    pub(crate) memory_len: Option<u32>,
    pub(crate) gas_remaining: Option<u64>,
    pub(crate) gas_failure: Option<(u32, u64, u64)>,
}
```

`Option` fields are only asserted when set. `gas_failure` is
`(pc, available, required)`.

### Call Info and Gas Tests

Tests that need a specific `CallInfo` (address, origin, caller, value,
calldata, gas limit) are plain `#[test]` functions that call
`run_both_modes` or `run_with_gas_both_modes`, which build the contracts and
run them under each stack mode.

### Running Tests

```bash
make test                                         # cargo nextest, all crates
cargo nextest run -p jet --test test_roms         # rom tests only
cargo nextest run -p jet basic_jump               # tests whose name contains basic_jump
cargo nextest run -p jet --no-capture basic_jump  # with test output
```

## References

- Test harness: `crates/jet/tests/roms/mod.rs`
- Rom tests: `crates/jet/tests/test_roms.rs`
- Opcode emitters: `crates/jet/src/builder/ops.rs`
- Opcode dispatch and the symbolic planner: `crates/jet/src/builder/contract.rs`
- Opcode definitions: `crates/jet/src/instructions.rs`
- Implementation status and known gaps: `../OPERATION_STATUS.md`
- EVM opcode reference: `.agents/skills/evm-opcodes/references/docs/<HEX>.md`
