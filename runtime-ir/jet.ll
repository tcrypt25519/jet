; ModuleID = 'JetVM Runtime'
source_filename = "jet.ll"

; 'e' - Little-endian
; 'm:o' - MACH-O mangling
; 'p270:32:32' - Addr space with 32-bit pointers, 32-bit alignment
; 'p271:32:32' - Addr space with 32-bit pointers, 32-bit alignment
; 'p272:64:64' - Addr space with 64-bit pointers, 64-bit alignment
; 'i64:64' - 64-bit integers have 64-bit alignment (Other widths are natural aligned by default)
; 'n8:16:32:64' - Native integer widths
; 'S128' - Stack alignment of 128 bits
;
; Original before modification:
; target datalayout = "e-m:o-p270:32:32-p271:32:32-p272:64:64-i64:64-f80:128-n8:16:32:64-S128"
;
; Changes:
;   - Removed 'f80:128'
target datalayout = "e-m:o-p270:32:32-p271:32:32-p272:64:64-i64:64-n8:16:32:64-S128"
target triple = "x86_64-apple-macosx14.0.0"

;
; Globals
;
@jet.jit_engine = external global ptr

;
; Types
;
%jet.types.word = type [32 x i8]

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


;
; Forward declarations of Rust runtime functions
;
declare i1 @jet.stack.push.word (ptr, i256)
declare i1 @jet.stack.push.ptr (ptr, ptr)
declare ptr @jet.stack.pop (ptr)
declare ptr @jet.stack.peek (ptr, i8)
declare i1 @jet.stack.swap (ptr, i8)

declare i8 @jet.mem.store.word (ptr, ptr, ptr)
declare i8 @jet.mem.store.byte (ptr, ptr, ptr)
declare ptr @jet.mem.load (ptr, ptr)

declare i8 @jet.contract.call(ptr, ptr, ptr, ptr, ptr)
declare i8 @jet.contracts.call_return_data_copy(ptr, ptr, i32, i32, i32)

declare i8 @jet.ops.keccak256(ptr)

;
; IR-based runtime function
;
define i1 @jet.stack.push.i256 (%jet.types.exec_ctx*, i256) {
entry:
  ; Load stack pointer
  %stack.ptr.addr = getelementptr inbounds %jet.types.exec_ctx, ptr %0, i32 0, i32 0
  %stack.ptr = load i32, ptr %stack.ptr.addr
  %stack.top.addr = getelementptr inbounds %jet.types.exec_ctx, ptr %0, i32 0, i32 5, i32 %stack.ptr

  ; Store word
  store i256 %1, ptr %stack.top.addr

  ; Increment stack pointer
  %stack.ptr.next = add i32 %stack.ptr, 1
  store i32 %stack.ptr.next, ptr %stack.ptr.addr

  ret i1 true
}

