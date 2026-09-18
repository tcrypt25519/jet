# EVM operation status

This inventory reflects the current tree. The categories describe separate kinds of work, so an operation appears in more than one category when it has independent gaps. This covers EVM instructions recognized by `Instruction` plus runtime stack operations used by the compiler. Per-opcode test coverage is in `docs/test_coverage.md`.

## Unimplemented

These opcodes are recognized during bytecode decoding, but compilation returns `Error::UnimplementedInstruction`.

| Operation | Opcode | Missing behavior |
|---|---:|---|
| `BALANCE` | `0x31` | Account balance lookup |
| `CALLDATACOPY` | `0x37` | Calldata copy into memory |
| `CODESIZE` | `0x38` | Current code length |
| `CODECOPY` | `0x39` | Current code copy into memory |
| `GASPRICE` | `0x3a` | Transaction gas price |
| `EXTCODESIZE` | `0x3b` | External account code length |
| `EXTCODECOPY` | `0x3c` | External account code copy |
| `EXTCODEHASH` | `0x3f` | External account code hash |
| `SELFBALANCE` | `0x47` | Current account balance |
| `BLOBHASH` | `0x49` | Transaction blob hash lookup |
| `SLOAD` | `0x54` | Persistent storage read |
| `SSTORE` | `0x55` | Persistent storage write |
| `TLOAD` | `0x5c` | Transient storage read |
| `TSTORE` | `0x5d` | Transient storage write |
| `MCOPY` | `0x5e` | Overlap-safe memory copy |
| `LOG0` | `0xa0` | Log with no topics |
| `LOG1` | `0xa1` | Log with one topic |
| `LOG2` | `0xa2` | Log with two topics |
| `LOG3` | `0xa3` | Log with three topics |
| `LOG4` | `0xa4` | Log with four topics |
| `CREATE` | `0xf0` | Contract creation |
| `CALLCODE` | `0xf2` | Call using the caller's storage context |
| `DELEGATECALL` | `0xf4` | Delegated call |
| `CREATE2` | `0xf5` | Deterministic contract creation |
| `STATICCALL` | `0xfa` | Read-only call |
| `SELFDESTRUCT` | `0xff` | Account destruction and beneficiary |

## Partially implemented

These operations compile and execute, but omit required EVM behavior.

| Operation | What exists | What is missing |
|---|---|---|
| Gas accounting | Osaka static costs charged once per basic block through SSA gas values, dynamic costs computed in generated IR for memory expansion, `KECCAK256`, `EXP`, `RETURNDATACOPY` and `CALL` memory, the `GAS` opcode, and `OutOfGas` failures that record the pc and the available and required gas. | Gas forwarding to callees (`CALL` runs the callee with an unbounded limit), the 63/64 rule, refunds, and the access-list costs of the unimplemented state opcodes. |
| `CALL` (`0xf1`) | Looks up another JIT-compiled contract, creates a sub-context carrying the callee address, caller, origin and value, charges input and output memory expansion, and copies return data to the caller. | It discards the gas, input offset and input length operands, so the callee sees empty calldata and an unbounded gas limit. It has no value transfer, account state, call-depth rule or external-account behavior. |

## Implemented with a bug

No confirmed entries remain in this category.

## Unchecked but should be checked

No confirmed entries remain in this category.

## Source locations

* Opcode dispatch, the symbolic stack planner and the explicit unimplemented list: `crates/jet/src/builder/contract.rs`
* Implemented opcode emitters: `crates/jet/src/builder/ops.rs`
* Static and dynamic gas tables: `crates/jet/src/builder/gas.rs`
* Runtime stack IR: `crates/jet_runtime/src/runtime_builder.rs`
* Call and return-data helpers: `crates/jet_runtime/src/builtins.rs`
* Symbolic stack design: `docs/architecture/symbolic-stack.md` and `docs/adrs/adr-007.md`
