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
    // Should error: string literal is not an integer
    generate_push_macros!("hello");
}
