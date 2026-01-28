; ModuleID = './examples/master.ll'
source_filename = "jet.ll"
target datalayout = "e-m:o-p270:32:32-p271:32:32-p272:64:64-i64:64-n8:16:32:64-S128"
target triple = "x86_64-apple-macosx14.0.0"

%jet.types.exec_ctx = type <{ i32, i32, i32, i32, ptr, [1024 x i256], [1024 x i8], i32, i32 }>

@jet.jit_engine = external global ptr

declare i8 @jet.contracts.lookup(ptr, ptr, ptr) local_unnamed_addr

declare i8 @jet.contracts.call_return_data_copy(ptr, ptr, i32, i32, i32) local_unnamed_addr

; Function Attrs: alwaysinline mustprogress nofree norecurse nosync nounwind willreturn memory(argmem: readwrite)
define noundef i1 @jet.stack.push.word(ptr nocapture %0, i256 %1) local_unnamed_addr #0 {
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
define noundef i1 @jet.stack.push.bytes(ptr nocapture %0, [32 x i8] %1) local_unnamed_addr #0 {
entry:
  %.fca.0.extract = extractvalue [32 x i8] %1, 0
  %.fca.1.extract = extractvalue [32 x i8] %1, 1
  %.fca.2.extract = extractvalue [32 x i8] %1, 2
  %.fca.3.extract = extractvalue [32 x i8] %1, 3
  %.fca.4.extract = extractvalue [32 x i8] %1, 4
  %.fca.5.extract = extractvalue [32 x i8] %1, 5
  %.fca.6.extract = extractvalue [32 x i8] %1, 6
  %.fca.7.extract = extractvalue [32 x i8] %1, 7
  %.fca.8.extract = extractvalue [32 x i8] %1, 8
  %.fca.9.extract = extractvalue [32 x i8] %1, 9
  %.fca.10.extract = extractvalue [32 x i8] %1, 10
  %.fca.11.extract = extractvalue [32 x i8] %1, 11
  %.fca.12.extract = extractvalue [32 x i8] %1, 12
  %.fca.13.extract = extractvalue [32 x i8] %1, 13
  %.fca.14.extract = extractvalue [32 x i8] %1, 14
  %.fca.15.extract = extractvalue [32 x i8] %1, 15
  %.fca.16.extract = extractvalue [32 x i8] %1, 16
  %.fca.17.extract = extractvalue [32 x i8] %1, 17
  %.fca.18.extract = extractvalue [32 x i8] %1, 18
  %.fca.19.extract = extractvalue [32 x i8] %1, 19
  %.fca.20.extract = extractvalue [32 x i8] %1, 20
  %.fca.21.extract = extractvalue [32 x i8] %1, 21
  %.fca.22.extract = extractvalue [32 x i8] %1, 22
  %.fca.23.extract = extractvalue [32 x i8] %1, 23
  %.fca.24.extract = extractvalue [32 x i8] %1, 24
  %.fca.25.extract = extractvalue [32 x i8] %1, 25
  %.fca.26.extract = extractvalue [32 x i8] %1, 26
  %.fca.27.extract = extractvalue [32 x i8] %1, 27
  %.fca.28.extract = extractvalue [32 x i8] %1, 28
  %.fca.29.extract = extractvalue [32 x i8] %1, 29
  %.fca.30.extract = extractvalue [32 x i8] %1, 30
  %.fca.31.extract = extractvalue [32 x i8] %1, 31
  %stack_bytes_ptr.sroa.32.0.insert.ext = zext i8 %.fca.31.extract to i256
  %stack_bytes_ptr.sroa.32.0.insert.shift = shl nuw i256 %stack_bytes_ptr.sroa.32.0.insert.ext, 248
  %stack_bytes_ptr.sroa.31.0.insert.ext = zext i8 %.fca.30.extract to i256
  %stack_bytes_ptr.sroa.31.0.insert.shift = shl nuw nsw i256 %stack_bytes_ptr.sroa.31.0.insert.ext, 240
  %stack_bytes_ptr.sroa.31.0.insert.insert = or disjoint i256 %stack_bytes_ptr.sroa.32.0.insert.shift, %stack_bytes_ptr.sroa.31.0.insert.shift
  %stack_bytes_ptr.sroa.30.0.insert.ext = zext i8 %.fca.29.extract to i256
  %stack_bytes_ptr.sroa.30.0.insert.shift = shl nuw nsw i256 %stack_bytes_ptr.sroa.30.0.insert.ext, 232
  %stack_bytes_ptr.sroa.30.0.insert.insert = or disjoint i256 %stack_bytes_ptr.sroa.31.0.insert.insert, %stack_bytes_ptr.sroa.30.0.insert.shift
  %stack_bytes_ptr.sroa.29.0.insert.ext = zext i8 %.fca.28.extract to i256
  %stack_bytes_ptr.sroa.29.0.insert.shift = shl nuw nsw i256 %stack_bytes_ptr.sroa.29.0.insert.ext, 224
  %stack_bytes_ptr.sroa.29.0.insert.insert = or disjoint i256 %stack_bytes_ptr.sroa.30.0.insert.insert, %stack_bytes_ptr.sroa.29.0.insert.shift
  %stack_bytes_ptr.sroa.28.0.insert.ext = zext i8 %.fca.27.extract to i256
  %stack_bytes_ptr.sroa.28.0.insert.shift = shl nuw nsw i256 %stack_bytes_ptr.sroa.28.0.insert.ext, 216
  %stack_bytes_ptr.sroa.28.0.insert.insert = or disjoint i256 %stack_bytes_ptr.sroa.29.0.insert.insert, %stack_bytes_ptr.sroa.28.0.insert.shift
  %stack_bytes_ptr.sroa.27.0.insert.ext = zext i8 %.fca.26.extract to i256
  %stack_bytes_ptr.sroa.27.0.insert.shift = shl nuw nsw i256 %stack_bytes_ptr.sroa.27.0.insert.ext, 208
  %stack_bytes_ptr.sroa.26.0.insert.ext = zext i8 %.fca.25.extract to i256
  %stack_bytes_ptr.sroa.26.0.insert.shift = shl nuw nsw i256 %stack_bytes_ptr.sroa.26.0.insert.ext, 200
  %stack_bytes_ptr.sroa.26.0.insert.mask = or disjoint i256 %stack_bytes_ptr.sroa.28.0.insert.insert, %stack_bytes_ptr.sroa.27.0.insert.shift
  %stack_bytes_ptr.sroa.25.0.insert.ext = zext i8 %.fca.24.extract to i256
  %stack_bytes_ptr.sroa.25.0.insert.shift = shl nuw nsw i256 %stack_bytes_ptr.sroa.25.0.insert.ext, 192
  %stack_bytes_ptr.sroa.24.0.insert.ext = zext i8 %.fca.23.extract to i256
  %stack_bytes_ptr.sroa.24.0.insert.shift = shl nuw nsw i256 %stack_bytes_ptr.sroa.24.0.insert.ext, 184
  %stack_bytes_ptr.sroa.23.0.insert.ext = zext i8 %.fca.22.extract to i256
  %stack_bytes_ptr.sroa.23.0.insert.shift = shl nuw nsw i256 %stack_bytes_ptr.sroa.23.0.insert.ext, 176
  %stack_bytes_ptr.sroa.22.0.insert.ext = zext i8 %.fca.21.extract to i256
  %stack_bytes_ptr.sroa.22.0.insert.shift = shl nuw nsw i256 %stack_bytes_ptr.sroa.22.0.insert.ext, 168
  %stack_bytes_ptr.sroa.21.0.insert.ext = zext i8 %.fca.20.extract to i256
  %stack_bytes_ptr.sroa.21.0.insert.shift = shl nuw nsw i256 %stack_bytes_ptr.sroa.21.0.insert.ext, 160
  %stack_bytes_ptr.sroa.20.0.insert.ext = zext i8 %.fca.19.extract to i256
  %stack_bytes_ptr.sroa.20.0.insert.shift = shl nuw nsw i256 %stack_bytes_ptr.sroa.20.0.insert.ext, 152
  %stack_bytes_ptr.sroa.19.0.insert.ext = zext i8 %.fca.18.extract to i256
  %stack_bytes_ptr.sroa.19.0.insert.shift = shl nuw nsw i256 %stack_bytes_ptr.sroa.19.0.insert.ext, 144
  %stack_bytes_ptr.sroa.18.0.insert.ext = zext i8 %.fca.17.extract to i256
  %stack_bytes_ptr.sroa.18.0.insert.shift = shl nuw nsw i256 %stack_bytes_ptr.sroa.18.0.insert.ext, 136
  %stack_bytes_ptr.sroa.17.0.insert.ext = zext i8 %.fca.16.extract to i256
  %stack_bytes_ptr.sroa.17.0.insert.shift = shl nuw nsw i256 %stack_bytes_ptr.sroa.17.0.insert.ext, 128
  %stack_bytes_ptr.sroa.16.0.insert.ext = zext i8 %.fca.15.extract to i256
  %stack_bytes_ptr.sroa.16.0.insert.shift = shl nuw nsw i256 %stack_bytes_ptr.sroa.16.0.insert.ext, 120
  %stack_bytes_ptr.sroa.15.0.insert.ext = zext i8 %.fca.14.extract to i256
  %stack_bytes_ptr.sroa.15.0.insert.shift = shl nuw nsw i256 %stack_bytes_ptr.sroa.15.0.insert.ext, 112
  %stack_bytes_ptr.sroa.14.0.insert.ext = zext i8 %.fca.13.extract to i256
  %stack_bytes_ptr.sroa.14.0.insert.shift = shl nuw nsw i256 %stack_bytes_ptr.sroa.14.0.insert.ext, 104
  %stack_bytes_ptr.sroa.13.0.insert.ext = zext i8 %.fca.12.extract to i256
  %stack_bytes_ptr.sroa.13.0.insert.shift = shl nuw nsw i256 %stack_bytes_ptr.sroa.13.0.insert.ext, 96
  %stack_bytes_ptr.sroa.12.0.insert.ext = zext i8 %.fca.11.extract to i256
  %stack_bytes_ptr.sroa.12.0.insert.shift = shl nuw nsw i256 %stack_bytes_ptr.sroa.12.0.insert.ext, 88
  %stack_bytes_ptr.sroa.11.0.insert.ext = zext i8 %.fca.10.extract to i256
  %stack_bytes_ptr.sroa.11.0.insert.shift = shl nuw nsw i256 %stack_bytes_ptr.sroa.11.0.insert.ext, 80
  %stack_bytes_ptr.sroa.10.0.insert.ext = zext i8 %.fca.9.extract to i256
  %stack_bytes_ptr.sroa.10.0.insert.shift = shl nuw nsw i256 %stack_bytes_ptr.sroa.10.0.insert.ext, 72
  %stack_bytes_ptr.sroa.9.0.insert.ext = zext i8 %.fca.8.extract to i256
  %stack_bytes_ptr.sroa.9.0.insert.shift = shl nuw nsw i256 %stack_bytes_ptr.sroa.9.0.insert.ext, 64
  %stack_bytes_ptr.sroa.8.0.insert.ext = zext i8 %.fca.7.extract to i256
  %stack_bytes_ptr.sroa.8.0.insert.shift = shl nuw nsw i256 %stack_bytes_ptr.sroa.8.0.insert.ext, 56
  %stack_bytes_ptr.sroa.7.0.insert.ext = zext i8 %.fca.6.extract to i256
  %stack_bytes_ptr.sroa.7.0.insert.shift = shl nuw nsw i256 %stack_bytes_ptr.sroa.7.0.insert.ext, 48
  %stack_bytes_ptr.sroa.6.0.insert.ext = zext i8 %.fca.5.extract to i256
  %stack_bytes_ptr.sroa.6.0.insert.shift = shl nuw nsw i256 %stack_bytes_ptr.sroa.6.0.insert.ext, 40
  %stack_bytes_ptr.sroa.5.0.insert.ext = zext i8 %.fca.4.extract to i256
  %stack_bytes_ptr.sroa.5.0.insert.shift = shl nuw nsw i256 %stack_bytes_ptr.sroa.5.0.insert.ext, 32
  %stack_bytes_ptr.sroa.4.0.insert.ext = zext i8 %.fca.3.extract to i256
  %stack_bytes_ptr.sroa.4.0.insert.shift = shl nuw nsw i256 %stack_bytes_ptr.sroa.4.0.insert.ext, 24
  %stack_bytes_ptr.sroa.3.0.insert.ext = zext i8 %.fca.2.extract to i256
  %stack_bytes_ptr.sroa.3.0.insert.shift = shl nuw nsw i256 %stack_bytes_ptr.sroa.3.0.insert.ext, 16
  %stack_bytes_ptr.sroa.2.0.insert.ext = zext i8 %.fca.1.extract to i256
  %stack_bytes_ptr.sroa.2.0.insert.shift = shl nuw nsw i256 %stack_bytes_ptr.sroa.2.0.insert.ext, 8
  %stack_bytes_ptr.sroa.0.0.insert.ext = zext i8 %.fca.0.extract to i256
  %stack_bytes_ptr.sroa.25.0.insert.mask.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked = or disjoint i256 %stack_bytes_ptr.sroa.26.0.insert.mask, %stack_bytes_ptr.sroa.26.0.insert.shift
  %stack_bytes_ptr.sroa.24.0.insert.mask.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked = or disjoint i256 %stack_bytes_ptr.sroa.25.0.insert.shift, %stack_bytes_ptr.sroa.0.0.insert.ext
  %stack_bytes_ptr.sroa.23.0.insert.mask.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked = or disjoint i256 %stack_bytes_ptr.sroa.24.0.insert.mask.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked, %stack_bytes_ptr.sroa.24.0.insert.shift
  %stack_bytes_ptr.sroa.22.0.insert.mask.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked = or disjoint i256 %stack_bytes_ptr.sroa.23.0.insert.mask.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked, %stack_bytes_ptr.sroa.23.0.insert.shift
  %stack_bytes_ptr.sroa.21.0.insert.mask.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked = or disjoint i256 %stack_bytes_ptr.sroa.22.0.insert.mask.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked, %stack_bytes_ptr.sroa.22.0.insert.shift
  %stack_bytes_ptr.sroa.20.0.insert.mask.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked = or disjoint i256 %stack_bytes_ptr.sroa.21.0.insert.mask.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked, %stack_bytes_ptr.sroa.21.0.insert.shift
  %stack_bytes_ptr.sroa.19.0.insert.mask.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked = or disjoint i256 %stack_bytes_ptr.sroa.20.0.insert.mask.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked, %stack_bytes_ptr.sroa.20.0.insert.shift
  %stack_bytes_ptr.sroa.18.0.insert.mask.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked = or disjoint i256 %stack_bytes_ptr.sroa.18.0.insert.shift, %stack_bytes_ptr.sroa.19.0.insert.shift
  %stack_bytes_ptr.sroa.17.0.insert.mask.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked = or disjoint i256 %stack_bytes_ptr.sroa.18.0.insert.mask.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked, %stack_bytes_ptr.sroa.17.0.insert.shift
  %stack_bytes_ptr.sroa.16.0.insert.mask.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked = or disjoint i256 %stack_bytes_ptr.sroa.17.0.insert.mask.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked, %stack_bytes_ptr.sroa.16.0.insert.shift
  %stack_bytes_ptr.sroa.15.0.insert.mask.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked = or disjoint i256 %stack_bytes_ptr.sroa.16.0.insert.mask.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked, %stack_bytes_ptr.sroa.15.0.insert.shift
  %stack_bytes_ptr.sroa.14.0.insert.mask.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked = or disjoint i256 %stack_bytes_ptr.sroa.15.0.insert.mask.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked, %stack_bytes_ptr.sroa.14.0.insert.shift
  %stack_bytes_ptr.sroa.13.0.insert.mask.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked = or i256 %stack_bytes_ptr.sroa.14.0.insert.mask.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked, %stack_bytes_ptr.sroa.13.0.insert.shift
  %stack_bytes_ptr.sroa.12.0.insert.mask.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked = or i256 %stack_bytes_ptr.sroa.13.0.insert.mask.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked, %stack_bytes_ptr.sroa.12.0.insert.shift
  %stack_bytes_ptr.sroa.11.0.insert.mask.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked = or i256 %stack_bytes_ptr.sroa.12.0.insert.mask.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked, %stack_bytes_ptr.sroa.11.0.insert.shift
  %stack_bytes_ptr.sroa.10.0.insert.mask.masked.masked.masked.masked.masked.masked.masked.masked.masked = or i256 %stack_bytes_ptr.sroa.11.0.insert.mask.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked, %stack_bytes_ptr.sroa.10.0.insert.shift
  %stack_bytes_ptr.sroa.9.0.insert.mask.masked.masked.masked.masked.masked.masked.masked.masked = or i256 %stack_bytes_ptr.sroa.10.0.insert.mask.masked.masked.masked.masked.masked.masked.masked.masked.masked, %stack_bytes_ptr.sroa.9.0.insert.shift
  %stack_bytes_ptr.sroa.8.0.insert.mask.masked.masked.masked.masked.masked.masked.masked = or i256 %stack_bytes_ptr.sroa.9.0.insert.mask.masked.masked.masked.masked.masked.masked.masked.masked, %stack_bytes_ptr.sroa.8.0.insert.shift
  %stack_bytes_ptr.sroa.7.0.insert.mask.masked.masked.masked.masked.masked.masked = or i256 %stack_bytes_ptr.sroa.8.0.insert.mask.masked.masked.masked.masked.masked.masked.masked, %stack_bytes_ptr.sroa.7.0.insert.shift
  %stack_bytes_ptr.sroa.6.0.insert.mask.masked.masked.masked.masked.masked = or i256 %stack_bytes_ptr.sroa.7.0.insert.mask.masked.masked.masked.masked.masked.masked, %stack_bytes_ptr.sroa.6.0.insert.shift
  %stack_bytes_ptr.sroa.5.0.insert.mask.masked.masked.masked.masked = or i256 %stack_bytes_ptr.sroa.6.0.insert.mask.masked.masked.masked.masked.masked, %stack_bytes_ptr.sroa.5.0.insert.shift
  %stack_bytes_ptr.sroa.4.0.insert.mask.masked.masked.masked = or i256 %stack_bytes_ptr.sroa.5.0.insert.mask.masked.masked.masked.masked, %stack_bytes_ptr.sroa.4.0.insert.shift
  %stack_bytes_ptr.sroa.3.0.insert.mask.masked.masked = or i256 %stack_bytes_ptr.sroa.4.0.insert.mask.masked.masked.masked, %stack_bytes_ptr.sroa.3.0.insert.shift
  %stack_bytes_ptr.sroa.2.0.insert.mask.masked = or i256 %stack_bytes_ptr.sroa.3.0.insert.mask.masked.masked, %stack_bytes_ptr.sroa.2.0.insert.shift
  %stack_bytes_ptr.sroa.0.0.insert.mask = or i256 %stack_bytes_ptr.sroa.2.0.insert.mask.masked, %stack_bytes_ptr.sroa.25.0.insert.mask.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked
  %stack_bytes_ptr.sroa.0.0.insert.insert = or i256 %stack_bytes_ptr.sroa.0.0.insert.mask, %stack_bytes_ptr.sroa.19.0.insert.mask.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked.masked
  %stack.ptr.i = load i32, ptr %0, align 4
  %2 = sext i32 %stack.ptr.i to i64
  %stack.top.addr.i = getelementptr inbounds %jet.types.exec_ctx, ptr %0, i64 0, i32 5, i64 %2
  store i256 %stack_bytes_ptr.sroa.0.0.insert.insert, ptr %stack.top.addr.i, align 8
  %stack.ptr.next.i = add i32 %stack.ptr.i, 1
  store i32 %stack.ptr.next.i, ptr %0, align 4
  ret i1 true
}

