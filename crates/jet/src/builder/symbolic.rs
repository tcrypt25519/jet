use std::collections::HashMap;

use inkwell::{basic_block::BasicBlock, values::IntValue};

use crate::builder::Error;

#[derive(Clone, Debug)]
pub(crate) enum StackValue<'ctx> {
    Word(IntValue<'ctx>),
}

#[derive(Clone, Debug, Default)]
pub(crate) struct SymbolicStack<'ctx> {
    slots: Vec<StackValue<'ctx>>,
}

impl<'ctx> SymbolicStack<'ctx> {
    pub(crate) fn new() -> Self {
        Self { slots: Vec::new() }
    }

    pub(crate) fn len(&self) -> usize {
        self.slots.len()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }

    pub(crate) fn push_word(&mut self, v: IntValue<'ctx>) {
        self.slots.push(StackValue::Word(v));
    }

    pub(crate) fn pop_word(&mut self) -> Result<IntValue<'ctx>, Error> {
        let value = self
            .slots
            .pop()
            .ok_or_else(|| Error::invariant_violation("symbolic stack underflow"))?;
        match value {
            StackValue::Word(v) => Ok(v),
        }
    }

    pub(crate) fn peek_word(&self, depth_from_top: usize) -> Result<IntValue<'ctx>, Error> {
        if depth_from_top >= self.slots.len() {
            return Err(Error::invariant_violation("symbolic stack peek out of bounds"));
        }

        let idx = self.slots.len() - 1 - depth_from_top;
        match self.slots[idx] {
            StackValue::Word(v) => Ok(v),
        }
    }

    pub(crate) fn dup(&mut self, index_1_based: u8) -> Result<(), Error> {
        if index_1_based == 0 {
            return Err(Error::invariant_violation("dup index must be >= 1"));
        }
        let value = self.peek_word((index_1_based - 1) as usize)?;
        self.push_word(value);
        Ok(())
    }

    pub(crate) fn swap(&mut self, index_1_based: u8) -> Result<(), Error> {
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

    pub(crate) fn clone_slots(&self) -> Vec<StackValue<'ctx>> {
        self.slots.clone()
    }

    pub(crate) fn from_slots(slots: Vec<StackValue<'ctx>>) -> Self {
        Self { slots }
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct BlockInState<'ctx> {
    pub(crate) stack: Option<SymbolicStack<'ctx>>,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct BlockOutState<'ctx> {
    pub(crate) stack: Option<SymbolicStack<'ctx>>,
}

#[derive(Clone, Debug)]
pub(crate) struct CfgNode<'ctx> {
    pub(crate) offset: usize,
    pub(crate) entry: BasicBlock<'ctx>,
    pub(crate) succ_offsets: Vec<usize>,
}

#[derive(Default)]
pub(crate) struct SymbolicGraph<'ctx> {
    pub(crate) nodes: HashMap<usize, CfgNode<'ctx>>,
    pub(crate) in_states: HashMap<usize, BlockInState<'ctx>>,
    pub(crate) out_states: HashMap<usize, BlockOutState<'ctx>>,
}
