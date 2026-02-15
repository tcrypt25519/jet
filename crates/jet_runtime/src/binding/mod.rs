use std::fmt;

use colored::Colorize;

use crate::exec;

impl fmt::Display for exec::BlockInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "BlockInfo:\n  {{ number: {}, difficulty: {}, gas_limit: {}, timestamp: {}, base_fee: {}, blob_base_fee: {}, chain_id: {}, hash: {}, coinbase: {} }}\n",
            self.number(),
            self.difficulty(),
            self.gas_limit(),
            self.timestamp(),
            self.base_fee(),
            self.blob_base_fee(),
            self.chain_id(),
            self.hash()
                .iter()
                .take(32)
                .fold(String::new(), |acc, x| acc.clone() + &format!("{:02X}", x)),
            self.coinbase()
        )
    }
}

impl fmt::Display for exec::Context {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Context:\n  {{ stack ptr: {}, jump ptr: {}, return_off: {}, return_len: {} }}\n",
            self.stack_ptr(),
            self.jump_ptr(),
            self.return_off(),
            self.return_len() /* "Context:\n  {{ stack ptr: {}, jump ptr: {}, return_off: {},
                               * return_len: {}, sub_call: {} }}\n",
                               * self.stack_ptr(), self.jump_ptr(), self.return_off(),
                               * self.return_len(), self.sub_call_ptr() */
        )?;

        write!(
            f,
            "Memory:\n  {{ len: {}, cap: {} }}\n",
            self.memory_len(),
            self.memory_cap()
        )?;
        for i in 0..1 {
            let offset = (32 * i) as usize;
            let end = offset + 32;
            writeln!(
                f,
                "  {}: {}",
                i,
                self.memory()[offset..end]
                    .iter()
                    .take(32)
                    .fold(String::new(), |acc, x| acc.clone() + &format!("{:02X}", x))
            )?;
        }

        writeln!(f, "Stack:")?;
        let stack = self.stack();
        let stack_size = self.stack_ptr() as usize;

        // Print out each 32 byte word, starting from the top of the stack and working down
        // for word_i in (stack_size - 1)..=0 {
        for i in 0..stack_size {
            let word_i = stack_size - i - 1;
            writeln!(
                f,
                "  {}: {}",
                word_i,
                stack[word_i]
                    .iter()
                    .take(32)
                    .rev()
                    .fold((String::new(), false), byte_formatter)
                    .0
            )?;
        }

        match self.sub_ctx() {
            Some(sub_ctx) => {
                writeln!(f, "Sub Call:\n{}", sub_ctx)?;
            }
            None => {
                writeln!(f, "Sub Call: None")?;
            }
        }

        Ok(())
    }
}

fn byte_formatter(acc: (String, bool), x: &u8) -> (String, bool) {
    let has_been_significant = acc.1;
    let is_significant = has_been_significant || *x != 0;

    let mut byte_str = format!("{:02X}", x);

    if is_significant {
        byte_str = if !has_been_significant && x < &16 {
            let chars = byte_str.chars();
            format!(
                "{}{}",
                chars.clone().next().unwrap(),
                chars.clone().nth(1).unwrap().to_string().blue()
            )
        } else {
            byte_str.blue().to_string()
        };
    }

    (acc.0.clone() + &byte_str, is_significant)
}

impl fmt::Display for exec::ContractRun {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "ContractRun:\nResult: {:?}\n{}",
            self.result(),
            self.ctx()
        )
    }
}