; Function Attrs: alwaysinline mustprogress nofree norecurse nosync nounwind willreturn memory(argmem: readwrite)
define i256 @jet.stack.pop(ptr nocapture %0) local_unnamed_addr #0 {
entry:
  %stack.ptr = load i32, ptr %0, align 4
  %stack.ptr.sub_1 = add i32 %stack.ptr, -1
  %1 = sext i32 %stack.ptr.sub_1 to i64
  %stack.top.addr = getelementptr inbounds %jet.types.exec_ctx, ptr %0, i64 0, i32 5, i64 %1
  %stack_word = load i256, ptr %stack.top.addr, align 8
  store i32 %stack.ptr.sub_1, ptr %0, align 4
  ret i256 %stack_word
}

; Function Attrs: alwaysinline mustprogress nofree norecurse nosync nounwind willreturn memory(argmem: read)
define i256 @jet.stack.peek(ptr nocapture readonly %0, i8 %peek_idx) local_unnamed_addr #1 {
entry:
  %stack.ptr = load i32, ptr %0, align 4
  %peek_idx.i32 = zext i8 %peek_idx to i32
  %stack.peek.ptr = sub i32 %stack.ptr, %peek_idx.i32
  %1 = sext i32 %stack.peek.ptr to i64
  %stack.peek.addr = getelementptr inbounds %jet.types.exec_ctx, ptr %0, i64 0, i32 5, i64 %1
  %stack_word = load i256, ptr %stack.peek.addr, align 8
  ret i256 %stack_word
}

