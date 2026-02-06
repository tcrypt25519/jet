use std::cell::{Ref, RefCell, RefMut};

use inkwell::{
    basic_block::BasicBlock,
    values::{FunctionValue, IntValue},
};
use log::{info, trace};

use jet_runtime::exec::ReturnCode;

use crate::{
    builder::{env::Env, ops, Error, InvalidOpcode},
    instructions,
    instructions::{Instruction, IteratorItem},
};

const VSTACK_INIT_SIZE: usize = 32;

pub(crate) struct Registers<'ctx> {
    // Function parameters
    pub(crate) exec_ctx: inkwell::values::PointerValue<'ctx>,
    pub(crate) block_info: inkwell::values::PointerValue<'ctx>,

    // Pointers into the exec context
    pub(crate) jump_ptr: inkwell::values::PointerValue<'ctx>,
    pub(crate) return_offset: inkwell::values::PointerValue<'ctx>,
    pub(crate) return_length: inkwell::values::PointerValue<'ctx>,
    pub(crate) sub_call: inkwell::values::PointerValue<'ctx>,
}

impl<'ctx> Registers<'ctx> {
    pub fn new(
        env: &Env<'ctx>,
        builder: &inkwell::builder::Builder<'ctx>,
        func: FunctionValue<'ctx>,
    ) -> Self {
        let t = env.types();
        let exec_ctx = func.get_nth_param(0).unwrap().into_pointer_value();
        let block_info = func.get_nth_param(1).unwrap().into_pointer_value();

        let jump_ptr = builder
            .build_struct_gep(t.exec_ctx, exec_ctx, 1, "jump_ptr")
            .unwrap();
        let return_offset = builder
            .build_struct_gep(t.exec_ctx, exec_ctx, 2, "return_offset")
            .unwrap();
        let return_length = builder
            .build_struct_gep(t.exec_ctx, exec_ctx, 3, "return_length")
            .unwrap();
        let sub_call = builder
            .build_struct_gep(t.exec_ctx, exec_ctx, 4, "sub_call")
            .unwrap();

        Self {
            exec_ctx,
            block_info,

            jump_ptr,
            return_offset,
            return_length,
            sub_call,
        }
    }
}

pub(crate) struct BuildCtx<'ctx, 'b> {
    pub(crate) env: &'b Env<'ctx>,
    pub(crate) builder: &'b inkwell::builder::Builder<'ctx>,
    pub(crate) registers: Registers<'ctx>,
    _vstack: RefCell<Vec<IntValue<'ctx>>>,
    func: FunctionValue<'ctx>,
}

impl<'ctx, 'b> BuildCtx<'ctx, 'b> {
    fn new(
        env: &'b Env<'ctx>,
        builder: &'b inkwell::builder::Builder<'ctx>,
        func: FunctionValue<'ctx>,
    ) -> Self {
        let vstack = RefCell::new(Vec::with_capacity(VSTACK_INIT_SIZE));
        Self {
            env,
            builder,
            _vstack: vstack,
            func,
            registers: Registers::new(env, builder, func),
        }
    }

    pub(crate) fn _vstack(&self) -> Ref<'_, Vec<IntValue<'ctx>>> {
        self._vstack.borrow()
    }

    pub(crate) fn _vstack_mut(&self) -> RefMut<'_, Vec<IntValue<'ctx>>> {
        self._vstack.borrow_mut()
    }
}

#[derive(Debug)]
struct CodeBlock<'ctx, 'b> {
    offset: usize,
    rom: &'b [u8],
    basic_block: BasicBlock<'ctx>,
    is_jumpdest: bool,
    terminates: bool,
}

impl CodeBlock<'_, '_> {
    pub(crate) fn is_jumpdest(&self) -> bool {
        self.is_jumpdest
    }

    pub(crate) fn terminates(&self) -> bool {
        self.terminates
    }

    pub(crate) fn set_is_jumpdest(&mut self) {
        self.is_jumpdest = true;
    }

    pub(crate) fn set_terminates(&mut self) {
        self.terminates = true;
    }
}

struct CodeBlocks<'ctx, 'b> {
    blocks: Vec<CodeBlock<'ctx, 'b>>,
}

