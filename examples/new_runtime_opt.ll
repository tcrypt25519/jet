; ModuleID = './target/debug/deps/jet_runtime.ll'
source_filename = "d7xneva4f7jgync6t34lz2407"
target datalayout = "e-m:o-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128"
target triple = "x86_64-apple-macosx10.12.0"

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(argmem: write) uwtable
define void @jet_new_context(ptr nocapture writeonly sret([65568 x i8]) align 8 %_0) unnamed_addr #0 !dbg !21 {
start:
  call void @llvm.dbg.declare(metadata ptr poison, metadata !44, metadata !DIExpression()), !dbg !46
  %0 = getelementptr inbounds i8, ptr %_0, i64 65564, !dbg !47
  tail call void @llvm.memset.p0.i64(ptr noundef nonnull align 8 dereferenceable(65564) %_0, i8 0, i64 65564, i1 false), !dbg !47
  store i32 32768, ptr %0, align 4, !dbg !47
  ret void, !dbg !48
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(argmem: readwrite) uwtable
define void @jet_new_contract_run(ptr nocapture writeonly sret([65576 x i8]) align 8 %_0, i32 %result, ptr nocapture readonly byval([65568 x i8]) align 8 %ctx) unnamed_addr #1 !dbg !49 {
start:
  tail call void @llvm.dbg.value(metadata i32 %result, metadata !57, metadata !DIExpression()), !dbg !59
  call void @llvm.dbg.declare(metadata ptr %ctx, metadata !58, metadata !DIExpression()), !dbg !60
  store i32 %result, ptr %_0, align 8, !dbg !61
  %0 = getelementptr inbounds i8, ptr %_0, i64 8, !dbg !61
  call void @llvm.memcpy.p0.p0.i64(ptr noundef nonnull align 8 dereferenceable(65568) %0, ptr noundef nonnull align 8 dereferenceable(65568) %ctx, i64 65568, i1 false), !dbg !61
  ret void, !dbg !62
}

; Function Attrs: mustprogress nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare void @llvm.dbg.declare(metadata, metadata, metadata) #2

; Function Attrs: mustprogress nocallback nofree nounwind willreturn memory(argmem: write)
declare void @llvm.memset.p0.i64(ptr nocapture writeonly, i8, i64, i1 immarg) #3

; Function Attrs: mustprogress nocallback nofree nounwind willreturn memory(argmem: readwrite)
declare void @llvm.memcpy.p0.p0.i64(ptr noalias nocapture writeonly, ptr noalias nocapture readonly, i64, i1 immarg) #4

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare void @llvm.dbg.value(metadata, metadata, metadata) #5

attributes #0 = { mustprogress nofree norecurse nosync nounwind willreturn memory(argmem: write) uwtable "frame-pointer"="all" "probe-stack"="inline-asm" "target-cpu"="penryn" }
attributes #1 = { mustprogress nofree norecurse nosync nounwind willreturn memory(argmem: readwrite) uwtable "frame-pointer"="all" "probe-stack"="inline-asm" "target-cpu"="penryn" }
attributes #2 = { mustprogress nocallback nofree nosync nounwind speculatable willreturn memory(none) }
attributes #3 = { mustprogress nocallback nofree nounwind willreturn memory(argmem: write) }
attributes #4 = { mustprogress nocallback nofree nounwind willreturn memory(argmem: readwrite) }
attributes #5 = { nocallback nofree nosync nounwind speculatable willreturn memory(none) }

!llvm.module.flags = !{!0, !1, !2}
!llvm.ident = !{!3}
!llvm.dbg.cu = !{!4}

