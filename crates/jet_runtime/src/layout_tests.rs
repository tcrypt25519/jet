/// Memory layout verification tests.
/// Ensures consistency between Rust Context struct and LLVM IR exec_ctx type.

#[cfg(test)]
mod layout_verification_tests {
    use inkwell::context::Context as LLVMContext;
    use jet_ir::Types;
    use crate::exec::Context;
    use std::mem;

    #[test]
    fn test_context_struct_size() {
        // Verify the Rust Context struct has expected size
        let context_size = mem::size_of::<Context>();
        
        // Expected: 4 + 4 + 4 + 4 + 8 + (1024 * 32) + 8 + 4 + 4 = 32,808 bytes
        // Fields: stack_ptr, jump_ptr, return_off, return_len, sub_call, stack, memory_ptr, memory_len, memory_cap
        
        println!("Context struct size: {} bytes", context_size);
        
        // Note: Actual size may be larger due to alignment padding
        // The minimum expected size is 32,808 bytes
        assert!(context_size >= 32_808, "Context size {} is less than expected minimum 32,808", context_size);
    }

    #[test]
    fn test_llvm_exec_ctx_type() {
        // Verify the LLVM exec_ctx type is constructed correctly
        let llvm_context = LLVMContext::create();
        let types = Types::new(&llvm_context);
        
        // Get the exec_ctx type
        let exec_ctx = types.exec_ctx;
        
        // Verify it has the expected number of fields
        let field_count = exec_ctx.count_fields();
        assert_eq!(field_count, 9, "exec_ctx should have 9 fields, got {}", field_count);
        
        println!("exec_ctx field count: {}", field_count);
    }

    #[test]
    fn test_memory_ptr_field_offset() {
        // Verify memory_ptr is at the expected field index (field 6)
        let llvm_context = LLVMContext::create();
        let types = Types::new(&llvm_context);
        
        // Field indices in exec_ctx:
        // 0: stack_ptr (i32)
        // 1: jump_ptr (i32)
        // 2: return_offset (i32)
        // 3: return_length (i32)
        // 4: sub_call (ptr)
        // 5: stack ([1024 x i256])
        // 6: memory_ptr (ptr)
        // 7: memory_len (i32)
        // 8: memory_cap (i32)
        
        let memory_ptr_field = types.exec_ctx.get_field_type_at_index(6).unwrap();
        assert!(memory_ptr_field.is_pointer_type(), "Field 6 should be a pointer type");
        
        let memory_len_field = types.exec_ctx.get_field_type_at_index(7).unwrap();
        assert!(memory_len_field.is_int_type(), "Field 7 should be an int type");
        
        let memory_cap_field = types.exec_ctx.get_field_type_at_index(8).unwrap();
        assert!(memory_cap_field.is_int_type(), "Field 8 should be an int type");
    }

    #[test]
    fn test_stack_field() {
        // Verify stack is an array of i256
        let llvm_context = LLVMContext::create();
        let types = Types::new(&llvm_context);
        
        let stack_field = types.exec_ctx.get_field_type_at_index(5).unwrap();
        assert!(stack_field.is_array_type(), "Field 5 (stack) should be an array type");
        
        let stack_array = stack_field.into_array_type();
        assert_eq!(stack_array.len(), 1024, "Stack array should have 1024 elements");
        
        let element_type = stack_array.get_element_type();
        assert!(element_type.is_int_type(), "Stack element should be int type");
        
        let element_int = element_type.into_int_type();
        assert_eq!(element_int.get_bit_width(), 256, "Stack elements should be i256");
    }
}
