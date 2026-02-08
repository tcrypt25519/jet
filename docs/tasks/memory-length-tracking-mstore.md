# Task: Memory length tracking for MSTORE and MSTORE8

## Goal
Track EVM memory length correctly so that memory expansion semantics are preserved and runtime
helpers (like return-data copying) can enforce bounds safely.

## References
- `docs/ext/evm/evm.md` (Memory Expansion section).
- `docs/ext/evm/opcodes.json` entries:
  - `0x52` **MSTORE**: “Save word to memory”.
  - `0x53` **MSTORE8**: “Save byte to memory”.

## EVM semantics to model
- Memory is byte-addressed, zero-initialized per call context, and discarded at the end of the
  call.
- Any memory access with a nonzero length (read or write) can expand memory. MSTORE writes
  32 bytes, MSTORE8 writes 1 byte.
- Expansion is to the highest accessed byte plus one, rounded up to a 32-byte word boundary:
  `new_size = ceil((offset + size) / 32) * 32`.
- The EVM tracks `memory_byte_size` in bytes and exposes it via `MSIZE`.
- Memory expansion gas cost uses:
  ```
  memory_size_word = (memory_byte_size + 31) / 32
  memory_cost = (memory_size_word ** 2) / 512 + (3 * memory_size_word)
  memory_expansion_cost = new_memory_cost - last_memory_cost
  ```
  The sizing logic above must be correct even before gas accounting is implemented.

## Current Jet state
- `Context` stores `memory_ptr`, `memory_len`, and `memory_cap` (`docs/adrs/adr-002.md`).
- Runtime IR helpers `jet.mem.store.word` and `jet.mem.store.byte` write directly to memory
  without updating `memory_len`.
- `jet_contract_call_return_data_copy` uses `memory_len` for bounds and can extend it, but the
  length never reflects writes from MSTORE/MSTORE8 yet.

## Required changes
1. **Shared expansion helper**
   - Implement a helper (Rust builtin or IR helper) that takes `offset` and `size` and returns the
     required memory length.
   - Use `checked_add` on `offset + size` and treat overflow as an execution error.
   - If `size == 0`, do not expand memory.
   - Round the required size up to a 32-byte boundary and update `memory_len` if it increases.
   - If the required length exceeds `memory_cap`, reallocate with 32-byte alignment, zero the new
     bytes, and update `memory_ptr` + `memory_cap` safely.

2. **Hook MSTORE / MSTORE8**
   - In `jet.mem.store.word`, expand with `size = 32` before the memcpy.
   - In `jet.mem.store.byte`, expand with `size = 1` before the byte store.
   - Keep `memory_len` in bytes so that `MSIZE` is correct when implemented.

3. **Extend to other memory opcodes**
   - MLOAD, CALLDATACOPY, RETURNDATACOPY, CODECOPY, KECCAK256, RETURN, and REVERT also
     access memory and should use the same helper so that reads expand memory in the same way
     as writes.

4. **Return-data bounds**
   - Once `memory_len` is updated by MSTORE/MSTORE8, validate destination ranges in
     `jet_contract_call_return_data_copy` after expansion and before copying to avoid unsafe
     slices.

5. **Tests**
   - Add tests in `crates/jet/tests/test_roms.rs` (or runtime-level tests) to validate that:
     - MSTORE at offsets `0`, `31`, and `32` expands memory to `32`, `32`, and `64` bytes.
     - MSTORE8 at offsets `0`, `31`, and `32` expands memory to `32`, `32`, and `64` bytes.
     - Memory expansion is word-aligned and monotonic.
