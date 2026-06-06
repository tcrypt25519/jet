use std::collections::{HashMap, HashSet, VecDeque};

use inkwell::{
    basic_block::BasicBlock,
    values::{FunctionValue, IntValue, PhiValue},
};
use log::{info, trace};

use jet_ir::STACK_SIZE_WORDS;
use jet_runtime::exec::ReturnCode;

use crate::{
    builder::{
        Error, InvalidOpcode,
        env::Env,
        ops,
        stack::{RuntimeStackBackend, StackBackend, SymbolicStackBackend},
        symbolic::{StackValue, SymbolicStack},
    },
    instructions,
    instructions::{Instruction, IterItem},
};

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

pub(crate) enum StackMode {
    RuntimeOnly,
    SymbolicPreferred,
}

impl StackMode {
    fn from_env() -> Self {
        match std::env::var("JET_SYMBOLIC_STACK") {
            Ok(value) => {
                let enabled = matches!(value.as_str(), "1" | "true" | "TRUE" | "True");
                if enabled {
                    StackMode::SymbolicPreferred
                } else {
                    StackMode::RuntimeOnly
                }
            }
            Err(_) => StackMode::RuntimeOnly,
        }
    }
}

pub(crate) struct BuildCtx<'ctx, 'b, S: StackBackend<'ctx>> {
    pub(crate) env: &'b Env<'ctx>,
    pub(crate) builder: &'b inkwell::builder::Builder<'ctx>,
    pub(crate) registers: Registers<'ctx>,
    pub(crate) func: FunctionValue<'ctx>,
    pub(crate) stack: S,
}

impl<'ctx, 'b, S: StackBackend<'ctx>> BuildCtx<'ctx, 'b, S> {
    fn new(
        env: &'b Env<'ctx>,
        builder: &'b inkwell::builder::Builder<'ctx>,
        func: FunctionValue<'ctx>,
        stack: S,
    ) -> Self {
        Self {
            env,
            builder,
            func,
            registers: Registers::new(env, builder, func),
            stack,
        }
    }
}

#[derive(Debug)]
struct CodeBlock<'ctx, 'b> {
    offset: usize,
    rom: &'b [u8],
    entry_block: BasicBlock<'ctx>,
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

    pub(crate) fn set_terminates(&mut self) {
        self.terminates = true;
    }
}

struct CodeBlocks<'ctx, 'b> {
    blocks: Vec<CodeBlock<'ctx, 'b>>,
    jumpdest_offsets: HashMap<u64, usize>,
}

impl<'ctx, 'b> CodeBlocks<'ctx, 'b> {
    pub(crate) fn new() -> Self {
        Self {
            blocks: Vec::new(),
            jumpdest_offsets: HashMap::new(),
        }
    }

    pub(crate) fn add(
        &mut self,
        offset: usize,
        entry_block: BasicBlock<'ctx>,
    ) -> Result<&mut CodeBlock<'ctx, 'b>, Error> {
        self.blocks.push(CodeBlock {
            offset,
            rom: &[],
            entry_block,
            is_jumpdest: false,
            terminates: false,
        });
        self.blocks.last_mut().ok_or_else(|| {
            Error::InvariantViolation("CodeBlocks::add: no block after push".to_string())
        })
    }

    pub(crate) fn add_jumpdest(
        &mut self,
        offset: usize,
        entry_block: BasicBlock<'ctx>,
    ) -> Result<&mut CodeBlock<'ctx, 'b>, Error> {
        let index = self.blocks.len();
        let jumpdest_pc = offset
            .checked_sub(1)
            .ok_or_else(|| Error::invariant_violation("Jump destination at offset 0"))?;
        self.blocks.push(CodeBlock {
            offset,
            rom: &[],
            entry_block,
            is_jumpdest: true,
            terminates: false,
        });
        self.jumpdest_offsets.insert(jumpdest_pc as u64, index);
        self.blocks.last_mut().ok_or_else(|| {
            Error::InvariantViolation("CodeBlocks::add_jumpdest: no block after push".to_string())
        })
    }

    pub(crate) fn len(&self) -> usize {
        self.blocks.len()
    }

    pub(crate) fn get(&self, index: usize) -> Option<&CodeBlock<'ctx, 'b>> {
        self.blocks.get(index)
    }

    pub(crate) fn first(&self) -> Option<&CodeBlock<'ctx, 'b>> {
        self.blocks.first()
    }

    pub(crate) fn iter(&self) -> std::slice::Iter<'_, CodeBlock<'ctx, 'b>> {
        self.blocks.iter()
    }

    pub(crate) fn has_jumpdest(&self) -> bool {
        !self.jumpdest_offsets.is_empty()
    }

    pub(crate) fn jumpdest_index(&self, pc: u64) -> Option<usize> {
        self.jumpdest_offsets.get(&pc).copied()
    }

    pub(crate) fn following_index(&self, index: usize) -> Option<usize> {
        (index + 1 < self.blocks.len()).then_some(index + 1)
    }
}

pub fn build(env: &'_ Env<'_>, name: &str, rom: &[u8]) -> Result<(), Error> {
    match StackMode::from_env() {
        StackMode::RuntimeOnly => build_with_stack(env, name, rom, RuntimeStackBackend),
        StackMode::SymbolicPreferred => build_with_symbolic_stack(env, name, rom),
    }
}

fn build_with_symbolic_stack<'ctx>(
    env: &'_ Env<'ctx>,
    name: &str,
    rom: &[u8],
) -> Result<(), Error> {
    let builder = env.context().create_builder();

    let func_type = env.types().contract_fn;
    let func = env.module().add_function(name, func_type, None);
    info!(
        "Created function {} in module {}",
        name,
        env.module()
            .get_name()
            .to_str()
            .unwrap_or("<invalid UTF-8>")
    );

    let preamble_block = env.context().append_basic_block(func, "preamble");
    builder.position_at_end(preamble_block);

    let bctx = BuildCtx::new(env, &builder, func, SymbolicStackBackend::new());
    let code_blocks = find_code_blocks(env, func, rom)?;
    let mut plan = build_symbolic_plan(env, func, &code_blocks)?;
    create_symbolic_entry_phis(&bctx, &mut plan)?;
    build_symbolic_contract_body(&bctx, &code_blocks, &plan)?;

    bctx.builder.position_at_end(preamble_block);
    bctx.builder
        .build_unconditional_branch(plan.entry_block())?;
    Ok(())
}

