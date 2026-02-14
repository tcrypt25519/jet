/// Runtime builder - generates LLVM IR for runtime functions.
use inkwell::{builder::Builder, context::Context, module::Module, values::FunctionValue};
use jet_ir::Types;

/// RuntimeBuilder generates LLVM IR for runtime functions.
pub struct RuntimeBuilder<'ctx> {
    context: &'ctx Context,
    module: Module<'ctx>,
    builder: Builder<'ctx>,
    types: Types<'ctx>,
}

impl<'ctx> RuntimeBuilder<'ctx> {
    /// Create a new runtime builder for generating runtime IR.
    pub fn new(context: &'ctx Context, module_name: &str) -> Self {
        let module = context.create_module(module_name);
        let builder = context.create_builder();
        let types = Types::new(context);

        Self {
            context,
            module,
            builder,
            types,
        }
    }

    /// Build all runtime functions and return the module containing them.
    pub fn build(self) -> Module<'ctx> {
        // Add global variable for JIT engine pointer
        self.module
            .add_global(self.types.ptr, None, crate::symbols::JIT_ENGINE);

        // Generate IR-based runtime functions for stack operations
        self.build_stack_push_i256();
        self.build_stack_push_word();
        self.build_stack_pop();
        self.build_stack_peek();
        self.build_stack_swap();

        // Generate IR-based runtime functions for memory operations
        self.build_mem_load();
        self.build_mem_store_word();
        self.build_mem_store_byte();

        // Declare external Rust functions for complex operations
        self.declare_external_builtins();