; Function Attrs: alwaysinline mustprogress nofree norecurse nosync nounwind willreturn memory(argmem: readwrite)
define noundef i1 @jet.stack.swap(ptr nocapture %0, i8 %swap.idx) local_unnamed_addr #0 {
entry:
  %stack.ptr = load i32, ptr %0, align 4
  %stack.ptr.sub_1 = add i32 %stack.ptr, -1
  %1 = sext i32 %stack.ptr.sub_1 to i64
  %stack.top.addr = getelementptr inbounds %jet.types.exec_ctx, ptr %0, i64 0, i32 5, i64 %1
  %top_word = load i256, ptr %stack.top.addr, align 8
  %swap.idx.i32 = zext i8 %swap.idx to i32
  %stack.swap.idx = sub i32 %stack.ptr.sub_1, %swap.idx.i32
  %2 = sext i32 %stack.swap.idx to i64
  %stack.swap.addr = getelementptr inbounds %jet.types.exec_ctx, ptr %0, i64 0, i32 5, i64 %2
  %swap_word = load i256, ptr %stack.swap.addr, align 8
  store i256 %top_word, ptr %stack.swap.addr, align 8
  store i256 %swap_word, ptr %stack.top.addr, align 8
  ret i1 true
}

; Function Attrs: alwaysinline mustprogress nofree norecurse nosync nounwind willreturn memory(argmem: write)
define noundef i8 @jet.mem.store.word(ptr nocapture writeonly %ctx, i256 %loc, i256 %val) local_unnamed_addr #2 {
entry:
  %loc_i32 = trunc i256 %loc to i64
  %sext = shl i64 %loc_i32, 32
  %0 = ashr exact i64 %sext, 32
  %mem_loc_ptr = getelementptr inbounds %jet.types.exec_ctx, ptr %ctx, i64 0, i32 6, i64 %0
  store i256 %val, ptr %mem_loc_ptr, align 1
  ret i8 0
}

