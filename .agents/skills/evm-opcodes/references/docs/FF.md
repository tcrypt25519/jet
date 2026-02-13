---
fork: Frontier
group: System operations
---

## Notes
From cancun behavior: send all Ether in the account to the target.
If executed in the same transaction a contract was created, or pre-Cancun, the current account is registered to be destroyed, and will be at the end of the current transaction. The transfer of the current balance to the given account cannot fail. In particular, the destination account code (if any) is not executed, or, if the account does not exist, the balance is still added to the given address.

Not allowed in EOFv1 code, code containing this instruction will fail validation.

## Stack input

0. `address`: account to send the current balance to (see [BALANCE](/#31) or [SELFBALANCE](/#47) since Istanbul fork).

# Example

[See in playground](/playground?unit=Wei&codeType=Mnemonic&code='ureatekcvhagwrites%20inkslotq14bx64p1p055p052p5601BF3jzMSTORE~14~18jzCREATEzzuvries%20to%20modify%20state%2C%20failsjjjjzDUP5q2bxFFFFzSTATICCALL'~q1%20z%5Cnvontracgtu%2F%2F%20CqzPUSHp600k%20a%20j~0gt%20b%200%01bgjkpquvz~_).

## Gas

The static gas is {gasPrices|selfdestruct}. If a positive balance is sent to an empty account, the dynamic gas is {gasPrices|callNewAccount}. An account is empty if its balance is 0, its nonce is 0 and it has no code. Additionally, if `address` is cold, there is an additional dynamic cost of {gasPrices|coldaccountaccess}. See section [access sets](./about.mdx). There is no additional cost otherwise.

## Error cases

The state changes done by the current context are [reverted](#FD) in those cases:
- Not enough gas.
- Not enough values on the stack.
- The current execution context is from a [STATICCALL](/#FA) (since Byzantium fork) or an [EXTSTATICCALL](/#FB) (since EOF fork).
