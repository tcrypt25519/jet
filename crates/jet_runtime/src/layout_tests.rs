/// Memory layout verification tests.
/// Ensures consistency between Rust Context struct and LLVM IR exec_ctx type.
#[cfg(test)]
mod layout_verification_tests {
    use crate::exec::Context;
    use inkwell::context::Context as LLVMContext;
    use inkwell::types::AnyTypeEnum;
    use jet_ir::Types;
    use std::mem;

    /// Field indices for exec_ctx structure
    #[derive(Debug, Clone, Copy)]
    #[repr(u32)]
    enum ExecCtxField {
        StackPtr = 0,
        JumpPtr = 1,
        ReturnOffset = 2,
        ReturnLength = 3,
        SubCall = 4,
        Stack = 5,
        MemoryPtr = 6,
        MemoryLen = 7,
        MemoryCap = 8,
    }

    impl ExecCtxField {
        const FIELD_COUNT: u32 = 9;

        fn index(self) -> u32 {
            self as u32
        }

        fn expected_type_kind(self) -> TypeKind {
            match self {
                ExecCtxField::StackPtr => TypeKind::Int,
                ExecCtxField::JumpPtr => TypeKind::Int,
                ExecCtxField::ReturnOffset => TypeKind::Int,
                ExecCtxField::ReturnLength => TypeKind::Int,
                ExecCtxField::SubCall => TypeKind::Pointer,
                ExecCtxField::Stack => TypeKind::Array,
                ExecCtxField::MemoryPtr => TypeKind::Pointer,
                ExecCtxField::MemoryLen => TypeKind::Int,
                ExecCtxField::MemoryCap => TypeKind::Int,
            }
        }
    }

    #[derive(Debug, PartialEq)]
    enum TypeKind {
        Int,
        Pointer,
        Array,
    }

    fn get_type_kind(ty: AnyTypeEnum) -> TypeKind {
        if ty.is_int_type() {
            TypeKind::Int
        } else if ty.is_pointer_type() {
            TypeKind::Pointer
        } else if ty.is_array_type() {
            TypeKind::Array
        } else {
            panic!("Unexpected type kind: {:?}", ty)
        }
    }

    #[test]
    fn test_context_struct_size() {
        let context_size = mem::size_of::<Context>();

        // Expected: 4 + 4 + 4 + 4 + 8 + (1024 * 32) + 8 + 4 + 4 = 32,808 bytes
        // Note: Actual size may be larger due to alignment padding
        const MIN_EXPECTED_SIZE: usize = 32_808;

        assert!(
            context_size >= MIN_EXPECTED_SIZE,
            "Context size {} is less than expected minimum {}",
            context_size,
            MIN_EXPECTED_SIZE
        );
    }

    #[test]
    fn test_llvm_exec_ctx_field_count() {
        let llvm_context = LLVMContext::create();
        let types = Types::new(&llvm_context);

        let field_count = types.exec_ctx.count_fields();
        assert_eq!(
            field_count,
            ExecCtxField::FIELD_COUNT,
            "exec_ctx should have {} fields, got {}",
            ExecCtxField::FIELD_COUNT,
            field_count
        );
    }

    #[test]
    fn test_exec_ctx_field_types() {
        let llvm_context = LLVMContext::create();
        let types = Types::new(&llvm_context);

        // Test each field has the expected type
        let fields_to_test = [
            ExecCtxField::StackPtr,
            ExecCtxField::JumpPtr,
            ExecCtxField::ReturnOffset,
            ExecCtxField::ReturnLength,
            ExecCtxField::SubCall,
            ExecCtxField::Stack,
            ExecCtxField::MemoryPtr,
            ExecCtxField::MemoryLen,
            ExecCtxField::MemoryCap,
        ];

        for field in fields_to_test {
            let field_type = types
                .exec_ctx
                .get_field_type_at_index(field.index())
                .unwrap_or_else(|| panic!("Failed to get field type for {:?}", field));

            let actual_kind = get_type_kind(match field_type {
                inkwell::types::BasicTypeEnum::ArrayType(t) => t.into(),
                inkwell::types::BasicTypeEnum::FloatType(t) => t.into(),
                inkwell::types::BasicTypeEnum::IntType(t) => t.into(),
                inkwell::types::BasicTypeEnum::PointerType(t) => t.into(),
                inkwell::types::BasicTypeEnum::StructType(t) => t.into(),
                inkwell::types::BasicTypeEnum::VectorType(t) => t.into(),
            });
            let expected_kind = field.expected_type_kind();

            assert_eq!(
                actual_kind,
                expected_kind,
                "Field {:?} (index {}) has wrong type: expected {:?}, got {:?}",
                field,
                field.index(),
                expected_kind,
                actual_kind
            );
        }
    }

    #[test]
    fn test_stack_field_details() {
        let llvm_context = LLVMContext::create();
        let types = Types::new(&llvm_context);

        let stack_field = types
            .exec_ctx
            .get_field_type_at_index(ExecCtxField::Stack.index())
            .unwrap();

        assert!(
            stack_field.is_array_type(),
            "Stack field should be array type"
        );

        let stack_array = stack_field.into_array_type();
        assert_eq!(stack_array.len(), 1024, "Stack should have 1024 elements");

        let element_type = stack_array.get_element_type();
        assert!(
            element_type.is_int_type(),
            "Stack elements should be int type"
        );

        let element_int = element_type.into_int_type();
        assert_eq!(
            element_int.get_bit_width(),
            256,
            "Stack elements should be i256"
        );
    }
}
