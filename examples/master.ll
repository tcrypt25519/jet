; ModuleID = 'runtime-ir/jet.ll'
source_filename = "jet.ll"
target datalayout = "e-m:o-p270:32:32-p271:32:32-p272:64:64-i64:64-n8:16:32:64-S128"
target triple = "x86_64-apple-macosx14.0.0"

%jet.types.exec_ctx = type <{ i32, i32, i32, i32, ptr, [1024 x i256], [1024 x i8], i32, i32 }>

@jet.jit_engine = external global ptr

declare i8 @jet.contracts.lookup(ptr, ptr, ptr)

declare ptr @jet.contracts.new_sub_ctx()

declare i8 @jet.contracts.call_return_data_copy(ptr, ptr, i32, i32, i32)

declare i8 @jet.ops.keccak256(ptr)

; Function Attrs: alwaysinline nounwind
define i1 @jet.stack.push.word(ptr %0, i256 %1) #0 {
entry:
    %stack.ptr.addr = getelementptr inbounds %jet.types.exec_ctx, ptr %0, i32 0, i32 0
    %stack.ptr = load i32, ptr %stack.ptr.addr, align 4
    %stack.top.addr = getelementptr inbounds %jet.types.exec_ctx, ptr %0, i32 0, i32 5, i32 %stack.ptr
    store i256 %1, ptr %stack.top.addr, align 8
    %stack.ptr.next = add i32 %stack.ptr, 1
    store i32 %stack.ptr.next, ptr %stack.ptr.addr, align 4
    ret i1 true
}

; Function Attrs: alwaysinline nounwind
define i1 @jet.stack.push.bytes(ptr %0, [32 x i8] %1) #0 {
entry:
    %stack_bytes_ptr = alloca [32 x i8], align 1
    store [32 x i8] %1, ptr %stack_bytes_ptr, align 1
    %stack_word = load i256, ptr %stack_bytes_ptr, align 8
    %result = call i1 @jet.stack.push.word(ptr %0, i256 %stack_word)
    ret i1 %result
}

; Function Attrs: alwaysinline nounwind
define i256 @jet.stack.pop(ptr %0) #0 {
entry:
    %stack.ptr.addr = getelementptr inbounds %jet.types.exec_ctx, ptr %0, i32 0, i32 0
    %stack.ptr = load i32, ptr %stack.ptr.addr, align 4
    %stack.ptr.sub_1 = sub i32 %stack.ptr, 1
    %stack.top.addr = getelementptr inbounds %jet.types.exec_ctx, ptr %0, i32 0, i32 5, i32 %stack.ptr.sub_1
    %stack_word = load i256, ptr %stack.top.addr, align 8
    store i32 %stack.ptr.sub_1, ptr %stack.ptr.addr, align 4
    ret i256 %stack_word
}

; Function Attrs: alwaysinline nounwind
define i256 @jet.stack.peek(ptr %0, i8 %peek_idx) #0 {
entry:
    %stack.ptr.addr = getelementptr inbounds %jet.types.exec_ctx, ptr %0, i32 0, i32 0
    %stack.ptr = load i32, ptr %stack.ptr.addr, align 4
    %peek_idx.i32 = zext i8 %peek_idx to i32
    %stack.peek.ptr = sub i32 %stack.ptr, %peek_idx.i32
    %stack.peek.addr = getelementptr inbounds %jet.types.exec_ctx, ptr %0, i32 0, i32 5, i32 %stack.peek.ptr
    %stack_word = load i256, ptr %stack.peek.addr, align 8
    ret i256 %stack_word
}

; Function Attrs: alwaysinline nounwind
define i1 @jet.stack.swap(ptr %0, i8 %swap.idx) #0 {
entry:
    %stack.ptr.addr = getelementptr inbounds %jet.types.exec_ctx, ptr %0, i32 0, i32 0
    %stack.ptr = load i32, ptr %stack.ptr.addr, align 4
    %stack.ptr.sub_1 = sub i32 %stack.ptr, 1
    %stack.top.addr = getelementptr inbounds %jet.types.exec_ctx, ptr %0, i32 0, i32 5, i32 %stack.ptr.sub_1
    %top_word = load i256, ptr %stack.top.addr, align 8
    %swap.idx.i32 = zext i8 %swap.idx to i32
    %stack.swap.idx = sub i32 %stack.ptr.sub_1, %swap.idx.i32
    %stack.swap.addr = getelementptr inbounds %jet.types.exec_ctx, ptr %0, i32 0, i32 5, i32 %stack.swap.idx
    %swap_word = load i256, ptr %stack.swap.addr, align 8
    store i256 %top_word, ptr %stack.swap.addr, align 8
    store i256 %swap_word, ptr %stack.top.addr, align 8
    ret i1 true
}

; Function Attrs: alwaysinline nounwind
define i8 @jet.mem.store.word(ptr %ctx, i256 %loc, i256 %val) #0 {
entry:
    %loc_i32 = trunc i256 %loc to i32
    %mem = getelementptr inbounds %jet.types.exec_ctx, ptr %ctx, i32 0, i32 6
    %mem_loc_ptr = getelementptr inbounds [1024 x i8], ptr %mem, i32 0, i32 %loc_i32
    store i256 %val, ptr %mem_loc_ptr, align 1
    ret i8 0
}