        self.module
    }

    /// Declare forward references to external Rust functions.
    /// These handle complex operations like contract calls and crypto that
    /// require Rust's standard library or external dependencies.
    fn declare_external_builtins(&self) {
        self.module.add_function(
            "jet.contract.call",
            self.types.i8.fn_type(
                &[
                    self.types.ptr.into(),
                    self.types.ptr.into(),
                    self.types.ptr.into(),
                    self.types.ptr.into(),
                    self.types.ptr.into(),
                ],
                false,
            ),
            None,
        );

        self.module.add_function(
            "jet.contracts.call_return_data_copy",
            self.types.i8.fn_type(
                &[
                    self.types.ptr.into(),
                    self.types.ptr.into(),
                    self.types.i32.into(),
                    self.types.i32.into(),
                    self.types.i32.into(),
                ],
                false,
            ),
            None,
        );

        // Crypto operations
        self.module.add_function(
            "jet.ops.keccak256",
            self.types.i8.fn_type(&[self.types.ptr.into()], false),
            None,
        );

        // Arithmetic operations
        self.module.add_function(
            "jet.ops.exp",
            self.types
                .i8
                .fn_type(&[self.types.ptr.into(), self.types.ptr.into()], false),
            None,
        );

        // Memory operations
        self.module.add_function(
            "jet.mem.expand",
            self.types.i8.fn_type(
                &[
                    self.types.ptr.into(),
                    self.types.i32.into(),
                    self.types.i32.into(),
                ],
                false,
            ),
            None,
        );
    }

    /// Build jet.stack.push.i256 function in IR.
    /// This function pushes an i256 value onto the EVM stack.
    fn build_stack_push_i256(&self) -> FunctionValue<'ctx> {
        let fn_type = self.context.bool_type().fn_type(
            &[
                self.context
                    .ptr_type(inkwell::AddressSpace::default())
                    .into(),
                self.types.i256.into(),
            ],
            false,
        );

        let function = self
            .module
            .add_function("jet.stack.push.i256", fn_type, None);
        let entry_block = self.context.append_basic_block(function, "entry");

        self.builder.position_at_end(entry_block);

        // Get function parameters
        let ctx_ptr = function.get_nth_param(0).unwrap().into_pointer_value();
        let value = function.get_nth_param(1).unwrap().into_int_value();

        // Load stack pointer (field 0)
        let stack_ptr_addr = self
            .builder
            .build_struct_gep(self.types.exec_ctx, ctx_ptr, 0, "stack.ptr.addr")
            .unwrap();
        let stack_ptr = self
            .builder
            .build_load(self.types.i32, stack_ptr_addr, "stack.ptr")
            .unwrap()
            .into_int_value();

        // Get address of stack[stack_ptr] (field 5 is stack array)
        let stack_field_ptr = self
            .builder
            .build_struct_gep(self.types.exec_ctx, ctx_ptr, 5, "stack.field")
            .unwrap();

        // Index into the stack array
        let stack_top_addr = unsafe {
            self.builder
                .build_gep(
                    self.types.i256,
                    stack_field_ptr,
                    &[stack_ptr],
                    "stack.top.addr",
                )
                .unwrap()
        };

        // Store the value
        self.builder.build_store(stack_top_addr, value).unwrap();

        // Increment stack pointer
        let stack_ptr_next = self
            .builder
            .build_int_add(
                stack_ptr,
                self.types.i32.const_int(1, false),
                "stack.ptr.next",
            )
            .unwrap();
        self.builder
            .build_store(stack_ptr_addr, stack_ptr_next)
            .unwrap();

        // Return true
        self.builder
            .build_return(Some(&self.context.bool_type().const_int(1, false)))
            .unwrap();

        function
    }

    /// Build jet.stack.push.word function in IR.
    /// Pushes a word (passed by pointer) onto the EVM stack.
    fn build_stack_push_word(&self) -> FunctionValue<'ctx> {
        let fn_type = self.context.bool_type().fn_type(
            &[
                self.context
                    .ptr_type(inkwell::AddressSpace::default())
                    .into(),
                self.types.ptr.into(),
            ],
            false,
        );

        let function = self
            .module
            .add_function("jet.stack.push.ptr", fn_type, None);
        let entry_block = self.context.append_basic_block(function, "entry");
        self.builder.position_at_end(entry_block);

        let ctx_ptr = function.get_nth_param(0).unwrap().into_pointer_value();
        let word_ptr = function.get_nth_param(1).unwrap().into_pointer_value();

        // Load the word value from the pointer
        let word_value = self
            .builder
            .build_load(self.types.i256, word_ptr, "word.value")
            .unwrap()
            .into_int_value();

        // Load stack pointer (field 0)
        let stack_ptr_addr = self
            .builder
            .build_struct_gep(self.types.exec_ctx, ctx_ptr, 0, "stack.ptr.addr")
            .unwrap();
        let stack_ptr = self
            .builder
            .build_load(self.types.i32, stack_ptr_addr, "stack.ptr")
            .unwrap()
            .into_int_value();

        // Get address of stack[stack_ptr] (field 5 is stack array)
        let stack_field_ptr = self
            .builder
            .build_struct_gep(self.types.exec_ctx, ctx_ptr, 5, "stack.field")
            .unwrap();

        let stack_top_addr = unsafe {
            self.builder
                .build_gep(
                    self.types.i256,
                    stack_field_ptr,
                    &[stack_ptr],
                    "stack.top.addr",
                )
                .unwrap()
        };

        // Store the word
        self.builder
            .build_store(stack_top_addr, word_value)
            .unwrap();

        // Increment stack pointer
        let stack_ptr_next = self
            .builder
            .build_int_add(
                stack_ptr,
                self.types.i32.const_int(1, false),
                "stack.ptr.next",
            )
            .unwrap();
        self.builder
            .build_store(stack_ptr_addr, stack_ptr_next)
            .unwrap();

        self.builder
            .build_return(Some(&self.context.bool_type().const_int(1, false)))
            .unwrap();
        function
    }

    /// Build jet.stack.pop function in IR.
    /// Pops a word from the stack and returns pointer to it.
    fn build_stack_pop(&self) -> FunctionValue<'ctx> {
        let fn_type = self.types.ptr.fn_type(
            &[self
                .context
                .ptr_type(inkwell::AddressSpace::default())
                .into()],
            false,
        );

        let function = self.module.add_function("jet.stack.pop", fn_type, None);
        let entry_block = self.context.append_basic_block(function, "entry");
        self.builder.position_at_end(entry_block);

        let ctx_ptr = function.get_nth_param(0).unwrap().into_pointer_value();

        // Load stack pointer (field 0)
        let stack_ptr_addr = self
            .builder
            .build_struct_gep(self.types.exec_ctx, ctx_ptr, 0, "stack.ptr.addr")
            .unwrap();
        let stack_ptr = self
            .builder
            .build_load(self.types.i32, stack_ptr_addr, "stack.ptr")
            .unwrap()
            .into_int_value();

        // Bounds check: ensure stack_ptr > 0 before popping
        let is_underflow = self
            .builder
            .build_int_compare(
                inkwell::IntPredicate::EQ,
                stack_ptr,
                self.types.i32.const_zero(),
                "is_underflow",
            )
            .unwrap();

        let underflow_block = self.context.append_basic_block(function, "underflow");
        let valid_block = self.context.append_basic_block(function, "valid");

        self.builder
            .build_conditional_branch(is_underflow, underflow_block, valid_block)
            .unwrap();

        // Underflow case: return null pointer
        self.builder.position_at_end(underflow_block);
        let null_ptr = self.types.ptr.const_null();
        self.builder.build_return(Some(&null_ptr)).unwrap();

        // Valid case: proceed with pop
        self.builder.position_at_end(valid_block);

        // Decrement stack pointer
        let stack_ptr_prev = self
            .builder
            .build_int_sub(
                stack_ptr,
                self.types.i32.const_int(1, false),
                "stack.ptr.prev",
            )
            .unwrap();
        self.builder
            .build_store(stack_ptr_addr, stack_ptr_prev)
            .unwrap();

        // Get address of stack[stack_ptr - 1] (field 5 is stack array)
        let stack_field_ptr = self
            .builder
            .build_struct_gep(self.types.exec_ctx, ctx_ptr, 5, "stack.field")
            .unwrap();

        let stack_top_addr = unsafe {
            self.builder
                .build_gep(
                    self.types.i256,
                    stack_field_ptr,
                    &[stack_ptr_prev],
                    "stack.top.addr",
                )
                .unwrap()
        };

        self.builder.build_return(Some(&stack_top_addr)).unwrap();
        function
    }

    /// Build jet.stack.peek function in IR.
    /// Peeks at a word in the stack without popping it.
    fn build_stack_peek(&self) -> FunctionValue<'ctx> {
        let fn_type = self.types.ptr.fn_type(
            &[
                self.context
                    .ptr_type(inkwell::AddressSpace::default())
                    .into(),
                self.types.i8.into(),
            ],
            false,
        );

        let function = self.module.add_function("jet.stack.peek", fn_type, None);
        let entry_block = self.context.append_basic_block(function, "entry");
        self.builder.position_at_end(entry_block);

        let ctx_ptr = function.get_nth_param(0).unwrap().into_pointer_value();
        let peek_idx = function.get_nth_param(1).unwrap().into_int_value();

        // Extend peek_idx from i8 to i32
        let peek_idx_32 = self
            .builder
            .build_int_z_extend(peek_idx, self.types.i32, "peek.idx.32")
            .unwrap();

        // Load stack pointer (field 0)
        let stack_ptr_addr = self
            .builder
            .build_struct_gep(self.types.exec_ctx, ctx_ptr, 0, "stack.ptr.addr")
            .unwrap();
        let stack_ptr = self
            .builder
            .build_load(self.types.i32, stack_ptr_addr, "stack.ptr")
            .unwrap()
            .into_int_value();

        // Calculate index: stack_ptr - peek_idx - 1
        let idx_temp = self
            .builder
            .build_int_sub(stack_ptr, peek_idx_32, "idx.temp")
            .unwrap();
        let idx = self
            .builder
            .build_int_sub(idx_temp, self.types.i32.const_int(1, false), "idx")
            .unwrap();

        // Get address of stack[idx] (field 5 is stack array)
        let stack_field_ptr = self
            .builder
            .build_struct_gep(self.types.exec_ctx, ctx_ptr, 5, "stack.field")
            .unwrap();

        let stack_elem_addr = unsafe {
            self.builder
                .build_gep(self.types.i256, stack_field_ptr, &[idx], "stack.elem.addr")
                .unwrap()
        };

        self.builder.build_return(Some(&stack_elem_addr)).unwrap();
        function
    }

    /// Build jet.stack.swap function in IR.
    /// Swaps the top word with the word at the given index.
    fn build_stack_swap(&self) -> FunctionValue<'ctx> {
        let fn_type = self.context.bool_type().fn_type(
            &[
                self.context
                    .ptr_type(inkwell::AddressSpace::default())
                    .into(),
                self.types.i8.into(),
            ],
            false,
        );

        let function = self.module.add_function("jet.stack.swap", fn_type, None);
        let entry_block = self.context.append_basic_block(function, "entry");
        self.builder.position_at_end(entry_block);

        let ctx_ptr = function.get_nth_param(0).unwrap().into_pointer_value();
        let swap_idx = function.get_nth_param(1).unwrap().into_int_value();

        // Extend swap_idx from i8 to i32
        let swap_idx_32 = self
            .builder
            .build_int_z_extend(swap_idx, self.types.i32, "swap.idx.32")
            .unwrap();

        // Load stack pointer (field 0)
        let stack_ptr_addr = self
            .builder
            .build_struct_gep(self.types.exec_ctx, ctx_ptr, 0, "stack.ptr.addr")
            .unwrap();
        let stack_ptr = self
            .builder
            .build_load(self.types.i32, stack_ptr_addr, "stack.ptr")
            .unwrap()
            .into_int_value();

        // Calculate top_idx: stack_ptr - 1
        let top_idx = self
            .builder
            .build_int_sub(stack_ptr, self.types.i32.const_int(1, false), "top.idx")
            .unwrap();

        // Calculate swap_with_idx: stack_ptr - 2 - swap_idx
        let temp = self
            .builder
            .build_int_sub(stack_ptr, self.types.i32.const_int(2, false), "temp")
            .unwrap();
        let swap_with_idx = self
            .builder
            .build_int_sub(temp, swap_idx_32, "swap.with.idx")
            .unwrap();

        // Get address of stack array (field 5)
        let stack_field_ptr = self
            .builder
            .build_struct_gep(self.types.exec_ctx, ctx_ptr, 5, "stack.field")
            .unwrap();

        // Get address of top element
        let top_addr = unsafe {
            self.builder
                .build_gep(self.types.i256, stack_field_ptr, &[top_idx], "top.addr")
                .unwrap()
        };

        // Get address of swap element
        let swap_addr = unsafe {
            self.builder
                .build_gep(
                    self.types.i256,
                    stack_field_ptr,
                    &[swap_with_idx],
                    "swap.addr",
                )
                .unwrap()
        };

        // Load both values
        let top_val = self
            .builder
            .build_load(self.types.i256, top_addr, "top.val")
            .unwrap()
            .into_int_value();
        let swap_val = self
            .builder
            .build_load(self.types.i256, swap_addr, "swap.val")
            .unwrap()
            .into_int_value();

        // Swap them
        self.builder.build_store(top_addr, swap_val).unwrap();
        self.builder.build_store(swap_addr, top_val).unwrap();

        self.builder
            .build_return(Some(&self.context.bool_type().const_int(1, false)))
            .unwrap();
        function
    }

    /// Build jet.mem.load function in IR.
    /// Loads a word from memory at the given offset and returns the value (not a pointer).
    /// This prevents UAF issues when memory is reallocated.
    fn build_mem_load(&self) -> FunctionValue<'ctx> {
        let fn_type = self.types.i256.fn_type(
            &[
                self.context
                    .ptr_type(inkwell::AddressSpace::default())
                    .into(),
                self.types.ptr.into(),
            ],
            false,
        );

        let function = self.module.add_function("jet.mem.load", fn_type, None);
        let entry_block = self.context.append_basic_block(function, "entry");
        self.builder.position_at_end(entry_block);

        let ctx_ptr = function.get_nth_param(0).unwrap().into_pointer_value();
        let loc_ptr = function.get_nth_param(1).unwrap().into_pointer_value();

        // Load the location value
        let loc = self
            .builder
            .build_load(self.types.i32, loc_ptr, "loc")
            .unwrap()
            .into_int_value();

        // Get memory_ptr (field 6)
        let mem_ptr_addr = self
            .builder
            .build_struct_gep(self.types.exec_ctx, ctx_ptr, 6, "mem.ptr.addr")
            .unwrap();
        let mem_ptr = self
            .builder
            .build_load(self.types.ptr, mem_ptr_addr, "mem.ptr")
            .unwrap()
            .into_pointer_value();

        // Calculate byte offset in memory
        let byte_ptr = unsafe {
            self.builder
                .build_gep(self.types.i8, mem_ptr, &[loc], "byte.ptr")
                .unwrap()
        };

        // Load the i256 value from memory and return it directly
        // This prevents UAF by not returning a pointer that could become dangling
        let value = self
            .builder
            .build_load(self.types.i256, byte_ptr, "mem.value")
            .unwrap()
            .into_int_value();

        self.builder.build_return(Some(&value)).unwrap();
        function
    }

    /// Build jet.mem.store.word function in IR.
    /// Stores a word to memory at the given offset.
    fn build_mem_store_word(&self) -> FunctionValue<'ctx> {
        let fn_type = self.types.i8.fn_type(
            &[
                self.context
                    .ptr_type(inkwell::AddressSpace::default())
                    .into(),
                self.types.ptr.into(),
                self.types.ptr.into(),
            ],
            false,
        );

        let function = self
            .module
            .add_function("jet.mem.store.word", fn_type, None);
        let entry_block = self.context.append_basic_block(function, "entry");
        self.builder.position_at_end(entry_block);

        let ctx_ptr = function.get_nth_param(0).unwrap().into_pointer_value();
        let loc_ptr = function.get_nth_param(1).unwrap().into_pointer_value();
        let val_ptr = function.get_nth_param(2).unwrap().into_pointer_value();

        // Load location
        let loc = self
            .builder
            .build_load(self.types.i32, loc_ptr, "loc")
            .unwrap()
            .into_int_value();

        // Get memory_ptr (field 6)
        let mem_ptr_addr = self
            .builder
            .build_struct_gep(self.types.exec_ctx, ctx_ptr, 6, "mem.ptr.addr")
            .unwrap();
        let mem_ptr = self
            .builder
            .build_load(self.types.ptr, mem_ptr_addr, "mem.ptr")
            .unwrap()
            .into_pointer_value();

        // Calculate destination address
        let dest_ptr = unsafe {
            self.builder
                .build_gep(self.types.i8, mem_ptr, &[loc], "dest.ptr")
                .unwrap()
        };

        // Copy 32 bytes from val_ptr to dest_ptr
        let word_size = self.types.i32.const_int(32, false);
        self.builder
            .build_memcpy(dest_ptr, 1, val_ptr, 1, word_size)
            .unwrap();

        self.builder
            .build_return(Some(&self.types.i8.const_int(0, false)))
            .unwrap();
        function
    }

    /// Build jet.mem.store.byte function in IR.
    /// Stores a single byte to memory at the given offset.
    fn build_mem_store_byte(&self) -> FunctionValue<'ctx> {
        let fn_type = self.types.i8.fn_type(
            &[
                self.context
                    .ptr_type(inkwell::AddressSpace::default())
                    .into(),
                self.types.ptr.into(),
                self.types.ptr.into(),
            ],
            false,
        );

        let function = self
            .module
            .add_function("jet.mem.store.byte", fn_type, None);
        let entry_block = self.context.append_basic_block(function, "entry");
        self.builder.position_at_end(entry_block);

        let ctx_ptr = function.get_nth_param(0).unwrap().into_pointer_value();
        let loc_ptr = function.get_nth_param(1).unwrap().into_pointer_value();
        let val_ptr = function.get_nth_param(2).unwrap().into_pointer_value();

        // Load location
        let loc = self
            .builder
            .build_load(self.types.i32, loc_ptr, "loc")
            .unwrap()
            .into_int_value();

        // Load byte value
        let byte_val = self
            .builder
            .build_load(self.types.i8, val_ptr, "byte.val")
            .unwrap()
            .into_int_value();

        // Get memory_ptr (field 6)
        let mem_ptr_addr = self
            .builder
            .build_struct_gep(self.types.exec_ctx, ctx_ptr, 6, "mem.ptr.addr")
            .unwrap();
        let mem_ptr = self
            .builder
            .build_load(self.types.ptr, mem_ptr_addr, "mem.ptr")
            .unwrap()
            .into_pointer_value();

        // Calculate destination address
        let dest_ptr = unsafe {
            self.builder
                .build_gep(self.types.i8, mem_ptr, &[loc], "dest.ptr")
                .unwrap()
        };

        // Store the byte
        self.builder.build_store(dest_ptr, byte_val).unwrap();

        self.builder
            .build_return(Some(&self.types.i8.const_int(0, false)))
            .unwrap();
        function
    }

    /// Get the generated module (consumes self).
    pub fn into_module(self) -> Module<'ctx> {
        self.module
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_builder_creates_module() {
        let context = Context::create();
        let builder = RuntimeBuilder::new(&context, "test_runtime");
        let module = builder.build();

        // Verify the module has expected functions
        assert!(module.get_function("jet.stack.push.i256").is_some());
        assert!(module.get_function("jet.stack.push.ptr").is_some());
        assert!(module.get_function("jet.stack.pop").is_some());
        assert!(module.get_function("jet.stack.peek").is_some());
        assert!(module.get_function("jet.stack.swap").is_some());
        assert!(module.get_function("jet.mem.load").is_some());
        assert!(module.get_function("jet.mem.store.word").is_some());
        assert!(module.get_function("jet.mem.store.byte").is_some());
    }

    #[test]
    fn test_runtime_builder_generates_valid_ir() {
        let context = Context::create();
        let builder = RuntimeBuilder::new(&context, "test_runtime");
        let module = builder.build();

        // Verify the module is valid
        assert!(module.verify().is_ok());
    }
}
