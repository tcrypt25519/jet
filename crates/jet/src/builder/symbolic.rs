use inkwell::values::IntValue;

use crate::builder::Error;

#[derive(Clone, Debug)]
pub(crate) enum StackValue<'ctx> {
    Word {
        value: IntValue<'ctx>,
        known_u64: Option<u64>,
    },
}

#[derive(Clone, Debug, Default)]
pub(crate) struct SymbolicStack<'ctx> {
    slots: Vec<StackValue<'ctx>>,
}

impl<'ctx> SymbolicStack<'ctx> {
    pub(crate) fn new() -> Self {
        Self { slots: Vec::new() }
    }

    pub(crate) fn from_slots(slots: Vec<StackValue<'ctx>>) -> Self {
        Self { slots }
    }

    pub(crate) fn len(&self) -> usize {
        self.slots.len()
    }

    pub(crate) fn push_word(&mut self, v: IntValue<'ctx>) {
        self.push_word_with_known_u64(v, None);
    }

    pub(crate) fn push_word_with_known_u64(&mut self, v: IntValue<'ctx>, known_u64: Option<u64>) {
        self.slots.push(StackValue::Word {
            value: v,
            known_u64,
        });
    }

    pub(crate) fn pop_word(&mut self) -> Result<IntValue<'ctx>, Error> {
        let value = self
            .slots
            .pop()
            .ok_or_else(|| Error::invariant_violation("symbolic stack underflow"))?;
        match value {
            StackValue::Word { value, .. } => Ok(value),
        }
    }

    pub(crate) fn peek_word_known_u64(&self, depth_from_top: usize) -> Result<Option<u64>, Error> {
        if depth_from_top >= self.slots.len() {
            return Err(Error::invariant_violation(
                "symbolic stack peek out of bounds",
            ));
        }

        let idx = self.slots.len() - 1 - depth_from_top;
        match self.slots[idx] {
            StackValue::Word { known_u64, .. } => Ok(known_u64),
        }
    }

    pub(crate) fn dup(&mut self, index_1_based: u8) -> Result<(), Error> {
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
        self.slots.push(slot);
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
}