fn build_with_stack<'ctx, S: StackBackend<'ctx>>(
    env: &'_ Env<'ctx>,
    name: &str,
    rom: &[u8],
    stack: S,
) -> Result<(), Error> {
    let builder = env.context().create_builder();

    // Declare the function in the module
    let func_type = env.types().contract_fn;
    let func = env.module().add_function(name, func_type, None);
    info!(
        "Created function {} in module {}",
        name,
        env.module()
            .get_name()
            .to_str()
            .unwrap_or("<invalid UTF-8>")
    );

    // Create the preamble block
    let preamble_block = env.context().append_basic_block(func, "preamble");
    builder.position_at_end(preamble_block);

    // Build ROM into IR
    let bctx = BuildCtx::new(env, &builder, func, stack);
    let code_blocks = find_code_blocks(env, func, rom)?;
    build_contract_body(&bctx, &code_blocks)?;

    // Connect the preamble block to the entry block
    let entry_block = code_blocks
        .first()
        .ok_or_else(|| Error::InvariantViolation("No code blocks found".to_string()))?;
    bctx.builder.position_at_end(preamble_block);
    bctx.builder
        .build_unconditional_branch(entry_block.entry_block)?;
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

    for item in instructions::Iter::new(bytecode) {
        match item {
            IterItem::PushData(pc, _, data) => {
                trace!("find_code_blocks: Found push data {:?} at PC {}", data, pc);
            }
            IterItem::Instr(pc, instr) => {
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
                        current_block.set_terminates();
                        current_block_starting_pc = pc + 1;
                        current_block = blocks.add(current_block_starting_pc, create_bb())?;
                    }

                    Instruction::JUMPDEST => {
                        trace!("find_code_blocks: Found JUMPDEST");
                        if current_block.rom.is_empty() {
                            current_block.rom = &bytecode[current_block_starting_pc..pc];
                        }

                        current_block_starting_pc = pc + 1;
                        current_block =
                            blocks.add_jumpdest(current_block_starting_pc, create_bb())?;
                    }
                    _ => {
                        trace!("find_code_blocks: instr {} is uninteresting", instr);
                    }
                }
            }
            IterItem::Invalid(pc) => {
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

fn build_contract_body<'ctx, 'b, S: StackBackend<'ctx>>(
    bctx: &'b BuildCtx<'ctx, 'b, S>,
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
            jump_cases.push((t.i32.const_int(offset, false), code_block.entry_block));
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
                bctx.builder
                    .build_unconditional_branch(next_block.entry_block)
                    .unwrap();
                Ok(())
            }
            None => ops::build_return(bctx, ReturnCode::ImplicitReturn),
        }?;
    }

    // Add jump table to the end of the function
    if let Some(jump_block) = jump_block {
        build_jump_table(bctx, jump_block, jump_cases.as_slice())?;
    }

    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct AbstractStackSlot {
    known_u64: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct AbstractStackState {
    slots: Vec<AbstractStackSlot>,
}

impl AbstractStackState {
    fn new() -> Self {
        Self { slots: Vec::new() }
    }

    fn len(&self) -> usize {
        self.slots.len()
    }

    fn push_unknown(&mut self) -> Result<(), Error> {
        self.push(AbstractStackSlot { known_u64: None })
    }

    fn push_known(&mut self, known_u64: Option<u64>) -> Result<(), Error> {
        self.push(AbstractStackSlot { known_u64 })
    }

    fn push(&mut self, slot: AbstractStackSlot) -> Result<(), Error> {
        if self.slots.len() >= STACK_SIZE_WORDS as usize {
            return Err(Error::invariant_violation("symbolic stack overflow"));
        }
        self.slots.push(slot);
        Ok(())
    }

    fn pop(&mut self) -> Result<AbstractStackSlot, Error> {
        self.slots
            .pop()
            .ok_or_else(|| Error::invariant_violation("symbolic stack underflow"))
    }

    fn pop_n(&mut self, n: usize) -> Result<(), Error> {
        for _ in 0..n {
            self.pop()?;
        }
        Ok(())
    }

    fn peek_known_u64(&self, depth_from_top: usize) -> Result<Option<u64>, Error> {
        if depth_from_top >= self.slots.len() {
            return Err(Error::invariant_violation(
                "symbolic stack peek out of bounds",
            ));
        }
        Ok(self.slots[self.slots.len() - 1 - depth_from_top].known_u64)
    }

    fn dup(&mut self, index_1_based: u8) -> Result<(), Error> {
        if index_1_based == 0 {
            return Err(Error::invariant_violation("dup index must be >= 1"));
        }
        let depth = (index_1_based - 1) as usize;
        if depth >= self.slots.len() {
            return Err(Error::invariant_violation(
                "symbolic stack peek out of bounds",
            ));
        }
        let slot = self.slots[self.slots.len() - 1 - depth].clone();
        self.push(slot)
    }

    fn swap(&mut self, index_1_based: u8) -> Result<(), Error> {
        if index_1_based == 0 {
            return Err(Error::invariant_violation("swap index must be >= 1"));
        }

        let len = self.slots.len();
        let top_idx = len
            .checked_sub(1)
            .ok_or_else(|| Error::invariant_violation("symbolic stack underflow"))?;
        let swap_idx = len
            .checked_sub(index_1_based as usize + 1)
            .ok_or_else(|| Error::invariant_violation("symbolic stack swap out of bounds"))?;
        self.slots.swap(top_idx, swap_idx);
        Ok(())
    }

    fn merge_known_u64(&mut self, other: &Self) -> Result<bool, Error> {
        if self.slots.len() != other.slots.len() {
            return Err(Error::invariant_violation(
                "cannot merge symbolic stack states with different heights",
            ));
        }

        let mut changed = false;
        for (slot, incoming) in self.slots.iter_mut().zip(other.slots.iter()) {
            if slot.known_u64 != incoming.known_u64 && slot.known_u64.is_some() {
                slot.known_u64 = None;
                changed = true;
            }
        }
        Ok(changed)
    }
}

struct AbstractSuccessor {
    block_index: usize,
    stack: AbstractStackState,
}

type SymbolicVariantId = usize;

struct SymbolicBlockVariant<'ctx> {
    block_index: usize,
    entry_state: AbstractStackState,
    entry_block: BasicBlock<'ctx>,
    entry_phis: Vec<PhiValue<'ctx>>,
}