impl<'ctx, 'b> CodeBlocks<'ctx, 'b> {
    pub(crate) fn new() -> Self {
        Self { blocks: Vec::new() }
    }

    pub(crate) fn add(
        &mut self,
        offset: usize,
        basic_block: BasicBlock<'ctx>,
        // bb_ctor: fn(usize) -> BasicBlock<'ctx>,
    ) -> Result<&mut CodeBlock<'ctx, 'b>, Error> {
        self.blocks.push(CodeBlock {
            offset,
            rom: &[],
            basic_block,
            is_jumpdest: false,
            terminates: false,
        });
        self.blocks
            .last_mut()
            .ok_or_else(|| Error::InvariantViolation("CodeBlocks::add: no block after push".to_string()))
    }

    pub(crate) fn len(&self) -> usize {
        self.blocks.len()
    }

    pub(crate) fn first(&self) -> Option<&CodeBlock<'ctx, 'b>> {
        self.blocks.first()
    }

    pub(crate) fn iter(&self) -> std::slice::Iter<CodeBlock<'ctx, 'b>> {
        self.blocks.iter()
    }

    pub(crate) fn has_jumpdest(&self) -> bool {
        self.blocks.iter().any(|b| b.is_jumpdest)
    }
}

pub fn build(env: &'_ Env<'_>, name: &str, rom: &[u8]) -> Result<(), Error> {
    let builder = env.context().create_builder();

    // Declare the function in the module
    let func_type = env.types().contract_fn;
    let func = env.module().add_function(name, func_type, None);
    info!(
        "Created function {} in module {}",
        name,
        env.module().get_name().to_str().unwrap_or("<invalid UTF-8>")
    );

    // Create the preamble block
    let preamble_block = env.context().append_basic_block(func, "preamble");
    builder.position_at_end(preamble_block);

    // Build ROM into IR
    let bctx = BuildCtx::new(env, &builder, func);
    let code_blocks = find_code_blocks(env, func, rom)?;
    build_contract_body(&bctx, &code_blocks)?;

    // Connect the preamble block to the entry block
    let entry_block = code_blocks.first()
        .ok_or_else(|| Error::InvariantViolation("No code blocks found".to_string()))?;
    bctx.builder.position_at_end(preamble_block);
    bctx.builder
        .build_unconditional_branch(entry_block.basic_block)?;
    Ok(())
}

fn find_code_blocks<'ctx, 'b>(
    env: &Env<'ctx>,
    func: FunctionValue<'ctx>,
    bytecode: &'b [u8],
) -> Result<CodeBlocks<'ctx, 'b>, Error> {
    trace!("find_code_blocks: Creating code blocks");
    trace!("find_code_blocks: ROM: {:?}", bytecode);

    let create_bb = || env.context().append_basic_block(func, "block");

    let mut blocks = CodeBlocks::new();
    let mut current_block: &mut CodeBlock = blocks.add(0, create_bb())?;
    let mut current_block_starting_pc = 0usize;

    for item in instructions::Iterator::new(bytecode) {
        match item {
            IteratorItem::PushData(pc, data) => {
                trace!("find_code_blocks: Found push data {:?} at PC {}", data, pc);
            }
            IteratorItem::Instr(pc, instr) => {
                trace!(
                    "find_code_blocks: Found instruction {:?} at PC {}",
                    instr, pc
                );
                match instr {
                    // Instructions that terminate a block
                    // When these appear we finish out the current block and mark it as
                    // terminating
                    Instruction::STOP
                    | Instruction::RETURN
                    | Instruction::REVERT
                    | Instruction::JUMP => {
                        trace!("find_code_blocks: Found terminator {}", instr);
                        current_block.rom = &bytecode[current_block_starting_pc..pc + 1];
                        current_block.set_terminates();
                        current_block_starting_pc = pc + 1;
                    }

                    Instruction::JUMPI => {
                        trace!("find_code_blocks: Found JUMPI");
                        current_block.rom = &bytecode[current_block_starting_pc..pc + 1];
                        current_block_starting_pc = pc + 1;
                        current_block = blocks.add(current_block_starting_pc, create_bb())?;
                    }

                    Instruction::JUMPDEST => {
                        trace!("find_code_blocks: Found JUMPDEST");
                        if current_block.rom.is_empty() {
                            current_block.rom = &bytecode[current_block_starting_pc..pc];
                        }

                        current_block_starting_pc = pc + 1;
                        current_block = blocks.add(current_block_starting_pc, create_bb())?;
                        current_block.set_is_jumpdest();
                    }
                    _ => {
                        trace!("find_code_blocks: instr {} is uninteresting", instr);
                    }
                }
            }
            IteratorItem::Invalid(pc) => {
                trace!("find_code_blocks: Found invalid instruction at PC {}", pc);
                return Err(InvalidOpcode {
                    pc,
                    opcode: bytecode[pc],
                }
                .into());
            }
        }
    }

    if current_block.rom.is_empty() {
        trace!(
            "find_code_blocks: Setting code block ROM from {} to end",
            current_block_starting_pc
        );
        current_block.rom = &bytecode[current_block_starting_pc..];
    } else {
        trace!("find_code_blocks: Block has ROM {:?}", current_block.rom);
    }

    trace!("find_code_blocks: Found {} code blocks", blocks.len());
    trace!("find_code_blocks: Bytecode: {:?}", bytecode);
    for block in blocks.iter() {
        trace!("find_code_blocks:   Block at offset {}:", block.offset);
        trace!("find_code_blocks:   {:?}", block.rom);
    }
    Ok(blocks)
}

