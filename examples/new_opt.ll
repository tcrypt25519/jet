; ModuleID = './examples/new.ll'
source_filename = "jet.ll"
target datalayout = "e-m:o-p270:32:32-p271:32:32-p272:64:64-i64:64-n8:16:32:64-S128"
target triple = "x86_64-apple-macosx14.0.0"

%jet.types.exec_ctx = type <{ i32, i32, i32, i32, ptr, [1024 x [32 x i8]], [1024 x i8], i32, i32 }>

@jet.jit_engine = external global ptr

declare i8 @jet.contracts.lookup(ptr, ptr, ptr) local_unnamed_addr

declare i8 @jet.contracts.call_return_data_copy(ptr, ptr, i32, i32, i32) local_unnamed_addr

; Function Attrs: alwaysinline mustprogress nofree norecurse nosync nounwind willreturn memory(argmem: readwrite)
define noundef i1 @jet.stack.push.i256(ptr nocapture %0, i256 %1) local_unnamed_addr #0 {
entry:
  %stack.ptr = load i32, ptr %0, align 4
  %2 = sext i32 %stack.ptr to i64
  %stack.top.addr = getelementptr inbounds %jet.types.exec_ctx, ptr %0, i64 0, i32 5, i64 %2
  store i256 %1, ptr %stack.top.addr, align 8
  %stack.ptr.next = add i32 %stack.ptr, 1
  store i32 %stack.ptr.next, ptr %0, align 4
  ret i1 true
}

; Function Attrs: alwaysinline mustprogress nofree norecurse nosync nounwind willreturn memory(argmem: readwrite)
define noundef i1 @jet.stack.push.ptr(ptr nocapture %0, ptr nocapture readonly %1) local_unnamed_addr #0 {
entry:
  %stack.ptr = load i32, ptr %0, align 4
  %2 = sext i32 %stack.ptr to i64
  %stack.top.addr = getelementptr inbounds %jet.types.exec_ctx, ptr %0, i64 0, i32 5, i64 %2
  tail call void @llvm.memcpy.inline.p0.p0.i64(ptr nonnull align 1 %stack.top.addr, ptr align 1 %1, i64 32, i1 false)
  %stack.ptr.next = add i32 %stack.ptr, 1
  store i32 %stack.ptr.next, ptr %0, align 4
  ret i1 true
}

; Function Attrs: alwaysinline mustprogress nofree norecurse nosync nounwind willreturn memory(argmem: readwrite)
define nonnull ptr @jet.stack.pop(ptr %0) local_unnamed_addr #0 {
entry:
  %stack.ptr = load i32, ptr %0, align 4
  %stack.ptr.sub_1 = add i32 %stack.ptr, -1
  %1 = sext i32 %stack.ptr.sub_1 to i64
  %stack.top.addr = getelementptr inbounds %jet.types.exec_ctx, ptr %0, i64 0, i32 5, i64 %1
  store i32 %stack.ptr.sub_1, ptr %0, align 4
  ret ptr %stack.top.addr
}

; Function Attrs: alwaysinline mustprogress nofree norecurse nosync nounwind willreturn memory(argmem: read)
define nonnull ptr @jet.stack.peek(ptr readonly %0, i8 %peek_idx) local_unnamed_addr #1 {
entry:
  %stack.ptr = load i32, ptr %0, align 4
  %peek_idx.i32 = zext i8 %peek_idx to i32
  %stack.peek.ptr = sub i32 %stack.ptr, %peek_idx.i32
  %1 = sext i32 %stack.peek.ptr to i64
  %stack.peek.addr = getelementptr inbounds %jet.types.exec_ctx, ptr %0, i64 0, i32 5, i64 %1
  ret ptr %stack.peek.addr
}