impl<'ctx> SymbolicBlockVariant<'ctx> {
    fn entry_stack(&self) -> SymbolicStack<'ctx> {
        let slots = self
            .entry_state
            .slots
            .iter()
            .zip(self.entry_phis.iter())
            .map(|(slot, phi)| StackValue::Word {
                value: phi.as_basic_value().into_int_value(),
                known_u64: slot.known_u64,
            })
            .collect();
        SymbolicStack::from_slots(slots)
    }
}

struct SymbolicPlan<'ctx> {
    variants: Vec<SymbolicBlockVariant<'ctx>>,
    by_block_height: HashMap<(usize, usize), SymbolicVariantId>,
    entry_variant: SymbolicVariantId,
    original_blocks_used: HashSet<usize>,
}

impl<'ctx> SymbolicPlan<'ctx> {
    fn entry_block(&self) -> BasicBlock<'ctx> {
        self.variants[self.entry_variant].entry_block
    }

    fn variant_for_stack_len(
        &self,
        block_index: usize,
        stack_len: usize,
    ) -> Result<SymbolicVariantId, Error> {
        self.by_block_height
            .get(&(block_index, stack_len))
            .copied()
            .ok_or_else(|| {
                Error::invariant_violation(format!(
                    "missing symbolic variant for block {block_index} with stack height {stack_len}"
                ))
            })
    }
}

fn build_symbolic_plan<'ctx>(
    env: &Env<'ctx>,
    func: FunctionValue<'ctx>,
    code_blocks: &CodeBlocks<'ctx, '_>,
) -> Result<SymbolicPlan<'ctx>, Error> {
    let entries = analyze_symbolic_entries(code_blocks)?;
    let mut variants = Vec::new();
    let mut by_block_height = HashMap::new();
    let mut original_blocks_used = HashSet::new();

    for block_index in 0..code_blocks.len() {
        let Some(states) = entries.get(&block_index) else {
            continue;
        };
        let mut states = states.clone();
        states.sort_by_key(AbstractStackState::len);

        for state in states {
            let code_block = code_blocks
                .get(block_index)
                .ok_or_else(|| Error::invariant_violation("missing code block"))?;
            let entry_block = if original_blocks_used.insert(block_index) {
                code_block.entry_block
            } else {
                env.context().append_basic_block(func, "block.specialized")
            };
            let variant_id = variants.len();
            by_block_height.insert((block_index, state.len()), variant_id);
            variants.push(SymbolicBlockVariant {
                block_index,
                entry_state: state,
                entry_block,
                entry_phis: Vec::new(),
            });
        }
    }

    let entry_variant = by_block_height
        .get(&(0, 0))
        .copied()
        .ok_or_else(|| Error::invariant_violation("missing symbolic entry variant"))?;

    Ok(SymbolicPlan {
        variants,
        by_block_height,
        entry_variant,
        original_blocks_used,
    })
}

fn analyze_symbolic_entries(
    code_blocks: &CodeBlocks<'_, '_>,
) -> Result<HashMap<usize, Vec<AbstractStackState>>, Error> {
    let mut entries: HashMap<usize, Vec<AbstractStackState>> = HashMap::new();
    let mut queue = VecDeque::new();

    if code_blocks.first().is_none() {
        return Err(Error::InvariantViolation(
            "No code blocks found".to_string(),
        ));
    }

    add_symbolic_entry_state(&mut entries, 0, AbstractStackState::new())?;
    queue.push_back(0);

    while let Some(block_index) = queue.pop_front() {
        let states = entries.get(&block_index).cloned().unwrap_or_default();
        for state in states {
            let successors = analyze_symbolic_successors(code_blocks, block_index, state)?;
            for successor in successors {
                if add_symbolic_entry_state(&mut entries, successor.block_index, successor.stack)? {
                    queue.push_back(successor.block_index);
                }
            }
        }
    }

    Ok(entries)
}

fn add_symbolic_entry_state(
    entries: &mut HashMap<usize, Vec<AbstractStackState>>,
    block_index: usize,
    incoming: AbstractStackState,
) -> Result<bool, Error> {
    if incoming.len() > STACK_SIZE_WORDS as usize {
        return Err(Error::invariant_violation("symbolic stack overflow"));
    }

    let states = entries.entry(block_index).or_default();
    if let Some(existing) = states
        .iter_mut()
        .find(|state| state.len() == incoming.len())
    {
        return existing.merge_known_u64(&incoming);
    }

    states.push(incoming);
    Ok(true)
}

