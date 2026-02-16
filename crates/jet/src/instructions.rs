macro_rules! instructions {
    ($($name:ident = $value:expr),* $(,)?) => {
        /// A single EVM opcode.
        ///
        /// Each variant is named after the corresponding mnemonic defined in the
        /// Ethereum Yellow Paper and carries its opcode byte as its discriminant.
        ///
        /// # Examples
        ///
        /// ```
        /// use jet::instructions::Instruction;
        ///
        /// assert_eq!(Instruction::ADD.opcode(), 0x01);
        /// assert_eq!(Instruction::PUSH1.opcode(), 0x60);
        /// assert!(Instruction::PUSH32.is_push());
        /// assert!(!Instruction::ADD.is_push());
        /// ```
        #[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Eq, Ord)]
        #[repr(u8)]
        pub enum Instruction {
            $($name = $value),*,
        }

        impl std::convert::TryFrom<u8> for Instruction {
            type Error = &'static str;

            fn try_from(value: u8) -> Result<Self, Self::Error> {
                match value {
                    $($value => Ok(Instruction::$name),)*
                    _ => Err("Invalid opcode"),
                }
            }
        }

        impl std::fmt::Display for Instruction {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(match self {
                    $(Instruction::$name => stringify!($name),)*
                })
            }
        }

        impl Instruction {
            /// Returns the opcode byte for this instruction.
            ///
            /// # Examples
            ///
            /// ```
            /// use jet::instructions::Instruction;
            ///
            /// assert_eq!(Instruction::STOP.opcode(), 0x00);
            /// assert_eq!(Instruction::ADD.opcode(),  0x01);
            /// ```
            #[inline]
            pub const fn opcode(self) -> u8 {
                self as u8
            }

            /// Returns `true` if this instruction is a `PUSH` variant (`PUSH0`–`PUSH32`).
            ///
            /// # Examples
            ///
            /// ```
            /// use jet::instructions::Instruction;
            ///
            /// assert!(Instruction::PUSH0.is_push());
            /// assert!(Instruction::PUSH32.is_push());
            /// assert!(!Instruction::ADD.is_push());
            /// ```
            #[inline]
            pub const fn is_push(self) -> bool {
                let op = self.opcode();
                op >= 0x5f && op <= 0x7f // PUSH0..PUSH32
            }

            /// Returns the number of immediate data bytes that follow a `PUSH` instruction.
            ///
            /// Returns `0` for all non-`PUSH` instructions. For `PUSH0` this is also
            /// `0` because `PUSH0` has no immediate operand.
            ///
            /// # Examples
            ///
            /// ```
            /// use jet::instructions::Instruction;
            ///
            /// assert_eq!(Instruction::PUSH0.push_len(),  0);
            /// assert_eq!(Instruction::PUSH1.push_len(),  1);
            /// assert_eq!(Instruction::PUSH32.push_len(), 32);
            /// assert_eq!(Instruction::ADD.push_len(),    0);
            /// ```
            #[inline]
            pub const fn push_len(self) -> usize {
                if self.is_push() {
                    (self.opcode() - 0x5f) as usize
                } else {
                    0
                }
            }
        }
    };
}

