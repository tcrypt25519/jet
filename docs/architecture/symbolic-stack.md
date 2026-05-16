# Symbolic Stack Lowering

This note describes the current symbolic stack lowering path. It is intentionally
lightweight: the implementation is still moving, but the main invariants should
be written down as they settle.

## Goal

Jet currently has two stack backends:

- `RuntimeStackBackend` keeps the EVM operand stack in `Context.stack` through
  runtime helper calls.
- `SymbolicStackBackend` keeps EVM stack slots as LLVM values during IR
  construction.

These are alternative compilation modes, not two live stacks that run together.
Runtime mode should continue to work while symbolic mode grows toward full
register/SSA lowering.

## Stack Values

The symbolic stack stores `StackValue::Word` entries. Each entry contains:

- the LLVM `IntValue` used by opcode emitters;
- optional `known_u64` metadata.

`known_u64` is not a separate semantic value. It is a compile-time fact about
the same EVM word, used when the compiler needs a small literal value without
trying to recover it from LLVM. It is currently produced for small `PUSH` values
and by `PC`.

The metadata is deliberately named as a known integer, not as a program counter.
Most static EVM jump targets are produced by `PUSH`, not by the `PC` opcode. A
known value becomes a program counter only when it is consumed as the target of
`JUMP` or `JUMPI`.

Operations that compute new values generally drop this metadata. Phi nodes also
drop it, since a merged stack slot is no longer one known literal unless later
analysis proves that all incoming values are the same.

## Code Blocks and Jumpdest Lookup

`find_code_blocks` partitions bytecode into `CodeBlock`s and records which
blocks are valid `JUMPDEST` targets. `CodeBlocks` owns both:

- the ordered block list used for code emission;
- a `jumpdest_pc -> block_index` map used for static target lookup.

The map avoids re-scanning every block when symbolic jump lowering sees a known
target value. The lookup rule is:

1. read the top symbolic stack slot's `known_u64`;
2. look it up in the jumpdest map;
3. if found, branch directly to that target block.

## Symbolic Control Flow

Symbolic mode uses a separate contract-body builder because control-flow edges
must carry symbolic stack state.

For each block:

1. Restore the block's entry stack.
2. Emit instructions in bytecode order.
3. For non-jump opcodes, call the shared generic opcode dispatcher.
4. For static `JUMP` or `JUMPI`, pop the target/condition, record the outgoing
   symbolic stack for each successor, and emit a direct LLVM branch.
5. For dynamic `JUMP` or taken `JUMPI`, emit a site-local LLVM `switch` over
   all known `JUMPDEST` blocks and record the outgoing stack for each possible
   target.
6. When a later block has multiple incoming stack states with the same height,
   build one LLVM phi per stack slot and use those phi values as that block's
   entry stack.

Only `JUMP` and `JUMPI` are mode-specific in this path. The normal
opcode-to-emitter match is shared so runtime and symbolic modes do not maintain
separate implemented-opcode lists.

## Current Limits

This is not full EVM symbolic correctness yet. Dynamic jump targets now create
conservative successor edges to all `JUMPDEST` blocks with the post-pop stack
state, but the symbolic builder still processes blocks in bytecode order. That
means loop/backedge handling and block specialization are still outstanding.

The current implementation covers:

- static `JUMP` targets;
- static `JUMPI` targets;
- dynamic forward `JUMP` and `JUMPI` targets through conservative switches;
- same-height joins through LLVM phi nodes;
- runtime stack mode preserved as its own backend.

The runtime backend remains useful as an oracle during development. It is not
the intended long-term fallback for valid EVM programs.
