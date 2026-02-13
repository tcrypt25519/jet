use jet::instructions::Instruction;
use jet_push_macros::generate_push_macros;
use jet_runtime::exec::ReturnCode;
use roms::*;

mod roms;

macro_rules! bytecode {
    ($($item:expr),* $(,)?) => {
        vec![$($item),*].into_iter().flatten().collect::<Vec<u8>>()
    };
}

#[allow(non_snake_case)]
macro_rules! define_ops {
    ($($op:ident),* $(,)?) => {
        $(
            #[allow(non_snake_case)]
            macro_rules! $op {
                () => { vec![Instruction::$op.opcode()] };
            }
        )*
    };
}

// Generate PUSH0..PUSH32 macros
generate_push_macros!(0..=32);

define_ops!(
    ADD,
    MUL,
    SUB,
    DIV,
    MOD,
    EXP,
    SIGNEXTEND,
    KECCAK256,
    JUMP,
    JUMPDEST,
    PC,
    MLOAD,
    MSTORE,
    MSTORE8,
    RETURN,
    CALL,
    RETURNDATASIZE,
    RETURNDATACOPY
);

rom_tests! {
    one_plus_two: Test {
        roms: vec![bytecode![
            PUSH1!(0x01),
            PUSH1!(0x02),
            ADD!(),
        ]],
        expected: TestContractRun {
            stack_ptr: 1,
            stack: vec![stack_word(&[0x03])],
            ..Default::default()
        },
    },

    basic_jump: Test {
        roms: vec![bytecode![
            PUSH1!(0x03),
            JUMP!(),
            JUMPDEST!(),
            PUSH1!(42),
        ]],
        expected: TestContractRun {
            stack_ptr: 1,
            jump_ptr: 3,
            stack: vec![stack_word(&[42])],
            ..Default::default()
        },
    },

    basic_mem_ops: Test {
        roms: vec![bytecode![
            PUSH1!(0xFF),
            PUSH1!(0x02),
            MSTORE!(),
            PUSH1!(0x00),
            MLOAD!(),
            PUSH2!(0xFF, 0xFF),
            PUSH1!(0x00),
            MSTORE8!(),
            PUSH1!(0x00),
            MLOAD!(),
        ]],
        expected: TestContractRun {
            stack_ptr: 2,
            stack: vec![stack_word(&[0x00, 0x00, 0xFF]), stack_word(&[0xFF, 0x00, 0xFF])],
            ..Default::default()
        },
    },

    vstack_accesses_real_stack_after_jump: Test{
        roms: vec![bytecode![
            PUSH1!(0x01),
            PUSH1!(0x02),
            PUSH1!(0x07),
            JUMP!(),
            JUMPDEST!(),
            ADD!(),
            PUSH1!(42),
        ]],
        expected: TestContractRun {
            stack_ptr: 2,
            jump_ptr: 7,
            stack: vec![stack_word(&[0x03]), stack_word(&[0x2A])],
            ..Default::default()
        },
    },

    return_sets_offset_and_length: Test{
        roms: vec![bytecode![
            PUSH1!(0x20),
            PUSH1!(0x03),
            RETURN!(),
        ]],
        expected: TestContractRun {
            result: ReturnCode::ExplicitReturn,
            return_offset: 0x03,
            return_length: 0x20,
            ..Default::default()
        },
    },

    basic_call_with_return_data: Test {
        roms: vec![bytecode![
            PUSH1!(0x0A),
            PUSH1!(0x00),
            PUSH1!(0x00),
            PUSH1!(0x00),
            PUSH1!(0x00),
            PUSH2!(0x00, 0x01),
            PUSH1!(0x00),
            CALL!(),
            RETURNDATASIZE!(),
            PUSH1!(0x02),
            PUSH1!(0x00),
            PUSH1!(0x02),
            RETURNDATACOPY!(),
        ], bytecode![
            PUSH1!(0xFF),
            PUSH1!(0x01),
            MSTORE!(),
            PUSH1!(0xFF),
            PUSH1!(0x0A),
            MSTORE!(),
            PUSH1!(0x0A),
            PUSH1!(0x00),
            RETURN!(),
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
        roms: vec![bytecode![
            PUSH0!(),
            KECCAK256!(),
        ]],
        expected: TestContractRun {
            stack_ptr: 1,
            stack: vec![stack_word(&[0x29, 0x0d, 0xec, 0xd9, 0x54, 0x8b, 0x62, 0xa8, 0xd6, 0x03, 0x45, 0xa9, 0x88, 0x38, 0x6f, 0xc8, 0x4b, 0xa6, 0xbc, 0x95, 0x48, 0x40, 0x08, 0xf6, 0x36, 0x2f, 0x93, 0x16, 0x0e, 0xf3, 0xe5, 0x63])],
            ..Default::default()
        },
    },

    exp_two_cubed: Test {
        roms: vec![bytecode![
            PUSH1!(0x03),
            PUSH1!(0x02),
            EXP!(),
        ]],
        expected: TestContractRun {
            stack_ptr: 1,
            stack: vec![stack_word(&[8])],
            ..Default::default()
        },
    },

    exp_base_256_squared: Test {
        roms: vec![bytecode![
            PUSH1!(0x02),
            PUSH2!(0x01, 0x00),
            EXP!(),
        ]],
        expected: TestContractRun {
            stack_ptr: 1,
            stack: vec![stack_word(&[0x00, 0x00, 0x01])], // 65536 LE
            ..Default::default()
        },
    },

    exp_zero_exponent: Test {
        roms: vec![bytecode![
            PUSH1!(0x00),
            PUSH1!(0x05),
            EXP!(),
        ]],
        expected: TestContractRun {
            stack_ptr: 1,
            stack: vec![stack_word(&[1])],
            ..Default::default()
        },
    },

    signextend_sign_bit_clear: Test {
        roms: vec![bytecode![
            PUSH1!(0x7F),
            PUSH1!(0x00),
            SIGNEXTEND!(),
        ]],
        expected: TestContractRun {
            stack_ptr: 1,
            stack: vec![stack_word(&[0x7F])],
            ..Default::default()
        },
    },

    signextend_sign_bit_set: Test {
        roms: vec![bytecode![
            PUSH1!(0x80),
            PUSH1!(0x00),
            SIGNEXTEND!(),
        ]],
        expected: TestContractRun {
            stack_ptr: 1,
            stack: vec![{
                let mut w = [0xFF_u8; 32];
                w[0] = 0x80;
                w
            }],
            ..Default::default()
        },
    },

    signextend_multi_byte: Test {
        roms: vec![bytecode![
            PUSH2!(0x80, 0x00),
            PUSH1!(0x01),
            SIGNEXTEND!(),
        ]],
        expected: TestContractRun {
            stack_ptr: 1,
            stack: vec![{
                let mut w = [0xFF_u8; 32];
                w[0] = 0x00;
                w[1] = 0x80;
                w
            }],
            ..Default::default()
        },
    },

    signextend_large_b_noop: Test {
        roms: vec![bytecode![
            PUSH1!(0xFF),
            PUSH1!(0x20),
            SIGNEXTEND!(),
        ]],
        expected: TestContractRun {
            stack_ptr: 1,
            stack: vec![stack_word(&[0xFF])],
            ..Default::default()
        },
    },

    program_counter: Test {
        roms: vec![bytecode![
            PC!(),
            PC!(),
            PC!(),
            PUSH1!(0x06),
            JUMP!(),
            JUMPDEST!(),
            PC!(),
        ]],
        expected: TestContractRun {
            stack_ptr: 4,
            jump_ptr: 6,
            stack: vec![stack_word(&[]), stack_word(&[0x01]), stack_word(&[0x02]), stack_word(&[0x07])],
            ..Default::default()
        },
    },

    mstore_at_zero_expands_to_32: Test {
        roms: vec![bytecode![
            PUSH1!(0x42),
            PUSH1!(0x00),
            MSTORE!(),
        ]],
        expected: TestContractRun {
            stack_ptr: 0,
            memory_len: Some(32),
            ..Default::default()
        },
    },

    mstore_at_31_expands_to_64: Test {
        roms: vec![bytecode![
            PUSH1!(0x42),
            PUSH1!(0x1F),
            MSTORE!(),
        ]],
        expected: TestContractRun {
            stack_ptr: 0,
            memory_len: Some(64),
            ..Default::default()
        },
    },

    mstore_at_32_expands_to_64: Test {
        roms: vec![bytecode![
            PUSH1!(0x42),
            PUSH1!(0x20),
            MSTORE!(),
        ]],
        expected: TestContractRun {
            stack_ptr: 0,
            memory_len: Some(64), // ceil((32 + 32) / 32) * 32 = 64
            ..Default::default()
        },
    },

    mstore8_at_zero_expands_to_32: Test {
        roms: vec![bytecode![
            PUSH1!(0x42),
            PUSH1!(0x00),
            MSTORE8!(),
        ]],
        expected: TestContractRun {
            stack_ptr: 0,
            memory_len: Some(32),
            ..Default::default()
        },
    },

    mstore8_at_31_expands_to_32: Test {
        roms: vec![bytecode![
            PUSH1!(0x42),
            PUSH1!(0x1F),
            MSTORE8!(),
        ]],
        expected: TestContractRun {
            stack_ptr: 0,
            memory_len: Some(32),
            ..Default::default()
        },
    },

    mstore8_at_32_expands_to_64: Test {
        roms: vec![bytecode![
            PUSH1!(0x42),
            PUSH1!(0x20),
            MSTORE8!(),
        ]],
        expected: TestContractRun {
            stack_ptr: 0,
            memory_len: Some(64),
            ..Default::default()
        },
    },

    memory_expansion_is_monotonic: Test {
        roms: vec![bytecode![
            PUSH1!(0x11),
            PUSH1!(0x00),
            MSTORE!(),
            PUSH1!(0x22),
            PUSH1!(0x00),
            MSTORE!(),
            PUSH1!(0x33),
            PUSH1!(0x20),
            MSTORE!(),
        ]],
        expected: TestContractRun {
            stack_ptr: 0,
            memory_len: Some(64), // Expanded monotonically: 0 -> 32 -> 32 -> 64
            ..Default::default()
        },
    },

    // Tests basic multiplication: 3 * 5 = 15
    mul_basic: Test {
        roms: vec![bytecode![
            PUSH1!(0x05),
            PUSH1!(0x03),
            MUL!(),
        ]],
        expected: TestContractRun {
            stack_ptr: 1,
            stack: vec![stack_word(&[0x0F])],
            ..Default::default()
        },
    },

    // Tests multiplication by zero: 255 * 0 = 0
    mul_zero: Test {
        roms: vec![bytecode![
            PUSH1!(0x00),
            PUSH1!(0xFF),
            MUL!(),
        ]],
        expected: TestContractRun {
            stack_ptr: 1,
            stack: vec![stack_word(&[0x00])],
            ..Default::default()
        },
    },

    // Tests multiplication overflow: result modulo 2^256
    // Large value * 2 causes overflow in 256-bit arithmetic
    mul_overflow: Test {
        roms: vec![vec![
            Instruction::PUSH1.opcode(), 0x02,
            Instruction::PUSH32.opcode(),
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            Instruction::MUL.opcode(),
        ]],
        expected: TestContractRun {
            stack_ptr: 1,
            stack: vec![{
                let mut w = [0xFF_u8; 32];
                w[0] = 0xFE;
                w
            }],
            ..Default::default()
        },
    },

    // Tests basic subtraction: 10 - 3 = 7
    sub_basic: Test {
        roms: vec![bytecode![
            PUSH1!(0x03),
            PUSH1!(0x0A),
            SUB!(),
        ]],
        expected: TestContractRun {
            stack_ptr: 1,
            stack: vec![stack_word(&[0x07])],
            ..Default::default()
        },
    },

    // Tests subtraction underflow: 3 - 10 wraps to 2^256 - 7
    sub_underflow: Test {
        roms: vec![bytecode![
            PUSH1!(0x0A),
            PUSH1!(0x03),
            SUB!(),
        ]],
        expected: TestContractRun {
            stack_ptr: 1,
            stack: vec![{
                let mut w = [0xFF_u8; 32];
                w[0] = 0xF9;
                w
            }],
            ..Default::default()
        },
    },

    // Tests basic division: 15 / 3 = 5
    div_basic: Test {
        roms: vec![bytecode![
            PUSH1!(0x03),
            PUSH1!(0x0F),
            DIV!(),
        ]],
        expected: TestContractRun {
            stack_ptr: 1,
            stack: vec![stack_word(&[0x05])],
            ..Default::default()
        },
    },

    // Tests division by zero: EVM spec requires 15 / 0 = 0
    div_by_zero: Test {
        roms: vec![bytecode![
            PUSH1!(0x00),
            PUSH1!(0x0F),
            DIV!(),
        ]],
        expected: TestContractRun {
            stack_ptr: 1,
            stack: vec![stack_word(&[0x00])],
            ..Default::default()
        },
    },

    // Tests basic modulo: 14 % 5 = 4
    mod_basic: Test {
        roms: vec![bytecode![
            PUSH1!(0x05),
            PUSH1!(0x0E),
            MOD!(),
        ]],
        expected: TestContractRun {
            stack_ptr: 1,
            stack: vec![stack_word(&[0x04])],
            ..Default::default()
        },
    },

    // Tests modulo by zero: EVM spec requires 14 % 0 = 0
    mod_by_zero: Test {
        roms: vec![bytecode![
            PUSH1!(0x00),
            PUSH1!(0x0E),
            MOD!(),
        ]],
        expected: TestContractRun {
            stack_ptr: 1,
            stack: vec![stack_word(&[0x00])],
            ..Default::default()
        },
    },
}