; Function Attrs: alwaysinline nounwind
define i8 @jet.mem.store.byte(ptr %ctx, i256 %loc, i256 %val) #0 {
entry:
    %loc_i32 = trunc i256 %loc to i32
    %val_i8 = trunc i256 %val to i8
    %mem = getelementptr inbounds %jet.types.exec_ctx, ptr %ctx, i32 0, i32 6
    %mem_loc_ptr = getelementptr inbounds [1024 x i8], ptr %mem, i32 0, i32 %loc_i32
    store i8 %val_i8, ptr %mem_loc_ptr, align 1
    ret i8 0
}

; Function Attrs: alwaysinline nounwind
define i256 @jet.mem.load(ptr %ctx, i256 %loc) #0 {
entry:
    %loc_i32 = trunc i256 %loc to i32
    %mem = getelementptr inbounds %jet.types.exec_ctx, ptr %ctx, i32 0, i32 6
    %mem_loc_ptr = getelementptr inbounds [1024 x i8], ptr %mem, i32 0, i32 %loc_i32
    %val = load i256, ptr %mem_loc_ptr, align 1
    ret i256 %val
}

; Function Attrs: alwaysinline nounwind
define i8 @jet.contracts.call(ptr %caller_ctx, ptr %callee_ctx, i160 %addr, i32 %ret.dest, i32 %ret.len) #0 {
entry:
    %addr_i160_ptr = alloca i160, align 8
    store i160 %addr, ptr %addr_i160_ptr, align 8
    %addr_bytes = bitcast ptr %addr_i160_ptr to ptr
    %fn_ptr_addr = alloca ptr, align 8
    %lookup_result = call i8 @jet.contracts.lookup(ptr @jet.jit_engine, ptr %fn_ptr_addr, ptr %addr_bytes)
    %success = icmp eq i8 %lookup_result, 0
    br i1 %success, label %invoke_fn, label %return

invoke_fn:                                        ; preds = %entry
    %fn_ptr = load ptr, ptr %fn_ptr_addr, align 8
    %typed_fn_ptr = bitcast ptr %fn_ptr to ptr
    %result = call i8 %typed_fn_ptr(ptr %callee_ctx)
    br i1 %success, label %set_return_info, label %return

set_return_info:                                  ; preds = %invoke_fn
    %caller.sub_ctx.addr = getelementptr inbounds %jet.types.exec_ctx, ptr %caller_ctx, i32 0, i32 4
    store ptr %callee_ctx, ptr %caller.sub_ctx.addr, align 8
    %callee.return.len.addr = getelementptr inbounds %jet.types.exec_ctx, ptr %callee_ctx, i32 0, i32 3
    %callee.return.len = load i32, ptr %callee.return.len.addr, align 4
    %callee.return.empty = icmp eq i32 %callee.return.len, 0
    br i1 %callee.return.empty, label %return, label %copy_return_data

copy_return_data:                                 ; preds = %set_return_info
    %copy.ret = call i8 @jet.contracts.call_return_data_copy(ptr %caller_ctx, ptr %callee_ctx, i32 %ret.dest, i32 0, i32 %ret.len)
    br label %return

return:                                           ; preds = %copy_return_data, %set_return_info, %invoke_fn, %entry
    %r = phi i8 [ %copy.ret, %copy_return_data ], [ 0, %set_return_info ], [ 2, %invoke_fn ], [ 1, %entry ]
    ret i8 %r
}

define i8 @jet.contracts.0x1234(ptr %0, ptr %1) {
preamble:
    %jump_ptr = getelementptr inbounds <{ i32, i32, i32, i32, ptr, [1024 x i256], <{ ptr, i32, i32 }> }>, ptr %0, i32 0, i32 1
    %return_offset = getelementptr inbounds <{ i32, i32, i32, i32, ptr, [1024 x i256], <{ ptr, i32, i32 }> }>, ptr %0, i32 0, i32 2
    %return_length = getelementptr inbounds <{ i32, i32, i32, i32, ptr, [1024 x i256], <{ ptr, i32, i32 }> }>, ptr %0, i32 0, i32 3
    %sub_call = getelementptr inbounds <{ i32, i32, i32, i32, ptr, [1024 x i256], <{ ptr, i32, i32 }> }>, ptr %0, i32 0, i32 4
    br label %block

block:                                            ; preds = %preamble
    %stack_push_bytes = call i1 @jet.stack.push.bytes(ptr %0, [32 x i8] c"\00\FF\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00")
    %stack_push_bytes1 = call i1 @jet.stack.push.bytes(ptr %0, [32 x i8] c"\FF\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00")
    %stack_pop_word_a = call i256 @jet.stack.pop(ptr %0)
    %stack_pop_word_a2 = call i256 @jet.stack.pop(ptr %0)
    %add_result = add i256 %stack_pop_word_a, %stack_pop_word_a2
    %stack_push_word = call i1 @jet.stack.push.word(ptr %0, i256 %add_result)
    %stack_push_bytes3 = call i1 @jet.stack.push.bytes(ptr %0, [32 x i8] c"\01\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00")
    %stack_pop_word_a4 = call i256 @jet.stack.pop(ptr %0)
    %stack_pop_word_a5 = call i256 @jet.stack.pop(ptr %0)
    %add_result6 = add i256 %stack_pop_word_a4, %stack_pop_word_a5
    %stack_push_word7 = call i1 @jet.stack.push.word(ptr %0, i256 %add_result6)
    ret i8 0
}

attributes #0 = { alwaysinline nounwind }

