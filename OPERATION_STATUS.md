# EVM operation status

This inventory reflects the current tree at `bece5c0`. The categories describe separate kinds of work, so an operation appears in more than one category when it has independent gaps. This covers EVM instructions recognized by `Instruction` plus runtime stack operations used by the compiler.

## Unimplemented

These opcodes are recognized during bytecode decoding, but compilation returns `Error::UnimplementedInstruction`.

| Operation | Opcode | Missing behavior |
|---|---:|---|
| `ADDRESS` | `0x30` | Current contract address |
| `BALANCE` | `0x31` | Account balance lookup |
| `ORIGIN` | `0x32` | Transaction origin |
| `CALLER` | `0x33` | Immediate caller address |
| `CALLVALUE` | `0x34` | Call value |
| `CALLDATALOAD` | `0x35` | Calldata word load |
| `CALLDATASIZE` | `0x36` | Calldata length |
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
| `GAS` | `0x5a` | Remaining gas |
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
| `SELFDESTRUCT` | `0xff` | Account destruction and beneficiary transfer |

## Partially implemented

These operations compile and execute, but omit required EVM behavior.

| Operation | What exists | What is missing |
|---|---|---|
| Gas accounting for every implemented opcode | Opcode semantics execute without a gas counter | Base gas, dynamic gas, memory expansion gas, out-of-gas failure and gas forwarding are future execution-layer work. |
| `CALL` (`0xf1`) | Looks up another JIT-compiled contract, creates a subcontext and copies a bounded amount of return data | The implementation discards the gas, value, input offset and input length operands. It has no value transfer, account state, call-depth rule, gas forwarding or normal external-account behavior. |
| `RETURN` (`0xf3`) | Pops offset and length, stores them in `Context` and returns `ExplicitReturn` | It doesn't expand or validate the selected memory range before exposing it as return data. |
| `RETURNDATACOPY` (`0x3e`) | Copies bytes from the latest subcall context when the existing allocation is large enough | It can't grow memory capacity and reports `MemoryExpansionNeeded`. The opcode emitter also ignores the copy helper's error result. |

## Implemented with a bug

No confirmed entries remain in this category.

## Unchecked but should be checked

These operations reach pointer arithmetic or memory access without validating the EVM-level precondition first.

| Operation | Missing check | Impact |
|---|---|---|
| `RETURNDATASIZE` (`0x3d`) | Doesn't check whether `Context.sub_call` is non-null before dereferencing it | Executing it before a call can dereference a null pointer instead of pushing zero. |
| `RETURNDATACOPY` (`0x3e`) | Doesn't check whether `Context.sub_call` is non-null before dereferencing it. The runtime helper doesn't validate that the callee's `return_offset + return_length` is within callee memory. | It can dereference a null subcontext or construct an out-of-bounds source range. |

## Source locations

* Opcode dispatch and the explicit unimplemented list: `crates/jet/src/builder/contract.rs`
* Implemented opcode emitters: `crates/jet/src/builder/ops.rs`
* Runtime stack IR: `crates/jet_runtime/src/runtime_builder.rs`
* Call and return-data helpers: `crates/jet_runtime/src/builtins.rs`
* Existing symbolic stack findings: `docs/symbolic-stack-completion-plan.md`
