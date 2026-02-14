# EVM Bytecode to LLVM Basic Blocks: Lowering and Control Flow Construction

## Overview of Compilation Structure

### LLVM Basic Block Representation

Each Jet-compiled contract is represented as a single LLVM function composed of a sequence of `BasicBlock` instances. The generated IR exhibits a relatively linear structure where blocks are emitted in bytecode offset order, but execution flow is determined explicitly by terminator instructions rather than implicit fallthrough.

The function signature for a compiled contract is:

```
i8 contract_fn(ptr %exec_ctx, ptr %block_info)
```

Returns: `i8` status code (ReturnCode enum value)

### Near-Linear Block Structure

While basic blocks are created sequentially during bytecode scanning, the resulting CFG is explicitly wired. Blocks do not implicitly fall through to the next block in memory. Every block must be explicitly connected via LLVM terminator instructions:

- Unconditional branch (`br`)
- Conditional branch (`br i1 %cond`)
- Return (`ret i8`)
- Switch (for jump tables)

This design ensures that control flow precisely mirrors EVM execution semantics, where execution can flow linearly, jump dynamically to JUMPDEST targets, or terminate via STOP/RETURN/REVERT.

### Bytecode-to-IR Mapping

Each EVM instruction within a `CodeBlock` is lowered to one or more LLVM IR instructions. The mapping operates at two levels:

1. **Block-level**: Raw bytecode is partitioned into contiguous chunks (CodeBlocks) based on control flow boundaries.
2. **Instruction-level**: Within each CodeBlock, individual EVM opcodes are translated to sequences of LLVM IR operations.

The boundary between CodeBlocks is determined by:
- JUMPDEST opcodes (mark valid jump destinations)
- Terminating instructions (STOP, RETURN, REVERT, INVALID, JUMP)
- Conditional branches (JUMPI)

## CodeBlock and CodeBlocks Types

### CodeBlock: Single Basic Block Representation

`CodeBlock` is a 1:1 representation of a single LLVM `BasicBlock`. Each instance owns:

```rust
struct CodeBlock<'ctx, 'b> {
    offset: usize,              // Absolute bytecode offset where this block begins
    rom: &'b [u8],              // Slice of bytecode belonging to this block
    basic_block: BasicBlock<'ctx>, // The corresponding LLVM BasicBlock
    is_jumpdest: bool,          // True if this block begins with JUMPDEST
    terminates: bool,           // True if this block ends with a terminator (STOP/RETURN/etc)
}
```

**Invariants:**

- `offset` points to the first byte of bytecode in this block.
- `rom` contains all bytes from `offset` up to (but not including) the start of the next block or end of bytecode.
- `basic_block` is the LLVM BasicBlock that will contain the IR for this bytecode range.
- If `is_jumpdest` is true, `offset` points to the byte *after* the JUMPDEST opcode.
- If `terminates` is true, the block ends with an instruction that prevents fallthrough (STOP, RETURN, REVERT, JUMP).

**Responsibilities:**

- Maintain the association between a bytecode range and its LLVM BasicBlock.
- Track metadata required for CFG construction:
  - Whether the block is a valid jump destination.
  - Whether the block has an explicit terminator.

### CodeBlocks: Container and Manager

`CodeBlocks` is a sequential container that owns all `CodeBlock` instances for a contract:

```rust
struct CodeBlocks<'ctx, 'b> {
    blocks: Vec<CodeBlock<'ctx, 'b>>,
}
```

**Responsibilities:**

- Maintain the ordered list of blocks (in bytecode offset order).
- Provide iteration and lookup operations.
- Track global properties (e.g., whether any JUMPDESTs exist).

**Ownership and Indexing:**

- Blocks are stored in a `Vec` and indexed by their position in the vector.
- Lookup by bytecode offset is linear (`O(n)`), but in practice, the compiler only needs to:
  - Iterate blocks sequentially during IR generation.
  - Check the `is_jumpdest` flag to build the jump table.

### One-to-One Mapping Guarantee

**Critical invariant:** Each `CodeBlock` contains exactly one LLVM `BasicBlock`. This 1:1 relationship ensures:

- Deterministic mapping from bytecode offset to IR location.
- Precise CFG construction with no ambiguity.
- Direct translation of EVM control flow semantics to LLVM.

## Bytecode Chunking Algorithm

### Purpose

The chunking algorithm partitions raw EVM bytecode into discrete basic blocks. This is necessary for several reasons:

