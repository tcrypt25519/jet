---
created: 2026-02-08
updated: 2026-02-08
title: "EVM Codes: About the EVM"
---

```table-of-contents
```

# EVM Codes: About the EVM

## About the EVM

### What is the Ethereum Virtual Machine?

The Ethereum Virtual Machine (or [EVM](https://ethereum.org/en/developers/docs/evm/)) is a stack-based computer responsible for the execution of smart contract instructions. All EVM instructions take their parameter from the stack, except for [PUSHx](https://www.evm.codes/#60), which takes their parameters from the code.

Each instruction has stack inputs, the parameters that they may need, and stack outputs (their return values). The list of these instructions, with their opcodes, is accessible in our [reference](/).

### What is a Smart Contract?

A smart contract is a set of instructions. Each instruction is an opcode (with their own handy mnemonic for reference, text representations of their assigned values between 0 and 255).

When the EVM executes a smart contract, it reads and executes each instruction sequentially, except for [JUMP](https://www.evm.codes/#56) and [JUMPI](https://www.evm.codes/#57) instructions. If an instruction cannot be executed (e.g., when there is insufficient gas or not enough values on the stack), the execution reverts.

Transaction reversion can also be triggered with the [REVERT](https://www.evm.codes/#FD) opcode (which refunds the unused gas fees of its call context, in contrast to all of the gas being consumed in all the other cases of reversion). In the event of a reverted transaction, any state changes dictated by the transaction instructions are returned to their state before the transaction.

### Execution Context & Data Regions

When the EVM executes a smart contract, a context is created for it. This context is made of several data regions, each with a distinct purpose, as well as variables such as the program counter, the current caller, the callee, and the address of the current code.

- **Code:** The region where instructions are stored. Instruction data stored in the code is persistent as part of a contract account state field. Externally owned accounts (or EOAs) have empty code regions. Code is the bytes read, interpreted, and executed by the EVM during smart contract execution. Code is immutable, which means it cannot be modified, but it can be read with the instructions [CODESIZE](https://www.evm.codes/#38) and [CODECOPY](https://www.evm.codes/#39). The code of one contract can be read by other contracts with instructions [EXTCODESIZE](https://www.evm.codes/#3B) and [EXTCODECOPY](https://www.evm.codes/#3C).
- **Program Counter (PC):** Encodes which instruction, stored in the code, should be next read by the EVM. The program counter is usually incremented by one byte to point to the following instruction, with some exceptions. For instance, the [PUSHx](https://www.evm.codes/#60) instruction is longer than a single byte and causes the PC to skip its parameter. The [JUMP](https://www.evm.codes/#56) instruction does not increase the PC's value; instead, it modifies the program counter to a position specified by the top of the stack. [JUMPI](https://www.evm.codes/#57) does this as well if its condition is true (a nonzero code value); otherwise, it increments the PC like other instructions.
- **Stack:** A list of 32-byte elements used to store smart contract instruction inputs and outputs. There is one stack created per call context, and it is destroyed when the call context ends. When a new value is put on the stack, it is put on top, and only the top values are used by the instructions. The stack currently has a maximum limit of 1024 values. All instructions interact with the stack, but it can be directly manipulated with instructions like [PUSH1](https://www.evm.codes/#60), [POP](https://www.evm.codes/#50), [DUP1](https://www.evm.codes/#80), or [SWAP1](https://www.evm.codes/#90).
- **Memory:** EVM memory is not persistent and is destroyed at the end of the call context. At the start of a call context, memory is initialized to 0. Reading and writing from memory is usually done with [MLOAD](https://www.evm.codes/#51) and [MSTORE](https://www.evm.codes/#52) instructions respectively, but can also be accessed by other instructions like [CREATE](https://www.evm.codes/#F0) or [EXTCODECOPY](https://www.evm.codes/#F3). We discuss [memory size calculations](#memory-expansion) later in this document.
- **Storage:** A map of 32-byte slots to 32-byte values. Storage is the persistent memory of smart contracts: each value written by the contract is retained past the completion of a call, unless its value is changed to 0, or the [SELFDESTRUCT](https://www.evm.codes/#FF) instruction is executed. Reading stored bytes from an unwritten key also returns 0. Each contract has its own storage and cannot read or modify storage from another contract. Storage is read and written with instructions [SLOAD](https://www.evm.codes/#54) and [SSTORE](https://www.evm.codes/#55).
- **Calldata:** The data sent to a transaction as part of a smart contract transaction. For example, when creating a contract, calldata would be the constructor code of the new contract. Calldata is immutable and can be read with instructions [CALLDATALOAD](https://www.evm.codes/#35), [CALLDATASIZE](https://www.evm.codes/#36), and [CALLDATACOPY](https://www.evm.codes/#37). It is important to note that when a contract executes an `xCALL` instruction, it also creates an internal transaction. As a result, when executing `xCALL`, there is a calldata region in the new context.
- **Return Data:** The way a smart contract can return a value after a call. It can be set by contract calls through the [RETURN](https://www.evm.codes/#F3) and [REVERT](https://www.evm.codes/#FD) instructions, and can be read by the calling contract with [RETURNDATASIZE](https://www.evm.codes/#3D) and [RETURNDATACOPY](https://www.evm.codes/#3E).

---

### Gas and Fees

Each transaction on the Ethereum blockchain is vetted by a third-party validator before it is added to the blockchain. These validators are compensated for conducting this vetting process and adding transactions to the blockchain with incentive fee payments.

Fees vary from transaction to transaction, contingent on different variables for different forks. Some variables in calculating fees include:

- **Current price of one gas unit:** Gas, or gwei, is a denomination of Ethereum used in fee payment. Gas prices vary over time based on the current demand for block space, measured in ETH per gas.
- **Calldata size:** Each calldata byte costs gas; the larger the size of the transaction data, the higher the gas fees. Calldata costs 4 gas per byte equal to 0, and 16 gas for the others (64 before the hardfork Istanbul).
- **Intrinsic Gas:** Each transaction has an intrinsic cost of 21,000 gas. Creating a contract costs 32,000 gas on top of the transaction cost. Again: calldata costs 4 gas per byte equal to 0, and 16 gas for the others (64 before the hardfork **Istanbul**). This cost is paid from the transaction before any opcode or transfer execution.
- **Opcode Fixed Execution Cost:** Each opcode has a fixed cost to be paid upon execution, measured in gas. This cost is the same for all executions, though this is subject to change in new hardforks. See our [reference](https://www.evm.codes/) to learn about the specific costs per opcode and fork.
- **Opcode Dynamic Execution Cost:** Some instructions conduct more work than others, depending on their parameters. Because of this, on top of fixed costs, some instructions have dynamic costs. These dynamic costs are dependent on several factors (which vary from hardfork to hardfork). See our [reference](https://www.evm.codes/) to learn about the specific computations per opcode and fork.

> **Note:** To get a complete estimation of the gas cost for your program, with your compiler options and specific state and inputs, use a tool like [Remix](https://remix.ethereum.org/) or [Truffle](https://trufflesuite.com/).

### Memory Expansion

During a smart contract execution, memory can be accessed with opcodes. When an offset is first accessed (either read or write), memory may trigger an expansion, which costs gas.

Memory expansion may be triggered when the byte offset (modulo 32) accessed is bigger than previous offsets. If a larger offset trigger of memory expansion occurs, the cost of accessing the higher offset is computed and removed from the total gas available at the current call context.

The total cost for a given memory size is computed as follows:

```solidity
memory_size_word = (memory_byte_size + 31) / 32
memory_cost = (memory_size_word ** 2) / 512 + (3 * memory_size_word)
```

When a memory expansion is triggered, only the additional bytes of memory must be paid for. Therefore, the cost of memory expansion for the specific opcode is:

```solidity
memory_expansion_cost = new_memory_cost - last_memory_cost
```

The `memory_byte_size` can be obtained with opcode [MSIZE](https://www.evm.codes/#59). The cost of memory expansion triggered by [MSIZE](https://www.evm.codes/#59) grows quadratically, disincentivizing the overuse of memory by making higher offsets more costly.

Any opcode that accesses memory may trigger an expansion (such as [MLOAD](https://www.evm.codes/#51), [RETURN](https://www.evm.codes/#F3) or [CALLDATACOPY](https://www.evm.codes/#37)). Use our [reference](/) to review which opcode is capable of accessing memory. Note that opcodes with a byte size parameter of 0 will not trigger memory expansion, regardless of their offset parameters.

---

### Access Sets

An **Access List** is defined per external transaction, not per call. Each transaction may be defined by some combination of its sender, calldata, or callee.

Transactions can either be external or internal:

- **External transactions** are sent to the Ethereum network.
- **Internal transactions** are triggered by external transactions that have executed the `xCALL` instruction. Internal transactions are also known as calls.

The access list can be thought of as two independent types of lists: those of touched addresses and those of touched contract storage slots.

#### Touched Addresses

When an address is accessed by a transaction, instruction, or used as caller or callee, it is put in the access list. Calling the opcode [BALANCE](https://www.evm.codes/#31) on an address not present in an access list costs more than if the address were already in the list.

Other opcodes that can modify the access list include [EXTCODESIZE](https://www.evm.codes/#3B), [EXTCODECOPY](https://www.evm.codes/#3C), [EXTCODEHASH](https://www.evm.codes/#3F), [CALL](https://www.evm.codes/#F1), [CALLCODE](https://www.evm.codes/#F2), [DELEGATECALL](https://www.evm.codes/#F4), [STATICCALL](https://www.evm.codes/#FA), [CREATE](https://www.evm.codes/#F0), [CREATE2](https://www.evm.codes/#F5) and [SELFDESTRUCT](https://www.evm.codes/#FF). Each opcode has its own cost when modifying the access set.

#### Touched Storage Slots

Touch slot lists are a list of storage slot keys accessed by contract addresses. Slot lists are initialized to empty. When an opcode accesses a slot that is not present in the list, it adds it to it. Opcodes that can modify the touched slot list are [SLOAD](https://www.evm.codes/#54) and [SSTORE](https://www.evm.codes/#55). Again, both opcodes have their own cost when modifying the access list.

#### Warm vs. Cold

If an address or storage slot is present in the set, it is called **'warm'**; otherwise, it is **'cold'**. Storage slots that are touched for the first time in a transaction change from cold to warm for the duration of the transaction. Transactions can pre-specify contracts as warm using EIP-2930 access lists. The dynamic cost of some opcodes depends on whether an address or slot is warm or cold.

At the start of a transaction's execution, the touched addresses set is initialized to include the following addresses, which are hence always 'warm':

- **After the Berlin hardfork:** All precompiled contract addresses as well as `tx.sender` and `tx.to` (or the address being created if it is a contract creation transaction).
- **After the Shanghai hardfork:** The [COINBASE](https://www.evm.codes/#41) address.

If a context is reverted, access warming effects are reverted to their state before the context.

### Gas Refunds

Some opcodes can trigger gas refunds, which reduce the gas cost of a transaction. Gas refunds are applied at the end of a transaction. If a transaction has insufficient gas to reach the end of its run, its gas refund cannot be triggered, and the transaction fails.

With the introduction of the **London** hardfork, two aspects of gas refunds changed:
1. The limit to how much gas can be refunded was lowered from half of the total transaction cost to one-fifth of the total transaction cost.
2. The [SELFDESTRUCT](https://www.evm.codes/#FF) opcode cannot trigger gas refunds, only [SSTORE](https://www.evm.codes/#55).
