use jet_push_macros::generate_push_macros;

enum Instruction {
    PUSH0,
    PUSH1,
    PUSH2,
    PUSH3,
    PUSH4,
    PUSH5,
    PUSH6,
    PUSH7,
    PUSH8,
    PUSH9,
    PUSH10,
}
impl Instruction {
    fn opcode(&self) -> u8 {
        0x5F
    }
}

fn main() {
    // Should error: end must be >= start
    generate_push_macros!(10..=5);
}
