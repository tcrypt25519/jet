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

## Symbolic Control-Flow Plan

Symbolic mode builds a control-flow plan before it emits the function body. The
plan is an abstract interpretation over `CodeBlock`s, not a second runtime stack.
It tracks:

- the current stack height;
- optional `known_u64` metadata for each slot;
- the block index reached by each successor edge.

The analysis starts at block 0 with an empty stack and runs a worklist to a fixed
point. Non-jump opcodes are modeled only by their stack effects and by whether
they preserve or destroy `known_u64` metadata. Static `JUMP` and `JUMPI` targets
use the `jumpdest_pc -> block_index` map. Dynamic jump targets conservatively add
successor edges to every known `JUMPDEST` with the post-pop stack shape.

Incoming states are grouped by `(block_index, stack_height)`. When two incoming
states for the same block and height disagree on a known literal, that slot's
metadata is dropped to `None`; the LLVM value is still represented by the entry
phi. When the same bytecode block is reached with different stack heights, the
planner creates distinct block variants so each variant has one stable symbolic
entry shape.

## Symbolic Emission

After planning, symbolic mode creates all block entry phis before emitting any
block body. Each planned block variant has one phi per entry stack slot, and
emission restores the symbolic stack from those phis before walking the block's
bytecode.

For each emitted block variant:

1. Restore the variant's entry stack from its phis.
2. Emit instructions in bytecode order.
3. For non-jump opcodes, call the shared generic opcode dispatcher.
4. For static `JUMP` or `JUMPI`, record phi incoming values for the planned
   successor variant and emit a direct LLVM branch.
5. For dynamic `JUMP` or taken `JUMPI`, emit a site-local LLVM `switch` over all
   `JUMPDEST` variants with the current stack height and record phi incoming
   values for each case.
6. For ordinary fallthrough, branch to the following block variant matching the
   current stack height.

Only `JUMP` and `JUMPI` are mode-specific in this path. The normal
opcode-to-emitter match is shared so runtime and symbolic modes do not maintain
separate implemented-opcode lists.

Original LLVM blocks that are not used by any planned variant are terminated with
an invalid return so LLVM never sees an unterminated block. Reachable variants
use normal EVM return codes and materialize the symbolic stack into
`Context.stack` only at contract exits.

## Current Limits

The symbolic path now handles fixed-point stack-shape planning, backedges, and
same-bytecode block specialization by stack height. The remaining correctness
work is less about whether stack values can cross CFG edges and more about
expanding the compiler's opcode surface and making dynamic jumps more precise.

The current implementation covers:

- static `JUMP` targets;
- static `JUMPI` targets;
- dynamic `JUMP` and `JUMPI` targets through conservative switches;
- same-height joins through LLVM phi nodes;
- loop/backedge stack values through pre-created entry phis;
- different-height joins through specialized block variants;
- runtime stack mode preserved as its own backend.

The runtime backend remains useful as an oracle during development. It is not
the intended long-term fallback for valid EVM programs.