; Function Attrs: alwaysinline mustprogress nofree norecurse nosync nounwind willreturn memory(argmem: readwrite)
define noundef i1 @jet.stack.swap(ptr %0, i8 %swap.idx) local_unnamed_addr #0 {
entry:
  %stack.ptr = load i32, ptr %0, align 4
  %stack.ptr.sub_1 = add i32 %stack.ptr, -1
  %1 = sext i32 %stack.ptr.sub_1 to i64
  %stack.top.addr = getelementptr inbounds %jet.types.exec_ctx, ptr %0, i64 0, i32 5, i64 %1
  %top_word.elt31 = getelementptr inbounds %jet.types.exec_ctx, ptr %0, i64 0, i32 5, i64 %1, i64 16
  %swap.idx.i32 = zext i8 %swap.idx to i32
  %stack.swap.idx = sub i32 %stack.ptr.sub_1, %swap.idx.i32
  %2 = sext i32 %stack.swap.idx to i64
  %stack.swap.addr = getelementptr inbounds %jet.types.exec_ctx, ptr %0, i64 0, i32 5, i64 %2
  %swap_word.elt94 = getelementptr inbounds %jet.types.exec_ctx, ptr %0, i64 0, i32 5, i64 %2, i64 16
  %3 = load <16 x i8>, ptr %stack.swap.addr, align 1
  %4 = load <16 x i8>, ptr %stack.top.addr, align 1
  store <16 x i8> %4, ptr %stack.swap.addr, align 1
  store <16 x i8> %3, ptr %stack.top.addr, align 1
  %5 = load <16 x i8>, ptr %swap_word.elt94, align 1
  %6 = load <16 x i8>, ptr %top_word.elt31, align 1
  store <16 x i8> %6, ptr %swap_word.elt94, align 1
  store <16 x i8> %5, ptr %top_word.elt31, align 1
  ret i1 true
}

; Function Attrs: alwaysinline mustprogress nofree norecurse nosync nounwind willreturn memory(argmem: readwrite)
define noundef i8 @jet.mem.store.word(ptr nocapture writeonly %ctx, ptr nocapture readonly %loc, ptr nocapture readonly %val) local_unnamed_addr #0 {
entry:
  %loc_i32 = load i32, ptr %loc, align 4
  %0 = sext i32 %loc_i32 to i64
  %mem_loc_ptr = getelementptr inbounds %jet.types.exec_ctx, ptr %ctx, i64 0, i32 6, i64 %0
  tail call void @llvm.memcpy.inline.p0.p0.i64(ptr nonnull align 1 %mem_loc_ptr, ptr align 1 %val, i64 32, i1 false)
  ret i8 0
}

; Function Attrs: alwaysinline mustprogress nofree norecurse nosync nounwind willreturn memory(argmem: readwrite)
define noundef i8 @jet.mem.store.byte(ptr nocapture writeonly %ctx, ptr nocapture readonly %loc, ptr nocapture readonly %val) local_unnamed_addr #0 {
entry:
  %loc_i32 = load i32, ptr %loc, align 4
  %val_i8 = load i8, ptr %val, align 1
  %0 = sext i32 %loc_i32 to i64
  %mem_loc_ptr = getelementptr inbounds %jet.types.exec_ctx, ptr %ctx, i64 0, i32 6, i64 %0
  store i8 %val_i8, ptr %mem_loc_ptr, align 1
  ret i8 0
}

; Function Attrs: alwaysinline mustprogress nofree norecurse nosync nounwind willreturn memory(argmem: read)
define nonnull ptr @jet.mem.load(ptr readnone %ctx, ptr nocapture readonly %loc) local_unnamed_addr #1 {
entry:
  %loc_i32 = load i32, ptr %loc, align 4
  %0 = sext i32 %loc_i32 to i64
  %mem_loc_ptr = getelementptr inbounds %jet.types.exec_ctx, ptr %ctx, i64 0, i32 6, i64 %0
  ret ptr %mem_loc_ptr
}

; Function Attrs: alwaysinline nounwind
define i8 @jet.contracts.call(ptr %caller_ctx, ptr %callee_ctx, i160 %addr, i32 %ret.dest, i32 %ret.len) local_unnamed_addr #2 {
entry:
  %addr_i160_ptr = alloca i160, align 8
  store i160 %addr, ptr %addr_i160_ptr, align 8
  %fn_ptr_addr = alloca ptr, align 8
  %lookup_result = call i8 @jet.contracts.lookup(ptr nonnull @jet.jit_engine, ptr nonnull %fn_ptr_addr, ptr nonnull %addr_i160_ptr) #6
  %success = icmp eq i8 %lookup_result, 0
  br i1 %success, label %invoke_fn, label %return