instructions! {
    STOP = 0x00,
    ADD = 0x01,
    MUL = 0x02,
    SUB = 0x03,
    DIV = 0x04,
    SDIV = 0x05,
    MOD = 0x06,
    SMOD = 0x07,
    ADDMOD = 0x08,
    MULMOD = 0x09,
    EXP = 0x0A,
    SIGNEXTEND = 0x0B,

    LT = 0x10,
    GT = 0x11,
    SLT = 0x12,
    SGT = 0x13,
    EQ = 0x14,
    ISZERO = 0x15,
    AND = 0x16,
    OR = 0x17,
    XOR = 0x18,
    NOT = 0x19,
    BYTE = 0x1A,
    SHL = 0x1B,
    SHR = 0x1C,
    SAR = 0x1D,

    KECCAK256 = 0x20,

    ADDRESS = 0x30,
    BALANCE = 0x31,
    ORIGIN = 0x32,
    CALLER = 0x33,
    CALLVALUE = 0x34,
    CALLDATALOAD = 0x35,
    CALLDATASIZE = 0x36,
    CALLDATACOPY = 0x37,
    CODESIZE = 0x38,
    CODECOPY = 0x39,
    GASPRICE = 0x3A,
    EXTCODESIZE = 0x3B,
    EXTCODECOPY = 0x3C,
    RETURNDATASIZE = 0x3D,
    RETURNDATACOPY = 0x3E,
    EXTCODEHASH = 0x3F,

    BLOCKHASH = 0x40,
    COINBASE = 0x41,
    TIMESTAMP = 0x42,
    NUMBER = 0x43,
    DIFFICULTY = 0x44,
    GASLIMIT = 0x45,
    CHAINID = 0x46,
    SELFBALANCE = 0x47,
    BASEFEE= 0x48,
    BLOBHASH = 0x49,
    BLOBBASEFEE = 0x4A,

    POP = 0x50,
    MLOAD = 0x51,
    MSTORE = 0x52,
    MSTORE8 = 0x53,
    SLOAD = 0x54,
    SSTORE = 0x55,
    JUMP = 0x56,
    JUMPI = 0x57,
    PC = 0x58,
    MSIZE = 0x59,
    GAS = 0x5A,
    JUMPDEST = 0x5B,
    TLOAD = 0x5C,
    TSTORE = 0x5D,
    MCOPY = 0x5E,
    PUSH0 = 0x5F,
    PUSH1 = 0x60,
    PUSH2 = 0x61,
    PUSH3 = 0x62,
    PUSH4 = 0x63,
    PUSH5 = 0x64,
    PUSH6 = 0x65,
    PUSH7 = 0x66,
    PUSH8 = 0x67,
    PUSH9 = 0x68,
    PUSH10 = 0x69,
    PUSH11 = 0x6A,
    PUSH12 = 0x6B,
    PUSH13 = 0x6C,
    PUSH14 = 0x6D,
    PUSH15 = 0x6E,
    PUSH16 = 0x6F,
    PUSH17 = 0x70,
    PUSH18 = 0x71,
    PUSH19 = 0x72,
    PUSH20 = 0x73,
    PUSH21 = 0x74,
    PUSH22 = 0x75,
    PUSH23 = 0x76,
    PUSH24 = 0x77,
    PUSH25 = 0x78,
    PUSH26 = 0x79,
    PUSH27 = 0x7A,
    PUSH28 = 0x7B,
    PUSH29 = 0x7C,
    PUSH30 = 0x7D,
    PUSH31 = 0x7E,
    PUSH32 = 0x7F,
    DUP1 = 0x80,
    DUP2 = 0x81,
    DUP3 = 0x82,
    DUP4 = 0x83,
    DUP5 = 0x84,
    DUP6 = 0x85,
    DUP7 = 0x86,
    DUP8 = 0x87,
    DUP9 = 0x88,
    DUP10 = 0x89,
    DUP11 = 0x8A,
    DUP12 = 0x8B,
    DUP13 = 0x8C,
    DUP14 = 0x8D,
    DUP15 = 0x8E,
    DUP16 = 0x8F,
    SWAP1 = 0x90,
    SWAP2 = 0x91,
    SWAP3 = 0x92,
    SWAP4 = 0x93,
    SWAP5 = 0x94,
    SWAP6 = 0x95,
    SWAP7 = 0x96,
    SWAP8 = 0x97,
    SWAP9 = 0x98,
    SWAP10 = 0x99,
    SWAP11 = 0x9A,
    SWAP12 = 0x9B,
    SWAP13 = 0x9C,
    SWAP14 = 0x9D,
    SWAP15 = 0x9E,
    SWAP16 = 0x9F,
    LOG0 = 0xA0,
    LOG1 = 0xA1,
    LOG2 = 0xA2,
    LOG3 = 0xA3,
    LOG4 = 0xA4,
    CREATE = 0xF0,
    CALL = 0xF1,
    CALLCODE = 0xF2,
    RETURN = 0xF3,
    DELEGATECALL = 0xF4,
    CREATE2 = 0xF5,
    STATICCALL = 0xFA,
    REVERT = 0xFD,
    INVALID = 0xFE,
    SELFDESTRUCT = 0xFF,
}

/// A forward-only iterator over an EVM bytecode sequence.
///
/// `Iter` scans a raw byte slice and yields one [`IterItem`] per logical
/// instruction, automatically skipping over `PUSH` immediate data so that the
/// program counter always advances to the next opcode.
///
/// # Examples
///
/// ```
/// use jet::instructions::{Instruction, Iter, IterItem};
///
/// // PUSH1 0x42  STOP
/// let rom: &[u8] = &[0x60, 0x42, 0x00];
/// let items: Vec<_> = Iter::new(rom).collect();
///
/// assert!(matches!(items[0], IterItem::PushData(0, Instruction::PUSH1, data) if data == &[0x42]));
/// assert!(matches!(items[1], IterItem::Instr(2, Instruction::STOP)));
/// ```
pub struct Iter<'a> {
    pc: usize,
    rom: &'a [u8],
}

impl<'a> Iter<'a> {
    /// Creates a new iterator over the given EVM bytecode slice.
    ///
    /// # Examples
    ///
    /// ```
    /// use jet::instructions::Iter;
    ///
    /// let iter = Iter::new(&[0x00]); // STOP
    /// assert_eq!(iter.count(), 1);
    /// ```
    pub const fn new(rom: &'a [u8]) -> Self {
        Self { pc: 0, rom }
    }
}

/// An item yielded by [`Iter`] when iterating over EVM bytecode.
///
/// Each variant carries the program counter (`pc`) of the first byte of the
/// instruction as its first field.
pub enum IterItem<'a> {
    /// A normal (non-`PUSH`) instruction at the given `pc`.
    Instr(usize, Instruction),
    /// A `PUSH` instruction at the given `pc`, along with its immediate data.
    ///
    /// The data slice may be shorter than expected when the bytecode is
    /// truncated (e.g. the last instruction in the ROM).
    PushData(usize, Instruction, &'a [u8]),
    /// An unrecognised opcode byte at the given `pc`.
    Invalid(usize),
}

impl<'a> std::iter::Iterator for Iter<'a> {
    type Item = IterItem<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pc >= self.rom.len() {
            return None;
        }

        let pc = self.pc;
        let instr = match Instruction::try_from(self.rom[pc]) {
            Ok(instr) => instr,
            Err(_) => {
                self.pc += 1;
                return Some(IterItem::Invalid(pc));
            }
        };

        if !instr.is_push() {
            self.pc += 1;
            return Some(IterItem::Instr(pc, instr));
        };

        let push_len = instr.push_len();
        let push_start = pc + 1;
        let push_end = std::cmp::min(push_start + push_len, self.rom.len());

        self.pc = push_end;
        Some(IterItem::PushData(
            pc,
            instr,
            &self.rom[push_start..push_end],
        ))
    }
}