!0 = !{i32 8, !"PIC Level", i32 2}
!1 = !{i32 2, !"Dwarf Version", i32 4}
!2 = !{i32 2, !"Debug Info Version", i32 3}
!3 = !{!"rustc version 1.81.0-nightly (a70b2ae57 2024-06-09)"}
!4 = distinct !DICompileUnit(language: DW_LANG_Rust, file: !5, producer: "clang LLVM (rustc version 1.81.0-nightly (a70b2ae57 2024-06-09))", isOptimized: false, runtimeVersion: 0, emissionKind: FullDebug, enums: !6, splitDebugInlining: false, nameTableKind: None)
!5 = !DIFile(filename: "runtime/src/lib.rs/@/d7xneva4f7jgync6t34lz2407", directory: "/Users/tcrypt/code/github.com/tyler-smith/jet")
!6 = !{!7}
!7 = !DICompositeType(tag: DW_TAG_enumeration_type, name: "ReturnCode", scope: !9, file: !8, baseType: !12, size: 32, align: 32, flags: DIFlagEnumClass, elements: !13)
!8 = !DIFile(filename: "<unknown>", directory: "")
!9 = !DINamespace(name: "types", scope: !10)
!10 = !DINamespace(name: "core", scope: !11)
!11 = !DINamespace(name: "jet_runtime", scope: null)
!12 = !DIBasicType(name: "i32", size: 32, encoding: DW_ATE_signed)
!13 = !{!14, !15, !16, !17, !18, !19, !20}
!14 = !DIEnumerator(name: "ImplicitReturn", value: 0)
!15 = !DIEnumerator(name: "ExplicitReturn", value: 1)
!16 = !DIEnumerator(name: "Stop", value: 2)
!17 = !DIEnumerator(name: "Revert", value: 64)
!18 = !DIEnumerator(name: "Invalid", value: 65)
!19 = !DIEnumerator(name: "JumpFailure", value: 66)
!20 = !DIEnumerator(name: "InvalidJumpBlock", value: -1)
!21 = distinct !DISubprogram(name: "jet_new_context", scope: !9, file: !22, line: 55, type: !23, scopeLine: 55, flags: DIFlagPrototyped, spFlags: DISPFlagDefinition, unit: !4, templateParams: !42, retainedNodes: !43)
!22 = !DIFile(filename: "runtime/src/core/types.rs", directory: "/Users/tcrypt/code/github.com/tyler-smith/jet", checksumkind: CSK_MD5, checksum: "842b2767406c527402e93db70fbb6294")
!23 = !DISubroutineType(types: !24)
!24 = !{!25}
!25 = distinct !DICompositeType(tag: DW_TAG_structure_type, name: "Context", scope: !9, file: !8, size: 524544, align: 64, flags: DIFlagPublic, elements: !26, templateParams: !42, identifier: "60cdca3f2601e086f2b208e23c131417")
!26 = !{!27, !29, !30, !31, !32, !34, !39, !40, !41}
!27 = !DIDerivedType(tag: DW_TAG_member, name: "stack_ptr", scope: !25, file: !8, baseType: !28, size: 32, align: 32, flags: DIFlagPrivate)
!28 = !DIBasicType(name: "u32", size: 32, encoding: DW_ATE_unsigned)
!29 = !DIDerivedType(tag: DW_TAG_member, name: "jump_ptr", scope: !25, file: !8, baseType: !28, size: 32, align: 32, offset: 32, flags: DIFlagPrivate)
!30 = !DIDerivedType(tag: DW_TAG_member, name: "return_off", scope: !25, file: !8, baseType: !28, size: 32, align: 32, offset: 64, flags: DIFlagPrivate)
!31 = !DIDerivedType(tag: DW_TAG_member, name: "return_len", scope: !25, file: !8, baseType: !28, size: 32, align: 32, offset: 96, flags: DIFlagPrivate)
!32 = !DIDerivedType(tag: DW_TAG_member, name: "sub_call", scope: !25, file: !8, baseType: !33, size: 64, align: 64, offset: 128, flags: DIFlagPrivate)
!33 = !DIBasicType(name: "usize", size: 64, encoding: DW_ATE_unsigned)
!34 = !DIDerivedType(tag: DW_TAG_member, name: "stack", scope: !25, file: !8, baseType: !35, size: 262144, align: 8, offset: 192, flags: DIFlagPrivate)
!35 = !DICompositeType(tag: DW_TAG_array_type, baseType: !36, size: 262144, align: 8, elements: !37)
!36 = !DIBasicType(name: "u8", size: 8, encoding: DW_ATE_unsigned)
!37 = !{!38}
!38 = !DISubrange(count: 32768, lowerBound: 0)
!39 = !DIDerivedType(tag: DW_TAG_member, name: "memory", scope: !25, file: !8, baseType: !35, size: 262144, align: 8, offset: 262336, flags: DIFlagPrivate)
!40 = !DIDerivedType(tag: DW_TAG_member, name: "memory_len", scope: !25, file: !8, baseType: !28, size: 32, align: 32, offset: 524480, flags: DIFlagPrivate)
!41 = !DIDerivedType(tag: DW_TAG_member, name: "memory_cap", scope: !25, file: !8, baseType: !28, size: 32, align: 32, offset: 524512, flags: DIFlagPrivate)
!42 = !{}
!43 = !{!44}
!44 = !DILocalVariable(name: "init_memory_buf", scope: !45, file: !22, line: 56, type: !35, align: 1)
!45 = distinct !DILexicalBlock(scope: !21, file: !22, line: 56, column: 5)
!46 = !DILocation(line: 56, column: 9, scope: !45)
!47 = !DILocation(line: 57, column: 5, scope: !45)
!48 = !DILocation(line: 68, column: 2, scope: !21)
!49 = distinct !DISubprogram(name: "jet_new_contract_run", scope: !9, file: !22, line: 71, type: !50, scopeLine: 71, flags: DIFlagPrototyped, spFlags: DISPFlagDefinition, unit: !4, templateParams: !42, retainedNodes: !56)
!50 = !DISubroutineType(types: !51)
!51 = !{!52, !7, !25}
!52 = distinct !DICompositeType(tag: DW_TAG_structure_type, name: "ContractRun", scope: !9, file: !8, size: 524608, align: 64, flags: DIFlagPublic, elements: !53, templateParams: !42, identifier: "681bab3e1138f10a215ce9e0654468e7")
!53 = !{!54, !55}
!54 = !DIDerivedType(tag: DW_TAG_member, name: "result", scope: !52, file: !8, baseType: !7, size: 32, align: 32, flags: DIFlagPrivate)
!55 = !DIDerivedType(tag: DW_TAG_member, name: "ctx", scope: !52, file: !8, baseType: !25, size: 524544, align: 64, offset: 64, flags: DIFlagPrivate)
!56 = !{!57, !58}
!57 = !DILocalVariable(name: "result", arg: 1, scope: !49, file: !22, line: 71, type: !7)
!58 = !DILocalVariable(name: "ctx", arg: 2, scope: !49, file: !22, line: 71, type: !25)
!59 = !DILocation(line: 0, scope: !49)
!60 = !DILocation(line: 71, column: 60, scope: !49)
!61 = !DILocation(line: 72, column: 5, scope: !49)
!62 = !DILocation(line: 73, column: 2, scope: !49)

