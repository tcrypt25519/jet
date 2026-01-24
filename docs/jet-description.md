# Jet: An LLVM-Based JIT Compiler for the Ethereum Virtual Machine

## Project Overview

Jet is an LLVM-based just-in-time (JIT) compiler for Ethereum Virtual Machine (EVM) bytecode. The project compiles EVM bytecode to LLVM intermediate representation (IR) and then uses LLVM's ORC (On-Request Compilation) JIT infrastructure to generate optimized native machine code at runtime.

## Genesis and Evolution

The project originated in 2020 at Ava Labs, where the initial concept was to build a native-machine smart contract platform that went beyond EVM optimization to rethink the execution substrate entirely. 

After the internal project was discontinued due to organizational changes, the concept was reimplemented from scratch in Rust. This clean-room rewrite served multiple purposes: learning Rust, ensuring complete IP provenance clarity, and signaling a fresh implementation with no connection to prior internal work. The Rust implementation also proved well-suited to the problem domain, with explicit ownership semantics for JIT lifetimes and intentional use of unsafe code around executable memory.

The project was renamed from its internal codename to "Jet," a name that naturally captures the "EVM in a JIT" concept while suggesting speed and providing short, composable naming for components like JetBuilder (IR construction) and JetEngine (ORC instantiation and execution).

## Compilation Architecture

### Granularity and Strategy

Jet can compile any amount of EVM bytecode, from manually-written test programs to complete contracts. The system employs a tiered compilation strategy at two levels:

**EVM to LLVM IR Phase**: This phase uses a mixture of eager and lazy compilation. The system analyzes contract execution frequency to determine compilation priorities. Contracts can be identified as frequently-executed by examining their deployment code—Solidity's optimizer makes size-versus-execution-frequency tradeoffs that signal expected usage patterns. Popular contracts (such as Uniswap pool contracts) can be pre-compiled during software initialization and loaded into the JIT engine.

**IR to Native Machine Code Phase**: Inside ORC, this phase also implements both eager and lazy components. ORC not only performs the initial compilation but actively analyzes executing code and recompiles with different optimizations as it determines beneficial. The database stores LLVM IR rather than machine code, making it portable across architectures—the same compiled IR can be moved between systems and will lower to the appropriate machine code at runtime.

### Gas Accounting Optimization

Gas accounting presents a significant performance challenge due to its overhead. Jet provides a toggle to enable or disable gas accounting—essential for production node usage but potentially unnecessary for certain development or analysis contexts.

The implementation optimizes gas accounting by exploiting LLVM's basic block structure. Since a basic block either executes completely or not at all (barring segfaults), gas costs can be amortized across the entire block. Many contracts have infrequent jumps, resulting in large basic blocks where gas accounting reduces to a single addition operation at the block's end.

The exception occurs when instructions have dynamic gas costs computed at runtime. In these cases, additional logic executes during gas accounting, but static-cost-only blocks achieve optimal performance with just one integer addition regardless of block size.

### Control Flow Handling

**Reverts and Exceptions**: These conditions cause execution to jump to a special basic block added to every compiled contract. This block performs cleanup and exits with a status code indicating the specific termination reason. The calling program (the JIT runtime daemon) checks this status code and handles reverts appropriately.

**Dynamic Jumps**: Before compilation begins, Jet iterates through opcodes to identify all basic block boundaries, mapping the EVM bytecode indices that start and stop each block. It then generates a jump table in a special basic block that loads the target opcode index and uses a switch statement to determine the destination basic block.

## Memory Model

The execution context uses a simple linear memory layout. Each contract invocation receives a memory section with three regions:

1. **Metadata Region** (beginning): Stores execution context information including the program counter, jump targets, return data information, and other runtime state
2. **EVM Memory** (middle): The bulk of the allocation, with tunable size based on expected memory requirements
3. **EVM Stack** (end): Fixed size determined by the maximum stack depth (either 1024 or 2048 elements) times 32 bytes per word

A simple arena allocator provisions and deallocates this entire memory slab as a single unit, avoiding fragmentation and allocation overhead during execution.

### Alias Analysis

The current implementation applies basic alias analysis by marking runtime functions (push, pop, etc.) with no-alias attributes where possible. This helps LLVM understand memory access patterns, though more sophisticated alias analysis could provide additional optimization opportunities.

## Symbol Management and Contract Calls

The system uses a shared symbol table for all compiled contracts rather than separate dynamic libraries. Each compiled contract appears in the symbol table with a mangled name consisting of a JET contract prefix followed by the hash of its EVM bytecode.

Contract-to-contract calls within ORC work by taking the called contract's code hash, applying the mangling scheme, and attempting to load the symbol. If the symbol doesn't exist, the system checks the database to determine whether to compile and load the contract or fall back to an interpreted EVM for non-compiled code.

The implementation currently lacks resource management for code eviction when memory limits are reached, though it does handle the self-destruct case by removing the contract's symbol table entry when that return code is received.

## System Architecture

The system comprises two primary components:

**JetEngine**: The core execution engine wrapping LLVM ORC. This component handles the actual compilation and execution of contracts within the JIT environment.

**Runtime Management Layer**: An outer layer that orchestrates the overall system, including:
- Managing the compilation pipeline and database operations
- Deciding when to compile versus load contracts
- Bridging compiled contract execution results to necessary actions (such as writing storage updates)
- Providing storage access to executing contracts through special runtime functions

The architecture aims to integrate into existing node implementations like Reth rather than requiring a completely separate node. The management layer serves as the interface between the JIT execution environment and the broader node infrastructure.

## Design Motivation: The MEV Use Case

A key insight driving the project's resumption came from observing MEV (Maximal Extractable Value) operations. MEV searchers commonly instantiate a Geth (or Reth) EVM locally to simulate contract executions—for example, calculating Uniswap trade outcomes by crafting transactions that call relevant pool functions and executing them locally.

The standard objection to EVM JIT compilation—that I/O bottlenecks in state management dominate execution time—doesn't apply in this context. MEV searchers load all relevant state data into memory once, then execute the same functions thousands of times over in-memory data. This scenario represents the ideal use case for JIT compilation: amortizing compilation costs over many executions with warm data.

This realization extends beyond MEV to suggest a broader architectural pattern: contracts could be lowered directly to shared libraries that any program could link against. For instance, Uniswap utility contracts could be compiled to native code libraries (like Uniswap.dll), allowing direct programmatic access to their functionality without EVM overhead.

## Current Status and Future Directions

The implementation successfully handles the core compilation pipeline from EVM bytecode through LLVM IR to native execution. Testing has progressed from manually-written Huff programs to compiled Solidity contracts including ERC20 implementations.

Outstanding development areas include:
- Completing the contract lifecycle management system for production deployment
- Implementing resource management and code eviction strategies  
- Deepening LLVM optimization integration through enhanced alias analysis
- Exploring alternative symbol table organizations and their performance tradeoffs
- Validating the shared library extraction pattern for compiled contracts