fn analyze_symbolic_successors(
    code_blocks: &CodeBlocks<'_, '_>,
    block_index: usize,
    entry_state: AbstractStackState,
) -> Result<Vec<AbstractSuccessor>, Error> {
    let code_block = code_blocks
        .get(block_index)
        .ok_or_else(|| Error::invariant_violation("missing code block"))?;
    let mut state = entry_state;

    for item in instructions::Iter::new(code_block.rom) {
        match item {
            IterItem::PushData(_, _, data) => {
                state.push_known(push_data_known_u64(data))?;
            }
            IterItem::Instr(pc, instr) => match instr {
                Instruction::JUMP => {
                    let target_pc = state.peek_known_u64(0)?;
                    state.pop()?;
                    return symbolic_jump_successors(code_blocks, target_pc, state);
                }
                Instruction::JUMPI => {
                    let following_index =
                        code_blocks.following_index(block_index).ok_or_else(|| {
                            Error::invariant_violation("JUMPI without following block")
                        })?;
                    let target_pc = state.peek_known_u64(0)?;
                    state.pop_n(2)?;
                    let mut successors = vec![AbstractSuccessor {
                        block_index: following_index,
                        stack: state.clone(),
                    }];
                    successors.extend(symbolic_jump_successors(code_blocks, target_pc, state)?);
                    return Ok(successors);
                }
                Instruction::STOP | Instruction::REVERT | Instruction::INVALID => {
                    return Ok(Vec::new());
                }
                Instruction::RETURN => {
                    state.pop_n(2)?;
                    return Ok(Vec::new());
                }
                _ => apply_abstract_instruction(&mut state, code_block, pc, instr)?,
            },
            IterItem::Invalid(pc) => {
                let absolute_pc = code_block.offset + pc;
                return Err(InvalidOpcode {
                    pc: absolute_pc,
                    opcode: code_block.rom[pc],
                }
                .into());
            }
        }
    }

    if let Some(following_index) = code_blocks.following_index(block_index) {
        Ok(vec![AbstractSuccessor {
            block_index: following_index,
            stack: state,
        }])
    } else {
        Ok(Vec::new())
    }
}

fn symbolic_jump_successors(
    code_blocks: &CodeBlocks<'_, '_>,
    target_pc: Option<u64>,
    stack: AbstractStackState,
) -> Result<Vec<AbstractSuccessor>, Error> {
    if let Some(target_pc) = target_pc {
        return Ok(code_blocks
            .jumpdest_index(target_pc)
            .map(|block_index| vec![AbstractSuccessor { block_index, stack }])
            .unwrap_or_default());
    }

    code_blocks
        .blocks
        .iter()
        .enumerate()
        .filter(|(_, block)| block.is_jumpdest())
        .map(|(block_index, _)| {
            Ok(AbstractSuccessor {
                block_index,
                stack: stack.clone(),
            })
        })
        .collect()
}

fn push_data_known_u64(data: &[u8]) -> Option<u64> {
    if data.len() > 8 {
        return None;
    }

    let mut value = 0u64;
    for byte in data {
        value = (value << 8) | u64::from(*byte);
    }
    Some(value)
}

fn apply_abstract_instruction(
    stack: &mut AbstractStackState,
    code_block: &CodeBlock<'_, '_>,
    pc: usize,
    instr: Instruction,
) -> Result<(), Error> {
    match instr {
        Instruction::ADD
        | Instruction::MUL
        | Instruction::SUB
        | Instruction::DIV
        | Instruction::SDIV
        | Instruction::MOD
        | Instruction::SMOD
        | Instruction::EXP
        | Instruction::SIGNEXTEND
        | Instruction::LT
        | Instruction::GT
        | Instruction::SLT
        | Instruction::SGT
        | Instruction::EQ
        | Instruction::AND
        | Instruction::OR
        | Instruction::XOR
        | Instruction::BYTE
        | Instruction::SHL
        | Instruction::SHR
        | Instruction::SAR
        | Instruction::KECCAK256 => {
            stack.pop_n(2)?;
            stack.push_unknown()
        }
        Instruction::ADDMOD | Instruction::MULMOD => {
            stack.pop_n(3)?;
            stack.push_unknown()
        }
        Instruction::ISZERO | Instruction::NOT | Instruction::MLOAD => {
            stack.pop()?;
            stack.push_unknown()
        }
        Instruction::POP => {
            stack.pop()?;
            Ok(())
        }
        Instruction::MSTORE | Instruction::MSTORE8 => {
            stack.pop_n(2)?;
            Ok(())
        }
        Instruction::PC => stack.push_known(Some((code_block.offset + pc) as u64)),
        Instruction::CALL => {
            stack.pop_n(7)?;
            stack.push_unknown()
        }
        Instruction::RETURNDATASIZE
        | Instruction::BLOCKHASH
        | Instruction::COINBASE
        | Instruction::TIMESTAMP
        | Instruction::NUMBER
        | Instruction::DIFFICULTY
        | Instruction::GASLIMIT
        | Instruction::CHAINID
        | Instruction::BASEFEE
        | Instruction::BLOBBASEFEE
        | Instruction::MSIZE => stack.push_unknown(),
        Instruction::RETURNDATACOPY => {
            stack.pop_n(3)?;
            Ok(())
        }
        Instruction::DUP1 => stack.dup(1),
        Instruction::DUP2 => stack.dup(2),
        Instruction::DUP3 => stack.dup(3),
        Instruction::DUP4 => stack.dup(4),
        Instruction::DUP5 => stack.dup(5),
        Instruction::DUP6 => stack.dup(6),
        Instruction::DUP7 => stack.dup(7),
        Instruction::DUP8 => stack.dup(8),
        Instruction::DUP9 => stack.dup(9),
        Instruction::DUP10 => stack.dup(10),
        Instruction::DUP11 => stack.dup(11),
        Instruction::DUP12 => stack.dup(12),
        Instruction::DUP13 => stack.dup(13),
        Instruction::DUP14 => stack.dup(14),
        Instruction::DUP15 => stack.dup(15),
        Instruction::DUP16 => stack.dup(16),
        Instruction::SWAP1 => stack.swap(1),
        Instruction::SWAP2 => stack.swap(2),
        Instruction::SWAP3 => stack.swap(3),
        Instruction::SWAP4 => stack.swap(4),
        Instruction::SWAP5 => stack.swap(5),
        Instruction::SWAP6 => stack.swap(6),
        Instruction::SWAP7 => stack.swap(7),
        Instruction::SWAP8 => stack.swap(8),
        Instruction::SWAP9 => stack.swap(9),
        Instruction::SWAP10 => stack.swap(10),
        Instruction::SWAP11 => stack.swap(11),
        Instruction::SWAP12 => stack.swap(12),
        Instruction::SWAP13 => stack.swap(13),
        Instruction::SWAP14 => stack.swap(14),
        Instruction::SWAP15 => stack.swap(15),
        Instruction::SWAP16 => stack.swap(16),
        Instruction::ADDRESS
        | Instruction::BALANCE
        | Instruction::ORIGIN
        | Instruction::CALLER
        | Instruction::CALLVALUE
        | Instruction::CALLDATALOAD
        | Instruction::CALLDATASIZE
        | Instruction::CALLDATACOPY
        | Instruction::CODESIZE
        | Instruction::CODECOPY
        | Instruction::GASPRICE
        | Instruction::EXTCODESIZE
        | Instruction::EXTCODECOPY
        | Instruction::EXTCODEHASH
        | Instruction::SELFBALANCE
        | Instruction::BLOBHASH
        | Instruction::SLOAD
        | Instruction::SSTORE
        | Instruction::GAS
        | Instruction::TLOAD
        | Instruction::TSTORE
        | Instruction::MCOPY
        | Instruction::LOG0
        | Instruction::LOG1
        | Instruction::LOG2
        | Instruction::LOG3
        | Instruction::LOG4
        | Instruction::CREATE
        | Instruction::CREATE2
        | Instruction::CALLCODE
        | Instruction::DELEGATECALL
        | Instruction::STATICCALL
        | Instruction::SELFDESTRUCT => Err(Error::UnimplementedInstruction(instr)),
        Instruction::JUMP | Instruction::JUMPI | Instruction::JUMPDEST => {
            Err(Error::UnexpectedInstruction(instr))
        }
        Instruction::STOP
        | Instruction::RETURN
        | Instruction::REVERT
        | Instruction::INVALID
        | Instruction::PUSH0
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
        | Instruction::PUSH32 => Err(Error::UnexpectedInstruction(instr)),
    }
}

