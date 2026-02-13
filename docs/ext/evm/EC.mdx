---
fork: EOF
group: System operations
---

## Notes

Behavior on accessed_addresses and address collision is same as [CREATE2](/#F5).

The destination address is calculated as follows:

    initialisation_code = eof_container[initcontainer_index]
    address = keccak256(0xff + sender_address + salt + keccak256(initialisation_code))[12:]

Deployment can fail due to:
- A contract already exists at the destination address.
- Insufficient value to transfer.
- Sub [context](./about.mdx) [reverted](/#FD).
- Insufficient gas to execute the initialisation code.
- Call depth limit reached.

Note that these failures only affect the return value and do not cause the calling context to revert (unlike the error cases below).

## Immediate argument 

0. `initcontainer_index`: 8-bit unsigned value as the index of EOF subcontainer in the containers.

## Stack input

0. `value`: value in [wei](https://www.investopedia.com/terms/w/wei.asp) to send to the new account.
1. `salt`: 32-byte value used to create the new account at a deterministic address.
2. `input_offset`: byte offset in the [memory](./about.mdx) in bytes, the initialisation code of the new account.
3. `input_size`: byte size to copy (size of the initialisation code).

## Stack output

0. `address`: the address of the deployed EOF contract, 0 if the deployment failed.

## Examples

*TBD: See in playground*

## Gas

    minimum_word_size = (initcontainer_size + 31) / 32
    hash_cost = {gasPrices|keccak256WordGas} * minimum_word_size
    code_deposit_cost = {gasPrices|createDataGas} * deployed_code_size

    static_gas = {gasPrices|eofcreateGas}
    dynamic_gas = hash_cost + memory_expansion_cost + deployment_code_execution_cost + code_deposit_cost

The `deployment_code_execution_cost` is the cost of whatever opcode is run to deploy the new contract.
On top of that, there are additional costs for storing the code of the new contract, respectively shown as `code_deposit_cost`.
The memory expansion cost explanation can be found [here](./about.mdx).

The new contract address is added in the warm addresses. See section [access sets](./about.mdx).

## Error cases

The state changes done by the current context are [reverted](#FD) in those cases:
- Not enough gas.
- The current execution context is from a [STATICCALL](/#FA) (since Byzantium fork) or an [EXTSTATICCALL](/#FB) (since EOF fork).
- Deploy container code size exceed maximum code size, which is updated within [RETURNCONTRACT](/#EE).
- Execution in a legacy (non-EOF) context.
