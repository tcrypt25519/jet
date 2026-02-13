---
fork: EOF
group: System operations
---

## Notes

Context from [EOFCREATE](/#EC), it loads auxiliary data from memory stack and that will be appended to deployed container's data. 

The container index and auxiliary data are used to construct deployed contract.

Should deployment succeed, the account's [code](./about.mdx) is set to the [return data](./about.mdx) resulting from executing the initialisation code.

## Immediate argument 

0. `deploy_container_index`: 8-bit unsigned value as the index of EOF containers.

## Stack input

0. `aux_data_offset`: byte offset in the [memory](./about.mdx) in bytes.
1. `aux_data_size`: byte size to copy (size of the auxiliary data).

## Examples

*TBD: See in playground*

## Gas

    static_gas = {gasPrices|returncontractGas}
    dynamic_gas = memory_expansion_cost

The memory expansion cost explanation can be found [here](./about.mdx).

The new contract address is added in the warm addresses. See section [access sets](./about.mdx).

## Error cases

The state changes done by the current context are [reverted](#FD) in those cases:
- Not enough gas.
- After appending, the data section size is less than declared in the header or exceeds the maximum data section size.
- After appending, the container size exceeds the maximum code size limit.
- Execution in a legacy (non-EOF) context.
