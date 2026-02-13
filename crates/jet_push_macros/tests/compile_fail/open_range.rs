use jet_push_macros::generate_push_macros;

enum Instruction {
    PUSH0,
}
impl Instruction {
    fn opcode(&self) -> u8 {
        0x5F
    }
}

fn main() {
    // Should error: must be closed range (..=), not open range (..)
    generate_push_macros!(0..32);
}