1. **SSA Construction:** LLVM requires control flow to be represented as explicit basic blocks with distinct entry and exit points.
2. **CFG Correctness:** EVM semantics allow dynamic jumps to JUMPDEST instructions. The compiler must identify all valid jump targets ahead of time.
3. **LLVM Structural Requirements:** Every basic block must end with a terminator instruction; the chunking algorithm identifies where these terminators belong.
4. **LLVM Fallthrough Modeling:** Although the EVM program counter advances sequentially by default (so execution may reach a `JUMPDEST` via linear fallthrough), LLVM basic blocks still require explicit terminators. Whenever EVM execution can continue into the next chunk, the compiler inserts an unconditional branch between the corresponding LLVM basic blocks.

### Block Boundary Conditions

A new block is created at the following bytecode boundaries:

1. **Start of contract:** Offset 0 begins the first block.
2. **After JUMPDEST:** The instruction immediately following a JUMPDEST begins a new block and is marked as a valid jump destination.
3. **After terminator:** The instruction after STOP, RETURN, REVERT, or JUMP begins a new block (if more bytecode exists).
4. **After JUMPI:** The instruction after JUMPI begins a new block (the conditional fallthrough path).

### Chunking Algorithm (Pseudo-code)

```
function find_code_blocks(bytecode):
    blocks = []
    current_block_offset = 0
    create_block_at(0)

    for (pc, item) in iterate_instructions(bytecode):
        match item:
            PushData(pc, instr, data):
                # PUSH instructions do not affect block boundaries
                continue

            Instr(pc, STOP | RETURN | REVERT | JUMP):
                # Close current block (includes this instruction)
                current_block.rom = bytecode[current_block_offset..pc+1]
                current_block.terminates = true
                
                # Advance the offset to the next instruction (if any);
                # block creation at this offset is handled elsewhere.
                if pc + 1 < len(bytecode):
                    current_block_offset = pc + 1

            Instr(pc, JUMPI):
                # Close current block (includes JUMPI)
                current_block.rom = bytecode[current_block_offset..pc+1]
                
                # Start a new block at the next instruction (fallthrough path)
                current_block_offset = pc + 1
                create_block_at(current_block_offset)

            Instr(pc, JUMPDEST):
                # Close current block if not empty (does NOT include JUMPDEST)
                if current_block.rom is empty:
                    current_block.rom = bytecode[current_block_offset..pc]
                
                # Start a new block AFTER the JUMPDEST instruction
                current_block_offset = pc + 1
                create_block_at(current_block_offset)
                current_block.is_jumpdest = true

            Instr(pc, other):
                # Regular instructions do not create boundaries
                continue

    # Finalize the last block
    if current_block.rom is empty:
        current_block.rom = bytecode[current_block_offset..end]

    return blocks
```

### Implementation Details (from `find_code_blocks` in `contract.rs`)

**Key observations from the implementation:**

1. **Instruction Iterator:** `instructions::Iter` handles PUSH data automatically, yielding `IterItem::PushData` for PUSH instructions and `IterItem::Instr` for all others.

2. **Block Lifecycle:**
   - A block is "open" from creation until its `rom` slice is assigned.
   - A block is "closed" when its `rom` slice is assigned (even if the assigned slice is empty for zero-length blocks).
   - The implementation checks `current_block.rom.is_empty()` to determine whether to assign the slice, but this check serves to prevent reassignment rather than indicate open/closed state.
   - Zero-length blocks (e.g., consecutive JUMPDESTs) will have empty `rom` slices after being closed.
   - Once a block's rom is assigned, a new block is immediately opened for the next bytecode.

3. **JUMPDEST Handling:**
   - JUMPDEST closes the current block at `offset..pc` (not including the JUMPDEST byte).
   - A new block is created at `pc+1` (after JUMPDEST) and marked with `is_jumpdest = true`.
   - This ensures that the JUMPDEST itself does not appear in any block's bytecode; it is purely a marker.

4. **Terminator Handling:**
   - STOP, RETURN, REVERT, JUMP close the block at `offset..pc+1` (including the terminator byte).
   - The block is marked with `terminates = true`.
   - Execution cannot fall through; the next block (if any) is unreachable unless jumped to.