fn create_symbolic_entry_phis<'ctx>(
    bctx: &BuildCtx<'ctx, '_, SymbolicStackBackend<'ctx>>,
    plan: &mut SymbolicPlan<'ctx>,
) -> Result<(), Error> {
    for variant in &mut plan.variants {
        bctx.builder.position_at_end(variant.entry_block);
        for _ in &variant.entry_state.slots {
            let phi = bctx.builder.build_phi(bctx.env.types().i256, "stack_phi")?;
            variant.entry_phis.push(phi);
        }
    }
    Ok(())
}

fn build_symbolic_contract_body<'ctx>(
    bctx: &BuildCtx<'ctx, '_, SymbolicStackBackend<'ctx>>,
    code_blocks: &CodeBlocks<'ctx, '_>,
    plan: &SymbolicPlan<'ctx>,
) -> Result<(), Error> {
    for variant in &plan.variants {
        let code_block = code_blocks
            .get(variant.block_index)
            .ok_or_else(|| Error::invariant_violation("missing code block"))?;
        bctx.builder.position_at_end(variant.entry_block);
        bctx.stack.restore(variant.entry_stack());

        build_symbolic_code_block(bctx, variant.block_index, code_block, code_blocks, plan)?;

        if code_block.terminates() {
            continue;
        }

        match code_blocks.following_index(variant.block_index) {
            Some(following_index) => {
                let target =
                    plan.variant_for_stack_len(following_index, bctx.stack.snapshot().len())?;
                record_symbolic_variant_incoming(bctx, plan, target)?;
                bctx.builder
                    .build_unconditional_branch(plan.variants[target].entry_block)?;
            }
            None => ops::build_return(bctx, ReturnCode::ImplicitReturn)?,
        }
    }

    terminate_unplanned_symbolic_blocks(bctx, code_blocks, plan)?;
    Ok(())
}

fn terminate_unplanned_symbolic_blocks<'ctx>(
    bctx: &BuildCtx<'ctx, '_, SymbolicStackBackend<'ctx>>,
    code_blocks: &CodeBlocks<'ctx, '_>,
    plan: &SymbolicPlan<'ctx>,
) -> Result<(), Error> {
    for (block_index, code_block) in code_blocks.blocks.iter().enumerate() {
        if plan.original_blocks_used.contains(&block_index) {
            continue;
        }
        if code_block.entry_block.get_terminator().is_some() {
            continue;
        }
        bctx.builder.position_at_end(code_block.entry_block);
        let return_value = bctx
            .env
            .types()
            .i8
            .const_int(ReturnCode::Invalid as u64, false);
        bctx.builder.build_return(Some(&return_value))?;
    }
    Ok(())
}

fn symbolic_slot_word<'ctx>(slot: &StackValue<'ctx>) -> Result<IntValue<'ctx>, Error> {
    match slot {
        StackValue::Word { value, .. } => Ok(*value),
    }
}

fn record_symbolic_variant_incoming<'ctx>(
    bctx: &BuildCtx<'ctx, '_, SymbolicStackBackend<'ctx>>,
    plan: &SymbolicPlan<'ctx>,
    target: SymbolicVariantId,
) -> Result<(), Error> {
    let pred = bctx
        .builder
        .get_insert_block()
        .ok_or_else(|| Error::invariant_violation("missing current block for symbolic edge"))?;
    record_symbolic_variant_incoming_from(bctx, plan, target, pred, bctx.stack.snapshot())
}

fn record_symbolic_variant_incoming_from<'ctx>(
    _bctx: &BuildCtx<'ctx, '_, SymbolicStackBackend<'ctx>>,
    plan: &SymbolicPlan<'ctx>,
    target: SymbolicVariantId,
    pred: BasicBlock<'ctx>,
    stack: SymbolicStack<'ctx>,
) -> Result<(), Error> {
    let variant = &plan.variants[target];
    if stack.len() != variant.entry_phis.len() {
        return Err(Error::invariant_violation(format!(
            "symbolic stack height mismatch at block variant {}",
            target
        )));
    }

    let slots = stack.clone_slots();
    for (phi, slot) in variant.entry_phis.iter().zip(slots.iter()) {
        let value = symbolic_slot_word(slot)?;
        phi.add_incoming(&[(&value, pred)]);
    }
    Ok(())
}

