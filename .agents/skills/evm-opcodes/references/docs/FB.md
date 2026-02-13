---
fork: EOF
group: System operations
---

## Notes

Creates a new sub [context](./about.mdx) and execute the [code](./about.mdx) of the given account, then resumes the current one. Note that an account with no code will return success status.

This instruction is equivalent to [EXTCALL](/#F8), except that it does not allow any state modifying instructions or sending ETH in the sub [context](./about.mdx). The disallowed instructions are [CREATE](/#F0), [CREATE2](/#F5), [LOG0](/#A0), [LOG1](/#A1), [LOG2](/#A2), [LOG3](/#A3), [LOG4](/#A4), [SSTORE](/#55), [SELFDESTRUCT](/#FF), [EOFCREATE](/#EC), as well as [CALL](/#F1) or [EXTCALL](/#F8) if the [value](/#34) sent is not 0.

The size of the [return data](./about.mdx) can be retrieved after the call with the instructions [RETURNDATASIZE](/#3D) and [RETURNDATACOPY](/#3E) (since the Byzantium fork), or load 32-byte word onto the stack with [RETURNDATALOAD](/#F7) instruction.

All but one 64th (`remaining_gas / 64`) of the remaining gas of the current context is sent to the sub [context](./about.mdx) to execute, but no less than 2300. The gas that is not used by the sub context is returned to this one. In case gas not sent would be lower than 5000, the call fails returning 1 as `status`, but the current [context](./about.mdx) is not reverted.

If the call stack depth is 1024, the call fails returning 1 as `status`, but the current [context](./about.mdx) is not reverted.

## Stack input

0. `target_address`: the account which [context](./about.mdx) to execute.
1. `input_offset`: byte offset in the [memory](./about.mdx) in bytes, the [calldata](./about.mdx) of the sub [context](./about.mdx).
2. `input_size`: byte size to copy (size of the [calldata](./about.mdx)).

## Stack output

0. `status`: return 0 for success, 1 for revert if the sub [context](./about.mdx) [reverted](/#FD), 2 for failure.

## Examples

*TBD: See in playground.*

## Gas

    static_gas = {gasPrices|extstaticcallGas}
    dynamic_gas = memory_expansion_cost + code_execution_cost + address_access_cost

The memory expansion cost explanation can be found [here](./about.mdx).

The different costs are:
 - `code_execution_cost` is the cost of the called code execution.
 - If `address` is not warm, then `address_access_cost` is {gasPrices|coldaccountaccess} minus {gasPrices|warmstorageread}, otherwise it is {gasPrices|warmstorageread}. See section [access sets](./about.mdx).

## Error cases

The state changes done by the current context are [reverted](#FD) in those cases:
- Not enough gas.
- The target address has more than 20 bytes.
- Execution in a legacy (non-EOF) context.
