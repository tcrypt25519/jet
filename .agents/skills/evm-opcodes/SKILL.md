---
name: evm-opcodes
description: Required for all work involving EVM opcodes or instructions in the jet project — these terms are interchangeable. Triggers whenever implementing, testing, reviewing, or debugging any EVM opcode. Use when an opcode is referenced by name (ADD, PUSH1, RETURN, etc.), by hex code (01, 60, F3, etc.), or when working on EVM stack behavior, gas accounting, or revert conditions for specific instructions.
---

# EVM Opcodes

## Mandatory: Load Documentation Before Working on Any Opcode

For **every** opcode you work on — whether implementing, testing, reviewing, or debugging — run this script from the skill's root directory:

```bash
scripts/opcode_doc <HEX>
```

- `<HEX>` is the two-character hex code for the opcode
- The script accepts uppercase or lowercase, with or without `0x` prefix
- Run it for **each** opcode when working on multiple at once

```bash
scripts/opcode_doc 01   # ADD
scripts/opcode_doc 60   # PUSH1
scripts/opcode_doc F3   # RETURN
scripts/opcode_doc 0A   # EXP
```

Output includes: stack inputs, stack outputs, gas cost, examples, and error cases.

## How to Use the Output

- **Stack inputs**: Verify the implementation pops exactly these values in this order
- **Stack output**: Verify the implementation pushes exactly these values
- **Gas cost**: Ensure gas accounting matches — static cost, dynamic cost, memory expansion
- **Error cases**: Ensure every listed revert condition is handled
- **Examples**: Use the provided input/output pairs as test cases
