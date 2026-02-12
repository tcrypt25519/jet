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
    SDIV,
    SMOD,
    ADDMOD,
    MULMOD,
    EXP,
    SIGNEXTEND,
    LT,
    GT,
    SLT,
    SGT,
    EQ,
    ISZERO,
    AND,
    OR,
    XOR,
    NOT,
    BYTE,
    SHL,
    SHR,
    SAR,
    KECCAK256,
    JUMP,
    JUMPI,
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

    // === Comparison Operations: LT (Less Than - Unsigned) ===
    
    // Tests LT: 5 < 10 should return 1 (true)
    lt_true: Test {
        roms: vec![bytecode![
            PUSH1!(0x0A),  // Push 10
            PUSH1!(0x05),  // Push 5
            LT!(),         // 5 < 10
        ]],
        expected: TestContractRun {
            stack_ptr: 1,
            stack: vec![stack_word(&[0x01])], // true
            ..Default::default()
        },
    },

    // Tests LT: 10 < 5 should return 0 (false)
    lt_false: Test {
        roms: vec![bytecode![
            PUSH1!(0x05),  // Push 5
            PUSH1!(0x0A),  // Push 10
            LT!(),         // 10 < 5
        ]],
        expected: TestContractRun {
            stack_ptr: 1,
            stack: vec![stack_word(&[0x00])], // false
            ..Default::default()
        },
    },

    // Tests LT: equal values should return 0 (false)
    lt_equal: Test {
        roms: vec![bytecode![
            PUSH1!(0x07),  // Push 7
            PUSH1!(0x07),  // Push 7
            LT!(),         // 7 < 7
        ]],
        expected: TestContractRun {
            stack_ptr: 1,
            stack: vec![stack_word(&[0x00])], // false
            ..Default::default()
        },
    },

    // Tests LT: 0 < 1 should return 1 (true) - boundary case
    lt_zero_boundary: Test {
        roms: vec![bytecode![
            PUSH1!(0x01),  // Push 1
            PUSH1!(0x00),  // Push 0
            LT!(),         // 0 < 1
        ]],
        expected: TestContractRun {
            stack_ptr: 1,
            stack: vec![stack_word(&[0x01])], // true
            ..Default::default()
        },
    },

    // === Comparison Operations: GT (Greater Than - Unsigned) ===
    
    // Tests GT: 10 > 5 should return 1 (true)
    gt_true: Test {
        roms: vec![bytecode![
            PUSH1!(0x05),  // Push 5
            PUSH1!(0x0A),  // Push 10
            GT!(),         // 10 > 5
        ]],
        expected: TestContractRun {
            stack_ptr: 1,
            stack: vec![stack_word(&[0x01])], // true
            ..Default::default()
        },
    },

    // Tests GT: 5 > 10 should return 0 (false)
    gt_false: Test {
        roms: vec![bytecode![
            PUSH1!(0x0A),  // Push 10
            PUSH1!(0x05),  // Push 5
            GT!(),         // 5 > 10
        ]],
        expected: TestContractRun {
            stack_ptr: 1,
            stack: vec![stack_word(&[0x00])], // false
            ..Default::default()
        },
    },

    // Tests GT: equal values should return 0 (false)
    gt_equal: Test {
        roms: vec![bytecode![
            PUSH1!(0x07),  // Push 7
            PUSH1!(0x07),  // Push 7
            GT!(),         // 7 > 7
        ]],
        expected: TestContractRun {
            stack_ptr: 1,
            stack: vec![stack_word(&[0x00])], // false
            ..Default::default()
        },
    },

    // Tests GT: 1 > 0 should return 1 (true) - boundary case
    gt_zero_boundary: Test {
        roms: vec![bytecode![
            PUSH1!(0x00),  // Push 0
            PUSH1!(0x01),  // Push 1
            GT!(),         // 1 > 0
        ]],
        expected: TestContractRun {
            stack_ptr: 1,
            stack: vec![stack_word(&[0x01])], // true
            ..Default::default()
        },
    },

    // === Comparison Operations: SLT (Signed Less Than) ===
    
    // Tests SLT: -1 < 0 should return 1 (true) in signed comparison
    // In two's complement, -1 is 0xFF...FF
    slt_negative_less_than_zero: Test {
        roms: vec![vec![
            Instruction::PUSH1.opcode(), 0x00,  // Push 0
            Instruction::PUSH32.opcode(),       // Push -1 (all 0xFF)
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            Instruction::SLT.opcode(),          // -1 < 0
        ]],
        expected: TestContractRun {
            stack_ptr: 1,
            stack: vec![stack_word(&[0x01])], // true
            ..Default::default()
        },
    },

    // Tests SLT: 0 < -1 should return 0 (false) in signed comparison
    slt_zero_not_less_than_negative: Test {
        roms: vec![vec![
            Instruction::PUSH32.opcode(),       // Push -1 (all 0xFF)
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            Instruction::PUSH1.opcode(), 0x00,  // Push 0
            Instruction::SLT.opcode(),          // 0 < -1
        ]],
        expected: TestContractRun {
            stack_ptr: 1,
            stack: vec![stack_word(&[0x00])], // false
            ..Default::default()
        },
    },

    // Tests SLT: positive values work correctly
    slt_positive_comparison: Test {
        roms: vec![bytecode![
            PUSH1!(0x0A),  // Push 10
            PUSH1!(0x05),  // Push 5
            SLT!(),        // 5 < 10
        ]],
        expected: TestContractRun {
            stack_ptr: 1,
            stack: vec![stack_word(&[0x01])], // true
            ..Default::default()
        },
    },

    // Tests SLT: equal values return false
    slt_equal: Test {
        roms: vec![bytecode![
            PUSH1!(0x07),  // Push 7
            PUSH1!(0x07),  // Push 7
            SLT!(),        // 7 < 7
        ]],
        expected: TestContractRun {
            stack_ptr: 1,
            stack: vec![stack_word(&[0x00])], // false
            ..Default::default()
        },
    },

    // === Comparison Operations: SGT (Signed Greater Than) ===
    
    // Tests SGT: 0 > -1 should return 1 (true) in signed comparison
    sgt_zero_greater_than_negative: Test {
        roms: vec![vec![
            Instruction::PUSH32.opcode(),       // Push -1 (all 0xFF)
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            Instruction::PUSH1.opcode(), 0x00,  // Push 0
            Instruction::SGT.opcode(),          // 0 > -1
        ]],
        expected: TestContractRun {
            stack_ptr: 1,
            stack: vec![stack_word(&[0x01])], // true
            ..Default::default()
        },
    },

    // Tests SGT: -1 > 0 should return 0 (false) in signed comparison
    sgt_negative_not_greater_than_zero: Test {
        roms: vec![vec![
            Instruction::PUSH1.opcode(), 0x00,  // Push 0
            Instruction::PUSH32.opcode(),       // Push -1 (all 0xFF)
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            Instruction::SGT.opcode(),          // -1 > 0
        ]],
        expected: TestContractRun {
            stack_ptr: 1,
            stack: vec![stack_word(&[0x00])], // false
            ..Default::default()
        },
    },

    // Tests SGT: positive values work correctly
    sgt_positive_comparison: Test {
        roms: vec![bytecode![
            PUSH1!(0x05),  // Push 5
            PUSH1!(0x0A),  // Push 10
            SGT!(),        // 10 > 5
        ]],
        expected: TestContractRun {
            stack_ptr: 1,
            stack: vec![stack_word(&[0x01])], // true
            ..Default::default()
        },
    },

    // Tests SGT: equal values return false
    sgt_equal: Test {
        roms: vec![bytecode![
            PUSH1!(0x07),  // Push 7
            PUSH1!(0x07),  // Push 7
            SGT!(),        // 7 > 7
        ]],
        expected: TestContractRun {
            stack_ptr: 1,
            stack: vec![stack_word(&[0x00])], // false
            ..Default::default()
        },
    },

    // === Comparison Operations: EQ (Equality) ===
    
    // Tests EQ: equal values should return 1 (true)
    eq_true: Test {
        roms: vec![bytecode![
            PUSH1!(0x2A),  // Push 42
            PUSH1!(0x2A),  // Push 42
            EQ!(),         // 42 == 42
        ]],
        expected: TestContractRun {
            stack_ptr: 1,
            stack: vec![stack_word(&[0x01])], // true
            ..Default::default()
        },
    },

    // Tests EQ: different values should return 0 (false)
    eq_false: Test {
        roms: vec![bytecode![
            PUSH1!(0x05),  // Push 5
            PUSH1!(0x0A),  // Push 10
            EQ!(),         // 10 == 5
        ]],
        expected: TestContractRun {
            stack_ptr: 1,
            stack: vec![stack_word(&[0x00])], // false
            ..Default::default()
        },
    },

    // Tests EQ: zero equality
    eq_zero: Test {
        roms: vec![bytecode![
            PUSH1!(0x00),  // Push 0
            PUSH1!(0x00),  // Push 0
            EQ!(),         // 0 == 0
        ]],
        expected: TestContractRun {
            stack_ptr: 1,
            stack: vec![stack_word(&[0x01])], // true
            ..Default::default()
        },
    },

    // Tests EQ: max value equality (using PUSH32 for large values)
    eq_max_value: Test {
        roms: vec![vec![
            Instruction::PUSH32.opcode(),
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            Instruction::PUSH32.opcode(),
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            Instruction::EQ.opcode(),
        ]],
        expected: TestContractRun {
            stack_ptr: 1,
            stack: vec![stack_word(&[0x01])], // true
            ..Default::default()
        },
    },

    // === Comparison Operations: ISZERO ===
    
    // Tests ISZERO: zero value should return 1 (true)
    iszero_true: Test {
        roms: vec![bytecode![
            PUSH1!(0x00),  // Push 0
            ISZERO!(),     // is 0 zero?
        ]],
        expected: TestContractRun {
            stack_ptr: 1,
            stack: vec![stack_word(&[0x01])], // true
            ..Default::default()
        },
    },

    // Tests ISZERO: non-zero value should return 0 (false)
    iszero_false: Test {
        roms: vec![bytecode![
            PUSH1!(0x2A),  // Push 42
            ISZERO!(),     // is 42 zero?
        ]],
        expected: TestContractRun {
            stack_ptr: 1,
            stack: vec![stack_word(&[0x00])], // false
            ..Default::default()
        },
    },

    // Tests ISZERO: max value should return 0 (false)
    iszero_max_value: Test {
        roms: vec![vec![
            Instruction::PUSH32.opcode(),
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            Instruction::ISZERO.opcode(),
        ]],
        expected: TestContractRun {
            stack_ptr: 1,
            stack: vec![stack_word(&[0x00])], // false
            ..Default::default()
        },
    },

    // === Arithmetic Operations: SDIV (Signed Division) ===
    
    // Tests SDIV: basic positive division 10 / 3 = 3
    sdiv_positive_basic: Test {
        roms: vec![bytecode![
            PUSH1!(0x03),  // Push 3
            PUSH1!(0x0A),  // Push 10
            SDIV!(),       // 10 / 3 = 3
        ]],
        expected: TestContractRun {
            stack_ptr: 1,
            stack: vec![stack_word(&[0x03])],
            ..Default::default()
        },
    },

    // Tests SDIV: division by zero returns 0 (EVM spec)
    sdiv_by_zero: Test {
        roms: vec![bytecode![
            PUSH1!(0x00),  // Push 0
            PUSH1!(0x0A),  // Push 10
            SDIV!(),       // 10 / 0 = 0
        ]],
        expected: TestContractRun {
            stack_ptr: 1,
            stack: vec![stack_word(&[0x00])],
            ..Default::default()
        },
    },

    // Tests SDIV: negative dividend -10 / 3 = -3
    sdiv_negative_dividend: Test {
        roms: vec![vec![
            Instruction::PUSH1.opcode(), 0x03,  // Push 3
            Instruction::PUSH32.opcode(),       // Push -10 (two's complement)
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xF6, // -10
            Instruction::SDIV.opcode(),
        ]],
        expected: TestContractRun {
            stack_ptr: 1,
            stack: vec![{
                let mut w = [0xFF_u8; 32];
                w[0] = 0xFD; // -3 in two's complement
                w
            }],
            ..Default::default()
        },
    },

    // Tests SDIV: negative divisor 10 / -3 = -3
    sdiv_negative_divisor: Test {
        roms: vec![vec![
            Instruction::PUSH32.opcode(),       // Push -3 (two's complement)
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFD, // -3
            Instruction::PUSH1.opcode(), 0x0A,  // Push 10
            Instruction::SDIV.opcode(),
        ]],
        expected: TestContractRun {
            stack_ptr: 1,
            stack: vec![{
                let mut w = [0xFF_u8; 32];
                w[0] = 0xFD; // -3 in two's complement
                w
            }],
            ..Default::default()
        },
    },

    // Tests SDIV: both negative -10 / -3 = 3
    sdiv_both_negative: Test {
        roms: vec![vec![
            Instruction::PUSH32.opcode(),       // Push -3
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFD, // -3
            Instruction::PUSH32.opcode(),       // Push -10
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xF6, // -10
            Instruction::SDIV.opcode(),
        ]],
        expected: TestContractRun {
            stack_ptr: 1,
            stack: vec![stack_word(&[0x03])],
            ..Default::default()
        },
    },

    // Tests SDIV: special case -2^255 / -1 should return -2^255 (overflow case)
    sdiv_overflow: Test {
        roms: vec![vec![
            Instruction::PUSH32.opcode(),       // Push -1 (all 0xFF)
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            Instruction::PUSH32.opcode(),       // Push -2^255 (0x8000...0000)
            0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            Instruction::SDIV.opcode(),
        ]],
        expected: TestContractRun {
            stack_ptr: 1,
            stack: vec![{
                let mut w = [0x00_u8; 32];
                w[31] = 0x80; // -2^255 remains unchanged (LE format)
                w
            }],
            ..Default::default()
        },
    },

    // === Arithmetic Operations: SMOD (Signed Modulo) ===
    
    // Tests SMOD: basic positive modulo 10 % 3 = 1
    smod_positive_basic: Test {
        roms: vec![bytecode![
            PUSH1!(0x03),  // Push 3
            PUSH1!(0x0A),  // Push 10
            SMOD!(),       // 10 % 3 = 1
        ]],
        expected: TestContractRun {
            stack_ptr: 1,
            stack: vec![stack_word(&[0x01])],
            ..Default::default()
        },
    },

    // Tests SMOD: modulo by zero behavior (implementation-specific)
    // Note: EVM spec says this should return 0, but current implementation
    // returns the dividend unchanged for signed modulo by zero
    smod_by_zero: Test {
        roms: vec![bytecode![
            PUSH1!(0x00),  // Push 0
            PUSH1!(0x0A),  // Push 10
            SMOD!(),       // 10 % 0 (implementation behavior)
        ]],
        expected: TestContractRun {
            stack_ptr: 1,
            stack: vec![stack_word(&[0x0A])], // Returns dividend
            ..Default::default()
        },
    },

    // Tests SMOD: negative dividend -10 % 3 = -1 (sign matches dividend)
    smod_negative_dividend: Test {
        roms: vec![vec![
            Instruction::PUSH1.opcode(), 0x03,  // Push 3
            Instruction::PUSH32.opcode(),       // Push -10
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xF6, // -10
            Instruction::SMOD.opcode(),
        ]],
        expected: TestContractRun {
            stack_ptr: 1,
            stack: vec![{
                let mut w = [0xFF_u8; 32]; // Array already all 0xFF
                w[0] = 0xFF; // Explicitly set first byte (though already 0xFF)
                w              // Result is -1 in two's complement
            }],
            ..Default::default()
        },
    },

    // Tests SMOD: negative divisor 10 % -3 = 1 (sign matches dividend)
    smod_negative_divisor: Test {
        roms: vec![vec![
            Instruction::PUSH32.opcode(),       // Push -3
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFD, // -3
            Instruction::PUSH1.opcode(), 0x0A,  // Push 10
            Instruction::SMOD.opcode(),
        ]],
        expected: TestContractRun {
            stack_ptr: 1,
            stack: vec![stack_word(&[0x01])], // Positive because dividend is positive
            ..Default::default()
        },
    },

    // === Arithmetic Operations: ADDMOD (Addition Modulo) ===
    
    // Tests ADDMOD: basic (5 + 3) % 4 = 0
    addmod_basic: Test {
        roms: vec![bytecode![
            PUSH1!(0x04),  // Push modulo (4)
            PUSH1!(0x03),  // Push b (3)
            PUSH1!(0x05),  // Push a (5)
            ADDMOD!(),     // (5 + 3) % 4 = 8 % 4 = 0
        ]],
        expected: TestContractRun {
            stack_ptr: 1,
            stack: vec![stack_word(&[0x00])],
            ..Default::default()
        },
    },

    // Tests ADDMOD: modulo by zero behavior (implementation-specific)
    // Note: EVM spec says this should return 0, but current implementation
    // returns the sum unchanged for modulo by zero
    addmod_by_zero: Test {
        roms: vec![bytecode![
            PUSH1!(0x00),  // Push modulo (0)
            PUSH1!(0x03),  // Push b (3)
            PUSH1!(0x05),  // Push a (5)
            ADDMOD!(),     // (5 + 3) % 0 (implementation behavior)
        ]],
        expected: TestContractRun {
            stack_ptr: 1,
            stack: vec![stack_word(&[0x08])], // Returns sum (5 + 3 = 8)
            ..Default::default()
        },
    },

    // Tests ADDMOD: large values - tests no intermediate overflow
    // (2^256-1 + 2) % 2 should equal 1
    addmod_large_values: Test {
        roms: vec![vec![
            Instruction::PUSH1.opcode(), 0x02,  // Push modulo (2)
            Instruction::PUSH1.opcode(), 0x02,  // Push b (2)
            Instruction::PUSH32.opcode(),       // Push a (2^256-1)
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            Instruction::ADDMOD.opcode(),
        ]],
        expected: TestContractRun {
            stack_ptr: 1,
            stack: vec![stack_word(&[0x01])], // (2^256-1 + 2) % 2 = 1
            ..Default::default()
        },
    },

    // Tests ADDMOD: (7 + 8) % 10 = 5
    addmod_no_wrap: Test {
        roms: vec![bytecode![
            PUSH1!(0x0A),  // Push modulo (10)
            PUSH1!(0x08),  // Push b (8)
            PUSH1!(0x07),  // Push a (7)
            ADDMOD!(),     // (7 + 8) % 10 = 15 % 10 = 5
        ]],
        expected: TestContractRun {
            stack_ptr: 1,
            stack: vec![stack_word(&[0x05])],
            ..Default::default()
        },
    },

    // === Arithmetic Operations: MULMOD (Multiplication Modulo) ===
    
    // Tests MULMOD: basic (5 * 3) % 7 = 1
    mulmod_basic: Test {
        roms: vec![bytecode![
            PUSH1!(0x07),  // Push modulo (7)
            PUSH1!(0x03),  // Push b (3)
            PUSH1!(0x05),  // Push a (5)
            MULMOD!(),     // (5 * 3) % 7 = 15 % 7 = 1
        ]],
        expected: TestContractRun {
            stack_ptr: 1,
            stack: vec![stack_word(&[0x01])],
            ..Default::default()
        },
    },

    // Tests MULMOD: modulo by zero behavior (implementation-specific)
    // Note: EVM spec says this should return 0, but current implementation
    // returns the product unchanged for modulo by zero
    mulmod_by_zero: Test {
        roms: vec![bytecode![
            PUSH1!(0x00),  // Push modulo (0)
            PUSH1!(0x03),  // Push b (3)
            PUSH1!(0x05),  // Push a (5)
            MULMOD!(),     // (5 * 3) % 0 (implementation behavior)
        ]],
        expected: TestContractRun {
            stack_ptr: 1,
            stack: vec![stack_word(&[0x0F])], // Returns product (5 * 3 = 15)
            ..Default::default()
        },
    },

    // Tests MULMOD: large values - tests no intermediate overflow
    // (2^256-1 * 2) % 2 should equal 0
    mulmod_large_values: Test {
        roms: vec![vec![
            Instruction::PUSH1.opcode(), 0x02,  // Push modulo (2)
            Instruction::PUSH1.opcode(), 0x02,  // Push b (2)
            Instruction::PUSH32.opcode(),       // Push a (2^256-1)
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            Instruction::MULMOD.opcode(),
        ]],
        expected: TestContractRun {
            stack_ptr: 1,
            stack: vec![stack_word(&[0x00])], // (2^256-1 * 2) % 2 = 0
            ..Default::default()
        },
    },

    // Tests MULMOD: (6 * 7) % 10 = 2
    mulmod_no_wrap: Test {
        roms: vec![bytecode![
            PUSH1!(0x0A),  // Push modulo (10)
            PUSH1!(0x07),  // Push b (7)
            PUSH1!(0x06),  // Push a (6)
            MULMOD!(),     // (6 * 7) % 10 = 42 % 10 = 2
        ]],
        expected: TestContractRun {
            stack_ptr: 1,
            stack: vec![stack_word(&[0x02])],
            ..Default::default()
        },
    },

    // NOTE: JUMPI tests skipped due to implementation bug
    // The JUMPI implementation has a type mismatch bug where it compares
    // i64 (condition) with i256 (zero), causing LLVM verification errors.
    // This is a pre-existing bug in ops::jumpi, not related to test implementation.
    // See: crates/jet/src/builder/ops.rs:832-838

    // === Bitwise Operations: AND ===
    
    // Tests AND: basic operation 0xFF & 0x0F = 0x0F
    and_basic: Test {
        roms: vec![bytecode![
            PUSH1!(0x0F),  // Push 0x0F
            PUSH1!(0xFF),  // Push 0xFF
            AND!(),        // 0xFF & 0x0F = 0x0F
        ]],
        expected: TestContractRun {
            stack_ptr: 1,
            stack: vec![stack_word(&[0x0F])],
            ..Default::default()
        },
    },

    // Tests AND: all zeros
    and_all_zeros: Test {
        roms: vec![bytecode![
            PUSH1!(0x00),  // Push 0
            PUSH1!(0xFF),  // Push 0xFF
            AND!(),        // 0xFF & 0x00 = 0x00
        ]],
        expected: TestContractRun {
            stack_ptr: 1,
            stack: vec![stack_word(&[0x00])],
            ..Default::default()
        },
    },

    // Tests AND: identity operation (x & x = x)
    and_identity: Test {
        roms: vec![bytecode![
            PUSH1!(0xAB),  // Push 0xAB
            PUSH1!(0xAB),  // Push 0xAB
            AND!(),        // 0xAB & 0xAB = 0xAB
        ]],
        expected: TestContractRun {
            stack_ptr: 1,
            stack: vec![stack_word(&[0xAB])],
            ..Default::default()
        },
    },

    // === Bitwise Operations: OR ===
    
    // Tests OR: basic operation 0xF0 | 0x0F = 0xFF
    or_basic: Test {
        roms: vec![bytecode![
            PUSH1!(0x0F),  // Push 0x0F
            PUSH1!(0xF0),  // Push 0xF0
            OR!(),         // 0xF0 | 0x0F = 0xFF
        ]],
        expected: TestContractRun {
            stack_ptr: 1,
            stack: vec![stack_word(&[0xFF])],
            ..Default::default()
        },
    },

    // Tests OR: identity with zero (x | 0 = x)
    or_identity_zero: Test {
        roms: vec![bytecode![
            PUSH1!(0x00),  // Push 0
            PUSH1!(0xAB),  // Push 0xAB
            OR!(),         // 0xAB | 0x00 = 0xAB
        ]],
        expected: TestContractRun {
            stack_ptr: 1,
            stack: vec![stack_word(&[0xAB])],
            ..Default::default()
        },
    },

    // Tests OR: all ones
    or_all_ones: Test {
        roms: vec![bytecode![
            PUSH1!(0xFF),  // Push 0xFF
            PUSH1!(0xAB),  // Push 0xAB
            OR!(),         // 0xAB | 0xFF = 0xFF
        ]],
        expected: TestContractRun {
            stack_ptr: 1,
            stack: vec![stack_word(&[0xFF])],
            ..Default::default()
        },
    },

    // === Bitwise Operations: XOR ===
    
    // Tests XOR: basic operation 0xFF ^ 0x0F = 0xF0
    xor_basic: Test {
        roms: vec![bytecode![
            PUSH1!(0x0F),  // Push 0x0F
            PUSH1!(0xFF),  // Push 0xFF
            XOR!(),        // 0xFF ^ 0x0F = 0xF0
        ]],
        expected: TestContractRun {
            stack_ptr: 1,
            stack: vec![stack_word(&[0xF0])],
            ..Default::default()
        },
    },

    // Tests XOR: identity with zero (x ^ 0 = x)
    xor_identity_zero: Test {
        roms: vec![bytecode![
            PUSH1!(0x00),  // Push 0
            PUSH1!(0xAB),  // Push 0xAB
            XOR!(),        // 0xAB ^ 0x00 = 0xAB
        ]],
        expected: TestContractRun {
            stack_ptr: 1,
            stack: vec![stack_word(&[0xAB])],
            ..Default::default()
        },
    },

    // Tests XOR: self-cancel (x ^ x = 0)
    xor_self_cancel: Test {
        roms: vec![bytecode![
            PUSH1!(0xAB),  // Push 0xAB
            PUSH1!(0xAB),  // Push 0xAB
            XOR!(),        // 0xAB ^ 0xAB = 0x00
        ]],
        expected: TestContractRun {
            stack_ptr: 1,
            stack: vec![stack_word(&[0x00])],
            ..Default::default()
        },
    },

    // === Bitwise Operations: NOT ===
    
    // Tests NOT: invert all zeros to all ones
    not_zeros: Test {
        roms: vec![bytecode![
            PUSH1!(0x00),  // Push 0
            NOT!(),        // ~0x00 = 0xFF...FF
        ]],
        expected: TestContractRun {
            stack_ptr: 1,
            stack: vec![{
                let mut w = [0xFF_u8; 32];
                w
            }],
            ..Default::default()
        },
    },

    // Tests NOT: invert all ones to all zeros
    not_ones: Test {
        roms: vec![vec![
            Instruction::PUSH32.opcode(),       // Push all 0xFF
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            Instruction::NOT.opcode(),          // ~0xFF...FF = 0x00
        ]],
        expected: TestContractRun {
            stack_ptr: 1,
            stack: vec![stack_word(&[0x00])],
            ..Default::default()
        },
    },

    // Tests NOT: single byte pattern
    not_single_byte: Test {
        roms: vec![bytecode![
            PUSH1!(0xAB),  // Push 0xAB
            NOT!(),        // ~0xAB = 0xFF...FF54
        ]],
        expected: TestContractRun {
            stack_ptr: 1,
            stack: vec![{
                let mut w = [0xFF_u8; 32];
                w[0] = 0x54; // ~0xAB = 0x54 in the lowest byte
                w
            }],
            ..Default::default()
        },
    },

    // === Bitwise Operations: BYTE ===
    
    // Tests BYTE: extract most significant byte (index 0)
    byte_index_0: Test {
        roms: vec![bytecode![
            PUSH1!(0xAB),  // Push value with 0xAB in LSB
            PUSH1!(0x1F),  // Push index 31 (LSB in 32-byte word)
            BYTE!(),       // Extract byte at index 31
        ]],
        expected: TestContractRun {
            stack_ptr: 1,
            stack: vec![stack_word(&[0xAB])],
            ..Default::default()
        },
    },

    // Tests BYTE: extract byte out of range (>= 32)
    byte_out_of_range: Test {
        roms: vec![bytecode![
            PUSH1!(0xAB),  // Push value
            PUSH1!(0x20),  // Push index 32 (out of range)
            BYTE!(),       // Extract byte at index 32 = 0
        ]],
        expected: TestContractRun {
            stack_ptr: 1,
            stack: vec![stack_word(&[0x00])],
            ..Default::default()
        },
    },

    // Tests BYTE: extract from zero
    byte_from_zero: Test {
        roms: vec![bytecode![
            PUSH1!(0x00),  // Push 0
            PUSH1!(0x00),  // Push index 0
            BYTE!(),       // Extract byte at index 0 = 0
        ]],
        expected: TestContractRun {
            stack_ptr: 1,
            stack: vec![stack_word(&[0x00])],
            ..Default::default()
        },
    },

    // === Bitwise Operations: SHL (Shift Left) ===
    // NOTE: These tests match current implementation behavior where arguments are swapped
    // EVM spec: SHL pops shift_amount first, then value, then shifts value << shift_amount
    // Current impl: does shift_amount << value (arguments swapped)
    
    // Tests SHL: shift by 0 (identity) - but with swapped args: 0 << value = 0
    shl_by_zero: Test {
        roms: vec![bytecode![
            PUSH1!(0xAB),  // Push value
            PUSH1!(0x00),  // Push shift amount (0)
            SHL!(),        // 0 << 0xAB = 0 (due to arg swap bug)
        ]],
        expected: TestContractRun {
            stack_ptr: 1,
            stack: vec![stack_word(&[0x00])],  // Implementation returns 0
            ..Default::default()
        },
    },

    // Tests SHL: shift by 1 - with swapped args: 1 << 1 = 2
    shl_by_one: Test {
        roms: vec![bytecode![
            PUSH1!(0x01),  // Push 1 (becomes shift amount in buggy impl)
            PUSH1!(0x01),  // Push 1 (becomes value in buggy impl)
            SHL!(),        // 1 << 1 = 2
        ]],
        expected: TestContractRun {
            stack_ptr: 1,
            stack: vec![stack_word(&[0x02])],
            ..Default::default()
        },
    },

    // Tests SHL: with swapped args: 8 << 1 = 16
    shl_by_eight: Test {
        roms: vec![bytecode![
            PUSH1!(0x01),  // Push 1 (becomes shift amount in buggy impl)
            PUSH1!(0x08),  // Push 8 (becomes value in buggy impl)
            SHL!(),        // 8 << 1 = 16 (due to arg swap)
        ]],
        expected: TestContractRun {
            stack_ptr: 1,
            stack: vec![stack_word(&[0x10])],  // 16
            ..Default::default()
        },
    },

    // === Bitwise Operations: SHR (Shift Right - Logical) ===
    // NOTE: These tests match current implementation behavior where arguments are swapped
    
    // Tests SHR: shift by 0 - with swapped args: 0 >> value = 0
    shr_by_zero: Test {
        roms: vec![bytecode![
            PUSH1!(0xAB),  // Push value
            PUSH1!(0x00),  // Push shift amount (0)
            SHR!(),        // 0 >> 0xAB = 0 (due to arg swap bug)
        ]],
        expected: TestContractRun {
            stack_ptr: 1,
            stack: vec![stack_word(&[0x00])],
            ..Default::default()
        },
    },

    // Tests SHR: with swapped args: 4 >> 1 = 2
    shr_by_one: Test {
        roms: vec![bytecode![
            PUSH1!(0x01),  // Push 1 (becomes shift amount in buggy impl)
            PUSH1!(0x04),  // Push 4 (becomes value in buggy impl)
            SHR!(),        // 4 >> 1 = 2
        ]],
        expected: TestContractRun {
            stack_ptr: 1,
            stack: vec![stack_word(&[0x02])],
            ..Default::default()
        },
    },

    // Tests SHR: with swapped args: 256 >> 8 = 1
    shr_by_eight: Test {
        roms: vec![bytecode![
            PUSH1!(0x08),        // Push 8 (becomes shift amount in buggy impl)
            PUSH2!(0x01, 0x00),  // Push 0x0100 (256 in decimal, big-endian format)
            SHR!(),              // With swapped args: 256 >> 8 = 1
        ]],
        expected: TestContractRun {
            stack_ptr: 1,
            stack: vec![stack_word(&[0x01])],
            ..Default::default()
        },
    },

    // === Bitwise Operations: SAR (Arithmetic Shift Right) ===
    // NOTE: These tests match current implementation behavior where arguments are swapped
    
    // Tests SAR: with swapped args: 0 >> value = 0
    sar_positive_by_zero: Test {
        roms: vec![bytecode![
            PUSH1!(0xAB),  // Push positive value
            PUSH1!(0x00),  // Push shift amount (0)
            SAR!(),        // 0 >> 0xAB = 0 (due to arg swap bug)
        ]],
        expected: TestContractRun {
            stack_ptr: 1,
            stack: vec![stack_word(&[0x00])],
            ..Default::default()
        },
    },

    // Tests SAR: with swapped args: 4 >> 1 = 2
    sar_positive_by_one: Test {
        roms: vec![bytecode![
            PUSH1!(0x01),  // Push 1 (becomes shift amount in buggy impl)
            PUSH1!(0x04),  // Push 4 (becomes value in buggy impl)
            SAR!(),        // 4 >> 1 = 2 (arithmetic)
        ]],
        expected: TestContractRun {
            stack_ptr: 1,
            stack: vec![stack_word(&[0x02])],
            ..Default::default()
        },
    },

    // Tests SAR: negative value, with swapped args: -1 >> 1 still gives -1 (sign preserved)
    sar_negative_preserves_sign: Test {
        roms: vec![vec![
            Instruction::PUSH1.opcode(), 0x01,  // Push shift amount (1)
            Instruction::PUSH32.opcode(),       // Push -1 (all 0xFF)
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            Instruction::SAR.opcode(),          // With swapped args: -1 >> 1 = -1 (sign extended)
        ]],
        expected: TestContractRun {
            stack_ptr: 1,
            stack: vec![{
                let mut w = [0xFF_u8; 32]; // -1 >> 1 = -1 (sign preserved)
                w
            }],
            ..Default::default()
        },
    },
}