fn build_symbolic_code_block<'ctx>(
    bctx: &BuildCtx<'ctx, '_, SymbolicStackBackend<'ctx>>,
    block_index: usize,
    code_block: &CodeBlock<'ctx, '_>,
    code_blocks: &CodeBlocks<'ctx, '_>,
    plan: &SymbolicPlan<'ctx>,
) -> Result<(), Error> {
    trace!("symbolic: Building code");
    trace!("symbolic: Offset: {}", code_block.offset);
    trace!("symbolic: ROM: {:?}", code_block.rom);

    for item in instructions::Iter::new(code_block.rom) {
        match item {
            IterItem::PushData(_, _, data) => {
                let mut new_data = [0u8; 32];
                new_data[..data.len()].copy_from_slice(data);
                new_data[..data.len()].reverse();
                ops::push(bctx, new_data)?;
            }
            IterItem::Instr(pc, instr) => match instr {
                Instruction::JUMP => build_symbolic_jump(bctx, code_blocks, plan)?,
                Instruction::JUMPI => {
                    let following_index =
                        code_blocks.following_index(block_index).ok_or_else(|| {
                            Error::invariant_violation("JUMPI without following block")
                        })?;
                    build_symbolic_jumpi(bctx, following_index, code_blocks, plan)?;
                }
                _ => build_non_jump_instruction(bctx, code_block, pc, instr)?,
            },
            IterItem::Invalid(pc) => {
                let absolute_pc = code_block.offset + pc;
                return Err(InvalidOpcode {
                    pc: absolute_pc,
                    opcode: code_block.rom[pc],
                }
                .into());
            }
        }
    }
    Ok(())
}

fn build_symbolic_jump<'ctx>(
    bctx: &BuildCtx<'ctx, '_, SymbolicStackBackend<'ctx>>,
    code_blocks: &CodeBlocks<'ctx, '_>,
    plan: &SymbolicPlan<'ctx>,
) -> Result<(), Error> {
    let target_pc = bctx.stack.peek_word_known_u64(0)?;
    let pc = bctx.stack.pop_word(bctx)?;
    let pc = truncate_jump_pc(bctx, pc, "jump_pc")?;
    bctx.builder.build_store(bctx.registers.jump_ptr, pc)?;

    match target_pc {
        Some(pc) => match code_blocks.jumpdest_index(pc) {
            Some(block_index) => {
                let target =
                    plan.variant_for_stack_len(block_index, bctx.stack.snapshot().len())?;
                record_symbolic_variant_incoming(bctx, plan, target)?;
                bctx.builder
                    .build_unconditional_branch(plan.variants[target].entry_block)?;
            }
            None => {
                let current_block = bctx.builder.get_insert_block().ok_or_else(|| {
                    Error::invariant_violation("missing current block for invalid static jump")
                })?;
                let jump_failure_block = build_jump_failure_block(bctx)?;
                bctx.builder.position_at_end(current_block);
                bctx.builder
                    .build_unconditional_branch(jump_failure_block)?;
            }
        },
        None => build_symbolic_dynamic_jump_switch(bctx, pc, code_blocks, plan)?,
    }
    Ok(())
}

fn build_symbolic_jumpi<'ctx>(
    bctx: &BuildCtx<'ctx, '_, SymbolicStackBackend<'ctx>>,
    following_index: usize,
    code_blocks: &CodeBlocks<'ctx, '_>,
    plan: &SymbolicPlan<'ctx>,
) -> Result<(), Error> {
    let target_pc = bctx.stack.peek_word_known_u64(0)?;
    let (pc, cond) = bctx.stack.pop_2(bctx)?;
    let pc = truncate_jump_pc(bctx, pc, "jumpi_pc")?;
    bctx.builder.build_store(bctx.registers.jump_ptr, pc)?;
    let cmp = bctx.builder.build_int_compare(
        inkwell::IntPredicate::EQ,
        cond,
        bctx.env.types().i256.const_zero(),
        "jumpi_cmp",
    )?;

    let following = plan.variant_for_stack_len(following_index, bctx.stack.snapshot().len())?;
    match target_pc {
        Some(pc) => match code_blocks.jumpdest_index(pc) {
            Some(block_index) => {
                let target =
                    plan.variant_for_stack_len(block_index, bctx.stack.snapshot().len())?;
                record_symbolic_variant_incoming(bctx, plan, following)?;
                record_symbolic_variant_incoming(bctx, plan, target)?;
                bctx.builder.build_conditional_branch(
                    cmp,
                    plan.variants[following].entry_block,
                    plan.variants[target].entry_block,
                )?;
            }
            None => {
                let current_block = bctx.builder.get_insert_block().ok_or_else(|| {
                    Error::invariant_violation("missing current block for invalid static JUMPI")
                })?;
                let jump_failure_block = build_jump_failure_block(bctx)?;
                bctx.builder.position_at_end(current_block);
                record_symbolic_variant_incoming(bctx, plan, following)?;
                bctx.builder.build_conditional_branch(
                    cmp,
                    plan.variants[following].entry_block,
                    jump_failure_block,
                )?;
            }
        },
        None => {
            build_symbolic_dynamic_jumpi_switch(bctx, pc, cmp, following, code_blocks, plan)?;
        }
    }
    Ok(())
}

fn build_symbolic_dynamic_jumpi_switch<'ctx>(
    bctx: &BuildCtx<'ctx, '_, SymbolicStackBackend<'ctx>>,
    pc: IntValue<'ctx>,
    cmp: IntValue<'ctx>,
    following: SymbolicVariantId,
    code_blocks: &CodeBlocks<'ctx, '_>,
    plan: &SymbolicPlan<'ctx>,
) -> Result<(), Error> {
    let branch_block = bctx
        .builder
        .get_insert_block()
        .ok_or_else(|| Error::invariant_violation("missing current block for dynamic JUMPI"))?;
    let target_switch_block = bctx
        .env
        .context()
        .append_basic_block(bctx.func, "jumpi_dynamic_targets");
    record_symbolic_variant_incoming_from(
        bctx,
        plan,
        following,
        branch_block,
        bctx.stack.snapshot(),
    )?;
    bctx.builder.build_conditional_branch(
        cmp,
        plan.variants[following].entry_block,
        target_switch_block,
    )?;

    bctx.builder.position_at_end(target_switch_block);
    build_symbolic_dynamic_jump_switch_from_current_block(bctx, pc, code_blocks, plan)
}