5. **JUMPI Handling:**
   - JUMPI closes the block at `offset..pc+1` (including the JUMPI byte).
   - A new block is created for the fallthrough path (the "condition is false" path).
   - The block is NOT marked as terminating because JUMPI has two outgoing edges.

### Edge Cases

1. **Consecutive JUMPDESTs:**
   - If two JUMPDESTs appear consecutively, the algorithm creates an empty block between them.
   - The empty block is assigned a zero-length `rom` slice but still corresponds to a valid BasicBlock.
   - This ensures that each JUMPDEST has a unique entry point.

2. **Terminator at End of Bytecode:**
   - If the last instruction is a terminator (e.g., RETURN), no new block is created afterward.
   - The final block's `rom` is set to include the terminator.

3. **Unreachable Code:**
   - Code following a terminator (but before the next JUMPDEST) is placed in a new block.
   - This block is unreachable unless explicitly jumped to.
   - LLVM's optimizer may eliminate it, but Jet creates the block regardless for correctness.

4. **Empty Bytecode:**
   - If bytecode is empty, `find_code_blocks` still creates a single block at offset 0 with an empty `rom` slice.
   - `build_contract_body` then inserts an implicit return for this block, so compilation succeeds and the contract immediately returns a status code without executing any bytecode.

5. **JUMPDEST at Offset 0:**
   - JUMPDEST at offset 0 is invalid in Jet's model.
   - The jump table encodes destinations as `offset - 1` (since JUMPDEST is 1 byte).
   - A JUMPDEST at offset 0 would map to offset -1, which is an error.

## Block Connectivity and Control Flow Construction

### Preamble Block

Every compiled contract begins with a **preamble block**:

```llvm
preamble:
  ; Extract pointers from exec_ctx (optional initialization)
  br label %block_0
```

**Purpose:**
- Provide a single, well-defined entry point for the function.
- Perform any necessary register setup (extracting pointers from `exec_ctx`).
- Unconditionally branch to the first real code block (offset 0).

The preamble ensures that the function conforms to LLVM's requirement that the entry block cannot be the target of a branch.

### Fallthrough Edges

After generating IR for each CodeBlock, the compiler connects blocks:

```rust
if code_block.terminates() {
    // Block ends with STOP/RETURN/REVERT/JUMP; no fallthrough.
    continue;
}

match following_block {
    Some(next_block) => {
        // Explicit fallthrough: insert unconditional branch
        bctx.builder.build_unconditional_branch(next_block.basic_block)?;
    }
    None => {
        // End of bytecode: implicit return
        __build_return(bctx, ReturnCode::ImplicitReturn)?;
    }
}
```

**Key insight:** LLVM does not allow implicit fallthrough. Even though blocks are emitted sequentially, execution only flows from block A to block B if an explicit branch instruction connects them.

### JUMPDEST and Explicit Branching

**Critical distinction:** The presence of a JUMPDEST does **not** imply that the previous instruction performed a jump.

- A JUMPDEST marks a *potential* jump target.
- Execution can reach a JUMPDEST via:
  1. Linear fallthrough from the previous instruction.
  2. Dynamic jump (JUMP or JUMPI).

**Example:**

```
Bytecode:
0x00: PUSH1 0x05
0x02: PUSH1 0x00
0x04: MSTORE
0x05: JUMPDEST
0x06: PUSH1 0x20
```

After chunking:

- Block 0: Offset 0, bytecode `[PUSH1, 0x05, PUSH1, 0x00, MSTORE]`
- Block 1: Offset 6, bytecode `[PUSH1, 0x20]`, marked `is_jumpdest = true`

CFG construction:

- Block 0 does not terminate (MSTORE is not a terminator).
- The compiler inserts: `br label %block_1`
- Block 1 is reachable both via fallthrough and via dynamic jump to offset 5.

### Conditional Branches (JUMPI)

JUMPI generates a conditional branch with two outgoing edges:

```llvm
jumpi_block:
  %pc = pop()
  %cond = pop()
  store i32 %pc, ptr %jump_ptr
  %cmp = icmp eq i64 %cond, 0
  br i1 %cmp, label %fallthrough_block, label %jump_table_block
```

- If `%cond == 0`, execution falls through to the next block.
- If `%cond != 0`, execution branches to the jump table.

The jump table performs a runtime switch on `%pc` to determine the target block.

### Unconditional Jumps (JUMP)

JUMP generates an unconditional branch to the jump table:

```llvm
jump_block:
  %pc = pop()
  store i32 %pc, ptr %jump_ptr
  br label %jump_table_block
```

