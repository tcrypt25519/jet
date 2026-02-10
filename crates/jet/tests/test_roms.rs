use jet::instructions::Instruction;
use jet_runtime::exec::ReturnCode;
use roms::*;

mod roms;

// Keep your flattening macro
macro_rules! bytecode {
    ($($item:expr),* $(,)?) => {
        vec![$($item),*].into_iter().flatten().collect::<Vec<u8>>()
    };
}

// 1. Meta-macro for simple opcodes (0 arguments)
// This iterates over the provided names and generates a macro for each.
macro_rules! define_ops {
    ($($op:ident),* $(,)?) => {
        $(
            macro_rules! $op {
                () => { vec![Instruction::$op.opcode()] };
            }
        )*
    };
}

use paste::paste;

macro_rules! generate_push_macros {
    ($($n:literal),*) => {
        paste! {
            $(
                // Generates macro_rules! PUSH1 { ... }
                macro_rules! [<PUSH $n>] {
                    ($($b:expr),+) => {{
                        // References Instruction::PUSH1
                        let mut v = vec![Instruction::[<PUSH $n>].opcode()];
                        $(v.push($b);)+
                        v
                    }};
                }
            )*
        }
    };
}

// Generate PUSH1 through PUSH32
generate_push_macros!(
    1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26,
    27, 28, 29, 30, 31, 32
);

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
            PUSH1!(0x0A), // Output len
            PUSH1!(0x00), // Output offset
            PUSH1!(0x00), // Input len
            PUSH1!(0x00), // Input offset
            PUSH1!(0x00), // Value
            PUSH2!(0x00, 0x01), // Address
            PUSH1!(0x00), // Gas
            CALL!(), // Mem: 0x00FF
            RETURNDATASIZE!(),
            PUSH1!(0x02), // Len
            PUSH1!(0x00), // Src offset
            PUSH1!(0x02), // Dest offset
            RETURNDATACOPY!(), // Mem: 0x00FF00FF0000000000000000
        ], bytecode![
            PUSH1!(0xFF),
            PUSH1!(0x01),
            MSTORE!(), // Mem: 0x00FF
            PUSH1!(0xFF),
            PUSH1!(0x0A),
            MSTORE!(), // Mem: 0x00FF0000000000000000FF
            PUSH1!(0x0A),
            PUSH1!(0x00),
            RETURN!(), // Return 0x00FF0000000000000000
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
            PUSH1!(0x03), // exponent
            PUSH1!(0x02), // base
            EXP!(),
        ]],
        expected: TestContractRun {
            stack_ptr: 1,
            stack: vec![stack_word(&[8])],
            ..Default::default()
        },
    },

    // Endianness-sensitive: base=256 is stored LE as [0x00, 0x01, ...].
    // A BE bug would misread it as 1, giving 1^2=1 instead of 65536.
    exp_base_256_squared: Test {
        roms: vec![bytecode![
            PUSH1!(0x02),        // exponent
            PUSH2!(0x01, 0x00),  // base = 256
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
            PUSH1!(0x00), // exponent = 0
            PUSH1!(0x05), // base
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
            PUSH1!(0x7F), // x = 127
            PUSH1!(0x00), // b = 0
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
            PUSH1!(0x80), // x = 128
            PUSH1!(0x00), // b = 0
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

    // Endianness-sensitive: 0x8000 stored LE as [0x00, 0x80, ...].
    // A BE bug would put the bytes reversed, so byte 1 = 0x00 and no extension
    // would occur, giving [0x80, 0x00, ...] instead of [0x00, 0x80, 0xFF, ...].
    signextend_multi_byte: Test {
        roms: vec![bytecode![
            PUSH2!(0x80, 0x00), // x = 0x8000
            PUSH1!(0x01),       // b = 1
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
            PUSH1!(0xFF), // x = 255
            PUSH1!(0x20), // b = 32 (>= 32, identity)
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

    // Memory expansion tests
    mstore_at_zero_expands_to_32: Test {
        roms: vec![bytecode![
            PUSH1!(0x42), // value
            PUSH1!(0x00), // offset = 0
            MSTORE!(),      // MSTORE writes 32 bytes
        ]],
        expected: TestContractRun {
            stack_ptr: 0,
            memory_len: Some(32), // ceil((0 + 32) / 32) * 32 = 32
            ..Default::default()
        },
    },

    mstore_at_31_expands_to_64: Test {
        roms: vec![bytecode![
            PUSH1!(0x42), // value
            PUSH1!(0x1F), // offset = 31
            MSTORE!(),      // MSTORE writes 32 bytes
        ]],
        expected: TestContractRun {
            stack_ptr: 0,
            memory_len: Some(64), // ceil((31 + 32) / 32) * 32 = 64
            ..Default::default()
        },
    },

    mstore_at_32_expands_to_64: Test {
        roms: vec![bytecode![
            PUSH1!(0x42), // value
            PUSH1!(0x20), // offset = 32
            MSTORE!(),      // MSTORE writes 32 bytes
        ]],
        expected: TestContractRun {
            stack_ptr: 0,
            memory_len: Some(64), // ceil((32 + 32) / 32) * 32 = 64
            ..Default::default()
        },
    },

    mstore8_at_zero_expands_to_32: Test {
        roms: vec![bytecode![
            PUSH1!(0x42), // value
            PUSH1!(0x00), // offset = 0
            MSTORE8!(),     // MSTORE8 writes 1 byte
        ]],
        expected: TestContractRun {
            stack_ptr: 0,
            memory_len: Some(32), // ceil((0 + 1) / 32) * 32 = 32
            ..Default::default()
        },
    },

    mstore8_at_31_expands_to_32: Test {
        roms: vec![bytecode![
            PUSH1!(0x42), // value
            PUSH1!(0x1F), // offset = 31
            MSTORE8!(),     // MSTORE8 writes 1 byte
        ]],
        expected: TestContractRun {
            stack_ptr: 0,
            memory_len: Some(32), // ceil((31 + 1) / 32) * 32 = 32
            ..Default::default()
        },
    },

    mstore8_at_32_expands_to_64: Test {
        roms: vec![bytecode![
            PUSH1!(0x42), // value
            PUSH1!(0x20), // offset = 32
            MSTORE8!(),     // MSTORE8 writes 1 byte
        ]],
        expected: TestContractRun {
            stack_ptr: 0,
            memory_len: Some(64), // ceil((32 + 1) / 32) * 32 = 64
            ..Default::default()
        },
    },

    memory_expansion_is_monotonic: Test {
        roms: vec![bytecode![
            // First MSTORE expands to 32
            PUSH1!(0x11),
            PUSH1!(0x00),
            MSTORE!(),
            // Second MSTORE at smaller offset doesn't shrink memory
            PUSH1!(0x22),
            PUSH1!(0x00),
            MSTORE!(),
            // Third MSTORE at higher offset expands to 64
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
}