fn build_symbolic_dynamic_jump_switch<'ctx>(
    bctx: &BuildCtx<'ctx, '_, SymbolicStackBackend<'ctx>>,
    pc: IntValue<'ctx>,
    code_blocks: &CodeBlocks<'ctx, '_>,
    plan: &SymbolicPlan<'ctx>,
) -> Result<(), Error> {
    build_symbolic_dynamic_jump_switch_from_current_block(bctx, pc, code_blocks, plan)
}

fn build_symbolic_dynamic_jump_switch_from_current_block<'ctx>(
    bctx: &BuildCtx<'ctx, '_, SymbolicStackBackend<'ctx>>,
    pc: IntValue<'ctx>,
    code_blocks: &CodeBlocks<'ctx, '_>,
    plan: &SymbolicPlan<'ctx>,
) -> Result<(), Error> {
    let switch_block = bctx
        .builder
        .get_insert_block()
        .ok_or_else(|| Error::invariant_violation("missing current block for dynamic jump"))?;
    let jump_failure_block = build_jump_failure_block(bctx)?;
    bctx.builder.position_at_end(switch_block);
    let stack_len = bctx.stack.snapshot().len();
    let jump_cases = symbolic_jump_cases(bctx, code_blocks, plan, switch_block, stack_len)?;

    if jump_cases.is_empty() {
        bctx.builder
            .build_unconditional_branch(jump_failure_block)?;
    } else {
        bctx.builder
            .build_switch(pc, jump_failure_block, jump_cases.as_slice())?;
    }
    Ok(())
}

fn symbolic_jump_cases<'ctx>(
    bctx: &BuildCtx<'ctx, '_, SymbolicStackBackend<'ctx>>,
    code_blocks: &CodeBlocks<'ctx, '_>,
    plan: &SymbolicPlan<'ctx>,
    pred: BasicBlock<'ctx>,
    stack_len: usize,
) -> Result<Vec<(IntValue<'ctx>, BasicBlock<'ctx>)>, Error> {
    let mut jump_cases = Vec::new();
    for (block_index, code_block) in code_blocks
        .blocks
        .iter()
        .enumerate()
        .filter(|(_, block)| block.is_jumpdest())
    {
        let target = plan.variant_for_stack_len(block_index, stack_len)?;
        let jumpdest_pc = code_block
            .offset
            .checked_sub(1)
            .ok_or_else(|| Error::invariant_violation("Jump destination at offset 0"))?;
        record_symbolic_variant_incoming_from(bctx, plan, target, pred, bctx.stack.snapshot())?;
        jump_cases.push((
            bctx.env.types().i32.const_int(jumpdest_pc as u64, false),
            plan.variants[target].entry_block,
        ));
    }
    Ok(jump_cases)
}

fn build_jump_failure_block<'ctx, S: StackBackend<'ctx>>(
    bctx: &BuildCtx<'ctx, '_, S>,
) -> Result<BasicBlock<'ctx>, Error> {
    let jump_failure_block = bctx
        .env
        .context()
        .append_basic_block(bctx.func, "jump_failure");
    bctx.builder.position_at_end(jump_failure_block);
    let return_value = bctx
        .env
        .types()
        .i8
        .const_int(ReturnCode::JumpFailure as u64, false);
    bctx.builder.build_return(Some(&return_value))?;
    Ok(jump_failure_block)
}

fn truncate_jump_pc<'ctx>(
    bctx: &BuildCtx<'ctx, '_, SymbolicStackBackend<'ctx>>,
    pc: IntValue<'ctx>,
    name: &str,
) -> Result<IntValue<'ctx>, Error> {
    let bit_width = pc.get_type().get_bit_width();
    match bit_width {
        32 => Ok(pc),
        256 => Ok(bctx
            .builder
            .build_int_truncate(pc, bctx.env.types().i32, name)?),
        _ => Err(Error::InvalidBitWidth(bit_width)),
    }
}

