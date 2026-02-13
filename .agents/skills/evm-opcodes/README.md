# evm-opcodes

A Claude Code skill that gives the agent authoritative, structured EVM opcode documentation on demand — covering stack behaviour, gas costs, error cases, and worked examples for all 170 opcodes.

## Purpose

When working on EVM opcode implementations (in [jet](https://github.com/jetccompiler/jet) or any EVM project), the agent must have accurate spec-level information before touching any opcode. This skill enforces that workflow: for every opcode encountered, the agent runs the lookup script and reads the canonical documentation before writing or reviewing code.

The skill activates automatically whenever an opcode is referenced by name (`ADD`, `PUSH1`, `RETURN`, …), by hex (`01`, `60`, `F3`, …), or when working on stack behaviour, gas accounting, or revert conditions.

## Structure

```
evm-opcodes/
├── SKILL.md                  # Skill definition loaded by Claude Code
├── README.md                 # This file
├── scripts/
│   └── opcode_doc            # Lookup script — prints docs for a given opcode
└── references/
    └── docs/
        ├── 00.md             # STOP
        ├── 01.md             # ADD
        └── ...               # 170 opcode files total
```

## Installation

This skill is consumed by [Claude Code](https://docs.anthropic.com/en/docs/claude-code). To install it, copy (or symlink) the `evm-opcodes/` directory into your Claude Code skills directory:

```bash
# If this repository is already cloned alongside your project:
ln -s /path/to/skills/evm-opcodes ~/.claude/skills/evm-opcodes
```

Or clone the whole skills repository into your Claude Code skills directory:

```bash
git clone <repo-url> ~/.claude/skills
```

Claude Code will detect and load `SKILL.md` automatically.

## Usage

The lookup script is invoked by the agent — but you can also run it manually:

```bash
# From the evm-opcodes skill root:
scripts/opcode_doc 01    # ADD
scripts/opcode_doc 60    # PUSH1
scripts/opcode_doc 0xF3  # RETURN (0x prefix is stripped automatically)
scripts/opcode_doc fa    # STATICCALL (lowercase accepted)
```

The script resolves the docs directory relative to its own location, so it works correctly regardless of the working directory it is called from.

Output for each opcode includes:

- **Fork** — the hardfork that introduced or last changed the opcode
- **Group** — opcode category (arithmetic, stack, memory, …)
- **Stack inputs / outputs** — exact values popped and pushed, in order
- **Gas cost** — static base cost and any dynamic components
- **Examples** — concrete input/output pairs suitable for use as test vectors
- **Error cases** — every condition that triggers a revert

## Acknowledgements

The opcode documentation files in `references/docs/` are sourced from the [evm.codes](https://evm.codes) project by Dune. evm.codes is the canonical open-source EVM opcode reference; all spec detail, examples, and gas tables originate there.

---

MIT License