; Function Attrs: alwaysinline mustprogress nofree norecurse nosync nounwind willreturn memory(argmem: write)
define noundef i8 @jet.mem.store.byte(ptr nocapture writeonly %ctx, i256 %loc, i256 %val) local_unnamed_addr #2 {
entry:
  %loc_i32 = trunc i256 %loc to i64
  %val_i8 = trunc i256 %val to i8
  %sext = shl i64 %loc_i32, 32
  %0 = ashr exact i64 %sext, 32
  %mem_loc_ptr = getelementptr inbounds %jet.types.exec_ctx, ptr %ctx, i64 0, i32 6, i64 %0
  store i8 %val_i8, ptr %mem_loc_ptr, align 1
  ret i8 0
}

; Function Attrs: alwaysinline mustprogress nofree norecurse nosync nounwind willreturn memory(argmem: read)
define i256 @jet.mem.load(ptr nocapture readonly %ctx, i256 %loc) local_unnamed_addr #1 {
entry:
  %loc_i32 = trunc i256 %loc to i64
  %sext = shl i64 %loc_i32, 32
  %0 = ashr exact i64 %sext, 32
  %mem_loc_ptr = getelementptr inbounds %jet.types.exec_ctx, ptr %ctx, i64 0, i32 6, i64 %0
  %val = load i256, ptr %mem_loc_ptr, align 1
  ret i256 %val
}