invoke_fn:                                        ; preds = %entry
  %fn_ptr = load ptr, ptr %fn_ptr_addr, align 8
  %result = call i8 %fn_ptr(ptr %callee_ctx) #6
  %caller.sub_ctx.addr = getelementptr inbounds %jet.types.exec_ctx, ptr %caller_ctx, i64 0, i32 4
  store ptr %callee_ctx, ptr %caller.sub_ctx.addr, align 8
  %callee.return.len.addr = getelementptr inbounds %jet.types.exec_ctx, ptr %callee_ctx, i64 0, i32 3
  %callee.return.len = load i32, ptr %callee.return.len.addr, align 4
  %callee.return.empty = icmp eq i32 %callee.return.len, 0
  br i1 %callee.return.empty, label %return, label %copy_return_data

copy_return_data:                                 ; preds = %invoke_fn
  %copy.ret = call i8 @jet.contracts.call_return_data_copy(ptr nonnull %caller_ctx, ptr nonnull %callee_ctx, i32 %ret.dest, i32 0, i32 %ret.len) #6
  br label %return

return:                                           ; preds = %copy_return_data, %invoke_fn, %entry
  %r = phi i8 [ %copy.ret, %copy_return_data ], [ 0, %invoke_fn ], [ 1, %entry ]
  ret i8 %r
}