fn build_contract_body<'ctx, 'b>(
    bctx: &'b BuildCtx<'ctx, 'b>,
    code_blocks: &CodeBlocks<'ctx, 'b>,
) -> Result<(), Error> {
    let t = bctx.env.types();

    let mut jump_cases = Vec::new();

    let jump_block = match code_blocks.has_jumpdest() {
        true => Some(
            bctx.env
                .context()
                .append_basic_block(bctx.func, "jump_block"),
        ),
        false => None,
    };

    // Iterate over the code blocks and interpret the bytecode of each one
    let mut code_blocks_iter = code_blocks.iter().peekable();
    while let Some(code_block) = code_blocks_iter.next() {
        // If this block is a jump destination then add it to the jump table
        if code_block.is_jumpdest() {
            // JUMPDEST code blocks start at the instruction after the JUMPDEST instruction, so
            // we subtract 1, and a given offset of 0 is invalid.
            let mut offset = code_block.offset as u64;
            if offset == 0 {
                return Err(Error::invariant_violation("Jump destination at offset 0"));
            }
            offset -= 1;
            jump_cases.push((t.i32.const_int(offset, false), code_block.basic_block));
        }

        let following_block = code_blocks_iter.peek();

        build_code_block(bctx, code_block, jump_block, following_block)?;

        // If the block terminated due to an instruction, e.g. STOP or RETURN, then it should
        // have taken care of terminating the block and we don't need to do anything else.
        if code_block.terminates() {
            continue;
        }

        // If we have reached the end of the bytecode but have no termination instruction then
        // we will either jump to the next block or return from the function.
        match following_block {
            Some(next_block) => {
                // Sync the vstack with the real stack
                // Terminator instructions will handle the stack themselves
                ops::__sync_vstack(bctx)?;

                bctx.builder
                    .build_unconditional_branch(next_block.basic_block)
                    .unwrap();
                Ok(())
            }
            None => ops::__build_return(bctx, ReturnCode::ImplicitReturn),
        }?;
    }

    // Add jump table to the end of the function
    if let Some(jump_block) = jump_block {
        build_jump_table(bctx, jump_block, jump_cases.as_slice())?;
    }

    Ok(())
}

