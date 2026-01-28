target datalayout = "e-m:o-p270:32:32-p271:32:32-p272:64:64-i64:64-n8:16:32:64-S128"
target triple = "x86_64-apple-macosx14.0.0"

;
; Globals
;
@jet.jit_engine = external global ptr

;
; Declarations of Rust-defined functions
;

declare i8 @jet.contracts.lookup(ptr, ptr, i8*)
declare %jet.types.exec_ctx* @jet.contracts.new_sub_ctx ()
declare i8 @jet.contracts.call_return_data_copy(ptr, ptr, i32, i32, i32)
declare i8 @jet.ops.keccak256(ptr)

;
; Types
;
%jet.types.word = type [32 x i8]

%jet.types.mem_buf = type [1024 x i8]

%jet.types.mem = type <{
  ;%jet.types.mem_buf *, ; data
  %jet.types.mem_buf, ; data
  i32, ; length
  i32 ; capacity
}>

%jet.types.block_info = type <{
  %jet.types.word, ; balance
  %jet.types.word, ; gas price
  i160, ; address
  i32,  ; code size
  i160  ; origin
}>

%jet.types.call_info = type <{}>

%jet.types.exec_ctx = type <{
  i32, ; stack.ptr
  i32, ; jump_ptr
  i32, ; return offset
  i32, ; return length
  ptr,; sub call ctx
  [1024 x %jet.types.word], ; stack

  [1024 x i8], ; memory
  i32, ; mem length
  i32 ; mem capacity
}>

%jet.types.contract_fn = type i8(%jet.types.exec_ctx*)

;
; Runtime functions
;
attributes #0 = { alwaysinline nounwind }


define i1 @jet.stack.push.ptr (%jet.types.exec_ctx*, %jet.types.word*) #0 {
entry:
  ; Load stack pointer
  %stack.ptr.addr = getelementptr inbounds %jet.types.exec_ctx, ptr %0, i32 0, i32 0
  %stack.ptr = load i32, ptr %stack.ptr.addr
  %stack.top.addr = getelementptr inbounds %jet.types.exec_ctx, ptr %0, i32 0, i32 5, i32 %stack.ptr

  ; TODO: Check if we'll break the stack

  ; Store word
  ; store %jet.types.word %1, ptr %stack.top.addr
  ; call @llvm.memcpy.p0i8.p0i8.i64(i8* %stack.top.addr, i8* %1, i64 32, i32 1, i1 false)
  call void @llvm.memcpy.inline.p0.p0.i64(ptr %stack.top.addr, ptr %1, i64 32, i1 false)

  ; Increment stack pointer
  %stack.ptr.next = add i32 %stack.ptr, 1
  store i32 %stack.ptr.next, ptr %stack.ptr.addr

  ret i1 true
}

define %jet.types.word* @jet.stack.pop (%jet.types.exec_ctx* %0) #0 {
entry:
  ; Load stack pointer
  %stack.ptr.addr = getelementptr inbounds %jet.types.exec_ctx, ptr %0, i32 0, i32 0
  %stack.ptr = load i32, ptr %stack.ptr.addr, align 4
  %stack.ptr.sub_1 = sub i32 %stack.ptr, 1
  %stack.top.addr = getelementptr inbounds %jet.types.exec_ctx, ptr %0, i32 0, i32 5, i32 %stack.ptr.sub_1

  ; Load word
  ; %stack_word = load %jet.types.word, ptr %stack.top.addr, align 8

  ; Decrement stack pointer
  store i32 %stack.ptr.sub_1, ptr %stack.ptr.addr, align 4

  ; ret %jet.types.word %stack_word
  ret %jet.types.word* %stack.top.addr
}

define %jet.types.word* @jet.stack.peek (%jet.types.exec_ctx* %0, i8 %peek_idx) #0 {
entry:
  ; Load stack pointer and subtract the peek index
  %stack.ptr.addr = getelementptr inbounds %jet.types.exec_ctx, ptr %0, i32 0, i32 0
  %stack.ptr = load i32, ptr %stack.ptr.addr, align 4

  %peek_idx.i32 = zext i8 %peek_idx to i32
  %stack.peek.ptr = sub i32 %stack.ptr, %peek_idx.i32
  %stack.peek.addr = getelementptr inbounds %jet.types.exec_ctx, ptr %0, i32 0, i32 5, i32 %stack.peek.ptr 

  ; Load word
  ; %stack_word = load %jet.types.word, ptr %stack.peek.addr
  ; ret %jet.types.word %stack_word
  ret %jet.types.word* %stack.peek.addr
}


define i1 @jet.stack.swap (%jet.types.exec_ctx* %0, i8 %swap.idx) #0 {
entry:
  ; Load top of stack
  %stack.ptr.addr = getelementptr inbounds %jet.types.exec_ctx, ptr %0, i32 0, i32 0
  %stack.ptr = load i32, ptr %stack.ptr.addr
  %stack.ptr.sub_1 = sub i32 %stack.ptr, 1
  %stack.top.addr = getelementptr inbounds %jet.types.exec_ctx, ptr %0, i32 0, i32 5, i32 %stack.ptr.sub_1
  %top_word = load %jet.types.word, ptr %stack.top.addr

  ; Load swap word
  %swap.idx.i32 = zext i8 %swap.idx to i32
  %stack.swap.idx = sub i32 %stack.ptr.sub_1, %swap.idx.i32
  %stack.swap.addr = getelementptr inbounds %jet.types.exec_ctx, ptr %0, i32 0, i32 5, i32 %stack.swap.idx 
  %swap_word = load %jet.types.word, ptr %stack.swap.addr

  ; Store words in each other's place
  store %jet.types.word %top_word, ptr %stack.swap.addr
  store %jet.types.word %swap_word, ptr %stack.top.addr

  ret i1 true
}