The jump table is responsible for dispatching to the correct JUMPDEST block.

### Jump Table Construction

If the contract contains any JUMPDEST instructions, the compiler generates a **jump table block**:

```llvm
jump_table_block:
  %pc = load i32, ptr %jump_ptr
  switch i32 %pc, label %jump_failure [
    i32 5, label %block_at_offset_6
    i32 10, label %block_at_offset_11
    ; ...
  ]

jump_failure:
  ret i8 <JumpFailure>
```

**Jump Offset Encoding:** The switch compares against `offset - 1` because JUMPDEST is 1 byte. The CodeBlock begins at `offset`, but the JUMPDEST instruction is at `offset - 1`.

**Example:**

- JUMPDEST at bytecode offset 5
- CodeBlock starts at offset 6
- Jump table case: `i32 5, label %block_offset_6`

### Terminator Insertion

Every LLVM BasicBlock must end with a terminator instruction. The compiler ensures this by:

1. **Explicit terminators:** STOP, RETURN, REVERT, INVALID translate directly to `ret i8 <code>`.
2. **JUMP:** Translates to an unconditional branch to the jump table.
3. **JUMPI:** Translates to a conditional branch (see above).
4. **Fallthrough:** If a block does not end with a terminator, the compiler inserts `br label %next_block`.
5. **End of bytecode:** If the last block has no terminator, the compiler inserts `ret i8 <ImplicitReturn>`.

## Terminators and Flow Semantics

### LLVM Terminator Instructions

LLVM requires that every BasicBlock ends with exactly one **terminator instruction**. Terminators define the control flow out of a block. Jet uses:

1. **`ret i8 %code`:** Returns from the function with a status code.
2. **`br label %block`:** Unconditional branch to another block.
3. **`br i1 %cond, label %true, label %false`:** Conditional branch.
4. **`switch i32 %val, label %default [ ... ]`:** Multi-way branch (jump table).

### EVM Terminator Mapping

| EVM Instruction | LLVM Terminator | Effect |
|-----------------|-----------------|--------|
| STOP | `ret i8 <Stop>` | Execution halts successfully |
| RETURN | `ret i8 <ExplicitReturn>` | Execution returns with data |
| REVERT | `ret i8 <Revert>` | Execution reverts state changes |
| INVALID | `ret i8 <Invalid>` | Execution fails (invalid opcode) |
| JUMP | `br label %jump_table` | Dynamic jump via jump table |
| JUMPI (true) | `br i1 %cond, label %jump_table, label %next` | Conditional jump |
| JUMPI (false) | `br i1 %cond, label %jump_table, label %next` | Fallthrough to next block |

### Structural Correctness

The one-to-one mapping between terminators and block exits ensures:

- **No implicit fallthrough:** Execution cannot "slip" into the next block without an explicit edge.
- **CFG validity:** Every block has at least one predecessor (except the entry block) and at least one successor (except return blocks).
- **EVM semantics preservation:** Control flow precisely matches the EVM execution model.

## Control Flow Graph Semantics

### CFG Shape

The resulting CFG exhibits the following properties:

1. **Single entry:** The preamble block is the unique entry point.
2. **Linear spine:** Blocks are emitted in bytecode offset order, with explicit edges connecting them.
3. **Dynamic dispatch:** JUMP/JUMPI branch to a centralized jump table, which dispatches to JUMPDEST blocks.
4. **Multiple exits:** Each STOP/RETURN/REVERT/INVALID creates a distinct exit point.

### Example CFG

**Bytecode:**

```
0x00: PUSH1 0x0A    ; Push 10
0x02: PUSH1 0x05    ; Push 5
0x04: JUMPI         ; Jump to 5 if 10 != 0
0x05: STOP          ; Unreachable via fallthrough
0x06: JUMPDEST      ; Offset 6
0x07: PUSH1 0x42
0x09: STOP
```

**CodeBlocks:**

- Block 0: Offset 0, bytecode `[PUSH1, 0x0A, PUSH1, 0x05, JUMPI]`
- Block 1: Offset 5, bytecode `[STOP]`, terminates
- Block 2: Offset 7, bytecode `[PUSH1, 0x42, STOP]`, is_jumpdest, terminates

**CFG:**