; Function Attrs: alwaysinline nounwind
define i8 @jet.contracts.call(ptr %caller_ctx, ptr %callee_ctx, i160 %addr, i32 %ret.dest, i32 %ret.len) local_unnamed_addr #3 {
entry:
  %addr_i160_ptr = alloca i160, align 8
  store i160 %addr, ptr %addr_i160_ptr, align 8
  %fn_ptr_addr = alloca ptr, align 8
  %lookup_result = call i8 @jet.contracts.lookup(ptr nonnull @jet.jit_engine, ptr nonnull %fn_ptr_addr, ptr nonnull %addr_i160_ptr) #5
  %success = icmp eq i8 %lookup_result, 0
  br i1 %success, label %invoke_fn, label %return

invoke_fn:                                        ; preds = %entry
  %fn_ptr = load ptr, ptr %fn_ptr_addr, align 8
  %result = call i8 %fn_ptr(ptr %callee_ctx) #5
  %caller.sub_ctx.addr = getelementptr inbounds %jet.types.exec_ctx, ptr %caller_ctx, i64 0, i32 4
  store ptr %callee_ctx, ptr %caller.sub_ctx.addr, align 8
  %callee.return.len.addr = getelementptr inbounds %jet.types.exec_ctx, ptr %callee_ctx, i64 0, i32 3
  %callee.return.len = load i32, ptr %callee.return.len.addr, align 4
  %callee.return.empty = icmp eq i32 %callee.return.len, 0
  br i1 %callee.return.empty, label %return, label %copy_return_data