define i8 @jet.mem.store.word (%jet.types.exec_ctx* %ctx, %jet.types.word* %loc, %jet.types.word* %val) #0 {
entry:
  %loc_i32 = load i32, ptr %loc
  ; %loc_i32 = trunc i-2-5-6 %loc to i32

  %mem = getelementptr inbounds %jet.types.exec_ctx, ptr %ctx, i32 0, i32 6
  %mem_loc_ptr = getelementptr inbounds %jet.types.mem_buf, ptr %mem, i32 0, i32 %loc_i32

  ; store %jet.types.word %val, ptr %mem_loc_ptr, align 1
  call void @llvm.memcpy.inline.p0.p0.i64(ptr %mem_loc_ptr, ptr %val, i64 32, i1 false)

  ret i8 0
}

define i8 @jet.mem.store.byte (%jet.types.exec_ctx* %ctx, %jet.types.word* %loc, %jet.types.word* %val) #0 {
entry:
  %loc_i32 = load i32, ptr %loc
  ; %loc_i32 = trunc i-2-5-6 %loc to i32
  %val_i8 = load i8, ptr %val
  ; %val_i8 = trunc i-2-5-6 %val to i8

  %mem = getelementptr inbounds %jet.types.exec_ctx, ptr %ctx, i32 0, i32 6
  %mem_loc_ptr = getelementptr inbounds %jet.types.mem_buf, ptr %mem, i32 0, i32 %loc_i32

  store i8 %val_i8, ptr %mem_loc_ptr, align 1
  ret i8 0
}

define %jet.types.word* @jet.mem.load (%jet.types.exec_ctx* %ctx, %jet.types.word* %loc) #0 {
entry:
  %loc_i32 = load i32, ptr %loc
  ; %loc_i32 = trunc i-2-5-6 %loc to i32

  %mem = getelementptr inbounds %jet.types.exec_ctx, ptr %ctx, i32 0, i32 6
  %mem_loc_ptr = getelementptr inbounds %jet.types.mem_buf, ptr %mem, i32 0, i32 %loc_i32

  ; %val = load %jet.types.word, ptr %mem_loc_ptr, align 1
  ; ret %jet.types.word %val
  ret %jet.types.word* %mem_loc_ptr
}

; @jet.contracts.call implements an internal call to another contract.
; Parameters:
;   %caller_ctx - The context of the calling contract
;   %callee_ctx - The context of the called contract
;   %addr - The address of the contract to call
;   %ret.dest - The offset in the caller's memory to copy to
;   %ret.len - The length of the data to copy
; Returns:
;   0 - Success
;   1 - Lookup failed
;   2 - Invocation failed
;   3 - Return data out of callee's return data bounds
;   4 - Return data out of caller's memory bounds
define i8 @jet.contracts.call(%jet.types.exec_ctx* %caller_ctx, %jet.types.exec_ctx* %callee_ctx, i160 %addr, i32 %ret.dest, i32 %ret.len) #0 {
entry:
    ; Convert the 160bit address to an array of 20 bytes
    %addr_i160_ptr = alloca i160, align 8
    store i160 %addr, i160* %addr_i160_ptr
    %addr_bytes = bitcast i160* %addr_i160_ptr to i8*

    ; Lookup the function
    %fn_ptr_addr = alloca i64*, align 8
    %lookup_result = call i8 @jet.contracts.lookup(ptr @jet.jit_engine, ptr %fn_ptr_addr, i8* %addr_bytes)
    %success = icmp eq i8 %lookup_result, 0
    br i1 %success, label %invoke_fn, label %return
invoke_fn:
    ; Load the function
    %fn_ptr = load i8*, i8** %fn_ptr_addr, align 8
    %typed_fn_ptr = bitcast i8* %fn_ptr to %jet.types.contract_fn*

    ; Call the function
    %result = call i8 %typed_fn_ptr(%jet.types.exec_ctx* %callee_ctx)
    br i1 %success, label %set_return_info, label %return
set_return_info:
    %caller.sub_ctx.addr = getelementptr inbounds %jet.types.exec_ctx, ptr %caller_ctx, i32 0, i32 4
    store %jet.types.exec_ctx* %callee_ctx, %jet.types.exec_ctx** %caller.sub_ctx.addr

    %callee.return.len.addr = getelementptr inbounds %jet.types.exec_ctx, ptr %callee_ctx, i32 0, i32 3
    %callee.return.len = load i32, i32* %callee.return.len.addr
    %callee.return.empty = icmp eq i32 %callee.return.len, 0
    br i1 %callee.return.empty, label %return, label %copy_return_data
copy_return_data:
    %copy.ret = call i8 @jet.contracts.call_return_data_copy(ptr %caller_ctx, ptr %callee_ctx, i32 %ret.dest, i32 0, i32 %ret.len)
    br label %return
return:
    %r = phi i8 [%copy.ret, %copy_return_data], [0, %set_return_info], [2, %invoke_fn], [1, %entry]
    ret i8 %r
}

