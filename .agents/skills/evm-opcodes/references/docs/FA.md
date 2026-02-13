---
fork: Byzantium
group: System operations
---

## Notes

Creates a new sub [context](./about.mdx) and executes the [code](./about.mdx) of the given account, then resumes the current one. Note that an account with no code will return success as true (1).

This instruction is equivalent to [CALL](/#F1), except that it does not allow any state modifying instructions or sending ETH in the sub [context](./about.mdx). The disallowed instructions are [CREATE](/#F0), [CREATE2](/#F5), [LOG0](/#A0), [LOG1](/#A1), [LOG2](/#A2), [LOG3](/#A3), [LOG4](/#A4), [SSTORE](/#55), [SELFDESTRUCT](/#FF), [EOFCREATE](/#EC), as well as [CALL](/#F1) or [EXTCALL](/#F8) if the [value](/#34) sent is not 0.

If the size of the [return data](./about.mdx) is not known, it can also be retrieved after the call with the instructions [RETURNDATASIZE](/#3D) and [RETURNDATACOPY](/#3E) (since the Byzantium fork).

From the Tangerine Whistle fork, `gas` is capped at all but one 64th (`remaining_gas / 64`) of the remaining gas of the current context. If a call tries to send more, the `gas` is changed to match the maximum allowed.

Not allowed in EOFv1 code, code containing this instruction will fail validation.

## Stack input

0. `gas`: amount of gas to send to the sub [context](./about.mdx) to execute. The gas that is not used by the sub context is returned to this one.
1. `address`: the account which [context](./about.mdx) to execute.
2. `argsOffset`: byte offset in the [memory](./about.mdx) in bytes, the [calldata](./about.mdx) of the sub [context](./about.mdx).
3. `argsSize`: byte size to copy (size of the [calldata](./about.mdx)).
4. `retOffset`: byte offset in the [memory](./about.mdx) in bytes, where to store the [return data](./about.mdx) of the sub [context](./about.mdx).
5. `retSize`: byte size to copy (size of the [return data](./about.mdx)).

## Stack output

0. `success`: return 0 if the sub [context](./about.mdx) [reverted](/#FD), 1 otherwise.

## Gas

    static_gas = {gasPrices|call}
    dynamic_gas = memory_expansion_cost + code_execution_cost + address_access_cost

The memory expansion cost explanation can be found [here](./about.mdx).

The different costs are:
 - `code_execution_cost` is the cost of the called code execution (limited by the `gas` parameter).
 - If `address` is warm, then `address_access_cost` is {gasPrices|warmstorageread}, otherwise it is {gasPrices|coldaccountaccess}. See section [access sets](./about.mdx).

## Error cases

The state changes done by the current context are [reverted](#FD) in those cases:
- Not enough gas.
- Not enough values on the stack.
