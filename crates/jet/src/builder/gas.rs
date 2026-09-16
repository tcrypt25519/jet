use crate::instructions::Instruction;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DynamicGas {
    Exp,
    Keccak256,
    Memory {
        fixed_size: Option<u32>,
        stack_inputs: u32,
    },
    ReturnDataCopy,
    Call,
    Gas,
}

impl DynamicGas {
    pub(crate) const fn stack_inputs(self) -> u32 {
        match self {
            DynamicGas::Gas => 0,
            DynamicGas::Exp | DynamicGas::Keccak256 => 2,
            DynamicGas::Memory { stack_inputs, .. } => stack_inputs,
            DynamicGas::ReturnDataCopy => 3,
            DynamicGas::Call => 7,
        }
    }
}

pub(crate) const fn dynamic_kind(instruction: Instruction) -> Option<DynamicGas> {
    match instruction {
        Instruction::EXP => Some(DynamicGas::Exp),
        Instruction::KECCAK256 => Some(DynamicGas::Keccak256),
        Instruction::MLOAD => Some(DynamicGas::Memory {
            fixed_size: Some(32),
            stack_inputs: 1,
        }),
        Instruction::MSTORE => Some(DynamicGas::Memory {
            fixed_size: Some(32),
            stack_inputs: 2,
        }),
        Instruction::MSTORE8 => Some(DynamicGas::Memory {
            fixed_size: Some(1),
            stack_inputs: 2,
        }),
        Instruction::RETURN | Instruction::REVERT => Some(DynamicGas::Memory {
            fixed_size: None,
            stack_inputs: 2,
        }),
        Instruction::RETURNDATACOPY => Some(DynamicGas::ReturnDataCopy),
        Instruction::CALL => Some(DynamicGas::Call),
        Instruction::GAS => Some(DynamicGas::Gas),
        _ => None,
    }
}

pub(crate) const fn static_cost(instruction: Instruction) -> u64 {
    match instruction {
        Instruction::STOP | Instruction::RETURN | Instruction::REVERT => 0,
        Instruction::ADD
        | Instruction::SUB
        | Instruction::LT
        | Instruction::GT
        | Instruction::SLT
        | Instruction::SGT
        | Instruction::EQ
        | Instruction::ISZERO
        | Instruction::AND
        | Instruction::OR
        | Instruction::XOR
        | Instruction::NOT
        | Instruction::BYTE
        | Instruction::SHL
        | Instruction::SHR
        | Instruction::SAR
        | Instruction::CALLDATALOAD
        | Instruction::CALLDATACOPY
        | Instruction::CODECOPY
        | Instruction::RETURNDATACOPY
        | Instruction::MLOAD
        | Instruction::MSTORE
        | Instruction::MSTORE8
        | Instruction::MCOPY
        | Instruction::PUSH1
        | Instruction::PUSH2
        | Instruction::PUSH3
        | Instruction::PUSH4
        | Instruction::PUSH5
        | Instruction::PUSH6
        | Instruction::PUSH7
        | Instruction::PUSH8
        | Instruction::PUSH9
        | Instruction::PUSH10
        | Instruction::PUSH11
        | Instruction::PUSH12
        | Instruction::PUSH13
        | Instruction::PUSH14
        | Instruction::PUSH15
        | Instruction::PUSH16
        | Instruction::PUSH17
        | Instruction::PUSH18
        | Instruction::PUSH19
        | Instruction::PUSH20
        | Instruction::PUSH21
        | Instruction::PUSH22
        | Instruction::PUSH23
        | Instruction::PUSH24
        | Instruction::PUSH25
        | Instruction::PUSH26
        | Instruction::PUSH27
        | Instruction::PUSH28
        | Instruction::PUSH29
        | Instruction::PUSH30
        | Instruction::PUSH31
        | Instruction::PUSH32
        | Instruction::DUP1
        | Instruction::DUP2
        | Instruction::DUP3
        | Instruction::DUP4
        | Instruction::DUP5
        | Instruction::DUP6
        | Instruction::DUP7
        | Instruction::DUP8
        | Instruction::DUP9
        | Instruction::DUP10
        | Instruction::DUP11
        | Instruction::DUP12
        | Instruction::DUP13
        | Instruction::DUP14
        | Instruction::DUP15
        | Instruction::DUP16
        | Instruction::SWAP1
        | Instruction::SWAP2
        | Instruction::SWAP3
        | Instruction::SWAP4
        | Instruction::SWAP5
        | Instruction::SWAP6
        | Instruction::SWAP7
        | Instruction::SWAP8
        | Instruction::SWAP9
        | Instruction::SWAP10
        | Instruction::SWAP11
        | Instruction::SWAP12
        | Instruction::SWAP13
        | Instruction::SWAP14
        | Instruction::SWAP15
        | Instruction::SWAP16 => 3,
        Instruction::MUL
        | Instruction::DIV
        | Instruction::SDIV
        | Instruction::MOD
        | Instruction::SMOD
        | Instruction::SIGNEXTEND => 5,
        Instruction::ADDMOD | Instruction::MULMOD | Instruction::JUMP => 8,
        Instruction::EXP | Instruction::JUMPI => 10,
        Instruction::KECCAK256 => 30,
        Instruction::ADDRESS
        | Instruction::ORIGIN
        | Instruction::CALLER
        | Instruction::CALLVALUE
        | Instruction::CALLDATASIZE
        | Instruction::CODESIZE
        | Instruction::GASPRICE
        | Instruction::RETURNDATASIZE
        | Instruction::COINBASE
        | Instruction::TIMESTAMP
        | Instruction::NUMBER
        | Instruction::DIFFICULTY
        | Instruction::GASLIMIT
        | Instruction::CHAINID
        | Instruction::SELFBALANCE
        | Instruction::BASEFEE
        | Instruction::BLOBBASEFEE
        | Instruction::PC
        | Instruction::MSIZE
        | Instruction::GAS
        | Instruction::POP => 2,
        Instruction::BLOCKHASH => 20,
        Instruction::JUMPDEST => 1,
        Instruction::PUSH0 => 2,
        Instruction::TLOAD | Instruction::TSTORE => 100,
        Instruction::LOG0
        | Instruction::LOG1
        | Instruction::LOG2
        | Instruction::LOG3
        | Instruction::LOG4 => 375,
        Instruction::CREATE | Instruction::CREATE2 => 32_000,
        Instruction::BALANCE
        | Instruction::EXTCODESIZE
        | Instruction::EXTCODECOPY
        | Instruction::EXTCODEHASH
        | Instruction::SLOAD
        | Instruction::SSTORE
        | Instruction::CALL
        | Instruction::CALLCODE
        | Instruction::DELEGATECALL
        | Instruction::STATICCALL
        | Instruction::SELFDESTRUCT
        | Instruction::BLOBHASH
        | Instruction::INVALID => 0,
    }
}