copy_return_data:                                 ; preds = %invoke_fn
  %copy.ret = call i8 @jet.contracts.call_return_data_copy(ptr nonnull %caller_ctx, ptr nonnull %callee_ctx, i32 %ret.dest, i32 0, i32 %ret.len) #5
  br label %return

return:                                           ; preds = %copy_return_data, %invoke_fn, %entry
  %r = phi i8 [ %copy.ret, %copy_return_data ], [ 0, %invoke_fn ], [ 1, %entry ]
  ret i8 %r
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(argmem: readwrite)
define noundef i8 @jet.contracts.0x1234(ptr nocapture %0, ptr nocapture readnone %1) local_unnamed_addr #4 {
preamble:
  %stack.ptr.i.i7 = load i32, ptr %0, align 4
  %2 = sext i32 %stack.ptr.i.i7 to i64
  %stack.top.addr.i.i8 = getelementptr inbounds %jet.types.exec_ctx, ptr %0, i64 0, i32 5, i64 %2
  store i256 65280, ptr %stack.top.addr.i.i8, align 8
  %stack.ptr.next.i.i9 = add i32 %stack.ptr.i.i7, 1
  %3 = sext i32 %stack.ptr.next.i.i9 to i64
  %stack.top.addr.i.i5 = getelementptr inbounds %jet.types.exec_ctx, ptr %0, i64 0, i32 5, i64 %3
  store i256 255, ptr %stack.top.addr.i.i5, align 8
  %stack.ptr.next.i.i6 = add i32 %stack.ptr.i.i7, 2
  store i32 %stack.ptr.next.i.i6, ptr %0, align 4
  %stack_word.i23 = load i256, ptr %stack.top.addr.i.i5, align 8
  store i32 %stack.ptr.next.i.i9, ptr %0, align 4
  %stack_word.i19 = load i256, ptr %stack.top.addr.i.i8, align 8
  %add_result = add i256 %stack_word.i19, %stack_word.i23
  store i256 %add_result, ptr %stack.top.addr.i.i8, align 8
  store i256 1, ptr %stack.top.addr.i.i5, align 8
  store i32 %stack.ptr.next.i.i6, ptr %0, align 4
  %stack_word.i15 = load i256, ptr %stack.top.addr.i.i5, align 8
  store i32 %stack.ptr.next.i.i9, ptr %0, align 4
  %stack_word.i = load i256, ptr %stack.top.addr.i.i8, align 8
  %add_result6 = add i256 %stack_word.i, %stack_word.i15
  store i256 %add_result6, ptr %stack.top.addr.i.i8, align 8
  store i32 %stack.ptr.next.i.i9, ptr %0, align 4
  ret i8 0
}

attributes #0 = { alwaysinline mustprogress nofree norecurse nosync nounwind willreturn memory(argmem: readwrite) }
attributes #1 = { alwaysinline mustprogress nofree norecurse nosync nounwind willreturn memory(argmem: read) }
attributes #2 = { alwaysinline mustprogress nofree norecurse nosync nounwind willreturn memory(argmem: write) }
attributes #3 = { alwaysinline nounwind }
attributes #4 = { mustprogress nofree norecurse nosync nounwind willreturn memory(argmem: readwrite) }
attributes #5 = { nounwind }