fn build_non_jump_instruction<'ctx, S: StackBackend<'ctx>>(
    bctx: &BuildCtx<'ctx, '_, S>,
    code_block: &CodeBlock<'ctx, '_>,
    pc: usize,
    instr: Instruction,
) -> Result<(), Error> {
    match instr {
        Instruction::STOP => ops::stop(bctx),

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

        Instruction::KECCAK256 => ops::keccak256(bctx),
        Instruction::RETURNDATASIZE => ops::returndatasize(bctx),
        Instruction::RETURNDATACOPY => ops::returndatacopy(bctx),
        Instruction::BLOCKHASH => ops::blockhash(bctx),
        Instruction::COINBASE => ops::coinbase(bctx),
        Instruction::TIMESTAMP => ops::timestamp(bctx),
        Instruction::NUMBER => ops::number(bctx),
        Instruction::DIFFICULTY => ops::difficulty(bctx),
        Instruction::GASLIMIT => ops::gaslimit(bctx),
        Instruction::CHAINID => ops::chainid(bctx),
        Instruction::BASEFEE => ops::basefee(bctx),
        Instruction::BLOBBASEFEE => ops::blobbasefee(bctx),
        Instruction::POP => ops::pop(bctx),
        Instruction::MLOAD => ops::mload(bctx),
        Instruction::MSTORE => ops::mstore(bctx),
        Instruction::MSTORE8 => ops::mstore8(bctx),
        Instruction::MSIZE => ops::msize(bctx),
        Instruction::PC => ops::pc(bctx, code_block.offset + pc),
        Instruction::CALL => ops::call(bctx),
        Instruction::RETURN => ops::_return(bctx),
        Instruction::REVERT => ops::revert(bctx),
        Instruction::INVALID => ops::invalid(bctx),
        Instruction::SELFDESTRUCT => ops::selfdestruct(bctx),

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

        Instruction::ADDRESS => Err(Error::UnimplementedInstruction(Instruction::ADDRESS)),
        Instruction::BALANCE => Err(Error::UnimplementedInstruction(Instruction::BALANCE)),
        Instruction::ORIGIN => Err(Error::UnimplementedInstruction(Instruction::ORIGIN)),
        Instruction::CALLER => Err(Error::UnimplementedInstruction(Instruction::CALLER)),
        Instruction::CALLVALUE => Err(Error::UnimplementedInstruction(Instruction::CALLVALUE)),
        Instruction::CALLDATALOAD => {
            Err(Error::UnimplementedInstruction(Instruction::CALLDATALOAD))
        }
        Instruction::CALLDATASIZE => {
            Err(Error::UnimplementedInstruction(Instruction::CALLDATASIZE))
        }
        Instruction::CALLDATACOPY => {
            Err(Error::UnimplementedInstruction(Instruction::CALLDATACOPY))
        }
        Instruction::CODESIZE => Err(Error::UnimplementedInstruction(Instruction::CODESIZE)),
        Instruction::CODECOPY => Err(Error::UnimplementedInstruction(Instruction::CODECOPY)),
        Instruction::GASPRICE => Err(Error::UnimplementedInstruction(Instruction::GASPRICE)),
        Instruction::EXTCODESIZE => Err(Error::UnimplementedInstruction(Instruction::EXTCODESIZE)),
        Instruction::EXTCODECOPY => Err(Error::UnimplementedInstruction(Instruction::EXTCODECOPY)),
        Instruction::EXTCODEHASH => Err(Error::UnimplementedInstruction(Instruction::EXTCODEHASH)),
        Instruction::SELFBALANCE => Err(Error::UnimplementedInstruction(Instruction::SELFBALANCE)),
        Instruction::BLOBHASH => Err(Error::UnimplementedInstruction(Instruction::BLOBHASH)),
        Instruction::SLOAD => Err(Error::UnimplementedInstruction(Instruction::SLOAD)),
        Instruction::SSTORE => Err(Error::UnimplementedInstruction(Instruction::SSTORE)),
        Instruction::GAS => Err(Error::UnimplementedInstruction(Instruction::GAS)),
        Instruction::TLOAD => Err(Error::UnimplementedInstruction(Instruction::TLOAD)),
        Instruction::TSTORE => Err(Error::UnimplementedInstruction(Instruction::TSTORE)),
        Instruction::MCOPY => Err(Error::UnimplementedInstruction(Instruction::MCOPY)),
        Instruction::LOG0 => Err(Error::UnimplementedInstruction(Instruction::LOG0)),
        Instruction::LOG1 => Err(Error::UnimplementedInstruction(Instruction::LOG1)),
        Instruction::LOG2 => Err(Error::UnimplementedInstruction(Instruction::LOG2)),
        Instruction::LOG3 => Err(Error::UnimplementedInstruction(Instruction::LOG3)),
        Instruction::LOG4 => Err(Error::UnimplementedInstruction(Instruction::LOG4)),
        Instruction::CREATE => Err(Error::UnimplementedInstruction(Instruction::CREATE)),
        Instruction::CREATE2 => Err(Error::UnimplementedInstruction(Instruction::CREATE2)),
        Instruction::CALLCODE => Err(Error::UnimplementedInstruction(Instruction::CALLCODE)),
        Instruction::DELEGATECALL => {
            Err(Error::UnimplementedInstruction(Instruction::DELEGATECALL))
        }
        Instruction::STATICCALL => Err(Error::UnimplementedInstruction(Instruction::STATICCALL)),

        Instruction::JUMP | Instruction::JUMPI | Instruction::JUMPDEST => {
            Err(Error::UnexpectedInstruction(instr))
        }
        Instruction::PUSH0
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
        | Instruction::PUSH32 => Err(Error::UnexpectedInstruction(instr)),
    }
}

fn build_code_block<'ctx, S: StackBackend<'ctx>>(
    bctx: &BuildCtx<'ctx, '_, S>,
    code_block: &CodeBlock<'ctx, '_>,
    jump_block: Option<BasicBlock<'ctx>>,
    following_block: Option<&&CodeBlock<'ctx, '_>>,
) -> Result<(), Error> {
    trace!("loop: Building code");
    trace!("loop: Offset: {}", code_block.offset);
    trace!("loop: ROM: {:?}", code_block.rom);

    // Prepare for building the IR for this code block. Move the builder to this basic block
    // and start a relative PC at 0.
    bctx.builder.position_at_end(code_block.entry_block);

    for item in instructions::Iter::new(code_block.rom) {
        match item {
            IterItem::PushData(_, _, data) => {
                trace!("loop: Data: {:?}", data);

                let mut new_data = [0u8; 32];
                new_data[..data.len()].copy_from_slice(data);
                new_data[..data.len()].reverse();

                ops::push(bctx, new_data)
            }
            IterItem::Instr(pc, instr) => {
                trace!("loop: Instruction: {:?}", instr);
                match instr {
                    Instruction::JUMP => match jump_block {
                        Some(jump_block) => ops::jump(bctx, jump_block),
                        _ => return Err(Error::invariant_violation("JUMP without jump block")),
                    },
                    Instruction::JUMPI => match (jump_block, following_block) {
                        (Some(jump_block), Some(following_block)) => {
                            ops::jumpi(bctx, jump_block, following_block.entry_block)
                        }
                        (Some(_), None) => {
                            return Err(Error::invariant_violation(
                                "JUMPI without following block",
                            ));
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
                    _ => build_non_jump_instruction(bctx, code_block, pc, instr),
                }
            }
            IterItem::Invalid(pc) => {
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

fn build_jump_table<'ctx, S: StackBackend<'ctx>>(
    bctx: &BuildCtx<'ctx, '_, S>,
    jump_block: BasicBlock<'ctx>,
    jump_cases: &[(IntValue<'ctx>, BasicBlock<'ctx>)],
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