; Function Attrs: mustprogress nocallback nofree nounwind willreturn memory(argmem: readwrite)
declare void @llvm.memcpy.inline.p0.p0.i64(ptr noalias nocapture writeonly, ptr noalias nocapture readonly, i64 immarg, i1 immarg) #3

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(argmem: readwrite)
define noundef i8 @jet.contracts.0x1234(ptr nocapture %0, ptr nocapture readnone %1) local_unnamed_addr #4 {
preamble:
  %stack.ptr.i74 = load i32, ptr %0, align 4
  %2 = sext i32 %stack.ptr.i74 to i64
  %stack.top.addr.i75 = getelementptr inbounds %jet.types.exec_ctx, ptr %0, i64 0, i32 5, i64 %2
  store i8 0, ptr %stack.top.addr.i75, align 1
  %push_bytes_ptr.sroa.2.0.stack.top.addr.i75.sroa_idx = getelementptr inbounds i8, ptr %stack.top.addr.i75, i64 1
  store i8 -1, ptr %push_bytes_ptr.sroa.2.0.stack.top.addr.i75.sroa_idx, align 1
  %push_bytes_ptr.sroa.3.0.stack.top.addr.i75.sroa_idx = getelementptr inbounds i8, ptr %stack.top.addr.i75, i64 2
  %stack.ptr.next.i76 = add i32 %stack.ptr.i74, 1
  tail call void @llvm.memset.p0.i64(ptr noundef nonnull align 1 dereferenceable(30) %push_bytes_ptr.sroa.3.0.stack.top.addr.i75.sroa_idx, i8 0, i64 30, i1 false)
  %3 = sext i32 %stack.ptr.next.i76 to i64
  %stack.top.addr.i72 = getelementptr inbounds %jet.types.exec_ctx, ptr %0, i64 0, i32 5, i64 %3
  store i8 -1, ptr %stack.top.addr.i72, align 1
  %push_bytes_ptr1.sroa.2.0.stack.top.addr.i72.sroa_idx = getelementptr inbounds i8, ptr %stack.top.addr.i72, i64 1
  tail call void @llvm.memset.p0.i64(ptr noundef nonnull align 1 dereferenceable(31) %push_bytes_ptr1.sroa.2.0.stack.top.addr.i72.sroa_idx, i8 0, i64 31, i1 false)
  store i32 %stack.ptr.i74, ptr %0, align 4
  %load_int = load i256, ptr %stack.top.addr.i72, align 8
  %load_int4 = load i256, ptr %stack.top.addr.i75, align 8
  %add_result = add i256 %load_int4, %load_int
  store i256 %add_result, ptr %stack.top.addr.i75, align 8
  store i8 1, ptr %stack.top.addr.i72, align 1
  tail call void @llvm.memset.p0.i64(ptr noundef nonnull align 1 dereferenceable(31) %push_bytes_ptr1.sroa.2.0.stack.top.addr.i72.sroa_idx, i8 0, i64 31, i1 false)
  store i32 %stack.ptr.i74, ptr %0, align 4
  %load_int9 = load i256, ptr %stack.top.addr.i72, align 8
  %load_int10 = load i256, ptr %stack.top.addr.i75, align 8
  %add_result11 = add i256 %load_int10, %load_int9
  store i256 %add_result11, ptr %stack.top.addr.i75, align 8
  store i32 %stack.ptr.next.i76, ptr %0, align 4
  ret i8 0
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(argmem: readwrite)
define noundef i8 @jet.contracts.0x0001(ptr nocapture %0, ptr nocapture readnone %1) local_unnamed_addr #4 {
preamble:
  %return_offset = getelementptr inbounds <{ i32, i32, i32, i32, ptr, [1024 x i256], <{ ptr, i32, i32 }> }>, ptr %0, i64 0, i32 2
  %return_length = getelementptr inbounds <{ i32, i32, i32, i32, ptr, [1024 x i256], <{ ptr, i32, i32 }> }>, ptr %0, i64 0, i32 3
  %stack.ptr.i173 = load i32, ptr %0, align 4
  %2 = sext i32 %stack.ptr.i173 to i64
  %stack.top.addr.i174 = getelementptr inbounds %jet.types.exec_ctx, ptr %0, i64 0, i32 5, i64 %2
  store i8 -1, ptr %stack.top.addr.i174, align 1
  %push_bytes_ptr.sroa.2.0.stack.top.addr.i174.sroa_idx = getelementptr inbounds i8, ptr %stack.top.addr.i174, i64 1
  %stack.ptr.next.i175 = add i32 %stack.ptr.i173, 1
  tail call void @llvm.memset.p0.i64(ptr noundef nonnull align 1 dereferenceable(31) %push_bytes_ptr.sroa.2.0.stack.top.addr.i174.sroa_idx, i8 0, i64 31, i1 false)
  %3 = sext i32 %stack.ptr.next.i175 to i64
  %stack.top.addr.i171 = getelementptr inbounds %jet.types.exec_ctx, ptr %0, i64 0, i32 5, i64 %3
  store i8 1, ptr %stack.top.addr.i171, align 1
  %push_bytes_ptr1.sroa.2.0.stack.top.addr.i171.sroa_idx = getelementptr inbounds i8, ptr %stack.top.addr.i171, i64 1
  tail call void @llvm.memset.p0.i64(ptr noundef nonnull align 1 dereferenceable(31) %push_bytes_ptr1.sroa.2.0.stack.top.addr.i171.sroa_idx, i8 0, i64 31, i1 false)
  store i32 %stack.ptr.i173, ptr %0, align 4
  %loc_i32.i193 = load i32, ptr %stack.top.addr.i171, align 4
  %4 = sext i32 %loc_i32.i193 to i64
  %mem_loc_ptr.i194 = getelementptr inbounds %jet.types.exec_ctx, ptr %0, i64 0, i32 6, i64 %4
  tail call void @llvm.memcpy.inline.p0.p0.i64(ptr nonnull align 1 %mem_loc_ptr.i194, ptr nonnull align 1 %stack.top.addr.i174, i64 32, i1 false)
  %stack.ptr.i167 = load i32, ptr %0, align 4
  %5 = sext i32 %stack.ptr.i167 to i64
  %stack.top.addr.i168 = getelementptr inbounds %jet.types.exec_ctx, ptr %0, i64 0, i32 5, i64 %5
  store i8 -1, ptr %stack.top.addr.i168, align 1
  %push_bytes_ptr4.sroa.2.0.stack.top.addr.i168.sroa_idx = getelementptr inbounds i8, ptr %stack.top.addr.i168, i64 1
  %stack.ptr.next.i169 = add i32 %stack.ptr.i167, 1
  tail call void @llvm.memset.p0.i64(ptr noundef nonnull align 1 dereferenceable(31) %push_bytes_ptr4.sroa.2.0.stack.top.addr.i168.sroa_idx, i8 0, i64 31, i1 false)
  %6 = sext i32 %stack.ptr.next.i169 to i64
  %stack.top.addr.i165 = getelementptr inbounds %jet.types.exec_ctx, ptr %0, i64 0, i32 5, i64 %6
  store i8 10, ptr %stack.top.addr.i165, align 1
  %push_bytes_ptr6.sroa.2.0.stack.top.addr.i165.sroa_idx = getelementptr inbounds i8, ptr %stack.top.addr.i165, i64 1
  tail call void @llvm.memset.p0.i64(ptr noundef nonnull align 1 dereferenceable(31) %push_bytes_ptr6.sroa.2.0.stack.top.addr.i165.sroa_idx, i8 0, i64 31, i1 false)
  store i32 %stack.ptr.i167, ptr %0, align 4
  %loc_i32.i = load i32, ptr %stack.top.addr.i165, align 4
  %7 = sext i32 %loc_i32.i to i64
  %mem_loc_ptr.i = getelementptr inbounds %jet.types.exec_ctx, ptr %0, i64 0, i32 6, i64 %7
  tail call void @llvm.memcpy.inline.p0.p0.i64(ptr nonnull align 1 %mem_loc_ptr.i, ptr nonnull align 1 %stack.top.addr.i168, i64 32, i1 false)
  %stack.ptr.i161 = load i32, ptr %0, align 4
  %8 = sext i32 %stack.ptr.i161 to i64
  %stack.top.addr.i162 = getelementptr inbounds %jet.types.exec_ctx, ptr %0, i64 0, i32 5, i64 %8
  store i8 10, ptr %stack.top.addr.i162, align 1
  %push_bytes_ptr11.sroa.2.0.stack.top.addr.i162.sroa_idx = getelementptr inbounds i8, ptr %stack.top.addr.i162, i64 1
  %stack.ptr.next.i163 = add i32 %stack.ptr.i161, 1
  tail call void @llvm.memset.p0.i64(ptr noundef nonnull align 1 dereferenceable(31) %push_bytes_ptr11.sroa.2.0.stack.top.addr.i162.sroa_idx, i8 0, i64 31, i1 false)
  %9 = sext i32 %stack.ptr.next.i163 to i64
  %stack.top.addr.i = getelementptr inbounds %jet.types.exec_ctx, ptr %0, i64 0, i32 5, i64 %9
  tail call void @llvm.memset.p0.i64(ptr noundef nonnull align 1 dereferenceable(32) %stack.top.addr.i, i8 0, i64 32, i1 false)
  store i32 %stack.ptr.i161, ptr %0, align 4
  %load_int17 = load i32, ptr %stack.top.addr.i162, align 4
  store i32 0, ptr %return_offset, align 4
  store i32 %load_int17, ptr %return_length, align 4
  ret i8 1
}

; Function Attrs: nocallback nofree nounwind willreturn memory(argmem: write)
declare void @llvm.memset.p0.i64(ptr nocapture writeonly, i8, i64, i1 immarg) #5

attributes #0 = { alwaysinline mustprogress nofree norecurse nosync nounwind willreturn memory(argmem: readwrite) }
attributes #1 = { alwaysinline mustprogress nofree norecurse nosync nounwind willreturn memory(argmem: read) }
attributes #2 = { alwaysinline nounwind }
attributes #3 = { mustprogress nocallback nofree nounwind willreturn memory(argmem: readwrite) }
attributes #4 = { mustprogress nofree norecurse nosync nounwind willreturn memory(argmem: readwrite) }
attributes #5 = { nocallback nofree nounwind willreturn memory(argmem: write) }
attributes #6 = { nounwind }

