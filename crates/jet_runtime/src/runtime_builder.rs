/// Runtime builder - generates LLVM IR for runtime functions.

use inkwell::{
    builder::Builder,
    context::Context,
    module::Module,
    values::FunctionValue,
};
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
        // Declare external Rust builtin functions
        self.declare_rust_builtins();

        // Generate IR-based runtime functions
        self.build_stack_push_i256();

        self.module
    }

    /// Declare forward references to Rust builtin functions.
    /// These are implemented in builtins.rs and linked at runtime.
    fn declare_rust_builtins(&self) {
        // Stack operations
        self.module.add_function(
            "jet.stack.push.word",
            self.context.bool_type().fn_type(
                &[self.types.ptr.into(), self.types.i256.into()],
                false,
            ),
            None,
        );

        self.module.add_function(
            "jet.stack.push.ptr",
            self.context.bool_type().fn_type(
                &[self.types.ptr.into(), self.types.ptr.into()],
                false,
            ),
            None,
        );

        self.module.add_function(
            "jet.stack.pop",
            self.types.ptr.fn_type(&[self.types.ptr.into()], false),
            None,
        );

        self.module.add_function(
            "jet.stack.peek",
            self.types.ptr.fn_type(
                &[self.types.ptr.into(), self.types.i8.into()],
                false,
            ),
            None,
        );

        self.module.add_function(
            "jet.stack.swap",
            self.context.bool_type().fn_type(
                &[self.types.ptr.into(), self.types.i8.into()],
                false,
            ),
            None,
        );

        // Memory operations
        self.module.add_function(
            "jet.mem.store.word",
            self.types.i8.fn_type(
                &[
                    self.types.ptr.into(),
                    self.types.ptr.into(),
                    self.types.ptr.into(),
                ],
                false,
            ),
            None,
        );

        self.module.add_function(
            "jet.mem.store.byte",
            self.types.i8.fn_type(
                &[
                    self.types.ptr.into(),
                    self.types.ptr.into(),
                    self.types.ptr.into(),
                ],
                false,
            ),
            None,
        );

        self.module.add_function(
            "jet.mem.load",
            self.types.ptr.fn_type(
                &[self.types.ptr.into(), self.types.ptr.into()],
                false,
            ),
            None,
        );

        // Contract operations
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
    }

    /// Build jet.stack.push.i256 function in IR.
    /// This function pushes an i256 value onto the EVM stack.
    fn build_stack_push_i256(&self) -> FunctionValue<'ctx> {
        let fn_type = self.context.bool_type().fn_type(
            &[self.types.exec_ctx.ptr_type(inkwell::AddressSpace::default()).into(), self.types.i256.into()],
            false,
        );
        
        let function = self.module.add_function("jet.stack.push.i256", fn_type, None);
        let entry_block = self.context.append_basic_block(function, "entry");
        
        self.builder.position_at_end(entry_block);

        // Get function parameters
        let ctx_ptr = function.get_nth_param(0).unwrap().into_pointer_value();
        let value = function.get_nth_param(1).unwrap().into_int_value();

        // Load stack pointer (field 0)
        let stack_ptr_addr = self.builder.build_struct_gep(
            self.types.exec_ctx,
            ctx_ptr,
            0,
            "stack.ptr.addr"
        ).unwrap();
        let stack_ptr = self.builder.build_load(
            self.types.i32,
            stack_ptr_addr,
            "stack.ptr"
        ).unwrap().into_int_value();

        // Get address of stack[stack_ptr] (field 5 is stack array)
        let stack_field_ptr = self.builder.build_struct_gep(
            self.types.exec_ctx,
            ctx_ptr,
            5,
            "stack.field"
        ).unwrap();
        
        // Index into the stack array
        let stack_top_addr = unsafe {
            self.builder.build_gep(
                self.types.i256,
                stack_field_ptr,
                &[stack_ptr],
                "stack.top.addr"
            ).unwrap()
        };

        // Store the value
        self.builder.build_store(stack_top_addr, value).unwrap();

        // Increment stack pointer
        let stack_ptr_next = self.builder.build_int_add(
            stack_ptr,
            self.types.i32.const_int(1, false),
            "stack.ptr.next"
        ).unwrap();
        self.builder.build_store(stack_ptr_addr, stack_ptr_next).unwrap();

        // Return true
        self.builder.build_return(Some(&self.context.bool_type().const_int(1, false))).unwrap();

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
        assert!(module.get_function("jet.stack.push.word").is_some());
        assert!(module.get_function("jet.mem.load").is_some());
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