```
preamble:
  br label %block_0

block_0:
  ; PUSH1 0x0A, PUSH1 0x05, JUMPI
  br i1 %cond, label %jump_table, label %block_1

block_1:
  ; STOP
  ret i8 <Stop>

block_2:
  ; PUSH1 0x42, STOP
  ret i8 <Stop>

jump_table:
  switch i32 %pc, label %jump_failure [
    i32 6, label %block_2
  ]

jump_failure:
  ret i8 <JumpFailure>
```

### Relationships

The following relationships define the CFG structure:

1. **Bytecode Offset → CodeBlock:** Each offset maps to at most one CodeBlock (some offsets may not be block starts).
2. **CodeBlock → BasicBlock:** Each CodeBlock contains exactly one BasicBlock (1:1).
3. **BasicBlock → Terminator:** Each BasicBlock ends with exactly one terminator (LLVM requirement).
4. **Terminator → Successor Blocks:** Each terminator defines 0+ successor blocks:
   - `ret`: 0 successors
   - `br`: 1 successor
   - `br i1`: 2 successors
   - `switch`: N+1 successors (N cases + default)

### EVM Execution Semantics Preservation

The CFG design ensures that:

- **PC Correspondence:** Every LLVM block corresponds to a specific range of EVM bytecode.
- **Jump Validity:** Only JUMPDEST-marked blocks are reachable via JUMP/JUMPI.
- **Reachability:** A block is reachable if:
  - It is the entry block (offset 0).
  - It is the target of an explicit branch.
  - It is marked as `is_jumpdest` and reachable via the jump table.
- **Determinism:** For a given bytecode offset and stack state, execution proceeds deterministically.

## Implications for System Diagramming

### Structural Summary

The Jet compiler's bytecode-to-LLVM lowering can be visualized as a three-layer system:

**Layer 1: Bytecode**
- Linear sequence of bytes
- Logical control flow (JUMP/JUMPI targets)

**Layer 2: CodeBlocks**
- Chunked bytecode ranges
- 1:1 mapping to LLVM BasicBlocks
- Metadata: `is_jumpdest`, `terminates`

**Layer 3: LLVM IR**
- Explicit CFG of BasicBlocks
- Terminator instructions define edges
- Jump table for dynamic dispatch

### Diagrammable Relationships

1. **Bytecode → CodeBlock:**
   - Partition bytecode at JUMPDEST, JUMP, JUMPI, STOP, RETURN, REVERT, INVALID
   - Each CodeBlock owns a contiguous bytecode range

2. **CodeBlock → BasicBlock:**
   - 1:1 correspondence
   - CodeBlock.basic_block points to the LLVM BasicBlock

3. **BasicBlock → Terminator:**
   - Each BasicBlock ends with exactly one terminator
   - Terminator type determines outgoing edges

4. **Terminator → Successors:**
   - `ret`: No successors
   - `br label %B`: One successor (B)
   - `br i1 %cond, label %T, label %F`: Two successors (T, F)
   - `switch`: N+1 successors (cases + default)

5. **Jump Table → JUMPDEST Blocks:**
   - Centralized dispatch node
   - Maps runtime PC values to JUMPDEST-marked blocks
   - Default case: jump_failure (returns JumpFailure)

### Graph Properties

- **Directed graph:** Edges have a clear source and target.
- **Reducible:** The CFG is reducible (no irreducible loops), matching EVM's structured control flow.
- **Single entry, multiple exits:** One preamble block, multiple return points.
- **Dynamic edges:** Jump table represents dynamic control flow (cannot be fully resolved at compile time).

### Visualization Recommendations

For system diagrams:

1. **Node representation:**
   - Rectangles: Regular BasicBlocks
   - Diamonds: Conditional branches (JUMPI)
   - Hexagons: Jump table (dynamic dispatch)
   - Rounded rectangles: Exit nodes (ret)

2. **Edge representation:**
   - Solid arrows: Unconditional branches (fallthrough, JUMP)
   - Dashed arrows: Conditional branches (JUMPI)
   - Dotted arrows: Jump table dispatch (to JUMPDEST blocks)

3. **Annotations:**
   - Label nodes with bytecode offset ranges
   - Mark JUMPDEST blocks with a distinctive color
   - Annotate edges with conditions (e.g., "cond == 0", "cond != 0")

This structural decomposition provides a complete foundation for constructing precise system diagrams that reflect both the static structure (blocks, edges) and dynamic behavior (jump table dispatch) of Jet's bytecode compilation.