fn build_code_block(
    bctx: &BuildCtx<'_, '_>,
    code_block: &CodeBlock,
    jump_block: Option<BasicBlock>,
    following_block: Option<&&CodeBlock>,
) -> Result<(), Error> {
    trace!("loop: Building code");
    trace!("loop: Offset: {}", code_block.offset);
    trace!("loop: ROM: {:?}", code_block.rom);

    // Prepare for building the IR for this code block. Move the builder to this basic block
    // and start a relative PC at 0.
    bctx.builder.position_at_end(code_block.basic_block);

    for item in instructions::Iterator::new(code_block.rom) {
        match item {
            IteratorItem::PushData(_, data) => {
                trace!("loop: Data: {:?}", data);
                ops::push(bctx, data)
            }
            IteratorItem::Instr(pc, instr) => {
                trace!("loop: Instruction: {:?}", instr);
                match instr {
                    Instruction::STOP => ops::stop(bctx),

                    // Arithmetic
                    Instruction::ADD => ops::add(bctx),
                    Instruction::MUL => ops::mul(bctx),
                    Instruction::SUB => ops::sub(bctx),
                    Instruction::DIV => ops::div(bctx),
                    Instruction::SDIV => ops::sdiv(bctx),
                    Instruction::MOD => ops::_mod(bctx),
                    Instruction::SMOD => ops::smod(bctx),
                    Instruction::ADDMOD => ops::addmod(bctx),
                    Instruction::MULMOD => ops::mulmod(bctx),
                    Instruction::EXP => ops::exp(bctx),
                    Instruction::SIGNEXTEND => ops::signextend(bctx),

                    // Comparisons
                    Instruction::LT => ops::lt(bctx),
                    Instruction::GT => ops::gt(bctx),
                    Instruction::SLT => ops::slt(bctx),
                    Instruction::SGT => ops::sgt(bctx),
                    Instruction::EQ => ops::eq(bctx),
                    Instruction::ISZERO => ops::iszero(bctx),
                    Instruction::AND => ops::and(bctx),
                    Instruction::OR => ops::or(bctx),
                    Instruction::XOR => ops::xor(bctx),
                    Instruction::NOT => ops::not(bctx),
                    Instruction::BYTE => ops::byte(bctx),
                    Instruction::SHL => ops::shl(bctx),
                    Instruction::SHR => ops::shr(bctx),
                    Instruction::SAR => ops::sar(bctx),

                    // Cryptographic
                    Instruction::KECCAK256 => ops::keccak256(bctx),

                    // Call data
                    Instruction::RETURNDATASIZE => ops::returndatasize(bctx),
                    Instruction::RETURNDATACOPY => ops::returndatacopy(bctx),

                    // Block information
                    Instruction::BLOCKHASH => ops::blockhash(bctx),
                    // Instruction::NUMBER => ops::blockhash(bctx),

                    // Runtime
                    Instruction::POP => ops::pop(bctx),

                    Instruction::MLOAD => ops::mload(bctx),
                    Instruction::MSTORE => ops::mstore(bctx),
                    Instruction::MSTORE8 => ops::mstore8(bctx),

                    Instruction::JUMP => match jump_block {
                        Some(jump_block) => ops::jump(bctx, jump_block),
                        _ => return Err(Error::invariant_violation("JUMP without jump block")),
                    },
                    Instruction::JUMPI => match (jump_block, following_block) {
                        (Some(jump_block), Some(following_block)) => {
                            ops::jumpi(bctx, jump_block, following_block.basic_block)
                        }
                        (Some(_), None) => {
                            return Err(Error::invariant_violation("JUMPI without following block"));
                        }
                        (None, Some(_)) => {
                            return Err(Error::invariant_violation("JUMPI without jump block"));
                        }
                        (None, None) => {
                            return Err(Error::invariant_violation(
                                "JUMPI without jump or following blocks",
                            ));
                        }
                    },

                    Instruction::PC => ops::pc(bctx, code_block.offset + pc),

                    Instruction::CALL => ops::call(bctx),

                    Instruction::RETURN => ops::_return(bctx),
                    Instruction::REVERT => ops::revert(bctx),
                    Instruction::INVALID => ops::invalid(bctx),
                    Instruction::SELFDESTRUCT => ops::selfdestruct(bctx),

                    // Stack manipulation
                    Instruction::DUP1 => ops::dup(bctx, 1),
                    Instruction::DUP2 => ops::dup(bctx, 2),
                    Instruction::DUP3 => ops::dup(bctx, 3),
                    Instruction::DUP4 => ops::dup(bctx, 4),
                    Instruction::DUP5 => ops::dup(bctx, 5),
                    Instruction::DUP6 => ops::dup(bctx, 6),
                    Instruction::DUP7 => ops::dup(bctx, 7),
                    Instruction::DUP8 => ops::dup(bctx, 8),
                    Instruction::DUP9 => ops::dup(bctx, 9),
                    Instruction::DUP10 => ops::dup(bctx, 10),
                    Instruction::DUP11 => ops::dup(bctx, 11),
                    Instruction::DUP12 => ops::dup(bctx, 12),
                    Instruction::DUP13 => ops::dup(bctx, 13),
                    Instruction::DUP14 => ops::dup(bctx, 14),
                    Instruction::DUP15 => ops::dup(bctx, 15),
                    Instruction::DUP16 => ops::dup(bctx, 16),

                    Instruction::SWAP1 => ops::swap(bctx, 1),
                    Instruction::SWAP2 => ops::swap(bctx, 2),
                    Instruction::SWAP3 => ops::swap(bctx, 3),
                    Instruction::SWAP4 => ops::swap(bctx, 4),
                    Instruction::SWAP5 => ops::swap(bctx, 5),
                    Instruction::SWAP6 => ops::swap(bctx, 6),
                    Instruction::SWAP7 => ops::swap(bctx, 7),
                    Instruction::SWAP8 => ops::swap(bctx, 8),
                    Instruction::SWAP9 => ops::swap(bctx, 9),
                    Instruction::SWAP10 => ops::swap(bctx, 10),
                    Instruction::SWAP11 => ops::swap(bctx, 11),
                    Instruction::SWAP12 => ops::swap(bctx, 12),
                    Instruction::SWAP13 => ops::swap(bctx, 13),
                    Instruction::SWAP14 => ops::swap(bctx, 14),
                    Instruction::SWAP15 => ops::swap(bctx, 15),
                    Instruction::SWAP16 => ops::swap(bctx, 16),

                    // Not yet implemented
                    Instruction::ADDRESS => {
                        Err(Error::UnimplementedInstruction(Instruction::ADDRESS))
                    }
                    Instruction::BALANCE => {
                        Err(Error::UnimplementedInstruction(Instruction::BALANCE))
                    }
                    Instruction::ORIGIN => {
                        Err(Error::UnimplementedInstruction(Instruction::ORIGIN))
                    }
                    Instruction::CALLER => {
                        Err(Error::UnimplementedInstruction(Instruction::CALLER))
                    }
                    Instruction::CALLVALUE => {
                        Err(Error::UnimplementedInstruction(Instruction::CALLVALUE))
                    }
                    Instruction::CALLDATALOAD => {
                        Err(Error::UnimplementedInstruction(Instruction::CALLDATALOAD))
                    }
                    Instruction::CALLDATASIZE => {
                        Err(Error::UnimplementedInstruction(Instruction::CALLDATASIZE))
                    }
                    Instruction::CALLDATACOPY => {
                        Err(Error::UnimplementedInstruction(Instruction::CALLDATACOPY))
                    }
                    Instruction::CODESIZE => {
                        Err(Error::UnimplementedInstruction(Instruction::CODESIZE))
                    }
                    Instruction::CODECOPY => {
                        Err(Error::UnimplementedInstruction(Instruction::CODECOPY))
                    }
                    Instruction::GASPRICE => {
                        Err(Error::UnimplementedInstruction(Instruction::GASPRICE))
                    }
                    Instruction::EXTCODESIZE => {
                        Err(Error::UnimplementedInstruction(Instruction::EXTCODESIZE))
                    }
                    Instruction::EXTCODECOPY => {
                        Err(Error::UnimplementedInstruction(Instruction::EXTCODECOPY))
                    }
                    Instruction::EXTCODEHASH => {
                        Err(Error::UnimplementedInstruction(Instruction::EXTCODEHASH))
                    }

                    Instruction::COINBASE => {
                        Err(Error::UnimplementedInstruction(Instruction::COINBASE))
                    }
                    Instruction::TIMESTAMP => {
                        Err(Error::UnimplementedInstruction(Instruction::TIMESTAMP))
                    }
                    Instruction::NUMBER => {
                        Err(Error::UnimplementedInstruction(Instruction::NUMBER))
                    }
                    Instruction::DIFFICULTY => {
                        Err(Error::UnimplementedInstruction(Instruction::DIFFICULTY))
                    }
                    Instruction::GASLIMIT => {
                        Err(Error::UnimplementedInstruction(Instruction::GASLIMIT))
                    }
                    Instruction::CHAINID => {
                        Err(Error::UnimplementedInstruction(Instruction::CHAINID))
                    }
                    Instruction::SELFBALANCE => {
                        Err(Error::UnimplementedInstruction(Instruction::SELFBALANCE))
                    }
                    Instruction::BASEFEE => {
                        Err(Error::UnimplementedInstruction(Instruction::BASEFEE))
                    }
                    Instruction::BLOBHASH => {
                        Err(Error::UnimplementedInstruction(Instruction::BLOBHASH))
                    }
                    Instruction::BLOBBASEFEE => {
                        Err(Error::UnimplementedInstruction(Instruction::BLOBBASEFEE))
                    }

                    Instruction::SLOAD => Err(Error::UnimplementedInstruction(Instruction::SLOAD)),
                    Instruction::SSTORE => {
                        Err(Error::UnimplementedInstruction(Instruction::SSTORE))
                    }

                    Instruction::MSIZE => Err(Error::UnimplementedInstruction(Instruction::MSIZE)),
                    Instruction::GAS => Err(Error::UnimplementedInstruction(Instruction::GAS)),

                    Instruction::TLOAD => Err(Error::UnimplementedInstruction(Instruction::TLOAD)),
                    Instruction::TSTORE => {
                        Err(Error::UnimplementedInstruction(Instruction::TSTORE))
                    }

                    Instruction::MCOPY => Err(Error::UnimplementedInstruction(Instruction::MCOPY)),

                    Instruction::LOG0 => Err(Error::UnimplementedInstruction(Instruction::LOG0)),
                    Instruction::LOG1 => Err(Error::UnimplementedInstruction(Instruction::LOG1)),
                    Instruction::LOG2 => Err(Error::UnimplementedInstruction(Instruction::LOG2)),
                    Instruction::LOG3 => Err(Error::UnimplementedInstruction(Instruction::LOG3)),
                    Instruction::LOG4 => Err(Error::UnimplementedInstruction(Instruction::LOG4)),

                    Instruction::CREATE => {
                        Err(Error::UnimplementedInstruction(Instruction::CREATE))
                    }
                    Instruction::CREATE2 => {
                        Err(Error::UnimplementedInstruction(Instruction::CREATE2))
                    }

                    Instruction::CALLCODE => {
                        Err(Error::UnimplementedInstruction(Instruction::CALLCODE))
                    }
                    Instruction::DELEGATECALL => {
                        Err(Error::UnimplementedInstruction(Instruction::DELEGATECALL))
                    }
                    Instruction::STATICCALL => {
                        Err(Error::UnimplementedInstruction(Instruction::STATICCALL))
                    }

                    // We should handle all of these before here
                    Instruction::JUMPDEST => {
                        Err(Error::UnexpectedInstruction(Instruction::JUMPDEST))
                    }
                    Instruction::PUSH0 => Err(Error::UnexpectedInstruction(Instruction::PUSH0)),
                    Instruction::PUSH1 => Err(Error::UnexpectedInstruction(Instruction::PUSH1)),
                    Instruction::PUSH2 => Err(Error::UnexpectedInstruction(Instruction::PUSH2)),
                    Instruction::PUSH3 => Err(Error::UnexpectedInstruction(Instruction::PUSH3)),
                    Instruction::PUSH4 => Err(Error::UnexpectedInstruction(Instruction::PUSH4)),
                    Instruction::PUSH5 => Err(Error::UnexpectedInstruction(Instruction::PUSH5)),
                    Instruction::PUSH6 => Err(Error::UnexpectedInstruction(Instruction::PUSH6)),
                    Instruction::PUSH7 => Err(Error::UnexpectedInstruction(Instruction::PUSH7)),
                    Instruction::PUSH8 => Err(Error::UnexpectedInstruction(Instruction::PUSH8)),
                    Instruction::PUSH9 => Err(Error::UnexpectedInstruction(Instruction::PUSH9)),
                    Instruction::PUSH10 => Err(Error::UnexpectedInstruction(Instruction::PUSH10)),
                    Instruction::PUSH11 => Err(Error::UnexpectedInstruction(Instruction::PUSH11)),
                    Instruction::PUSH12 => Err(Error::UnexpectedInstruction(Instruction::PUSH12)),
                    Instruction::PUSH13 => Err(Error::UnexpectedInstruction(Instruction::PUSH13)),
                    Instruction::PUSH14 => Err(Error::UnexpectedInstruction(Instruction::PUSH14)),
                    Instruction::PUSH15 => Err(Error::UnexpectedInstruction(Instruction::PUSH15)),
                    Instruction::PUSH16 => Err(Error::UnexpectedInstruction(Instruction::PUSH16)),
                    Instruction::PUSH17 => Err(Error::UnexpectedInstruction(Instruction::PUSH17)),
                    Instruction::PUSH18 => Err(Error::UnexpectedInstruction(Instruction::PUSH18)),
                    Instruction::PUSH19 => Err(Error::UnexpectedInstruction(Instruction::PUSH19)),
                    Instruction::PUSH20 => Err(Error::UnexpectedInstruction(Instruction::PUSH20)),
                    Instruction::PUSH21 => Err(Error::UnexpectedInstruction(Instruction::PUSH21)),
                    Instruction::PUSH22 => Err(Error::UnexpectedInstruction(Instruction::PUSH22)),
                    Instruction::PUSH23 => Err(Error::UnexpectedInstruction(Instruction::PUSH23)),
                    Instruction::PUSH24 => Err(Error::UnexpectedInstruction(Instruction::PUSH24)),
                    Instruction::PUSH25 => Err(Error::UnexpectedInstruction(Instruction::PUSH25)),
                    Instruction::PUSH26 => Err(Error::UnexpectedInstruction(Instruction::PUSH26)),
                    Instruction::PUSH27 => Err(Error::UnexpectedInstruction(Instruction::PUSH27)),
                    Instruction::PUSH28 => Err(Error::UnexpectedInstruction(Instruction::PUSH28)),
                    Instruction::PUSH29 => Err(Error::UnexpectedInstruction(Instruction::PUSH29)),
                    Instruction::PUSH30 => Err(Error::UnexpectedInstruction(Instruction::PUSH30)),
                    Instruction::PUSH31 => Err(Error::UnexpectedInstruction(Instruction::PUSH31)),
                    Instruction::PUSH32 => Err(Error::UnexpectedInstruction(Instruction::PUSH32)),
                }
            }
            IteratorItem::Invalid(pc) => {
                trace!("loop: Invalid");
                let absolute_pc = code_block.offset + pc;
                return Err(InvalidOpcode {
                    pc: absolute_pc,
                    opcode: code_block.rom[pc],
                }
                .into());
            }
        }?
    }
    Ok(())
}

fn build_jump_table(
    bctx: &BuildCtx,
    jump_block: BasicBlock,
    jump_cases: &[(IntValue, BasicBlock)],
) -> Result<(), Error> {
    let t = bctx.env.types();

    let jump_failure_block = bctx
        .env
        .context()
        .append_basic_block(bctx.func, "jump_failure");
    bctx.builder.position_at_end(jump_failure_block);
    let return_value = t.i8.const_int(ReturnCode::JumpFailure as u64, false);
    bctx.builder.build_return(Some(&return_value))?;

    // Build jump table logic
    // If there are no jump cases then all jumps are failures
    // If there are jump cases then we build a switch statement to jump to the correct block
    bctx.builder.position_at_end(jump_block);
    if jump_cases.is_empty() {
        bctx.builder
            .build_unconditional_branch(jump_failure_block)?;
        return Ok(());
    }

    let jump_value = bctx
        .builder
        .build_load(t.i32, bctx.registers.jump_ptr, "jump_ptr")?;
    bctx.builder.build_switch(
        IntValue::try_from(jump_value).unwrap(),
        jump_failure_block,
        jump_cases,
    )?;
    Ok(())
}
