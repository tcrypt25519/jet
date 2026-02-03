use jet::instructions::Instruction;
use jet_runtime::exec::ReturnCode;
use roms::*;

mod roms;

rom_tests! {
    one_plus_two: Test {
        roms: vec![vec![
            Instruction::PUSH1.opcode(),
            0x01,
            Instruction::PUSH1.opcode(),
            0x02,
            Instruction::ADD.opcode(),
        ]],
        expected: TestContractRun {
            stack_ptr: 1,
            stack: vec![stack_word(&[0x03])],
            ..Default::default()
        },
    },

    basic_jump: Test {
        roms: vec![vec![
            Instruction::PUSH1.opcode(),
            0x03,
            Instruction::JUMP.opcode(),
            Instruction::JUMPDEST.opcode(),
            Instruction::PUSH1.opcode(),
            42,
        ]],
        expected: TestContractRun {
            stack_ptr: 1,
            jump_ptr: 3,
            stack: vec![stack_word(&[42])],
            ..Default::default()
        },
    },

    basic_mem_ops: Test {
        roms: vec![vec![
            Instruction::PUSH1.opcode(),
            0xFF,
            Instruction::PUSH1.opcode(),
            0x02,
            Instruction::MSTORE.opcode(),
            Instruction::PUSH1.opcode(),
            0x00,
            Instruction::MLOAD.opcode(),
            Instruction::PUSH2.opcode(),
            0xFF,
            0xFF,
            Instruction::PUSH1.opcode(),
            0x00,
            Instruction::MSTORE8.opcode(),
            Instruction::PUSH1.opcode(),
            0x00,
            Instruction::MLOAD.opcode(),
        ]],
        expected: TestContractRun {
            stack_ptr: 2,
            stack: vec![stack_word(&[0x00, 0x00, 0xFF]), stack_word(&[0xFF, 0x00, 0xFF])],
            ..Default::default()
        },
    },

    vstack_accesses_real_stack_after_jump: Test{
        roms: vec![vec![
            Instruction::PUSH1.opcode(),
            0x01,
            Instruction::PUSH1.opcode(),
            0x02,
            Instruction::PUSH1.opcode(),
            0x07,
            Instruction::JUMP.opcode(),
            Instruction::JUMPDEST.opcode(),
            Instruction::ADD.opcode(),
            Instruction::PUSH1.opcode(),
            42,
        ]],
        expected: TestContractRun {
            stack_ptr: 2,
            jump_ptr: 7,
            stack: vec![stack_word(&[0x03]), stack_word(&[0x2A])],
            ..Default::default()
        },
    },

    return_sets_offset_and_length: Test{
        roms: vec![vec![
            Instruction::PUSH1.opcode(),
            0x20,
            Instruction::PUSH1.opcode(),
            0x03,
            Instruction::RETURN.opcode(),
        ]],
        expected: TestContractRun {
            result: ReturnCode::ExplicitReturn,
            return_offset: 0x03,
            return_length: 0x20,
            ..Default::default()
        },
    },

    basic_call_with_return_data: Test {
        roms: vec![vec![
            Instruction::PUSH1.opcode(), // Output len
            0x0A,
            Instruction::PUSH1.opcode(), // Output offset
            0x00,
            Instruction::PUSH1.opcode(), // Input len
            0x00,
            Instruction::PUSH1.opcode(), // Input offset
            0x00,
            Instruction::PUSH1.opcode(), // Value
            0x00,
            Instruction::PUSH2.opcode(), // Address
            0x00,
            0x01,
            Instruction::PUSH1.opcode(), // Gas
            0x00,
            Instruction::CALL.opcode(), // Mem: 0x00FF
            Instruction::RETURNDATASIZE.opcode(),
            Instruction::PUSH1.opcode(), // Len
            0x02,
            Instruction::PUSH1.opcode(), // Src offset
            0x00,
            Instruction::PUSH1.opcode(), // Dest offset
            0x02,
            Instruction::RETURNDATACOPY.opcode(), // Mem: 0x00FF00FF0000000000000000
        ], vec![
            Instruction::PUSH1.opcode(),
            0xFF,
            Instruction::PUSH1.opcode(),
            0x01,
            Instruction::MSTORE.opcode(), // Mem: 0x00FF
            Instruction::PUSH1.opcode(),
            0xFF,
            Instruction::PUSH1.opcode(),
            0x0A,
            Instruction::MSTORE.opcode(), // Mem: 0x00FF0000000000000000FF
            Instruction::PUSH1.opcode(),
            0x0A,
            Instruction::PUSH1.opcode(),
            0x00,
            Instruction::RETURN.opcode(), // Return 0x00FF0000000000000000
        ]],
        expected: TestContractRun {
            stack_ptr: 2,
            stack: vec![
                stack_word(&[0x00]),
                stack_word(&[0x0A])
            ],
            memory: Some(vec![0x00, 0xFF, 0x00, 0xFF]),
            ..Default::default()
        },
    },

    keccak256_empty_hash: Test {
        roms: vec![vec![
            Instruction::PUSH0.opcode(),
            Instruction::KECCAK256.opcode(),
        ]],
        expected: TestContractRun {
            stack_ptr: 1,
            stack: vec![stack_word(&[0x29, 0x0d, 0xec, 0xd9, 0x54, 0x8b, 0x62, 0xa8, 0xd6, 0x03, 0x45, 0xa9, 0x88, 0x38, 0x6f, 0xc8, 0x4b, 0xa6, 0xbc, 0x95, 0x48, 0x40, 0x08, 0xf6, 0x36, 0x2f, 0x93, 0x16, 0x0e, 0xf3, 0xe5, 0x63])],
            ..Default::default()
        },
    },

    program_counter: Test {
        roms: vec![vec![
            Instruction::PC.opcode(),
            Instruction::PC.opcode(),
            Instruction::PC.opcode(),
            Instruction::PUSH1.opcode(),
            0x06,
            Instruction::JUMP.opcode(),
            Instruction::JUMPDEST.opcode(),
            Instruction::PC.opcode(),
        ]],
        expected: TestContractRun {
            stack_ptr: 4,
            jump_ptr: 6,
            stack: vec![stack_word(&[]), stack_word(&[0x01]), stack_word(&[0x02]), stack_word(&[0x07])],
            ..Default::default()
        },
    },
}
