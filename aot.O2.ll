; ModuleID = 'aot.o.ll'
source_filename = "tcg_aot"
target datalayout = "e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128"
target triple = "x86_64-pc-linux-gnu"

@tb_index = local_unnamed_addr constant [59 x i64] [i64 143662, i64 66112, i64 133348, i64 133526, i64 320346, i64 322544, i64 318342, i64 322602, i64 322654, i64 322738, i64 68302, i64 68970, i64 68604, i64 68490, i64 68622, i64 68632, i64 74642, i64 71022, i64 71036, i64 71136, i64 71150, i64 71246, i64 71260, i64 71356, i64 71528, i64 68454, i64 72478, i64 72562, i64 72628, i64 72638, i64 68576, i64 69394, i64 69610, i64 69468, i64 69526, i64 69546, i64 69996, i64 70004, i64 70016, i64 70024, i64 77362, i64 89340, i64 83278, i64 133428, i64 199100, i64 83294, i64 84240, i64 84956, i64 83790, i64 83804, i64 89352, i64 199036, i64 89358, i64 77410, i64 199292, i64 199240, i64 202460, i64 201610, i64 0]

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(read, argmem: readwrite, inaccessiblemem: none)
define noundef i64 @tb_2312e(ptr captures(none) initializes((104, 144), (512, 520)) %0, i64 %1) local_unnamed_addr #0 {
entry:
  %2 = getelementptr i8, ptr %0, i64 80
  %3 = load i64, ptr %2, align 4, !tbaa !1
  %4 = getelementptr i8, ptr %0, i64 104
  %5 = getelementptr i8, ptr %0, i64 120
  %6 = getelementptr i8, ptr %0, i64 128
  %7 = getelementptr i8, ptr %0, i64 136
  %8 = and i64 %3, 4294967295
  %9 = shl nuw nsw i64 %8, 7
  %10 = add nuw nsw i64 %9, 505624
  %11 = add i64 %1, 505672
  %12 = add i64 %11, %9
  %13 = inttoptr i64 %12 to ptr
  %14 = load i32, ptr %13, align 4, !tbaa !4
  %15 = sext i32 %14 to i64
  %16 = icmp ugt i32 %14, 2
  store i64 %8, ptr %2, align 4, !tbaa !1
  store i64 %10, ptr %4, align 4, !tbaa !1
  store i64 505624, ptr %5, align 4, !tbaa !1
  store i64 %15, ptr %6, align 4, !tbaa !1
  store i64 2, ptr %7, align 4, !tbaa !1
  br i1 %16, label %L0, label %fall

common.ret:                                       ; preds = %fall, %L0
  %.sink = phi i64 [ %19, %L0 ], [ %23, %fall ]
  %storemerge = phi i64 [ 143698, %L0 ], [ %..i, %fall ]
  %17 = getelementptr i8, ptr %0, i64 112
  %18 = getelementptr i8, ptr %0, i64 512
  store i64 %.sink, ptr %17, align 4, !tbaa !1
  store i64 %storemerge, ptr %18, align 4, !tbaa !1
  ret i64 4294967298

L0:                                               ; preds = %entry
  %19 = add nuw nsw i64 %9, 505712
  br label %common.ret

fall:                                             ; preds = %entry
  %20 = add i64 %1, 88
  %21 = add i64 %20, %10
  %22 = inttoptr i64 %21 to ptr
  %23 = load i64, ptr %22, align 4, !tbaa !4
  %24 = icmp eq i32 %14, 0
  %..i = select i1 %24, i64 143724, i64 143698
  br label %common.ret
}

; Function Attrs: nofree nosync nounwind memory(readwrite, inaccessiblemem: none)
define range(i64 2, 4294967299) i64 @tb_10240(ptr initializes((48, 56), (224, 232), (512, 520)) %0, i64 %1) local_unnamed_addr #1 {
entry:
  %2 = getelementptr i8, ptr %0, i64 48
  %3 = getelementptr i8, ptr %0, i64 224
  %4 = getelementptr i8, ptr %0, i64 512
  %5 = add i64 %1, 512000
  %6 = inttoptr i64 %5 to ptr
  %7 = load i64, ptr %6, align 4, !tbaa !4
  %8 = and i64 %7, -2
  store i64 66124, ptr %2, align 4, !tbaa !1
  store i64 %7, ptr %3, align 4, !tbaa !1
  store i64 %8, ptr %4, align 4, !tbaa !1
  %9 = musttail call i64 @aot_dispatch(ptr %0, i64 %1)
  ret i64 %9
}

; Function Attrs: nofree nosync nounwind memory(readwrite, inaccessiblemem: none)
define hidden range(i64 2, 4294967299) i64 @aot_dispatch(ptr %0, i64 %1) local_unnamed_addr #1 {
entry:
  %2 = getelementptr i8, ptr %0, i64 512
  %3 = getelementptr i8, ptr %0, i64 8
  %.promoted461 = load i64, ptr %2, align 8
  %4 = getelementptr i8, ptr %0, i64 48
  %5 = getelementptr i8, ptr %0, i64 224
  %6 = add i64 %1, 512000
  %7 = inttoptr i64 %6 to ptr
  br label %tailrecurse

tailrecurse:                                      ; preds = %tailrecurse.backedge, %entry
  %8 = phi i64 [ %.promoted461, %entry ], [ %.be, %tailrecurse.backedge ]
  switch i64 %8, label %common.ret [
    i64 143662, label %pc_2312e
    i64 66112, label %pc_10240
    i64 133348, label %pc_208e4
    i64 133526, label %pc_20996
    i64 320346, label %pc_4e35a
    i64 322544, label %pc_4ebf0
    i64 318342, label %pc_4db86
    i64 322602, label %pc_4ec2a
    i64 322654, label %pc_4ec5e
    i64 322738, label %pc_4ecb2
    i64 68302, label %pc_10ace
    i64 68970, label %pc_10d6a
    i64 135670, label %pc_211f6
    i64 68604, label %pc_10bfc
    i64 68490, label %pc_10b8a
    i64 68622, label %pc_10c0e
    i64 68632, label %pc_10c18
    i64 74642, label %pc_12392
    i64 71022, label %pc_1156e
    i64 71036, label %pc_1157c
    i64 71136, label %pc_115e0
    i64 71150, label %pc_115ee
    i64 71246, label %pc_1164e
    i64 71260, label %pc_1165c
    i64 71356, label %pc_116bc
    i64 71528, label %pc_11768
    i64 68454, label %pc_10b66
    i64 72478, label %pc_11b1e
    i64 72562, label %pc_11b72
    i64 72628, label %pc_11bb4
    i64 72638, label %pc_11bbe
    i64 68576, label %pc_10be0
    i64 69394, label %pc_10f12
    i64 69610, label %pc_10fea
    i64 69468, label %pc_10f5c
    i64 69526, label %pc_10f96
    i64 69546, label %pc_10faa
    i64 69996, label %pc_1116c
    i64 70004, label %pc_11174
    i64 70016, label %pc_11180
    i64 70024, label %pc_11188
    i64 77362, label %pc_12e32
    i64 89340, label %pc_15cfc
    i64 83278, label %pc_1454e
    i64 133428, label %pc_20934
    i64 199100, label %pc_309bc
    i64 83294, label %pc_1455e
    i64 84240, label %pc_14910
    i64 84956, label %pc_14bdc
    i64 83790, label %pc_1474e
    i64 83804, label %pc_1475c
    i64 89352, label %pc_15d08
    i64 199036, label %pc_3097c
    i64 89358, label %pc_15d0e
    i64 77410, label %pc_12e62
    i64 199292, label %pc_30a7c
    i64 199240, label %pc_30a48
    i64 202460, label %pc_316dc
    i64 201610, label %pc_3138a
    i64 322674, label %pc_4ec72
    i64 68594, label %pc_10bf2
    i64 322754, label %pc_4ecc2
    i64 322776, label %pc_4ecd8
    i64 199176, label %pc_30a08
    i64 83298, label %pc_14562
    i64 84732, label %pc_14afc
    i64 69456, label %pc_10f50
    i64 70648, label %pc_113f8
    i64 72470, label %pc_11b16
    i64 72644, label %pc_11bc4
    i64 202452, label %pc_316d4
    i64 320362, label %pc_4e36a
    i64 201632, label %pc_313a0
    i64 133488, label %pc_20970
    i64 199020, label %pc_3096c
    i64 198232, label %pc_30658
    i64 73664, label %pc_11fc0
    i64 83476, label %pc_14614
    i64 70460, label %pc_1133c
    i64 143692, label %pc_2314c
    i64 68316, label %pc_10adc
    i64 83210, label %pc_1450a
    i64 70536, label %pc_11388
    i64 84266, label %pc_1492a
    i64 68598, label %pc_10bf6
    i64 69054, label %pc_10dbe
    i64 68372, label %pc_10b14
    i64 71070, label %pc_1159e
    i64 322610, label %pc_4ec32
    i64 320562, label %pc_4e432
    i64 89022, label %pc_15bbe
    i64 199268, label %pc_30a64
    i64 199244, label %pc_30a4c
    i64 202470, label %pc_316e6
    i64 322664, label %pc_4ec68
    i64 69114, label %pc_10dfa
    i64 71292, label %pc_1167c
    i64 73184, label %pc_11de0
    i64 70030, label %pc_1118e
    i64 72586, label %pc_11b8a
    i64 69552, label %pc_10fb0
    i64 69444, label %pc_10f44
    i64 70034, label %pc_11192
    i64 71362, label %pc_116c2
    i64 201628, label %pc_3139c
    i64 199432, label %pc_30b08
    i64 318226, label %pc_4db12
    i64 135676, label %pc_211fc
    i64 69434, label %common.ret.sink.split
    i64 69478, label %pc_10f66
    i64 83818, label %pc_1476a
    i64 199326, label %pc_30a9e
    i64 133436, label %pc_2093c
    i64 72618, label %pc_11baa
    i64 71182, label %pc_1160e
    i64 199052, label %pc_3098c
    i64 322746, label %pc_4ecba
    i64 68304, label %pc_10ad0
    i64 69620, label %pc_10ff4
    i64 68974, label %pc_10d6e
    i64 143728, label %pc_23170
    i64 133360, label %pc_208f0
    i64 71394, label %pc_116e2
    i64 135674, label %pc_211fa
    i64 133462, label %pc_20956
    i64 322882, label %pc_4ed42
    i64 199116, label %pc_309cc
    i64 83812, label %pc_14764
    i64 322554, label %pc_4ebfa
    i64 69532, label %pc_10f9c
    i64 72554, label %pc_11b6a
    i64 72502, label %pc_11b36
    i64 133480, label %pc_20968
    i64 83776, label %pc_14740
    i64 68472, label %pc_10b78
  ]

common.ret.sink.split:                            ; preds = %tailrecurse, %fall.i50, %L0.i46, %fall.i40, %pc_3139c, %fall.i1, %L0.i4, %fall.i, %L0.i, %pc_1156e, %pc_115ee, %pc_1165c, %tb_11188.exit, %pc_14bdc, %pc_1474e, %pc_15d08, %pc_4ec72, %pc_10bf2, %pc_4ecc2, %pc_4ecd8, %pc_113f8, %pc_11b16, %pc_316d4, %pc_4e36a, %pc_313a0, %pc_20970, %pc_3096c, %pc_30658, %pc_1133c, %pc_2314c, %pc_11388, %pc_1492a, %pc_10bf6, %pc_10dbe, %pc_10b14, %pc_4ec32, %pc_4e432, %pc_316e6, %pc_4ec68, %pc_1118e, %pc_11b8a, %pc_11192, %pc_4db12, %pc_211fc, %pc_30a9e, %pc_3098c, %pc_10d6e, %pc_23170, %pc_116e2, %pc_4ed42, %pc_309cc, %pc_4ebfa, %pc_11b6a, %pc_14740
  %.sink = phi i64 [ 130504, %pc_14740 ], [ 71878, %pc_11b6a ], [ %..i56, %pc_4ebfa ], [ %..i55, %pc_309cc ], [ 322688, %pc_4ed42 ], [ %..i53, %pc_116e2 ], [ 143698, %pc_23170 ], [ %..i52, %pc_10d6e ], [ %..i44, %pc_3098c ], [ 199344, %pc_30a9e ], [ %..i43, %pc_211fc ], [ %..i42, %pc_4db12 ], [ %..i39, %pc_11192 ], [ %..i37, %pc_11b8a ], [ %..i35, %pc_1118e ], [ %..i33, %pc_4ec68 ], [ 201536, %pc_316e6 ], [ %..i31, %pc_4e432 ], [ %..i29, %pc_4ec32 ], [ %..i27, %pc_10b14 ], [ %..i26, %pc_10dbe ], [ 68480, %pc_10bf6 ], [ %..i25, %pc_1492a ], [ %..i24, %pc_11388 ], [ %..i23, %pc_2314c ], [ %..i22, %pc_1133c ], [ %..i21, %pc_30658 ], [ %..i20, %pc_3096c ], [ %..i19, %pc_20970 ], [ 201586, %pc_313a0 ], [ %..i18, %pc_4e36a ], [ 199192, %pc_316d4 ], [ 71878, %pc_11b16 ], [ %..i17, %pc_113f8 ], [ %..i16, %pc_4ecd8 ], [ 322626, %pc_4ecc2 ], [ 68480, %pc_10bf2 ], [ %..i, %pc_4ec72 ], [ %..i.i13, %pc_15d08 ], [ %..i.i12, %pc_1474e ], [ 130504, %pc_14bdc ], [ %..i110.i, %tb_11188.exit ], [ %..i.i9, %pc_1165c ], [ %..i.i7, %pc_115ee ], [ %..i.i5, %pc_1156e ], [ 322688, %L0.i ], [ %..i.i, %fall.i ], [ %..i.i2, %fall.i1 ], [ 322626, %L0.i4 ], [ 201586, %fall.i40 ], [ 201636, %pc_3139c ], [ %..i.i48, %L0.i46 ], [ 322626, %fall.i50 ], [ 69576, %tailrecurse ]
  store i64 %.sink, ptr %2, align 4, !tbaa !1
  br label %common.ret

common.ret:                                       ; preds = %tailrecurse, %common.ret.sink.split
  %common.ret.op = phi i64 [ 4294967298, %common.ret.sink.split ], [ 2, %tailrecurse ]
  ret i64 %common.ret.op

pc_2312e:                                         ; preds = %tailrecurse
  %9 = musttail call i64 @tb_2312e(ptr nonnull %0, i64 %1)
  ret i64 %9

pc_10240:                                         ; preds = %tailrecurse
  %10 = load i64, ptr %7, align 4, !tbaa !4
  store i64 66124, ptr %4, align 4, !tbaa !1
  store i64 %10, ptr %5, align 4, !tbaa !1
  br label %tailrecurse.backedge

pc_208e4:                                         ; preds = %tailrecurse
  %11 = musttail call i64 @tb_208e4(ptr nonnull %0, i64 %1)
  ret i64 %11

pc_20996:                                         ; preds = %tailrecurse
  %12 = musttail call i64 @tb_20996(ptr nonnull %0, i64 %1)
  ret i64 %12

pc_4e35a:                                         ; preds = %tailrecurse
  %13 = musttail call i64 @tb_4e35a(ptr nonnull %0, i64 %1)
  ret i64 %13

pc_4ebf0:                                         ; preds = %tailrecurse
  %14 = getelementptr i8, ptr %0, i64 80
  %15 = load i64, ptr %14, align 4, !tbaa !1
  %16 = getelementptr i8, ptr %0, i64 112
  %17 = getelementptr i8, ptr %0, i64 176
  %18 = icmp eq i64 %15, 255
  store i64 255, ptr %16, align 4, !tbaa !1
  store i64 %15, ptr %17, align 4, !tbaa !1
  br i1 %18, label %L0.i, label %fall.i

L0.i:                                             ; preds = %pc_4ebf0
  %19 = getelementptr i8, ptr %0, i64 160
  store i64 -1, ptr %19, align 4, !tbaa !1
  br label %common.ret.sink.split

fall.i:                                           ; preds = %pc_4ebf0
  %20 = getelementptr i8, ptr %0, i64 192
  %21 = getelementptr i8, ptr %0, i64 96
  %22 = getelementptr i8, ptr %0, i64 104
  %23 = and i64 %15, 112
  %24 = and i64 %15, 255
  %25 = icmp eq i64 %23, 32
  store i64 32, ptr %21, align 4, !tbaa !1
  store i64 %23, ptr %22, align 4, !tbaa !1
  store i64 %24, ptr %20, align 4, !tbaa !1
  %..i.i = select i1 %25, i64 322844, i64 322570
  br label %common.ret.sink.split

pc_4db86:                                         ; preds = %tailrecurse
  %26 = musttail call i64 @tb_4db86(ptr nonnull %0, i64 %1)
  ret i64 %26

pc_4ec2a:                                         ; preds = %tailrecurse
  %27 = getelementptr i8, ptr %0, i64 88
  %28 = getelementptr i8, ptr %0, i64 168
  %29 = load i64, ptr %28, align 4, !tbaa !1
  %30 = getelementptr i8, ptr %0, i64 192
  %31 = load i64, ptr %30, align 4, !tbaa !1
  %32 = and i64 %31, 7
  %33 = icmp eq i64 %32, %29
  store i64 %32, ptr %27, align 4, !tbaa !1
  br i1 %33, label %L0.i4, label %fall.i1

L0.i4:                                            ; preds = %pc_4ec2a
  %34 = getelementptr i8, ptr %0, i64 120
  %35 = getelementptr i8, ptr %0, i64 104
  store i64 65535, ptr %35, align 4, !tbaa !1
  store i64 65536, ptr %34, align 4, !tbaa !1
  br label %common.ret.sink.split

fall.i1:                                          ; preds = %pc_4ec2a
  %.not.i.i = icmp ult i64 %29, %32
  %..i.i2 = select i1 %.not.i.i, i64 322614, i64 322860
  br label %common.ret.sink.split

pc_4ec5e:                                         ; preds = %tailrecurse
  %36 = musttail call i64 @tb_4ec5e(ptr nonnull %0, i64 %1)
  ret i64 %36

pc_4ecb2:                                         ; preds = %tailrecurse
  %37 = musttail call i64 @tb_4ecb2(ptr nonnull %0, i64 %1)
  ret i64 %37

pc_10ace:                                         ; preds = %tailrecurse
  %38 = musttail call i64 @tb_10ace(ptr nonnull %0, i64 %1)
  ret i64 %38

pc_10d6a:                                         ; preds = %tailrecurse
  %39 = musttail call i64 @tb_10d6a(ptr nonnull %0, i64 %1)
  ret i64 %39

pc_211f6:                                         ; preds = %tailrecurse
  %40 = musttail call i64 @tb_211f6(ptr nonnull %0, i64 %1)
  ret i64 %40

pc_10bfc:                                         ; preds = %tailrecurse
  %41 = musttail call i64 @tb_10bfc(ptr nonnull %0, i64 %1)
  ret i64 %41

pc_10b8a:                                         ; preds = %tailrecurse
  %42 = musttail call i64 @tb_10b8a(ptr nonnull %0, i64 %1)
  ret i64 %42

pc_10c0e:                                         ; preds = %tailrecurse
  %43 = musttail call i64 @tb_10c0e(ptr nonnull %0, i64 %1)
  ret i64 %43

pc_10c18:                                         ; preds = %tailrecurse
  %44 = musttail call i64 @tb_10c18(ptr nonnull %0, i64 %1)
  ret i64 %44

pc_12392:                                         ; preds = %tailrecurse
  %45 = musttail call i64 @tb_12392(ptr nonnull %0, i64 %1)
  ret i64 %45

pc_1156e:                                         ; preds = %tailrecurse
  %46 = getelementptr i8, ptr %0, i64 64
  %47 = load i64, ptr %46, align 4, !tbaa !1
  %48 = getelementptr i8, ptr %0, i64 72
  %49 = load i64, ptr %48, align 4, !tbaa !1
  %50 = getelementptr i8, ptr %0, i64 80
  %51 = load i64, ptr %50, align 4, !tbaa !1
  %52 = getelementptr i8, ptr %0, i64 88
  %53 = getelementptr i8, ptr %0, i64 96
  %54 = getelementptr i8, ptr %0, i64 144
  %55 = getelementptr i8, ptr %0, i64 184
  store i64 71036, ptr %3, align 4, !tbaa !1
  store i64 %49, ptr %50, align 4, !tbaa !1
  store i64 %47, ptr %52, align 4, !tbaa !1
  %56 = load <2 x i64>, ptr %54, align 4, !tbaa !1
  %57 = shufflevector <2 x i64> %56, <2 x i64> poison, <2 x i32> <i32 1, i32 0>
  store <2 x i64> %57, ptr %53, align 4, !tbaa !1
  store i64 %51, ptr %55, align 4, !tbaa !1
  %58 = icmp eq i64 %49, 0
  %..i.i5 = select i1 %58, i64 70534, i64 70462
  br label %common.ret.sink.split

pc_1157c:                                         ; preds = %tailrecurse
  %59 = musttail call i64 @tb_1157c(ptr nonnull %0, i64 %1)
  ret i64 %59

pc_115e0:                                         ; preds = %tailrecurse
  %60 = musttail call i64 @tb_115e0(ptr nonnull %0, i64 poison)
  ret i64 %60

pc_115ee:                                         ; preds = %tailrecurse
  %61 = getelementptr i8, ptr %0, i64 80
  %62 = getelementptr i8, ptr %0, i64 88
  %63 = getelementptr i8, ptr %0, i64 96
  %64 = getelementptr i8, ptr %0, i64 104
  %65 = getelementptr i8, ptr %0, i64 120
  %66 = getelementptr i8, ptr %0, i64 128
  %67 = getelementptr i8, ptr %0, i64 168
  %68 = load i64, ptr %67, align 4, !tbaa !1
  store i64 0, ptr %61, align 4, !tbaa !1
  tail call void @llvm.memset.p0.i64(ptr noundef nonnull align 4 dereferenceable(24) %63, i8 0, i64 24, i1 false)
  tail call void @llvm.memset.p0.i64(ptr noundef nonnull align 4 dereferenceable(16) %66, i8 0, i64 16, i1 false)
  %69 = getelementptr i8, ptr %0, i64 64
  %70 = load i64, ptr %69, align 4, !tbaa !1
  %71 = add i64 %70, %1
  %72 = inttoptr i64 %71 to ptr
  %73 = load i32, ptr %72, align 4, !tbaa !4
  %74 = sext i32 %73 to i64
  %75 = icmp sgt i32 %73, 0
  %76 = zext i1 %75 to i64
  %.not.i.i6 = icmp slt i64 %68, %74
  store i64 %76, ptr %62, align 4, !tbaa !1
  store i64 %74, ptr %63, align 4, !tbaa !1
  store i64 %74, ptr %64, align 4, !tbaa !1
  store i64 10, ptr %65, align 4, !tbaa !1
  %..i.i7 = select i1 %.not.i.i6, i64 71214, i64 71164
  br label %common.ret.sink.split

pc_1164e:                                         ; preds = %tailrecurse
  %77 = musttail call i64 @tb_1164e(ptr nonnull %0, i64 poison)
  ret i64 %77

pc_1165c:                                         ; preds = %tailrecurse
  %78 = getelementptr i8, ptr %0, i64 80
  %79 = getelementptr i8, ptr %0, i64 88
  %80 = getelementptr i8, ptr %0, i64 96
  %81 = getelementptr i8, ptr %0, i64 112
  %82 = getelementptr i8, ptr %0, i64 120
  %83 = getelementptr i8, ptr %0, i64 128
  %84 = getelementptr i8, ptr %0, i64 168
  %85 = load i64, ptr %84, align 4, !tbaa !1
  store i64 0, ptr %78, align 4, !tbaa !1
  tail call void @llvm.memset.p0.i64(ptr noundef nonnull align 4 dereferenceable(24) %80, i8 0, i64 24, i1 false)
  tail call void @llvm.memset.p0.i64(ptr noundef nonnull align 4 dereferenceable(16) %83, i8 0, i64 16, i1 false)
  %86 = getelementptr i8, ptr %0, i64 64
  %87 = load i64, ptr %86, align 4, !tbaa !1
  %88 = add i64 %87, %1
  %89 = inttoptr i64 %88 to ptr
  %90 = load i32, ptr %89, align 4, !tbaa !4
  %91 = sext i32 %90 to i64
  %92 = icmp sgt i32 %90, 0
  %93 = zext i1 %92 to i64
  %.not.i.i8 = icmp slt i64 %85, %91
  store i64 %93, ptr %79, align 4, !tbaa !1
  store i64 %91, ptr %80, align 4, !tbaa !1
  store i64 %91, ptr %81, align 4, !tbaa !1
  store i64 10, ptr %82, align 4, !tbaa !1
  %..i.i9 = select i1 %.not.i.i8, i64 71324, i64 71274
  br label %common.ret.sink.split

pc_116bc:                                         ; preds = %tailrecurse
  %94 = musttail call i64 @tb_116bc(ptr nonnull %0, i64 %1)
  ret i64 %94

pc_11768:                                         ; preds = %tailrecurse
  %95 = musttail call i64 @tb_11768(ptr nonnull %0, i64 %1)
  ret i64 %95

pc_10b66:                                         ; preds = %tailrecurse
  %96 = musttail call i64 @tb_10b66(ptr nonnull %0, i64 %1)
  ret i64 %96

pc_11b1e:                                         ; preds = %tailrecurse
  %97 = musttail call i64 @tb_11b1e(ptr nonnull %0, i64 %1)
  ret i64 %97

pc_11b72:                                         ; preds = %tailrecurse
  %98 = musttail call i64 @tb_11b72(ptr nonnull %0, i64 %1)
  ret i64 %98

pc_11bb4:                                         ; preds = %tailrecurse
  %99 = musttail call i64 @tb_11bb4(ptr nonnull %0, i64 %1)
  ret i64 %99

pc_11bbe:                                         ; preds = %tailrecurse
  %100 = musttail call i64 @tb_11bbe(ptr nonnull %0, i64 %1)
  ret i64 %100

pc_10be0:                                         ; preds = %tailrecurse
  %101 = musttail call i64 @tb_10be0(ptr nonnull %0, i64 %1)
  ret i64 %101

pc_10f12:                                         ; preds = %tailrecurse
  %102 = musttail call i64 @tb_10f12(ptr nonnull %0, i64 %1)
  ret i64 %102

pc_10fea:                                         ; preds = %tailrecurse
  %103 = musttail call i64 @tb_10fea(ptr nonnull %0, i64 %1)
  ret i64 %103

pc_10f5c:                                         ; preds = %tailrecurse
  %104 = musttail call i64 @tb_10f5c(ptr nonnull %0, i64 %1)
  ret i64 %104

pc_10f96:                                         ; preds = %tailrecurse
  %105 = musttail call i64 @tb_10f96(ptr nonnull %0, i64 %1)
  ret i64 %105

pc_10faa:                                         ; preds = %tailrecurse
  %106 = musttail call i64 @tb_10faa(ptr nonnull %0, i64 %1)
  ret i64 %106

pc_1116c:                                         ; preds = %tailrecurse
  %107 = getelementptr i8, ptr %0, i64 64
  %108 = load i64, ptr %107, align 4, !tbaa !1
  %109 = getelementptr i8, ptr %0, i64 88
  %110 = add i64 %1, 96
  %111 = add i64 %110, %108
  %112 = inttoptr i64 %111 to ptr
  %113 = load i16, ptr %112, align 2, !tbaa !4
  %114 = zext i16 %113 to i64
  store i64 70004, ptr %3, align 4, !tbaa !1
  store i64 %114, ptr %109, align 4, !tbaa !1
  store i64 73184, ptr %2, align 4, !tbaa !1
  %115 = musttail call range(i64 2, 4294967299) i64 @tb_11de0(ptr nonnull %0, i64 %1)
  ret i64 %115

pc_11174:                                         ; preds = %tailrecurse
  %116 = musttail call i64 @tb_11174(ptr nonnull %0, i64 %1)
  ret i64 %116

pc_11180:                                         ; preds = %tailrecurse
  %117 = getelementptr i8, ptr %0, i64 64
  %118 = load i64, ptr %117, align 4, !tbaa !1
  %119 = getelementptr i8, ptr %0, i64 88
  %120 = add i64 %1, 96
  %121 = add i64 %120, %118
  %122 = inttoptr i64 %121 to ptr
  %123 = load i16, ptr %122, align 2, !tbaa !4
  %124 = zext i16 %123 to i64
  store i64 70024, ptr %3, align 4, !tbaa !1
  store i64 %124, ptr %119, align 4, !tbaa !1
  store i64 73184, ptr %2, align 4, !tbaa !1
  %125 = musttail call range(i64 2, 4294967299) i64 @tb_11de0(ptr nonnull %0, i64 %1)
  ret i64 %125

pc_11188:                                         ; preds = %tailrecurse
  %126 = getelementptr i8, ptr %0, i64 64
  %127 = load i64, ptr %126, align 4, !tbaa !1
  %128 = getelementptr i8, ptr %0, i64 72
  %129 = load i64, ptr %128, align 4, !tbaa !1
  %130 = getelementptr i8, ptr %0, i64 80
  %131 = load i64, ptr %130, align 4, !tbaa !1
  %132 = getelementptr i8, ptr %0, i64 144
  %133 = load i64, ptr %132, align 4, !tbaa !1
  %134 = add i64 %127, %1
  %135 = add i64 %134, 96
  %136 = inttoptr i64 %135 to ptr
  %137 = trunc i64 %131 to i16
  store i16 %137, ptr %136, align 2, !tbaa !4
  %.not.i = icmp eq i64 %129, 0
  br i1 %.not.i, label %fall.i11, label %L0.i10

L0.i10:                                           ; preds = %pc_11188
  %138 = add i64 %129, 1
  br label %tb_11188.exit

fall.i11:                                         ; preds = %pc_11188
  %139 = add i64 %134, 98
  %140 = inttoptr i64 %139 to ptr
  store i16 %137, ptr %140, align 2, !tbaa !4
  br label %tb_11188.exit

tb_11188.exit:                                    ; preds = %L0.i10, %fall.i11
  %.sink112.i = phi i64 [ 1, %fall.i11 ], [ %138, %L0.i10 ]
  %.not.i109.i = icmp eq i64 %133, %.sink112.i
  %..i110.i = select i1 %.not.i109.i, i64 70040, i64 69988
  store i64 %.sink112.i, ptr %128, align 4, !tbaa !1
  br label %common.ret.sink.split

pc_12e32:                                         ; preds = %tailrecurse
  %141 = musttail call i64 @tb_12e32(ptr nonnull %0, i64 %1)
  ret i64 %141

pc_15cfc:                                         ; preds = %tailrecurse
  %142 = musttail call i64 @tb_15cfc(ptr nonnull %0, i64 %1)
  ret i64 %142

pc_1454e:                                         ; preds = %tailrecurse
  %143 = musttail call i64 @tb_1454e(ptr nonnull %0, i64 %1)
  ret i64 %143

pc_20934:                                         ; preds = %tailrecurse
  %144 = musttail call i64 @tb_20934(ptr nonnull %0, i64 %1)
  ret i64 %144

pc_309bc:                                         ; preds = %tailrecurse
  %145 = musttail call i64 @tb_309bc(ptr nonnull %0, i64 %1)
  ret i64 %145

pc_1455e:                                         ; preds = %tailrecurse
  %146 = musttail call i64 @tb_1455e(ptr nonnull %0, i64 %1)
  ret i64 %146

pc_14910:                                         ; preds = %tailrecurse
  %147 = musttail call i64 @tb_14910(ptr nonnull %0, i64 %1)
  ret i64 %147

pc_14bdc:                                         ; preds = %tailrecurse
  %148 = getelementptr i8, ptr %0, i64 80
  %149 = getelementptr i8, ptr %0, i64 88
  %150 = getelementptr i8, ptr %0, i64 152
  %151 = getelementptr i8, ptr %0, i64 208
  %152 = load i64, ptr %151, align 4, !tbaa !1
  %153 = add i64 %152, 1
  store i64 83790, ptr %3, align 4, !tbaa !1
  store i64 %153, ptr %148, align 4, !tbaa !1
  store i64 37, ptr %149, align 4, !tbaa !1
  store i64 %153, ptr %150, align 4, !tbaa !1
  br label %common.ret.sink.split

pc_1474e:                                         ; preds = %tailrecurse
  %154 = getelementptr i8, ptr %0, i64 64
  %155 = load i64, ptr %154, align 4, !tbaa !1
  %156 = getelementptr i8, ptr %0, i64 80
  %157 = load i64, ptr %156, align 4, !tbaa !1
  %158 = getelementptr i8, ptr %0, i64 88
  %159 = getelementptr i8, ptr %0, i64 96
  %160 = getelementptr i8, ptr %0, i64 120
  %161 = getelementptr i8, ptr %0, i64 152
  %162 = load i64, ptr %161, align 4, !tbaa !1
  %163 = getelementptr i8, ptr %0, i64 208
  %164 = sub i64 %157, %162
  store i64 83804, ptr %3, align 4, !tbaa !1
  store i64 %155, ptr %156, align 4, !tbaa !1
  store i64 %162, ptr %158, align 4, !tbaa !1
  store i64 %164, ptr %159, align 4, !tbaa !1
  store i64 %157, ptr %163, align 4, !tbaa !1
  %165 = add i64 %1, 32
  %166 = add i64 %165, %155
  %167 = inttoptr i64 %166 to ptr
  %168 = load i32, ptr %167, align 4, !tbaa !4
  %169 = sext i32 %168 to i64
  %170 = icmp eq i32 %168, 0
  store i64 %169, ptr %160, align 4, !tbaa !1
  %..i.i12 = select i1 %170, i64 199190, i64 199056
  br label %common.ret.sink.split

pc_1475c:                                         ; preds = %tailrecurse
  %171 = musttail call i64 @tb_1475c(ptr nonnull %0, i64 %1)
  ret i64 %171

pc_15d08:                                         ; preds = %tailrecurse
  %172 = getelementptr i8, ptr %0, i64 64
  %173 = load i64, ptr %172, align 4, !tbaa !1
  %174 = getelementptr i8, ptr %0, i64 80
  %175 = getelementptr i8, ptr %0, i64 112
  store i64 89358, ptr %3, align 4, !tbaa !1
  store i64 %173, ptr %174, align 4, !tbaa !1
  %176 = add i64 %1, 32
  %177 = add i64 %176, %173
  %178 = inttoptr i64 %177 to ptr
  %179 = load i32, ptr %178, align 4, !tbaa !4
  %180 = sext i32 %179 to i64
  %181 = icmp eq i32 %179, 0
  store i64 %180, ptr %175, align 4, !tbaa !1
  %..i.i13 = select i1 %181, i64 199048, i64 199024
  br label %common.ret.sink.split

pc_3097c:                                         ; preds = %tailrecurse
  %182 = musttail call i64 @tb_3097c(ptr nonnull %0, i64 %1)
  ret i64 %182

pc_15d0e:                                         ; preds = %tailrecurse
  %183 = musttail call i64 @tb_15d0e(ptr nonnull %0, i64 %1)
  ret i64 %183

pc_12e62:                                         ; preds = %tailrecurse
  %184 = musttail call i64 @tb_12e62(ptr nonnull %0, i64 %1)
  ret i64 %184

pc_30a7c:                                         ; preds = %tailrecurse
  %185 = musttail call i64 @tb_30a7c(ptr nonnull %0, i64 %1)
  ret i64 %185

pc_30a48:                                         ; preds = %tailrecurse
  %186 = musttail call i64 @tb_30a48(ptr nonnull %0, i64 %1)
  ret i64 %186

pc_316dc:                                         ; preds = %tailrecurse
  %187 = musttail call i64 @tb_316dc(ptr nonnull %0, i64 %1)
  ret i64 %187

pc_3138a:                                         ; preds = %tailrecurse
  %188 = musttail call i64 @tb_3138a(ptr nonnull %0, i64 %1)
  ret i64 %188

pc_4ec72:                                         ; preds = %tailrecurse
  %189 = getelementptr i8, ptr %0, i64 64
  %190 = load i64, ptr %189, align 4, !tbaa !1
  %191 = getelementptr i8, ptr %0, i64 208
  %192 = load i64, ptr %191, align 4, !tbaa !1
  %193 = and i64 %190, 4294967295
  %194 = add nuw nsw i64 %193, 4
  %195 = add i64 %194, %192
  %196 = add i64 %195, %1
  %197 = inttoptr i64 %196 to ptr
  %198 = load i32, ptr %197, align 4, !tbaa !4
  %199 = sext i32 %198 to i64
  %.not.i14 = icmp eq i32 %198, 0
  store i64 %199, ptr %189, align 4, !tbaa !1
  store i64 %195, ptr %191, align 4, !tbaa !1
  %..i = select i1 %.not.i14, i64 322688, i64 322520
  br label %common.ret.sink.split

pc_10bf2:                                         ; preds = %tailrecurse
  %200 = getelementptr i8, ptr %0, i64 80
  %201 = load i64, ptr %200, align 4, !tbaa !1
  %202 = getelementptr i8, ptr %0, i64 120
  %203 = getelementptr i8, ptr %0, i64 136
  %204 = load i64, ptr %203, align 4, !tbaa !1
  %205 = add i64 %204, %1
  %206 = add i64 %205, 102
  %207 = inttoptr i64 %206 to ptr
  %208 = trunc i64 %201 to i16
  store i16 %208, ptr %207, align 2, !tbaa !4
  %209 = add i64 %205, 96
  %210 = inttoptr i64 %209 to ptr
  %211 = load i16, ptr %210, align 2, !tbaa !4
  %212 = zext i16 %211 to i64
  store i64 %212, ptr %202, align 4, !tbaa !1
  br label %common.ret.sink.split

pc_4ecc2:                                         ; preds = %tailrecurse
  %213 = getelementptr i8, ptr %0, i64 104
  %214 = getelementptr i8, ptr %0, i64 120
  store i64 65535, ptr %213, align 4, !tbaa !1
  store i64 65536, ptr %214, align 4, !tbaa !1
  br label %common.ret.sink.split

pc_4ecd8:                                         ; preds = %tailrecurse
  %215 = getelementptr i8, ptr %0, i64 104
  %216 = load i64, ptr %215, align 4, !tbaa !1
  %217 = getelementptr i8, ptr %0, i64 184
  %218 = load i64, ptr %217, align 4, !tbaa !1
  %.not.i15 = icmp ult i64 %218, %216
  %..i16 = select i1 %.not.i15, i64 322780, i64 322784
  br label %common.ret.sink.split

pc_30a08:                                         ; preds = %tailrecurse
  %219 = musttail call i64 @tb_30a08(ptr nonnull %0, i64 %1)
  ret i64 %219

pc_14562:                                         ; preds = %tailrecurse
  %220 = musttail call i64 @tb_14562(ptr nonnull %0, i64 %1)
  ret i64 %220

pc_14afc:                                         ; preds = %tailrecurse
  %221 = musttail call i64 @tb_14afc(ptr nonnull %0, i64 %1)
  ret i64 %221

pc_10f50:                                         ; preds = %tailrecurse
  %222 = musttail call i64 @tb_10f50(ptr nonnull %0, i64 %1)
  ret i64 %222

pc_113f8:                                         ; preds = %tailrecurse
  %223 = getelementptr i8, ptr %0, i64 40
  %224 = getelementptr i8, ptr %0, i64 56
  %225 = getelementptr i8, ptr %0, i64 80
  %226 = load i64, ptr %225, align 4, !tbaa !1
  %227 = icmp eq i64 %226, 0
  store i64 0, ptr %223, align 4, !tbaa !1
  store i64 0, ptr %224, align 4, !tbaa !1
  store i64 %226, ptr %5, align 4, !tbaa !1
  %..i17 = select i1 %227, i64 70778, i64 70656
  br label %common.ret.sink.split

pc_11b16:                                         ; preds = %tailrecurse
  %228 = getelementptr i8, ptr %0, i64 80
  %229 = getelementptr i8, ptr %0, i64 152
  store i64 72478, ptr %3, align 4, !tbaa !1
  %230 = load <2 x i64>, ptr %229, align 4, !tbaa !1
  %231 = shufflevector <2 x i64> %230, <2 x i64> poison, <2 x i32> <i32 1, i32 0>
  store <2 x i64> %231, ptr %228, align 4, !tbaa !1
  br label %common.ret.sink.split

pc_11bc4:                                         ; preds = %tailrecurse
  %232 = musttail call i64 @tb_11bc4(ptr nonnull %0, i64 %1)
  ret i64 %232

pc_316d4:                                         ; preds = %tailrecurse
  %233 = getelementptr i8, ptr %0, i64 80
  %234 = getelementptr i8, ptr %0, i64 144
  %235 = load i64, ptr %234, align 4, !tbaa !1
  %236 = getelementptr i8, ptr %0, i64 152
  %237 = load i64, ptr %236, align 4, !tbaa !1
  %238 = add i64 %235, 1
  store i64 202460, ptr %3, align 4, !tbaa !1
  store i64 %237, ptr %233, align 4, !tbaa !1
  store i64 %238, ptr %234, align 4, !tbaa !1
  br label %common.ret.sink.split

pc_4e36a:                                         ; preds = %tailrecurse
  %239 = getelementptr i8, ptr %0, i64 72
  %240 = load i64, ptr %239, align 4, !tbaa !1
  %241 = getelementptr i8, ptr %0, i64 112
  %242 = getelementptr i8, ptr %0, i64 120
  %243 = add i64 %1, 9
  %244 = add i64 %243, %240
  %245 = inttoptr i64 %244 to ptr
  %246 = load i8, ptr %245, align 1, !tbaa !4
  %247 = zext i8 %246 to i64
  %248 = icmp eq i8 %246, 122
  store i64 %247, ptr %241, align 4, !tbaa !1
  store i64 122, ptr %242, align 4, !tbaa !1
  %..i18 = select i1 %248, i64 320386, i64 320374
  br label %common.ret.sink.split

pc_313a0:                                         ; preds = %tailrecurse
  %249 = getelementptr i8, ptr %0, i64 144
  store i64 1, ptr %249, align 4, !tbaa !1
  br label %common.ret.sink.split

pc_20970:                                         ; preds = %tailrecurse
  %250 = getelementptr i8, ptr %0, i64 80
  %251 = load i64, ptr %250, align 4, !tbaa !1
  %252 = getelementptr i8, ptr %0, i64 88
  %253 = load i64, ptr %252, align 4, !tbaa !1
  %254 = getelementptr i8, ptr %0, i64 96
  %255 = load i64, ptr %254, align 4, !tbaa !1
  %256 = getelementptr i8, ptr %0, i64 112
  %257 = getelementptr i8, ptr %0, i64 120
  %258 = xor i64 %253, -1
  %259 = add i64 %251, %258
  %260 = add i64 %255, %253
  %261 = icmp eq i64 %255, 0
  store i64 %260, ptr %252, align 4, !tbaa !1
  store i64 %259, ptr %256, align 4, !tbaa !1
  store i64 %253, ptr %257, align 4, !tbaa !1
  %..i19 = select i1 %261, i64 133518, i64 133500
  br label %common.ret.sink.split

pc_3096c:                                         ; preds = %tailrecurse
  %262 = getelementptr i8, ptr %0, i64 80
  %263 = load i64, ptr %262, align 4, !tbaa !1
  %264 = getelementptr i8, ptr %0, i64 112
  %265 = add i64 %1, 32
  %266 = add i64 %265, %263
  %267 = inttoptr i64 %266 to ptr
  %268 = load i32, ptr %267, align 4, !tbaa !4
  %269 = sext i32 %268 to i64
  %270 = icmp eq i32 %268, 0
  store i64 %269, ptr %264, align 4, !tbaa !1
  %..i20 = select i1 %270, i64 199048, i64 199024
  br label %common.ret.sink.split

pc_30658:                                         ; preds = %tailrecurse
  %271 = getelementptr i8, ptr %0, i64 80
  %272 = load i64, ptr %271, align 4, !tbaa !1
  %273 = getelementptr i8, ptr %0, i64 120
  %274 = add i64 %1, 32
  %275 = add i64 %274, %272
  %276 = inttoptr i64 %275 to ptr
  %277 = load i32, ptr %276, align 4, !tbaa !4
  %278 = sext i32 %277 to i64
  %279 = icmp eq i32 %277, 0
  store i64 %278, ptr %273, align 4, !tbaa !1
  %..i21 = select i1 %279, i64 198284, i64 198236
  br label %common.ret.sink.split

pc_11fc0:                                         ; preds = %tailrecurse
  %280 = musttail call i64 @tb_11fc0(ptr nonnull %0, i64 %1)
  ret i64 %280

pc_14614:                                         ; preds = %tailrecurse
  %281 = musttail call i64 @tb_14614(ptr nonnull %0, i64 %1)
  ret i64 %281

pc_1133c:                                         ; preds = %tailrecurse
  %282 = getelementptr i8, ptr %0, i64 80
  %283 = load i64, ptr %282, align 4, !tbaa !1
  %284 = icmp eq i64 %283, 0
  %..i22 = select i1 %284, i64 70534, i64 70462
  br label %common.ret.sink.split

pc_2314c:                                         ; preds = %tailrecurse
  %285 = getelementptr i8, ptr %0, i64 104
  %286 = load i64, ptr %285, align 4, !tbaa !1
  %287 = getelementptr i8, ptr %0, i64 112
  %288 = getelementptr i8, ptr %0, i64 128
  %289 = load i64, ptr %288, align 4, !tbaa !1
  %290 = add i64 %1, 88
  %291 = add i64 %290, %286
  %292 = inttoptr i64 %291 to ptr
  %293 = load i64, ptr %292, align 4, !tbaa !4
  %294 = icmp eq i64 %289, 0
  store i64 %293, ptr %287, align 4, !tbaa !1
  %..i23 = select i1 %294, i64 143724, i64 143698
  br label %common.ret.sink.split

pc_10adc:                                         ; preds = %tailrecurse
  %295 = musttail call i64 @tb_10adc(ptr nonnull %0, i64 %1)
  ret i64 %295

pc_1450a:                                         ; preds = %tailrecurse
  %296 = musttail call i64 @tb_1450a(ptr nonnull %0, i64 %1)
  ret i64 %296

pc_11388:                                         ; preds = %tailrecurse
  %297 = getelementptr i8, ptr %0, i64 40
  %298 = getelementptr i8, ptr %0, i64 56
  %299 = getelementptr i8, ptr %0, i64 80
  %300 = load i64, ptr %299, align 4, !tbaa !1
  %301 = icmp eq i64 %300, 0
  store i64 0, ptr %297, align 4, !tbaa !1
  store i64 0, ptr %298, align 4, !tbaa !1
  store i64 %300, ptr %5, align 4, !tbaa !1
  %..i24 = select i1 %301, i64 70646, i64 70544
  br label %common.ret.sink.split

pc_1492a:                                         ; preds = %tailrecurse
  %302 = getelementptr i8, ptr %0, i64 160
  %303 = load i64, ptr %302, align 4, !tbaa !1
  %304 = icmp eq i64 %303, 0
  %..i25 = select i1 %304, i64 86538, i64 84270
  br label %common.ret.sink.split

pc_10bf6:                                         ; preds = %tailrecurse
  %305 = getelementptr i8, ptr %0, i64 120
  %306 = getelementptr i8, ptr %0, i64 136
  %307 = load i64, ptr %306, align 4, !tbaa !1
  %308 = add i64 %1, 96
  %309 = add i64 %308, %307
  %310 = inttoptr i64 %309 to ptr
  %311 = load i16, ptr %310, align 2, !tbaa !4
  %312 = zext i16 %311 to i64
  store i64 %312, ptr %305, align 4, !tbaa !1
  br label %common.ret.sink.split

pc_10dbe:                                         ; preds = %tailrecurse
  %313 = getelementptr i8, ptr %0, i64 64
  %314 = load i64, ptr %313, align 4, !tbaa !1
  %315 = getelementptr i8, ptr %0, i64 72
  %316 = load i64, ptr %315, align 4, !tbaa !1
  %317 = getelementptr i8, ptr %0, i64 112
  %318 = getelementptr i8, ptr %0, i64 152
  %319 = load i64, ptr %318, align 4, !tbaa !1
  %320 = add i64 %319, %1
  %321 = inttoptr i64 %320 to ptr
  %322 = load i64, ptr %321, align 4, !tbaa !4
  %323 = add i64 %314, -1
  %324 = icmp eq i64 %316, 0
  store i64 %323, ptr %313, align 4, !tbaa !1
  store i64 %322, ptr %317, align 4, !tbaa !1
  %..i26 = select i1 %324, i64 69072, i64 69062
  br label %common.ret.sink.split

pc_10b14:                                         ; preds = %tailrecurse
  %325 = getelementptr i8, ptr %0, i64 80
  %326 = load i64, ptr %325, align 4, !tbaa !1
  %327 = getelementptr i8, ptr %0, i64 112
  %328 = getelementptr i8, ptr %0, i64 120
  %329 = getelementptr i8, ptr %0, i64 128
  %330 = add i64 %326, %1
  %331 = inttoptr i64 %330 to ptr
  %332 = load i16, ptr %331, align 2, !tbaa !4
  %333 = sext i16 %332 to i64
  %334 = shl i64 %333, 56
  %335 = lshr i64 %334, 63
  %336 = icmp sgt i64 %334, -1
  store i64 %334, ptr %327, align 4, !tbaa !1
  store i64 %335, ptr %328, align 4, !tbaa !1
  store i64 %333, ptr %329, align 4, !tbaa !1
  %..i27 = select i1 %336, i64 68392, i64 68386
  br label %common.ret.sink.split

pc_1159e:                                         ; preds = %tailrecurse
  %337 = musttail call i64 @tb_1159e(ptr nonnull %0, i64 %1)
  ret i64 %337

pc_4ec32:                                         ; preds = %tailrecurse
  %338 = getelementptr i8, ptr %0, i64 88
  %339 = load i64, ptr %338, align 4, !tbaa !1
  %340 = getelementptr i8, ptr %0, i64 168
  %341 = load i64, ptr %340, align 4, !tbaa !1
  %.not.i28 = icmp ult i64 %341, %339
  %..i29 = select i1 %.not.i28, i64 322614, i64 322860
  br label %common.ret.sink.split

pc_4e432:                                         ; preds = %tailrecurse
  %342 = getelementptr i8, ptr %0, i64 80
  %343 = getelementptr i8, ptr %0, i64 96
  %344 = load i64, ptr %343, align 4, !tbaa !1
  %345 = getelementptr i8, ptr %0, i64 112
  %346 = getelementptr i8, ptr %0, i64 120
  %347 = add i64 %344, %1
  %348 = inttoptr i64 %347 to ptr
  %349 = load i8, ptr %348, align 1, !tbaa !4
  %350 = zext i8 %349 to i64
  %.not.i30 = icmp eq i8 %349, 8
  store i64 255, ptr %342, align 4, !tbaa !1
  store i64 %350, ptr %345, align 4, !tbaa !1
  store i64 8, ptr %346, align 4, !tbaa !1
  %..i31 = select i1 %.not.i30, i64 320576, i64 320376
  br label %common.ret.sink.split

pc_15bbe:                                         ; preds = %tailrecurse
  %351 = musttail call i64 @tb_15bbe(ptr nonnull %0, i64 %1)
  ret i64 %351

pc_30a64:                                         ; preds = %tailrecurse
  %352 = musttail call i64 @tb_30a64(ptr nonnull %0, i64 %1)
  ret i64 %352

pc_30a4c:                                         ; preds = %tailrecurse
  %353 = musttail call i64 @tb_30a4c(ptr nonnull %0, i64 %1)
  ret i64 %353

pc_316e6:                                         ; preds = %tailrecurse
  %354 = getelementptr i8, ptr %0, i64 64
  %355 = load i64, ptr %354, align 4, !tbaa !1
  %356 = getelementptr i8, ptr %0, i64 120
  %357 = add i64 %1, -232
  %358 = add i64 %357, %355
  %359 = inttoptr i64 %358 to ptr
  %360 = load i64, ptr %359, align 4, !tbaa !4
  %361 = add i64 %360, 1
  store i64 %361, ptr %356, align 4, !tbaa !1
  br label %common.ret.sink.split

pc_4ec68:                                         ; preds = %tailrecurse
  %362 = getelementptr i8, ptr %0, i64 96
  %363 = load i64, ptr %362, align 4, !tbaa !1
  %.not.i32 = icmp eq i64 %363, 0
  %..i33 = select i1 %.not.i32, i64 322666, i64 322784
  br label %common.ret.sink.split

pc_10dfa:                                         ; preds = %tailrecurse
  %364 = musttail call i64 @tb_10dfa(ptr nonnull %0, i64 %1)
  ret i64 %364

pc_1167c:                                         ; preds = %tailrecurse
  %365 = musttail call i64 @tb_1167c(ptr nonnull %0, i64 %1)
  ret i64 %365

pc_11de0:                                         ; preds = %tailrecurse
  %366 = musttail call i64 @tb_11de0(ptr nonnull %0, i64 %1)
  ret i64 %366

pc_1118e:                                         ; preds = %tailrecurse
  %367 = getelementptr i8, ptr %0, i64 64
  %368 = load i64, ptr %367, align 4, !tbaa !1
  %369 = getelementptr i8, ptr %0, i64 72
  %370 = load i64, ptr %369, align 4, !tbaa !1
  %371 = getelementptr i8, ptr %0, i64 80
  %372 = load i64, ptr %371, align 4, !tbaa !1
  %373 = getelementptr i8, ptr %0, i64 144
  %374 = load i64, ptr %373, align 4, !tbaa !1
  %375 = add i64 %1, 98
  %376 = add i64 %375, %368
  %377 = inttoptr i64 %376 to ptr
  %378 = trunc i64 %372 to i16
  store i16 %378, ptr %377, align 2, !tbaa !4
  %379 = add i64 %370, 1
  %.not.i34 = icmp eq i64 %374, %379
  store i64 %379, ptr %369, align 4, !tbaa !1
  %..i35 = select i1 %.not.i34, i64 70040, i64 69988
  br label %common.ret.sink.split

pc_11b8a:                                         ; preds = %tailrecurse
  %380 = getelementptr i8, ptr %0, i64 72
  %381 = load i64, ptr %380, align 4, !tbaa !1
  %382 = getelementptr i8, ptr %0, i64 104
  %383 = getelementptr i8, ptr %0, i64 192
  %384 = load i64, ptr %383, align 4, !tbaa !1
  %.not.i36 = icmp ult i64 %381, %384
  store i64 44, ptr %382, align 4, !tbaa !1
  %..i37 = select i1 %.not.i36, i64 72594, i64 72616
  br label %common.ret.sink.split

pc_10fb0:                                         ; preds = %tailrecurse
  %385 = musttail call i64 @tb_10fb0(ptr nonnull %0, i64 %1)
  ret i64 %385

pc_10f44:                                         ; preds = %tailrecurse
  %386 = musttail call i64 @tb_10f44(ptr nonnull %0, i64 %1)
  ret i64 %386

pc_11192:                                         ; preds = %tailrecurse
  %387 = getelementptr i8, ptr %0, i64 72
  %388 = load i64, ptr %387, align 4, !tbaa !1
  %389 = getelementptr i8, ptr %0, i64 144
  %390 = load i64, ptr %389, align 4, !tbaa !1
  %391 = add i64 %388, 1
  %.not.i38 = icmp eq i64 %390, %391
  store i64 %391, ptr %387, align 4, !tbaa !1
  %..i39 = select i1 %.not.i38, i64 70040, i64 69988
  br label %common.ret.sink.split

pc_116c2:                                         ; preds = %tailrecurse
  %392 = musttail call i64 @tb_116c2(ptr nonnull %0, i64 %1)
  ret i64 %392

pc_3139c:                                         ; preds = %tailrecurse
  %393 = getelementptr i8, ptr %0, i64 144
  %394 = load i64, ptr %393, align 4, !tbaa !1
  %395 = icmp eq i64 %394, 0
  br i1 %395, label %common.ret.sink.split, label %fall.i40

fall.i40:                                         ; preds = %pc_3139c
  store i64 1, ptr %393, align 4, !tbaa !1
  br label %common.ret.sink.split

pc_30b08:                                         ; preds = %tailrecurse
  %396 = musttail call i64 @tb_30b08(ptr nonnull %0, i64 %1)
  ret i64 %396

pc_4db12:                                         ; preds = %tailrecurse
  %397 = getelementptr i8, ptr %0, i64 120
  %398 = load i64, ptr %397, align 4, !tbaa !1
  %399 = icmp eq i64 %398, 0
  %..i42 = select i1 %399, i64 318254, i64 318228
  br label %common.ret.sink.split

pc_211fc:                                         ; preds = %tailrecurse
  %400 = getelementptr i8, ptr %0, i64 80
  %401 = load i64, ptr %400, align 4, !tbaa !1
  %402 = getelementptr i8, ptr %0, i64 120
  %403 = icmp eq i64 %401, -38
  store i64 -38, ptr %402, align 4, !tbaa !1
  %..i43 = select i1 %403, i64 135724, i64 135684
  br label %common.ret.sink.split

pc_10f66:                                         ; preds = %tailrecurse
  %404 = musttail call i64 @tb_10f66(ptr nonnull %0, i64 %1)
  ret i64 %404

pc_1476a:                                         ; preds = %tailrecurse
  %405 = musttail call i64 @tb_1476a(ptr nonnull %0, i64 %1)
  ret i64 %405

pc_30a9e:                                         ; preds = %tailrecurse
  %406 = getelementptr i8, ptr %0, i64 88
  %407 = getelementptr i8, ptr %0, i64 112
  %408 = getelementptr i8, ptr %0, i64 120
  %409 = load i64, ptr %408, align 4, !tbaa !1
  %410 = getelementptr i8, ptr %0, i64 128
  %411 = load i64, ptr %410, align 4, !tbaa !1
  %412 = add i64 %1, 16
  %413 = add i64 %412, %411
  %414 = inttoptr i64 %413 to ptr
  %415 = load i64, ptr %414, align 4, !tbaa !4
  %416 = shl i64 %409, 3
  %417 = add i64 %415, %416
  store i64 %415, ptr %406, align 4, !tbaa !1
  store i64 %417, ptr %407, align 4, !tbaa !1
  br label %common.ret.sink.split

pc_2093c:                                         ; preds = %tailrecurse
  %418 = musttail call i64 @tb_2093c(ptr nonnull %0, i64 %1)
  ret i64 %418

pc_11baa:                                         ; preds = %tailrecurse
  %419 = musttail call i64 @tb_11baa(ptr nonnull %0, i64 %1)
  ret i64 %419

pc_1160e:                                         ; preds = %tailrecurse
  %420 = musttail call i64 @tb_1160e(ptr nonnull %0, i64 %1)
  ret i64 %420

pc_3098c:                                         ; preds = %tailrecurse
  %421 = getelementptr i8, ptr %0, i64 80
  %422 = load i64, ptr %421, align 4, !tbaa !1
  %423 = getelementptr i8, ptr %0, i64 120
  %424 = add i64 %1, 32
  %425 = add i64 %424, %422
  %426 = inttoptr i64 %425 to ptr
  %427 = load i32, ptr %426, align 4, !tbaa !4
  %428 = sext i32 %427 to i64
  %429 = icmp eq i32 %427, 0
  store i64 %428, ptr %423, align 4, !tbaa !1
  %..i44 = select i1 %429, i64 199190, i64 199056
  br label %common.ret.sink.split

pc_4ecba:                                         ; preds = %tailrecurse
  %430 = getelementptr i8, ptr %0, i64 88
  %431 = getelementptr i8, ptr %0, i64 168
  %432 = load i64, ptr %431, align 4, !tbaa !1
  %433 = getelementptr i8, ptr %0, i64 192
  %434 = load i64, ptr %433, align 4, !tbaa !1
  %435 = and i64 %434, 7
  %.not.i45 = icmp eq i64 %435, %432
  store i64 %435, ptr %430, align 4, !tbaa !1
  br i1 %.not.i45, label %fall.i50, label %L0.i46

L0.i46:                                           ; preds = %pc_4ecba
  %.not.i.i47 = icmp ult i64 %432, %435
  %..i.i48 = select i1 %.not.i.i47, i64 322614, i64 322860
  br label %common.ret.sink.split

fall.i50:                                         ; preds = %pc_4ecba
  %436 = getelementptr i8, ptr %0, i64 120
  %437 = getelementptr i8, ptr %0, i64 104
  store i64 65535, ptr %437, align 4, !tbaa !1
  store i64 65536, ptr %436, align 4, !tbaa !1
  br label %common.ret.sink.split

pc_10ad0:                                         ; preds = %tailrecurse
  %438 = musttail call i64 @tb_10ad0(ptr nonnull %0, i64 %1)
  ret i64 %438

pc_10ff4:                                         ; preds = %tailrecurse
  %439 = musttail call i64 @tb_10ff4(ptr nonnull %0, i64 %1)
  ret i64 %439

pc_10d6e:                                         ; preds = %tailrecurse
  %440 = getelementptr i8, ptr %0, i64 72
  %441 = load i64, ptr %440, align 4, !tbaa !1
  %442 = getelementptr i8, ptr %0, i64 120
  %443 = getelementptr i8, ptr %0, i64 144
  %444 = load i64, ptr %443, align 4, !tbaa !1
  %445 = getelementptr i8, ptr %0, i64 160
  %446 = load i64, ptr %445, align 4, !tbaa !1
  %447 = add i64 %446, -1
  %448 = add i64 %444, %1
  %449 = inttoptr i64 %448 to ptr
  %450 = load i64, ptr %449, align 4, !tbaa !4
  %.not.i51 = icmp eq i64 %441, 0
  store i64 %444, ptr %440, align 4, !tbaa !1
  store i64 %441, ptr %442, align 4, !tbaa !1
  store i64 %450, ptr %443, align 4, !tbaa !1
  store i64 %447, ptr %445, align 4, !tbaa !1
  %..i52 = select i1 %.not.i51, i64 68986, i64 68940
  br label %common.ret.sink.split

pc_23170:                                         ; preds = %tailrecurse
  %451 = getelementptr i8, ptr %0, i64 112
  %452 = load i64, ptr %451, align 4, !tbaa !1
  %453 = getelementptr i8, ptr %0, i64 120
  %454 = load i64, ptr %453, align 4, !tbaa !1
  %455 = add i64 %452, 88
  %456 = add i64 %455, %454
  store i64 %456, ptr %451, align 4, !tbaa !1
  br label %common.ret.sink.split

pc_208f0:                                         ; preds = %tailrecurse
  %457 = musttail call i64 @tb_208f0(ptr nonnull %0, i64 %1)
  ret i64 %457

pc_116e2:                                         ; preds = %tailrecurse
  %458 = getelementptr i8, ptr %0, i64 72
  %459 = load i64, ptr %458, align 4, !tbaa !1
  %460 = getelementptr i8, ptr %0, i64 88
  %461 = load i64, ptr %460, align 4, !tbaa !1
  %462 = getelementptr i8, ptr %0, i64 96
  %463 = load i64, ptr %462, align 4, !tbaa !1
  %464 = add i64 %461, 1
  %465 = add i64 %463, %459
  %466 = icmp ult i64 %464, %459
  store i64 %464, ptr %460, align 4, !tbaa !1
  store i64 %465, ptr %462, align 4, !tbaa !1
  %..i53 = select i1 %466, i64 71360, i64 71402
  br label %common.ret.sink.split

pc_211fa:                                         ; preds = %tailrecurse
  %467 = load i64, ptr %3, align 4, !tbaa !1
  br label %tailrecurse.backedge

tailrecurse.backedge:                             ; preds = %pc_211fa, %pc_10240
  %.be.in = phi i64 [ %467, %pc_211fa ], [ %10, %pc_10240 ]
  %.be = and i64 %.be.in, -2
  store i64 %.be, ptr %2, align 4, !tbaa !1
  br label %tailrecurse

pc_20956:                                         ; preds = %tailrecurse
  %468 = musttail call i64 @tb_20956(ptr nonnull %0, i64 %1)
  ret i64 %468

pc_4ed42:                                         ; preds = %tailrecurse
  %469 = getelementptr i8, ptr %0, i64 160
  store i64 -1, ptr %469, align 4, !tbaa !1
  br label %common.ret.sink.split

pc_309cc:                                         ; preds = %tailrecurse
  %470 = getelementptr i8, ptr %0, i64 64
  %471 = getelementptr i8, ptr %0, i64 80
  %472 = load i64, ptr %471, align 4, !tbaa !1
  %473 = getelementptr i8, ptr %0, i64 144
  %474 = load i64, ptr %473, align 4, !tbaa !1
  %475 = add i64 %1, 16
  %476 = add i64 %475, %474
  %477 = inttoptr i64 %476 to ptr
  %478 = load i64, ptr %477, align 4, !tbaa !4
  %.not.i54 = icmp eq i64 %478, %472
  store i64 %478, ptr %470, align 4, !tbaa !1
  %..i55 = select i1 %.not.i54, i64 199124, i64 199080
  br label %common.ret.sink.split

pc_14764:                                         ; preds = %tailrecurse
  %479 = musttail call i64 @tb_14764(ptr nonnull %0, i64 %1)
  ret i64 %479

pc_4ebfa:                                         ; preds = %tailrecurse
  %480 = getelementptr i8, ptr %0, i64 80
  %481 = load i64, ptr %480, align 4, !tbaa !1
  %482 = getelementptr i8, ptr %0, i64 96
  %483 = getelementptr i8, ptr %0, i64 104
  %484 = getelementptr i8, ptr %0, i64 192
  %485 = and i64 %481, 112
  %486 = and i64 %481, 255
  %487 = icmp eq i64 %485, 32
  store i64 32, ptr %482, align 4, !tbaa !1
  store i64 %485, ptr %483, align 4, !tbaa !1
  store i64 %486, ptr %484, align 4, !tbaa !1
  %..i56 = select i1 %487, i64 322844, i64 322570
  br label %common.ret.sink.split

pc_10f9c:                                         ; preds = %tailrecurse
  %488 = musttail call i64 @tb_10f9c(ptr nonnull %0, i64 %1)
  ret i64 %488

pc_11b6a:                                         ; preds = %tailrecurse
  %489 = getelementptr i8, ptr %0, i64 80
  %490 = getelementptr i8, ptr %0, i64 152
  store i64 72562, ptr %3, align 4, !tbaa !1
  %491 = load <2 x i64>, ptr %490, align 4, !tbaa !1
  %492 = shufflevector <2 x i64> %491, <2 x i64> poison, <2 x i32> <i32 1, i32 0>
  store <2 x i64> %492, ptr %489, align 4, !tbaa !1
  br label %common.ret.sink.split

pc_11b36:                                         ; preds = %tailrecurse
  %493 = musttail call i64 @tb_11b36(ptr nonnull %0, i64 %1)
  ret i64 %493

pc_20968:                                         ; preds = %tailrecurse
  %494 = musttail call i64 @tb_20968(ptr nonnull %0, i64 %1)
  ret i64 %494

pc_14740:                                         ; preds = %tailrecurse
  %495 = getelementptr i8, ptr %0, i64 80
  %496 = getelementptr i8, ptr %0, i64 88
  %497 = getelementptr i8, ptr %0, i64 152
  %498 = getelementptr i8, ptr %0, i64 208
  %499 = load i64, ptr %498, align 4, !tbaa !1
  %500 = add i64 %499, 1
  store i64 83790, ptr %3, align 4, !tbaa !1
  store i64 %500, ptr %495, align 4, !tbaa !1
  store i64 37, ptr %496, align 4, !tbaa !1
  store i64 %500, ptr %497, align 4, !tbaa !1
  br label %common.ret.sink.split

pc_10b78:                                         ; preds = %tailrecurse
  %501 = musttail call i64 @tb_10b78(ptr nonnull %0, i64 %1)
  ret i64 %501
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(write, argmem: readwrite, inaccessiblemem: none)
define noundef i64 @tb_208e4(ptr captures(none) initializes((104, 128), (136, 144), (512, 520)) %0, i64 %1) local_unnamed_addr #2 {
entry:
  %2 = getelementptr i8, ptr %0, i64 80
  %3 = load i64, ptr %2, align 4, !tbaa !1
  %4 = getelementptr i8, ptr %0, i64 88
  %5 = load i64, ptr %4, align 4, !tbaa !1
  %6 = getelementptr i8, ptr %0, i64 96
  %7 = load i64, ptr %6, align 4, !tbaa !1
  %8 = getelementptr i8, ptr %0, i64 104
  %9 = getelementptr i8, ptr %0, i64 112
  %10 = getelementptr i8, ptr %0, i64 120
  %11 = getelementptr i8, ptr %0, i64 136
  %12 = icmp ult i64 %7, 16
  store i64 %5, ptr %8, align 4, !tbaa !1
  store i64 15, ptr %9, align 4, !tbaa !1
  store i64 %3, ptr %10, align 4, !tbaa !1
  store i64 %3, ptr %11, align 4, !tbaa !1
  br i1 %12, label %L0, label %fall

common.ret:                                       ; preds = %fall, %L0
  %storemerge = phi i64 [ %..i, %L0 ], [ %..i114, %fall ]
  %13 = getelementptr i8, ptr %0, i64 512
  store i64 %storemerge, ptr %13, align 4, !tbaa !1
  ret i64 4294967298

L0:                                               ; preds = %entry
  %14 = xor i64 %5, -1
  %15 = add i64 %3, %14
  %16 = add i64 %7, %5
  %17 = icmp eq i64 %7, 0
  store i64 %16, ptr %4, align 4, !tbaa !1
  store i64 %15, ptr %9, align 4, !tbaa !1
  store i64 %5, ptr %10, align 4, !tbaa !1
  %..i = select i1 %17, i64 133518, i64 133500
  br label %common.ret

fall:                                             ; preds = %entry
  %18 = getelementptr i8, ptr %0, i64 128
  %19 = getelementptr i8, ptr %0, i64 8
  %20 = load i64, ptr %19, align 4, !tbaa !1
  %21 = getelementptr i8, ptr %0, i64 16
  %22 = load i64, ptr %21, align 4, !tbaa !1
  %23 = getelementptr i8, ptr %0, i64 48
  %24 = add i64 %22, -48
  %25 = sub i64 0, %3
  %26 = and i64 %25, 7
  %27 = add i64 %1, -8
  %28 = add i64 %27, %22
  %29 = inttoptr i64 %28 to ptr
  store i64 %20, ptr %29, align 4, !tbaa !4
  %30 = sub nuw i64 %7, %26
  %31 = icmp eq i64 %26, 0
  store i64 %24, ptr %21, align 4, !tbaa !1
  store i64 %26, ptr %23, align 4, !tbaa !1
  store i64 %30, ptr %18, align 4, !tbaa !1
  %..i114 = select i1 %31, i64 133404, i64 133380
  br label %common.ret
}

; Function Attrs: nofree nosync nounwind memory(readwrite, inaccessiblemem: none)
define range(i64 2, 4294967299) i64 @tb_20996(ptr initializes((80, 144), (512, 520)) %0, i64 %1) local_unnamed_addr #1 {
entry:
  %2 = getelementptr i8, ptr %0, i64 16
  %3 = load i64, ptr %2, align 4, !tbaa !1
  %4 = getelementptr i8, ptr %0, i64 80
  %5 = getelementptr i8, ptr %0, i64 104
  %6 = getelementptr i8, ptr %0, i64 128
  %7 = getelementptr i8, ptr %0, i64 136
  %8 = getelementptr i8, ptr %0, i64 512
  %9 = add i64 %3, %1
  %10 = inttoptr i64 %9 to ptr
  %11 = load i64, ptr %10, align 4, !tbaa !4
  %12 = add i64 %9, 8
  %13 = inttoptr i64 %12 to ptr
  %14 = load i64, ptr %13, align 4, !tbaa !4
  %15 = add i64 %9, 16
  %16 = inttoptr i64 %15 to ptr
  %17 = load i64, ptr %16, align 4, !tbaa !4
  %18 = add i64 %9, 24
  %19 = inttoptr i64 %18 to ptr
  %20 = load i64, ptr %19, align 4, !tbaa !4
  store i64 %17, ptr %6, align 4, !tbaa !1
  store i64 %20, ptr %7, align 4, !tbaa !1
  %21 = getelementptr i8, ptr %0, i64 88
  %22 = getelementptr i8, ptr %0, i64 96
  %23 = getelementptr i8, ptr %0, i64 112
  %24 = getelementptr i8, ptr %0, i64 120
  %25 = and i64 %17, -8
  %26 = add i64 %25, %14
  %27 = add i64 %25, %11
  %28 = and i64 %17, 7
  %29 = xor i64 %26, -1
  %30 = add i64 %27, %29
  %31 = add i64 %26, %28
  %32 = icmp eq i64 %28, 0
  store i64 %31, ptr %21, align 4, !tbaa !1
  store i64 %26, ptr %5, align 4, !tbaa !1
  store i64 %30, ptr %23, align 4, !tbaa !1
  store i64 %26, ptr %24, align 4, !tbaa !1
  br i1 %32, label %L0.i, label %fall.i

L0.i:                                             ; preds = %entry
  %33 = getelementptr i8, ptr %0, i64 8
  %34 = add i64 %9, 40
  %35 = inttoptr i64 %34 to ptr
  %36 = load i64, ptr %35, align 4, !tbaa !4
  %37 = add i64 %3, 48
  %38 = and i64 %36, -2
  store i64 %36, ptr %33, align 4, !tbaa !1
  store i64 %37, ptr %2, align 4, !tbaa !1
  store i64 %20, ptr %4, align 4, !tbaa !1
  store i64 0, ptr %22, align 4, !tbaa !1
  store i64 %38, ptr %8, align 4, !tbaa !1
  %39 = musttail call range(i64 2, 4294967299) i64 @aot_dispatch(ptr nonnull %0, i64 %1)
  ret i64 %39

fall.i:                                           ; preds = %entry
  %40 = add i64 %26, %1
  %41 = inttoptr i64 %40 to ptr
  %42 = load i8, ptr %41, align 1, !tbaa !4
  %43 = add i64 %27, %1
  %44 = inttoptr i64 %43 to ptr
  store i8 %42, ptr %44, align 1, !tbaa !4
  %.not129.i = icmp eq i64 %28, 1
  br i1 %.not129.i, label %fall.i47, label %L0.preheader.i

L0.preheader.i:                                   ; preds = %fall.i
  %45 = add i64 %26, 1
  %invariant.op.i = add i64 %30, %1
  br label %L0.i46

L0.i46:                                           ; preds = %L0.i46, %L0.preheader.i
  %46 = phi i64 [ %50, %L0.i46 ], [ %45, %L0.preheader.i ]
  %47 = add i64 %46, %1
  %48 = inttoptr i64 %47 to ptr
  %49 = load i8, ptr %48, align 1, !tbaa !4
  %50 = add i64 %46, 1
  %.reass.i = add i64 %invariant.op.i, %50
  %51 = inttoptr i64 %.reass.i to ptr
  store i8 %49, ptr %51, align 1, !tbaa !4
  %.not.i = icmp eq i64 %31, %50
  br i1 %.not.i, label %fall.i47.loopexit, label %L0.i46

fall.i47.loopexit:                                ; preds = %L0.i46
  %52 = add i64 %31, %30
  br label %fall.i47

fall.i47:                                         ; preds = %fall.i47.loopexit, %fall.i
  %.lcssa128.i = phi i64 [ %27, %fall.i ], [ %52, %fall.i47.loopexit ]
  %.lcssa127.in.i = phi i8 [ %42, %fall.i ], [ %49, %fall.i47.loopexit ]
  %.lcssa127.i = zext i8 %.lcssa127.in.i to i64
  %53 = getelementptr i8, ptr %0, i64 8
  %54 = add i64 %9, 40
  %55 = inttoptr i64 %54 to ptr
  %56 = load i64, ptr %55, align 4, !tbaa !4
  %57 = add i64 %3, 48
  %58 = and i64 %56, -2
  store i64 %56, ptr %53, align 4, !tbaa !1
  store i64 %57, ptr %2, align 4, !tbaa !1
  store i64 %20, ptr %4, align 4, !tbaa !1
  store i64 %.lcssa127.i, ptr %22, align 4, !tbaa !1
  store i64 %.lcssa128.i, ptr %5, align 4, !tbaa !1
  store i64 %31, ptr %24, align 4, !tbaa !1
  store i64 %58, ptr %8, align 4, !tbaa !1
  %59 = musttail call range(i64 2, 4294967299) i64 @aot_dispatch(ptr nonnull %0, i64 %1)
  ret i64 %59
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(read, argmem: readwrite, inaccessiblemem: none)
define noundef i64 @tb_4e35a(ptr captures(none) initializes((96, 128), (512, 520)) %0, i64 %1) local_unnamed_addr #0 {
entry:
  %2 = getelementptr i8, ptr %0, i64 64
  %3 = load i64, ptr %2, align 4, !tbaa !1
  %4 = getelementptr i8, ptr %0, i64 72
  %5 = load i64, ptr %4, align 4, !tbaa !1
  %6 = getelementptr i8, ptr %0, i64 80
  %7 = load i64, ptr %6, align 4, !tbaa !1
  %8 = getelementptr i8, ptr %0, i64 96
  %9 = getelementptr i8, ptr %0, i64 104
  %10 = add i64 %5, %1
  %11 = add i64 %10, 8
  %12 = inttoptr i64 %11 to ptr
  %13 = load i8, ptr %12, align 1, !tbaa !4
  %14 = zext i8 %13 to i64
  %15 = add i64 %7, 1
  %16 = add i64 %15, %3
  %17 = icmp ugt i8 %13, 3
  store i64 %15, ptr %6, align 4, !tbaa !1
  store i64 %16, ptr %8, align 4, !tbaa !1
  store i64 %14, ptr %9, align 4, !tbaa !1
  br i1 %17, label %L0, label %fall

common.ret:                                       ; preds = %fall, %L0
  %.sink117.in = phi i8 [ %23, %L0 ], [ %26, %fall ]
  %.sink = phi i64 [ 8, %L0 ], [ 122, %fall ]
  %storemerge = phi i64 [ %..i, %L0 ], [ %..i116, %fall ]
  %18 = getelementptr i8, ptr %0, i64 120
  %.sink117 = zext i8 %.sink117.in to i64
  %19 = getelementptr i8, ptr %0, i64 512
  %20 = getelementptr i8, ptr %0, i64 112
  store i64 %.sink117, ptr %20, align 4, !tbaa !1
  store i64 %.sink, ptr %18, align 4, !tbaa !1
  store i64 %storemerge, ptr %19, align 4, !tbaa !1
  ret i64 4294967298

L0:                                               ; preds = %entry
  %21 = add i64 %16, %1
  %22 = inttoptr i64 %21 to ptr
  %23 = load i8, ptr %22, align 1, !tbaa !4
  %.not.i = icmp eq i8 %23, 8
  store i64 255, ptr %6, align 4, !tbaa !1
  %..i = select i1 %.not.i, i64 320576, i64 320376
  br label %common.ret

fall:                                             ; preds = %entry
  %24 = add i64 %10, 9
  %25 = inttoptr i64 %24 to ptr
  %26 = load i8, ptr %25, align 1, !tbaa !4
  %27 = icmp eq i8 %26, 122
  %..i116 = select i1 %27, i64 320386, i64 320374
  br label %common.ret
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(argmem: readwrite)
define noundef i64 @tb_4ebf0(ptr captures(none) initializes((112, 120), (176, 184), (512, 520)) %0, i64 %1) local_unnamed_addr #3 {
entry:
  %2 = getelementptr i8, ptr %0, i64 80
  %3 = load i64, ptr %2, align 4, !tbaa !1
  %4 = getelementptr i8, ptr %0, i64 112
  %5 = getelementptr i8, ptr %0, i64 176
  %6 = icmp eq i64 %3, 255
  store i64 255, ptr %4, align 4, !tbaa !1
  store i64 %3, ptr %5, align 4, !tbaa !1
  br i1 %6, label %L0, label %fall

common.ret:                                       ; preds = %fall, %L0
  %storemerge = phi i64 [ 322688, %L0 ], [ %..i, %fall ]
  %7 = getelementptr i8, ptr %0, i64 512
  store i64 %storemerge, ptr %7, align 4, !tbaa !1
  ret i64 4294967298

L0:                                               ; preds = %entry
  %8 = getelementptr i8, ptr %0, i64 160
  store i64 -1, ptr %8, align 4, !tbaa !1
  br label %common.ret

fall:                                             ; preds = %entry
  %9 = getelementptr i8, ptr %0, i64 192
  %10 = getelementptr i8, ptr %0, i64 96
  %11 = getelementptr i8, ptr %0, i64 104
  %12 = and i64 %3, 112
  %13 = and i64 %3, 255
  %14 = icmp eq i64 %12, 32
  store i64 32, ptr %10, align 4, !tbaa !1
  store i64 %12, ptr %11, align 4, !tbaa !1
  store i64 %13, ptr %9, align 4, !tbaa !1
  %..i = select i1 %14, i64 322844, i64 322570
  br label %common.ret
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(read, argmem: readwrite, inaccessiblemem: none)
define noundef i64 @tb_4db86(ptr captures(none) initializes((112, 144), (512, 520)) %0, i64 %1) local_unnamed_addr #0 {
entry:
  %2 = getelementptr i8, ptr %0, i64 96
  %3 = load i64, ptr %2, align 4, !tbaa !1
  %4 = getelementptr i8, ptr %0, i64 112
  %5 = getelementptr i8, ptr %0, i64 120
  %6 = getelementptr i8, ptr %0, i64 128
  %7 = getelementptr i8, ptr %0, i64 136
  %8 = getelementptr i8, ptr %0, i64 512
  %9 = add i64 %3, %1
  %10 = add i64 %9, 1
  %11 = inttoptr i64 %10 to ptr
  %12 = load i8, ptr %11, align 1, !tbaa !4
  %13 = zext i8 %12 to i64
  %14 = inttoptr i64 %9 to ptr
  %15 = load i8, ptr %14, align 1, !tbaa !4
  %16 = zext i8 %15 to i64
  %17 = add i64 %9, 2
  %18 = inttoptr i64 %17 to ptr
  %19 = load i8, ptr %18, align 1, !tbaa !4
  %20 = zext i8 %19 to i64
  %21 = add i64 %9, 3
  %22 = inttoptr i64 %21 to ptr
  %23 = load i8, ptr %22, align 1, !tbaa !4
  %24 = zext i8 %23 to i64
  %25 = shl nuw nsw i64 %13, 8
  %26 = or disjoint i64 %25, %16
  %27 = shl nuw nsw i64 %20, 16
  %28 = shl nuw nsw i64 %24, 24
  %29 = or disjoint i64 %27, %28
  %30 = or disjoint i64 %29, %26
  %31 = add i64 %3, 4
  store i64 %31, ptr %4, align 4, !tbaa !1
  store i64 %30, ptr %5, align 4, !tbaa !1
  store i64 %26, ptr %6, align 4, !tbaa !1
  store i64 %16, ptr %7, align 4, !tbaa !1
  %32 = icmp eq i64 %30, 0
  %..i = select i1 %32, i64 318254, i64 318228
  store i64 %..i, ptr %8, align 4, !tbaa !1
  ret i64 4294967298
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(argmem: readwrite)
define noundef i64 @tb_4ec2a(ptr captures(none) initializes((88, 96), (512, 520)) %0, i64 %1) local_unnamed_addr #3 {
entry:
  %2 = getelementptr i8, ptr %0, i64 88
  %3 = getelementptr i8, ptr %0, i64 168
  %4 = load i64, ptr %3, align 4, !tbaa !1
  %5 = getelementptr i8, ptr %0, i64 192
  %6 = load i64, ptr %5, align 4, !tbaa !1
  %7 = and i64 %6, 7
  %8 = icmp eq i64 %7, %4
  store i64 %7, ptr %2, align 4, !tbaa !1
  br i1 %8, label %L0, label %fall

common.ret:                                       ; preds = %fall, %L0
  %storemerge = phi i64 [ %..i, %fall ], [ 322626, %L0 ]
  %9 = getelementptr i8, ptr %0, i64 512
  store i64 %storemerge, ptr %9, align 4, !tbaa !1
  ret i64 4294967298

L0:                                               ; preds = %entry
  %10 = getelementptr i8, ptr %0, i64 120
  %11 = getelementptr i8, ptr %0, i64 104
  store i64 65535, ptr %11, align 4, !tbaa !1
  store i64 65536, ptr %10, align 4, !tbaa !1
  br label %common.ret

fall:                                             ; preds = %entry
  %.not.i = icmp ult i64 %4, %7
  %..i = select i1 %.not.i, i64 322614, i64 322860
  br label %common.ret
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(read, argmem: readwrite, inaccessiblemem: none)
define noundef i64 @tb_4ec5e(ptr captures(none) initializes((96, 120), (512, 520)) %0, i64 %1) local_unnamed_addr #0 {
entry:
  %2 = getelementptr i8, ptr %0, i64 16
  %3 = load i64, ptr %2, align 4, !tbaa !1
  %4 = getelementptr i8, ptr %0, i64 72
  %5 = load i64, ptr %4, align 4, !tbaa !1
  %6 = getelementptr i8, ptr %0, i64 96
  %7 = getelementptr i8, ptr %0, i64 104
  %8 = getelementptr i8, ptr %0, i64 112
  %9 = getelementptr i8, ptr %0, i64 184
  %10 = load i64, ptr %9, align 4, !tbaa !1
  %11 = add i64 %1, 24
  %12 = add i64 %11, %3
  %13 = inttoptr i64 %12 to ptr
  %14 = load i64, ptr %13, align 4, !tbaa !4
  %15 = add i64 %5, %1
  %16 = inttoptr i64 %15 to ptr
  %17 = load i64, ptr %16, align 4, !tbaa !4
  %18 = add i64 %15, 8
  %19 = inttoptr i64 %18 to ptr
  %20 = load i64, ptr %19, align 4, !tbaa !4
  %21 = add i64 %14, %10
  %.not = icmp eq i64 %17, 0
  store i64 %20, ptr %6, align 4, !tbaa !1
  store i64 %17, ptr %7, align 4, !tbaa !1
  store i64 %21, ptr %8, align 4, !tbaa !1
  %.not.i = icmp ult i64 %10, %17
  %..i = select i1 %.not.i, i64 322780, i64 322784
  %.not.i117 = icmp eq i64 %20, 0
  %..i118 = select i1 %.not.i117, i64 322666, i64 322784
  %storemerge = select i1 %.not, i64 %..i118, i64 %..i
  %22 = getelementptr i8, ptr %0, i64 512
  store i64 %storemerge, ptr %22, align 4, !tbaa !1
  ret i64 4294967298
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(read, argmem: readwrite, inaccessiblemem: none)
define noundef i64 @tb_4ecb2(ptr captures(none) initializes((104, 112), (512, 520)) %0, i64 %1) local_unnamed_addr #0 {
entry:
  %2 = getelementptr i8, ptr %0, i64 104
  %3 = getelementptr i8, ptr %0, i64 120
  %4 = getelementptr i8, ptr %0, i64 168
  %5 = load i64, ptr %4, align 4, !tbaa !1
  %6 = getelementptr i8, ptr %0, i64 192
  %7 = load i64, ptr %6, align 4, !tbaa !1
  %8 = getelementptr i8, ptr %0, i64 208
  %9 = load i64, ptr %8, align 4, !tbaa !1
  %10 = icmp eq i64 %7, 255
  store i64 255, ptr %2, align 4, !tbaa !1
  br i1 %10, label %L0, label %fall

common.ret:                                       ; preds = %fall.i, %L0.i, %L0
  %storemerge = phi i64 [ %..i, %L0 ], [ %..i.i, %L0.i ], [ 322626, %fall.i ]
  %11 = getelementptr i8, ptr %0, i64 512
  store i64 %storemerge, ptr %11, align 4, !tbaa !1
  ret i64 4294967298

L0:                                               ; preds = %entry
  %12 = getelementptr i8, ptr %0, i64 64
  %13 = load i64, ptr %12, align 4, !tbaa !1
  %14 = and i64 %13, 4294967295
  %15 = add i64 %9, 4
  %16 = add i64 %15, %14
  %17 = add i64 %16, %1
  %18 = inttoptr i64 %17 to ptr
  %19 = load i32, ptr %18, align 4, !tbaa !4
  %20 = sext i32 %19 to i64
  %.not.i = icmp eq i32 %19, 0
  store i64 %20, ptr %12, align 4, !tbaa !1
  store i64 %16, ptr %8, align 4, !tbaa !1
  %..i = select i1 %.not.i, i64 322688, i64 322520
  br label %common.ret

fall:                                             ; preds = %entry
  %21 = getelementptr i8, ptr %0, i64 88
  %22 = and i64 %7, 7
  %.not.i108 = icmp eq i64 %22, %5
  store i64 %22, ptr %21, align 4, !tbaa !1
  br i1 %.not.i108, label %fall.i, label %L0.i

L0.i:                                             ; preds = %fall
  %.not.i.i = icmp ult i64 %5, %22
  %..i.i = select i1 %.not.i.i, i64 322614, i64 322860
  br label %common.ret

fall.i:                                           ; preds = %fall
  store i64 65535, ptr %2, align 4, !tbaa !1
  store i64 65536, ptr %3, align 4, !tbaa !1
  br label %common.ret
}

; Function Attrs: nofree nosync nounwind memory(readwrite, inaccessiblemem: none)
define range(i64 2, 4294967299) i64 @tb_10ace(ptr initializes((120, 128), (512, 520)) %0, i64 %1) local_unnamed_addr #1 {
entry:
  %2 = getelementptr i8, ptr %0, i64 96
  %3 = load i64, ptr %2, align 4, !tbaa !1
  %4 = getelementptr i8, ptr %0, i64 512
  %5 = icmp eq i64 %3, 0
  %6 = getelementptr i8, ptr %0, i64 8
  %7 = load i64, ptr %6, align 4, !tbaa !1
  %8 = getelementptr i8, ptr %0, i64 80
  %9 = load i64, ptr %8, align 4, !tbaa !1
  %10 = getelementptr i8, ptr %0, i64 88
  %11 = load i64, ptr %10, align 4, !tbaa !1
  br i1 %5, label %L0, label %fall

L0:                                               ; preds = %entry
  %12 = getelementptr i8, ptr %0, i64 104
  %13 = getelementptr i8, ptr %0, i64 112
  %14 = getelementptr i8, ptr %0, i64 120
  %15 = add i64 %9, %1
  %16 = inttoptr i64 %15 to ptr
  %17 = load i16, ptr %16, align 2, !tbaa !4
  %18 = and i16 %17, -256
  %19 = lshr i16 %17, 8
  %20 = or disjoint i16 %19, %18
  store i16 %20, ptr %16, align 2, !tbaa !4
  %21 = add i64 %11, %1
  %22 = inttoptr i64 %21 to ptr
  %23 = load i16, ptr %22, align 2, !tbaa !4
  %24 = sext i16 %23 to i64
  %25 = add i64 %1, 2
  %26 = add i64 %9, %25
  %27 = inttoptr i64 %26 to ptr
  %28 = load i16, ptr %27, align 2, !tbaa !4
  %29 = sext i16 %28 to i64
  %30 = shl nsw i64 %24, 48
  %31 = and i64 %24, -256
  %32 = lshr i64 %30, 56
  %33 = or disjoint i64 %32, %31
  %34 = trunc nsw i64 %33 to i16
  store i16 %34, ptr %22, align 2, !tbaa !4
  %35 = add i64 %11, %25
  %36 = inttoptr i64 %35 to ptr
  %37 = load i16, ptr %36, align 2, !tbaa !4
  %38 = sext i16 %37 to i64
  %39 = sub nsw i64 %29, %38
  %40 = and i64 %7, -2
  store i64 %39, ptr %8, align 4, !tbaa !1
  store i64 %30, ptr %12, align 4, !tbaa !1
  store i64 %31, ptr %13, align 4, !tbaa !1
  store i64 %38, ptr %14, align 4, !tbaa !1
  store i64 %40, ptr %4, align 4, !tbaa !1
  %41 = musttail call range(i64 2, 4294967299) i64 @aot_dispatch(ptr nonnull %0, i64 %1)
  ret i64 %41

fall:                                             ; preds = %entry
  %42 = getelementptr i8, ptr %0, i64 120
  %43 = add i64 %1, 2
  %44 = add i64 %9, %43
  %45 = inttoptr i64 %44 to ptr
  %46 = load i16, ptr %45, align 2, !tbaa !4
  %47 = sext i16 %46 to i64
  %48 = add i64 %11, %43
  %49 = inttoptr i64 %48 to ptr
  %50 = load i16, ptr %49, align 2, !tbaa !4
  %51 = sext i16 %50 to i64
  %52 = sub nsw i64 %47, %51
  %53 = and i64 %7, -2
  store i64 %52, ptr %8, align 4, !tbaa !1
  store i64 %51, ptr %42, align 4, !tbaa !1
  store i64 %53, ptr %4, align 4, !tbaa !1
  %54 = musttail call range(i64 2, 4294967299) i64 @aot_dispatch(ptr nonnull %0, i64 %1)
  ret i64 %54
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(read, argmem: readwrite, inaccessiblemem: none)
define noundef i64 @tb_10d6a(ptr captures(none) initializes((512, 520)) %0, i64 %1) local_unnamed_addr #0 {
entry:
  %2 = getelementptr i8, ptr %0, i64 80
  %3 = load i64, ptr %2, align 4, !tbaa !1
  %4 = icmp slt i64 %3, 1
  br i1 %4, label %L0, label %fall

common.ret:                                       ; preds = %fall, %L0
  %storemerge = phi i64 [ %..i, %L0 ], [ %..i106, %fall ]
  %5 = getelementptr i8, ptr %0, i64 512
  store i64 %storemerge, ptr %5, align 4, !tbaa !1
  ret i64 4294967298

L0:                                               ; preds = %entry
  %6 = getelementptr i8, ptr %0, i64 64
  %7 = load i64, ptr %6, align 4, !tbaa !1
  %8 = getelementptr i8, ptr %0, i64 72
  %9 = load i64, ptr %8, align 4, !tbaa !1
  %10 = getelementptr i8, ptr %0, i64 112
  %11 = getelementptr i8, ptr %0, i64 152
  %12 = load i64, ptr %11, align 4, !tbaa !1
  %13 = add i64 %12, %1
  %14 = inttoptr i64 %13 to ptr
  %15 = load i64, ptr %14, align 4, !tbaa !4
  %16 = add i64 %7, -1
  %17 = icmp eq i64 %9, 0
  store i64 %16, ptr %6, align 4, !tbaa !1
  store i64 %15, ptr %10, align 4, !tbaa !1
  %..i = select i1 %17, i64 69072, i64 69062
  br label %common.ret

fall:                                             ; preds = %entry
  %18 = getelementptr i8, ptr %0, i64 72
  %19 = load i64, ptr %18, align 4, !tbaa !1
  %20 = getelementptr i8, ptr %0, i64 120
  %21 = getelementptr i8, ptr %0, i64 144
  %22 = load i64, ptr %21, align 4, !tbaa !1
  %23 = getelementptr i8, ptr %0, i64 160
  %24 = load i64, ptr %23, align 4, !tbaa !1
  %25 = add i64 %24, -1
  %26 = add i64 %22, %1
  %27 = inttoptr i64 %26 to ptr
  %28 = load i64, ptr %27, align 4, !tbaa !4
  %.not.i = icmp eq i64 %19, 0
  store i64 %22, ptr %18, align 4, !tbaa !1
  store i64 %19, ptr %20, align 4, !tbaa !1
  store i64 %28, ptr %21, align 4, !tbaa !1
  store i64 %25, ptr %23, align 4, !tbaa !1
  %..i106 = select i1 %.not.i, i64 68986, i64 68940
  br label %common.ret
}

; Function Attrs: nofree nosync nounwind memory(readwrite, inaccessiblemem: none)
define hidden range(i64 2, 4294967299) i64 @tb_211f6(ptr initializes((512, 520)) %0, i64 %1) local_unnamed_addr #1 {
entry:
  %2 = getelementptr i8, ptr %0, i64 80
  %3 = load i64, ptr %2, align 4, !tbaa !1
  %4 = getelementptr i8, ptr %0, i64 512
  %.not = icmp eq i64 %3, 0
  br i1 %.not, label %fall, label %L0

L0:                                               ; preds = %entry
  %5 = getelementptr i8, ptr %0, i64 120
  %6 = icmp eq i64 %3, -38
  store i64 -38, ptr %5, align 4, !tbaa !1
  %..i = select i1 %6, i64 135724, i64 135684
  store i64 %..i, ptr %4, align 4, !tbaa !1
  ret i64 4294967298

fall:                                             ; preds = %entry
  %7 = getelementptr i8, ptr %0, i64 8
  %8 = load i64, ptr %7, align 4, !tbaa !1
  %9 = and i64 %8, -2
  store i64 %9, ptr %4, align 4, !tbaa !1
  %10 = musttail call range(i64 2, 4294967299) i64 @aot_dispatch(ptr nonnull %0, i64 %1)
  ret i64 %10
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(readwrite, inaccessiblemem: none)
define noundef i64 @tb_10bfc(ptr captures(none) initializes((112, 136), (512, 520)) %0, i64 %1) local_unnamed_addr #4 {
entry:
  %2 = getelementptr i8, ptr %0, i64 8
  %3 = load i64, ptr %2, align 4, !tbaa !1
  %4 = getelementptr i8, ptr %0, i64 16
  %5 = load i64, ptr %4, align 4, !tbaa !1
  %6 = getelementptr i8, ptr %0, i64 64
  %7 = load i64, ptr %6, align 4, !tbaa !1
  %8 = getelementptr i8, ptr %0, i64 72
  %9 = load i64, ptr %8, align 4, !tbaa !1
  %10 = getelementptr i8, ptr %0, i64 80
  %11 = load i64, ptr %10, align 4, !tbaa !1
  %12 = getelementptr i8, ptr %0, i64 88
  %13 = load i64, ptr %12, align 4, !tbaa !1
  %14 = getelementptr i8, ptr %0, i64 96
  %15 = load i64, ptr %14, align 4, !tbaa !1
  %16 = getelementptr i8, ptr %0, i64 112
  %17 = getelementptr i8, ptr %0, i64 120
  %18 = getelementptr i8, ptr %0, i64 128
  %19 = getelementptr i8, ptr %0, i64 512
  %20 = add i64 %5, -48
  %21 = add i64 %5, %1
  %22 = add i64 %21, -24
  %23 = inttoptr i64 %22 to ptr
  store i64 %9, ptr %23, align 4, !tbaa !4
  %24 = add i64 %21, -8
  %25 = inttoptr i64 %24 to ptr
  store i64 %3, ptr %25, align 4, !tbaa !4
  %26 = add i64 %21, -16
  %27 = inttoptr i64 %26 to ptr
  store i64 %7, ptr %27, align 4, !tbaa !4
  %28 = add i64 %21, -40
  %29 = inttoptr i64 %28 to ptr
  store i64 %15, ptr %29, align 4, !tbaa !4
  store i64 68622, ptr %2, align 4, !tbaa !1
  store i64 %20, ptr %4, align 4, !tbaa !1
  store i64 %13, ptr %8, align 4, !tbaa !1
  store i64 %15, ptr %12, align 4, !tbaa !1
  %30 = add i64 %11, %1
  %31 = inttoptr i64 %30 to ptr
  %32 = load i16, ptr %31, align 2, !tbaa !4
  %33 = sext i16 %32 to i64
  %34 = shl i64 %33, 56
  %35 = lshr i64 %34, 63
  %36 = icmp sgt i64 %34, -1
  store i64 %34, ptr %16, align 4, !tbaa !1
  store i64 %35, ptr %17, align 4, !tbaa !1
  store i64 %33, ptr %18, align 4, !tbaa !1
  %..i = select i1 %36, i64 68392, i64 68386
  store i64 %..i, ptr %19, align 4, !tbaa !1
  ret i64 4294967298
}

; Function Attrs: nofree nosync nounwind memory(readwrite, inaccessiblemem: none)
define range(i64 2, 4294967299) i64 @tb_10b8a(ptr initializes((8, 16), (120, 144), (512, 520)) %0, i64 %1) local_unnamed_addr #1 {
entry:
  %2 = getelementptr i8, ptr %0, i64 8
  %3 = getelementptr i8, ptr %0, i64 16
  %4 = load i64, ptr %3, align 4, !tbaa !1
  %5 = getelementptr i8, ptr %0, i64 64
  %6 = load i64, ptr %5, align 4, !tbaa !1
  %7 = getelementptr i8, ptr %0, i64 72
  %8 = load i64, ptr %7, align 4, !tbaa !1
  %9 = getelementptr i8, ptr %0, i64 80
  %10 = load i64, ptr %9, align 4, !tbaa !1
  %11 = getelementptr i8, ptr %0, i64 120
  %12 = getelementptr i8, ptr %0, i64 128
  %13 = getelementptr i8, ptr %0, i64 136
  %14 = getelementptr i8, ptr %0, i64 512
  %15 = add i64 %4, %1
  %16 = inttoptr i64 %15 to ptr
  %17 = load i64, ptr %16, align 4, !tbaa !4
  %18 = add i64 %15, 8
  %19 = inttoptr i64 %18 to ptr
  %20 = load i64, ptr %19, align 4, !tbaa !4
  %21 = and i64 %17, -256
  %22 = and i64 %8, 127
  %23 = add i64 %1, 96
  %24 = add i64 %23, %20
  %25 = inttoptr i64 %24 to ptr
  %26 = trunc i64 %10 to i16
  store i16 %26, ptr %25, align 2, !tbaa !4
  %27 = or disjoint i64 %22, %21
  %28 = or disjoint i64 %27, 128
  %29 = add i64 %15, 40
  %30 = inttoptr i64 %29 to ptr
  %31 = load i64, ptr %30, align 4, !tbaa !4
  %32 = add i64 %6, %1
  %33 = inttoptr i64 %32 to ptr
  %34 = trunc i64 %28 to i16
  store i16 %34, ptr %33, align 2, !tbaa !4
  %35 = add i64 %15, 32
  %36 = inttoptr i64 %35 to ptr
  %37 = load i64, ptr %36, align 4, !tbaa !4
  %38 = add i64 %15, 24
  %39 = inttoptr i64 %38 to ptr
  %40 = load i64, ptr %39, align 4, !tbaa !4
  %41 = add i64 %4, 48
  %42 = and i64 %31, -2
  store i64 %31, ptr %2, align 4, !tbaa !1
  store i64 %41, ptr %3, align 4, !tbaa !1
  store i64 %37, ptr %5, align 4, !tbaa !1
  store i64 %40, ptr %7, align 4, !tbaa !1
  store i64 %22, ptr %9, align 4, !tbaa !1
  store i64 %10, ptr %11, align 4, !tbaa !1
  store i64 %28, ptr %12, align 4, !tbaa !1
  store i64 %20, ptr %13, align 4, !tbaa !1
  store i64 %42, ptr %14, align 4, !tbaa !1
  %43 = musttail call i64 @aot_dispatch(ptr nonnull %0, i64 %1)
  ret i64 %43
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(read, argmem: readwrite, inaccessiblemem: none)
define noundef i64 @tb_10c0e(ptr captures(none) initializes((8, 16), (64, 72), (88, 96), (112, 136), (512, 520)) %0, i64 %1) local_unnamed_addr #0 {
entry:
  %2 = getelementptr i8, ptr %0, i64 8
  %3 = getelementptr i8, ptr %0, i64 16
  %4 = load i64, ptr %3, align 4, !tbaa !1
  %5 = getelementptr i8, ptr %0, i64 64
  %6 = getelementptr i8, ptr %0, i64 72
  %7 = load i64, ptr %6, align 4, !tbaa !1
  %8 = getelementptr i8, ptr %0, i64 80
  %9 = load i64, ptr %8, align 4, !tbaa !1
  %10 = getelementptr i8, ptr %0, i64 88
  %11 = getelementptr i8, ptr %0, i64 112
  %12 = getelementptr i8, ptr %0, i64 120
  %13 = getelementptr i8, ptr %0, i64 128
  %14 = getelementptr i8, ptr %0, i64 512
  %15 = add i64 %1, 8
  %16 = add i64 %15, %4
  %17 = inttoptr i64 %16 to ptr
  %18 = load i64, ptr %17, align 4, !tbaa !4
  store i64 68632, ptr %2, align 4, !tbaa !1
  store i64 %9, ptr %5, align 4, !tbaa !1
  store i64 %7, ptr %8, align 4, !tbaa !1
  store i64 %18, ptr %10, align 4, !tbaa !1
  %19 = add i64 %7, %1
  %20 = inttoptr i64 %19 to ptr
  %21 = load i16, ptr %20, align 2, !tbaa !4
  %22 = sext i16 %21 to i64
  %23 = shl i64 %22, 56
  %24 = lshr i64 %23, 63
  %25 = icmp sgt i64 %23, -1
  store i64 %23, ptr %11, align 4, !tbaa !1
  store i64 %24, ptr %12, align 4, !tbaa !1
  store i64 %22, ptr %13, align 4, !tbaa !1
  %..i = select i1 %25, i64 68392, i64 68386
  store i64 %..i, ptr %14, align 4, !tbaa !1
  ret i64 4294967298
}

; Function Attrs: nofree nosync nounwind memory(readwrite, inaccessiblemem: none)
define range(i64 2, 4294967299) i64 @tb_10c18(ptr initializes((8, 16), (72, 80), (512, 520)) %0, i64 %1) local_unnamed_addr #1 {
entry:
  %2 = getelementptr i8, ptr %0, i64 8
  %3 = getelementptr i8, ptr %0, i64 16
  %4 = load i64, ptr %3, align 4, !tbaa !1
  %5 = getelementptr i8, ptr %0, i64 64
  %6 = load i64, ptr %5, align 4, !tbaa !1
  %7 = getelementptr i8, ptr %0, i64 72
  %8 = getelementptr i8, ptr %0, i64 80
  %9 = load i64, ptr %8, align 4, !tbaa !1
  %10 = getelementptr i8, ptr %0, i64 512
  %11 = add i64 %4, %1
  %12 = add i64 %11, 40
  %13 = inttoptr i64 %12 to ptr
  %14 = load i64, ptr %13, align 4, !tbaa !4
  %15 = sub i64 %6, %9
  %16 = add i64 %11, 32
  %17 = inttoptr i64 %16 to ptr
  %18 = load i64, ptr %17, align 4, !tbaa !4
  %19 = add i64 %11, 24
  %20 = inttoptr i64 %19 to ptr
  %21 = load i64, ptr %20, align 4, !tbaa !4
  %22 = add i64 %4, 48
  %23 = and i64 %14, -2
  store i64 %14, ptr %2, align 4, !tbaa !1
  store i64 %22, ptr %3, align 4, !tbaa !1
  store i64 %18, ptr %5, align 4, !tbaa !1
  store i64 %21, ptr %7, align 4, !tbaa !1
  store i64 %15, ptr %8, align 4, !tbaa !1
  store i64 %23, ptr %10, align 4, !tbaa !1
  %24 = musttail call i64 @aot_dispatch(ptr %0, i64 %1)
  ret i64 %24
}

; Function Attrs: nofree nosync nounwind memory(readwrite, inaccessiblemem: none)
define range(i64 2, 4294967299) i64 @tb_12392(ptr initializes((48, 56), (96, 144), (224, 248), (512, 520)) %0, i64 %1) local_unnamed_addr #1 {
entry:
  %2 = getelementptr i8, ptr %0, i64 8
  %3 = load i64, ptr %2, align 4, !tbaa !1
  %4 = getelementptr i8, ptr %0, i64 48
  %5 = getelementptr i8, ptr %0, i64 80
  %6 = load i64, ptr %5, align 4, !tbaa !1
  %7 = getelementptr i8, ptr %0, i64 88
  %8 = load i64, ptr %7, align 4, !tbaa !1
  %9 = getelementptr i8, ptr %0, i64 96
  %10 = getelementptr i8, ptr %0, i64 104
  %11 = getelementptr i8, ptr %0, i64 112
  %12 = getelementptr i8, ptr %0, i64 120
  %13 = getelementptr i8, ptr %0, i64 128
  %14 = getelementptr i8, ptr %0, i64 136
  %15 = getelementptr i8, ptr %0, i64 224
  %16 = getelementptr i8, ptr %0, i64 232
  %17 = getelementptr i8, ptr %0, i64 512
  %18 = shl i64 %8, 8
  %19 = and i64 %18, 65280
  %20 = lshr i64 %8, 8
  %21 = or i64 %19, %20
  %22 = lshr i64 %21, 4
  %23 = and i64 %22, 268435215
  %24 = shl nuw nsw i64 %21, 4
  %25 = and i64 %24, 61680
  %26 = or i64 %25, %23
  %27 = lshr i64 %26, 2
  %28 = and i64 %27, 67105587
  %29 = shl nuw nsw i64 %26, 2
  %30 = and i64 %29, 52428
  %31 = or i64 %30, %28
  %32 = shl nuw nsw i64 %31, 1
  %33 = and i64 %32, 43690
  %34 = lshr i64 %31, 1
  %35 = and i64 %34, 33543509
  %36 = or i64 %33, %35
  %37 = lshr i64 %36, 8
  %trunc = trunc i64 %6 to i8
  %rev = tail call i8 @llvm.bitreverse.i8(i8 %trunc)
  %38 = zext i8 %rev to i64
  %39 = xor i64 %37, %38
  %40 = shl nuw nsw i64 %39, 1
  %41 = add i64 %1, 361072
  %42 = add i64 %41, %40
  %43 = inttoptr i64 %42 to ptr
  %44 = load i16, ptr %43, align 2, !tbaa !4
  %45 = zext i16 %44 to i64
  %46 = shl i64 %36, 56
  %47 = shl nuw i64 %45, 48
  %48 = xor i64 %46, %47
  %49 = lshr i64 %48, 56
  %50 = shl nuw nsw i64 %45, 8
  %51 = and i64 %50, 65280
  %52 = or disjoint i64 %49, %51
  %53 = lshr i64 %52, 4
  %54 = and i64 %53, 3855
  %55 = shl nuw nsw i64 %52, 4
  %56 = and i64 %55, 61680
  %57 = or disjoint i64 %56, %54
  %58 = lshr i64 %57, 2
  %59 = and i64 %58, 13107
  %60 = shl nuw nsw i64 %57, 2
  %61 = and i64 %60, 52428
  %62 = or disjoint i64 %61, %59
  %63 = lshr i64 %62, 1
  %64 = and i64 %63, 21845
  %65 = shl nuw nsw i64 %62, 1
  %66 = and i64 %65, 43690
  %67 = or disjoint i64 %66, %64
  %68 = shl nuw nsw i64 %67, 8
  %69 = and i64 %68, 65280
  %70 = lshr i64 %67, 8
  %71 = or disjoint i64 %69, %70
  %72 = shl nuw nsw i64 %71, 4
  %73 = lshr i64 %6, 12
  %74 = and i64 %3, -2
  store i64 -3856, ptr %4, align 4, !tbaa !1
  store i64 21845, ptr %7, align 4, !tbaa !1
  store i64 -21846, ptr %13, align 4, !tbaa !1
  store i64 -13108, ptr %14, align 4, !tbaa !1
  store i64 361072, ptr %15, align 4, !tbaa !1
  %75 = insertelement <2 x i64> poison, i64 %72, i64 0
  %76 = insertelement <2 x i64> %75, i64 %73, i64 1
  %77 = and <2 x i64> %76, <i64 61680, i64 15>
  %78 = insertelement <2 x i64> poison, i64 %71, i64 0
  %79 = insertelement <2 x i64> %78, i64 %6, i64 1
  %80 = lshr <2 x i64> %79, splat (i64 4)
  %81 = and <2 x i64> %80, <i64 3855, i64 240>
  %82 = or disjoint <2 x i64> %81, %77
  %83 = lshr <2 x i64> %82, splat (i64 2)
  %84 = and <2 x i64> %83, <i64 13107, i64 51>
  %85 = shl nuw nsw <2 x i64> %82, splat (i64 2)
  %86 = and <2 x i64> %85, <i64 52428, i64 204>
  %87 = or disjoint <2 x i64> %86, %84
  %88 = lshr <2 x i64> %87, splat (i64 1)
  %89 = and <2 x i64> %88, <i64 21845, i64 85>
  %90 = shl nuw nsw <2 x i64> %87, splat (i64 1)
  %91 = and <2 x i64> %90, <i64 43690, i64 170>
  %92 = or disjoint <2 x i64> %91, %89
  %93 = extractelement <2 x i64> %92, i64 0
  %94 = lshr i64 %93, 8
  %95 = extractelement <2 x i64> %92, i64 1
  %96 = xor i64 %94, %95
  %97 = shl nuw nsw i64 %96, 1
  %98 = add i64 %41, %97
  %99 = inttoptr i64 %98 to ptr
  %100 = load i16, ptr %99, align 2, !tbaa !4
  %101 = zext i16 %100 to i64
  %102 = shl i64 %93, 56
  %103 = shl nuw i64 %101, 48
  %104 = xor i64 %102, %103
  %105 = lshr i64 %104, 56
  %106 = shl nuw nsw i64 %101, 8
  %107 = and i64 %106, 65280
  %108 = or disjoint i64 %105, %107
  %109 = lshr i64 %108, 4
  %110 = and i64 %109, 3855
  %111 = shl nuw nsw i64 %108, 4
  %112 = and i64 %111, 61680
  %113 = or disjoint i64 %112, %110
  %114 = lshr i64 %113, 2
  %115 = and i64 %114, 13107
  %116 = shl nuw nsw i64 %113, 2
  %117 = and i64 %116, 52428
  %118 = or disjoint i64 %117, %115
  %119 = lshr i64 %118, 1
  %120 = and i64 %119, 21845
  %121 = shl nuw nsw i64 %118, 1
  %122 = and i64 %121, 43690
  %123 = or disjoint i64 %122, %120
  store i64 %123, ptr %5, align 4, !tbaa !1
  store i64 %105, ptr %9, align 4, !tbaa !1
  store i64 %110, ptr %10, align 4, !tbaa !1
  store i64 %115, ptr %11, align 4, !tbaa !1
  store i64 %120, ptr %12, align 4, !tbaa !1
  store <2 x i64> %92, ptr %16, align 4, !tbaa !1
  store i64 %74, ptr %17, align 4, !tbaa !1
  %124 = musttail call i64 @aot_dispatch(ptr %0, i64 %1)
  ret i64 %124
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(argmem: readwrite)
define noundef i64 @tb_1156e(ptr captures(none) initializes((8, 16), (88, 112), (184, 192), (512, 520)) %0, i64 %1) local_unnamed_addr #3 {
entry:
  %2 = getelementptr i8, ptr %0, i64 8
  %3 = getelementptr i8, ptr %0, i64 64
  %4 = load i64, ptr %3, align 4, !tbaa !1
  %5 = getelementptr i8, ptr %0, i64 72
  %6 = load i64, ptr %5, align 4, !tbaa !1
  %7 = getelementptr i8, ptr %0, i64 80
  %8 = load i64, ptr %7, align 4, !tbaa !1
  %9 = getelementptr i8, ptr %0, i64 88
  %10 = getelementptr i8, ptr %0, i64 96
  %11 = getelementptr i8, ptr %0, i64 144
  %12 = getelementptr i8, ptr %0, i64 184
  %13 = getelementptr i8, ptr %0, i64 512
  store i64 71036, ptr %2, align 4, !tbaa !1
  store i64 %6, ptr %7, align 4, !tbaa !1
  store i64 %4, ptr %9, align 4, !tbaa !1
  %14 = load <2 x i64>, ptr %11, align 4, !tbaa !1
  %15 = shufflevector <2 x i64> %14, <2 x i64> poison, <2 x i32> <i32 1, i32 0>
  store <2 x i64> %15, ptr %10, align 4, !tbaa !1
  store i64 %8, ptr %12, align 4, !tbaa !1
  %16 = icmp eq i64 %6, 0
  %..i = select i1 %16, i64 70534, i64 70462
  store i64 %..i, ptr %13, align 4, !tbaa !1
  ret i64 4294967298
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(read, argmem: readwrite, inaccessiblemem: none)
define noundef i64 @tb_1157c(ptr captures(none) initializes((48, 56), (80, 144), (512, 520)) %0, i64 %1) local_unnamed_addr #0 {
entry:
  %2 = getelementptr i8, ptr %0, i64 48
  %3 = getelementptr i8, ptr %0, i64 64
  %4 = load i64, ptr %3, align 4, !tbaa !1
  %5 = getelementptr i8, ptr %0, i64 72
  %6 = load i64, ptr %5, align 4, !tbaa !1
  %7 = getelementptr i8, ptr %0, i64 80
  %8 = getelementptr i8, ptr %0, i64 88
  %9 = getelementptr i8, ptr %0, i64 96
  %10 = getelementptr i8, ptr %0, i64 104
  %11 = getelementptr i8, ptr %0, i64 112
  %12 = getelementptr i8, ptr %0, i64 128
  %13 = getelementptr i8, ptr %0, i64 136
  %14 = getelementptr i8, ptr %0, i64 168
  %15 = load i64, ptr %14, align 4, !tbaa !1
  %16 = getelementptr i8, ptr %0, i64 512
  store i64 0, ptr %2, align 4, !tbaa !1
  store i64 0, ptr %7, align 4, !tbaa !1
  tail call void @llvm.memset.p0.i64(ptr noundef nonnull align 4 dereferenceable(16) %11, i8 0, i64 16, i1 false)
  store i64 %6, ptr %12, align 4, !tbaa !1
  store i64 0, ptr %13, align 4, !tbaa !1
  %17 = add i64 %4, %1
  %18 = inttoptr i64 %17 to ptr
  %19 = load i32, ptr %18, align 4, !tbaa !4
  %20 = sext i32 %19 to i64
  %21 = icmp sgt i32 %19, 0
  %22 = zext i1 %21 to i64
  %.not.i = icmp slt i64 %15, %20
  store i64 %22, ptr %8, align 4, !tbaa !1
  store i64 %20, ptr %9, align 4, !tbaa !1
  store i64 10, ptr %10, align 4, !tbaa !1
  store i64 %20, ptr %11, align 4, !tbaa !1
  %..i = select i1 %.not.i, i64 71098, i64 71052
  store i64 %..i, ptr %16, align 4, !tbaa !1
  ret i64 4294967298
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(argmem: readwrite)
define noundef i64 @tb_115e0(ptr captures(none) initializes((8, 16), (40, 48), (56, 64), (88, 112), (176, 184), (224, 232), (512, 520)) %0, i64 %1) local_unnamed_addr #3 {
entry:
  %2 = getelementptr i8, ptr %0, i64 8
  %3 = getelementptr i8, ptr %0, i64 40
  %4 = getelementptr i8, ptr %0, i64 56
  %5 = getelementptr i8, ptr %0, i64 64
  %6 = load i64, ptr %5, align 4, !tbaa !1
  %7 = getelementptr i8, ptr %0, i64 72
  %8 = load i64, ptr %7, align 4, !tbaa !1
  %9 = getelementptr i8, ptr %0, i64 80
  %10 = load i64, ptr %9, align 4, !tbaa !1
  %11 = getelementptr i8, ptr %0, i64 88
  %12 = getelementptr i8, ptr %0, i64 96
  %13 = getelementptr i8, ptr %0, i64 144
  %14 = getelementptr i8, ptr %0, i64 176
  %15 = getelementptr i8, ptr %0, i64 224
  %16 = getelementptr i8, ptr %0, i64 512
  store i64 71150, ptr %2, align 4, !tbaa !1
  store i64 %8, ptr %9, align 4, !tbaa !1
  store i64 %6, ptr %11, align 4, !tbaa !1
  %17 = load <2 x i64>, ptr %13, align 4, !tbaa !1
  %18 = shufflevector <2 x i64> %17, <2 x i64> poison, <2 x i32> <i32 1, i32 0>
  store <2 x i64> %18, ptr %12, align 4, !tbaa !1
  store i64 %10, ptr %14, align 4, !tbaa !1
  %19 = icmp eq i64 %8, 0
  store i64 0, ptr %3, align 4, !tbaa !1
  store i64 0, ptr %4, align 4, !tbaa !1
  store i64 %8, ptr %15, align 4, !tbaa !1
  %..i = select i1 %19, i64 70646, i64 70544
  store i64 %..i, ptr %16, align 4, !tbaa !1
  ret i64 4294967298
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(read, argmem: readwrite, inaccessiblemem: none)
define noundef i64 @tb_115ee(ptr captures(none) initializes((80, 144), (512, 520)) %0, i64 %1) local_unnamed_addr #0 {
entry:
  %2 = getelementptr i8, ptr %0, i64 80
  %3 = getelementptr i8, ptr %0, i64 88
  %4 = getelementptr i8, ptr %0, i64 96
  %5 = getelementptr i8, ptr %0, i64 104
  %6 = getelementptr i8, ptr %0, i64 120
  %7 = getelementptr i8, ptr %0, i64 128
  %8 = getelementptr i8, ptr %0, i64 168
  %9 = load i64, ptr %8, align 4, !tbaa !1
  %10 = getelementptr i8, ptr %0, i64 512
  store i64 0, ptr %2, align 4, !tbaa !1
  tail call void @llvm.memset.p0.i64(ptr noundef nonnull align 4 dereferenceable(24) %4, i8 0, i64 24, i1 false)
  tail call void @llvm.memset.p0.i64(ptr noundef nonnull align 4 dereferenceable(16) %7, i8 0, i64 16, i1 false)
  %11 = getelementptr i8, ptr %0, i64 64
  %12 = load i64, ptr %11, align 4, !tbaa !1
  %13 = add i64 %12, %1
  %14 = inttoptr i64 %13 to ptr
  %15 = load i32, ptr %14, align 4, !tbaa !4
  %16 = sext i32 %15 to i64
  %17 = icmp sgt i32 %15, 0
  %18 = zext i1 %17 to i64
  %.not.i = icmp slt i64 %9, %16
  store i64 %18, ptr %3, align 4, !tbaa !1
  store i64 %16, ptr %4, align 4, !tbaa !1
  store i64 %16, ptr %5, align 4, !tbaa !1
  store i64 10, ptr %6, align 4, !tbaa !1
  %..i = select i1 %.not.i, i64 71214, i64 71164
  store i64 %..i, ptr %10, align 4, !tbaa !1
  ret i64 4294967298
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(argmem: readwrite)
define noundef i64 @tb_1164e(ptr captures(none) initializes((8, 16), (40, 48), (56, 64), (88, 112), (176, 184), (224, 232), (512, 520)) %0, i64 %1) local_unnamed_addr #3 {
entry:
  %2 = getelementptr i8, ptr %0, i64 8
  %3 = getelementptr i8, ptr %0, i64 40
  %4 = getelementptr i8, ptr %0, i64 56
  %5 = getelementptr i8, ptr %0, i64 64
  %6 = load i64, ptr %5, align 4, !tbaa !1
  %7 = getelementptr i8, ptr %0, i64 72
  %8 = load i64, ptr %7, align 4, !tbaa !1
  %9 = getelementptr i8, ptr %0, i64 80
  %10 = load i64, ptr %9, align 4, !tbaa !1
  %11 = getelementptr i8, ptr %0, i64 88
  %12 = getelementptr i8, ptr %0, i64 96
  %13 = getelementptr i8, ptr %0, i64 144
  %14 = getelementptr i8, ptr %0, i64 176
  %15 = getelementptr i8, ptr %0, i64 224
  %16 = getelementptr i8, ptr %0, i64 512
  store i64 71260, ptr %2, align 4, !tbaa !1
  store i64 %8, ptr %9, align 4, !tbaa !1
  store i64 %6, ptr %11, align 4, !tbaa !1
  %17 = load <2 x i64>, ptr %13, align 4, !tbaa !1
  %18 = shufflevector <2 x i64> %17, <2 x i64> poison, <2 x i32> <i32 1, i32 0>
  store <2 x i64> %18, ptr %12, align 4, !tbaa !1
  store i64 %10, ptr %14, align 4, !tbaa !1
  %19 = icmp eq i64 %8, 0
  store i64 0, ptr %3, align 4, !tbaa !1
  store i64 0, ptr %4, align 4, !tbaa !1
  store i64 %8, ptr %15, align 4, !tbaa !1
  %..i = select i1 %19, i64 70778, i64 70656
  store i64 %..i, ptr %16, align 4, !tbaa !1
  ret i64 4294967298
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(read, argmem: readwrite, inaccessiblemem: none)
define noundef i64 @tb_1165c(ptr captures(none) initializes((80, 144), (512, 520)) %0, i64 %1) local_unnamed_addr #0 {
entry:
  %2 = getelementptr i8, ptr %0, i64 80
  %3 = getelementptr i8, ptr %0, i64 88
  %4 = getelementptr i8, ptr %0, i64 96
  %5 = getelementptr i8, ptr %0, i64 112
  %6 = getelementptr i8, ptr %0, i64 120
  %7 = getelementptr i8, ptr %0, i64 128
  %8 = getelementptr i8, ptr %0, i64 168
  %9 = load i64, ptr %8, align 4, !tbaa !1
  %10 = getelementptr i8, ptr %0, i64 512
  store i64 0, ptr %2, align 4, !tbaa !1
  tail call void @llvm.memset.p0.i64(ptr noundef nonnull align 4 dereferenceable(24) %4, i8 0, i64 24, i1 false)
  tail call void @llvm.memset.p0.i64(ptr noundef nonnull align 4 dereferenceable(16) %7, i8 0, i64 16, i1 false)
  %11 = getelementptr i8, ptr %0, i64 64
  %12 = load i64, ptr %11, align 4, !tbaa !1
  %13 = add i64 %12, %1
  %14 = inttoptr i64 %13 to ptr
  %15 = load i32, ptr %14, align 4, !tbaa !4
  %16 = sext i32 %15 to i64
  %17 = icmp sgt i32 %15, 0
  %18 = zext i1 %17 to i64
  %.not.i = icmp slt i64 %9, %16
  store i64 %18, ptr %3, align 4, !tbaa !1
  store i64 %16, ptr %4, align 4, !tbaa !1
  store i64 %16, ptr %5, align 4, !tbaa !1
  store i64 10, ptr %6, align 4, !tbaa !1
  %..i = select i1 %.not.i, i64 71324, i64 71274
  store i64 %..i, ptr %10, align 4, !tbaa !1
  ret i64 4294967298
}

; Function Attrs: nofree norecurse nosync nounwind memory(readwrite, inaccessiblemem: none)
define noundef i64 @tb_116bc(ptr captures(none) initializes((88, 128)) %0, i64 %1) local_unnamed_addr #5 {
entry:
  %2 = getelementptr i8, ptr %0, i64 72
  %3 = load i64, ptr %2, align 4, !tbaa !1
  %4 = getelementptr i8, ptr %0, i64 88
  %5 = getelementptr i8, ptr %0, i64 104
  %6 = getelementptr i8, ptr %0, i64 112
  %7 = getelementptr i8, ptr %0, i64 120
  %8 = getelementptr i8, ptr %0, i64 152
  %9 = load i64, ptr %8, align 4, !tbaa !1
  %10 = getelementptr i8, ptr %0, i64 160
  %11 = load i64, ptr %10, align 4, !tbaa !1
  %12 = add i64 %9, %1
  %13 = inttoptr i64 %12 to ptr
  %14 = load i16, ptr %13, align 2, !tbaa !4
  %15 = zext i16 %14 to i64
  %16 = sub i64 %15, %11
  %17 = trunc i64 %16 to i16
  store i16 %17, ptr %13, align 2, !tbaa !4
  %18 = icmp ugt i64 %3, 1
  tail call void @llvm.memset.p0.i64(ptr noundef nonnull align 4 dereferenceable(16) %4, i8 0, i64 16, i1 false)
  store i64 1, ptr %5, align 4, !tbaa !1
  store i64 %16, ptr %6, align 4, !tbaa !1
  store i64 %9, ptr %7, align 4, !tbaa !1
  br i1 %18, label %L0, label %common.ret

L0:                                               ; preds = %entry
  %.reass135.i = add i64 %12, 2
  %19 = inttoptr i64 %.reass135.i to ptr
  %20 = load i16, ptr %19, align 2, !tbaa !4
  %21 = zext i16 %20 to i64
  %22 = sub i64 %21, %11
  %23 = trunc i64 %22 to i16
  store i16 %23, ptr %19, align 2, !tbaa !4
  %.not = icmp eq i64 %3, 2
  br i1 %.not, label %tb_116c2.exit, label %L0.i

L0.i:                                             ; preds = %L0, %L0.i
  %24 = phi i64 [ %30, %L0.i ], [ 2, %L0 ]
  %25 = shl i64 %24, 1
  %26 = and i64 %25, 8589934590
  %.reass.i = add i64 %26, %12
  %27 = inttoptr i64 %.reass.i to ptr
  %28 = load i16, ptr %27, align 2, !tbaa !4
  %29 = zext i16 %28 to i64
  %30 = add nuw i64 %24, 1
  %31 = sub i64 %29, %11
  %32 = trunc i64 %31 to i16
  store i16 %32, ptr %27, align 2, !tbaa !4
  %exitcond.not.i = icmp eq i64 %30, %3
  br i1 %exitcond.not.i, label %tb_116c2.exit, label %L0.i

common.ret:                                       ; preds = %entry, %tb_116c2.exit
  %storemerge = phi i64 [ 71360, %tb_116c2.exit ], [ 71402, %entry ]
  %33 = getelementptr i8, ptr %0, i64 512
  %34 = getelementptr i8, ptr %0, i64 96
  store i64 1, ptr %4, align 4, !tbaa !1
  store i64 %3, ptr %34, align 4, !tbaa !1
  store i64 %storemerge, ptr %33, align 4, !tbaa !1
  ret i64 4294967298

tb_116c2.exit:                                    ; preds = %L0.i, %L0
  %.pn.i = phi i64 [ 2, %L0 ], [ %26, %L0.i ]
  %.lcssa133.i = phi i64 [ %22, %L0 ], [ %31, %L0.i ]
  %.lcssa134.i = add i64 %.pn.i, %9
  store i64 %3, ptr %5, align 4, !tbaa !1
  store i64 %.lcssa133.i, ptr %6, align 4, !tbaa !1
  store i64 %.lcssa134.i, ptr %7, align 4, !tbaa !1
  br label %common.ret
}

; Function Attrs: nofree nosync nounwind memory(readwrite, inaccessiblemem: none)
define range(i64 2, 4294967299) i64 @tb_11768(ptr initializes((8, 16), (48, 56), (88, 144), (224, 248), (512, 520)) %0, i64 %1) local_unnamed_addr #1 {
entry:
  %2 = getelementptr i8, ptr %0, i64 8
  %3 = getelementptr i8, ptr %0, i64 16
  %4 = load i64, ptr %3, align 4, !tbaa !1
  %5 = getelementptr i8, ptr %0, i64 64
  %6 = load i64, ptr %5, align 4, !tbaa !1
  %7 = getelementptr i8, ptr %0, i64 88
  %8 = getelementptr i8, ptr %0, i64 512
  %9 = add i64 %4, %1
  %10 = inttoptr i64 %9 to ptr
  %11 = load i64, ptr %10, align 4, !tbaa !4
  %12 = add i64 %9, 8
  %13 = inttoptr i64 %12 to ptr
  %14 = load i64, ptr %13, align 4, !tbaa !4
  %15 = add i64 %4, 16
  store i64 %14, ptr %2, align 4, !tbaa !1
  store i64 %15, ptr %3, align 4, !tbaa !1
  store i64 %11, ptr %5, align 4, !tbaa !1
  store i64 %6, ptr %7, align 4, !tbaa !1
  store i64 74642, ptr %8, align 4, !tbaa !1
  %16 = musttail call i64 @tb_12392(ptr %0, i64 %1)
  ret i64 %16
}

; Function Attrs: nofree nosync nounwind memory(readwrite, inaccessiblemem: none)
define range(i64 2, 4294967299) i64 @tb_10b66(ptr initializes((72, 80), (120, 144), (512, 520)) %0, i64 %1) local_unnamed_addr #1 {
entry:
  %2 = getelementptr i8, ptr %0, i64 16
  %3 = load i64, ptr %2, align 4, !tbaa !1
  %4 = getelementptr i8, ptr %0, i64 72
  %5 = getelementptr i8, ptr %0, i64 80
  %6 = load i64, ptr %5, align 4, !tbaa !1
  %7 = getelementptr i8, ptr %0, i64 120
  %8 = getelementptr i8, ptr %0, i64 128
  %9 = getelementptr i8, ptr %0, i64 136
  %10 = getelementptr i8, ptr %0, i64 512
  %11 = add i64 %3, %1
  %12 = inttoptr i64 %11 to ptr
  %13 = load i64, ptr %12, align 4, !tbaa !4
  %14 = add i64 %11, 8
  %15 = inttoptr i64 %14 to ptr
  %16 = load i64, ptr %15, align 4, !tbaa !4
  %17 = add i64 %13, %1
  %18 = add i64 %17, 100
  %19 = inttoptr i64 %18 to ptr
  %20 = load i16, ptr %19, align 2, !tbaa !4
  %21 = shl i64 %6, 48
  %22 = ashr exact i64 %21, 48
  %.not = icmp eq i16 %20, 0
  store i64 %22, ptr %4, align 4, !tbaa !1
  store i64 %16, ptr %8, align 4, !tbaa !1
  store i64 %13, ptr %9, align 4, !tbaa !1
  br i1 %.not, label %fall, label %L0

L0:                                               ; preds = %entry
  %23 = add i64 %17, 96
  %24 = inttoptr i64 %23 to ptr
  %25 = load i16, ptr %24, align 2, !tbaa !4
  %26 = zext i16 %25 to i64
  store i64 %26, ptr %7, align 4, !tbaa !1
  store i64 68480, ptr %10, align 4, !tbaa !1
  ret i64 4294967298

fall:                                             ; preds = %entry
  %27 = getelementptr i8, ptr %0, i64 8
  %28 = getelementptr i8, ptr %0, i64 88
  %29 = add i64 %17, 96
  %30 = inttoptr i64 %29 to ptr
  %31 = load i16, ptr %30, align 2, !tbaa !4
  %32 = zext i16 %31 to i64
  %33 = trunc i64 %6 to i16
  store i16 %33, ptr %19, align 2, !tbaa !4
  store i64 %13, ptr %15, align 4, !tbaa !4
  store i64 %16, ptr %12, align 4, !tbaa !4
  store i64 68490, ptr %27, align 4, !tbaa !1
  store i64 %32, ptr %28, align 4, !tbaa !1
  store i64 %32, ptr %7, align 4, !tbaa !1
  store i64 73184, ptr %10, align 4, !tbaa !1
  %34 = musttail call range(i64 2, 4294967299) i64 @tb_11de0(ptr nonnull %0, i64 %1)
  ret i64 %34
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(readwrite, inaccessiblemem: none)
define noundef i64 @tb_11b1e(ptr captures(none) initializes((104, 128), (512, 520)) %0, i64 %1) local_unnamed_addr #4 {
entry:
  %2 = getelementptr i8, ptr %0, i64 8
  %3 = getelementptr i8, ptr %0, i64 16
  %4 = load i64, ptr %3, align 4, !tbaa !1
  %5 = getelementptr i8, ptr %0, i64 64
  %6 = load i64, ptr %5, align 4, !tbaa !1
  %7 = getelementptr i8, ptr %0, i64 72
  %8 = load i64, ptr %7, align 4, !tbaa !1
  %9 = getelementptr i8, ptr %0, i64 80
  %10 = load i64, ptr %9, align 4, !tbaa !1
  %11 = getelementptr i8, ptr %0, i64 104
  %12 = getelementptr i8, ptr %0, i64 112
  %13 = getelementptr i8, ptr %0, i64 120
  %14 = getelementptr i8, ptr %0, i64 152
  %15 = load <2 x i64>, ptr %14, align 4, !tbaa !1
  %16 = getelementptr i8, ptr %0, i64 192
  %17 = load i64, ptr %16, align 4, !tbaa !1
  %18 = shl i64 %10, 2
  %19 = and i64 %18, 17179869180
  %20 = add i64 %19, %6
  %21 = add i64 %1, 8
  %22 = add i64 %21, %4
  %23 = inttoptr i64 %22 to ptr
  %24 = load i64, ptr %23, align 4, !tbaa !4
  %25 = add i64 %20, %1
  %26 = inttoptr i64 %25 to ptr
  %27 = load i32, ptr %26, align 4, !tbaa !4
  %28 = sext i32 %27 to i64
  %29 = add i64 %24, %1
  %30 = inttoptr i64 %29 to ptr
  %31 = load i8, ptr %30, align 1, !tbaa !4
  %32 = zext i8 %31 to i64
  %33 = add nsw i64 %28, 1
  %34 = trunc i64 %33 to i32
  store i32 %34, ptr %26, align 4, !tbaa !4
  %.not = icmp eq i8 %31, 0
  store i64 %32, ptr %11, align 4, !tbaa !1
  store i64 %20, ptr %12, align 4, !tbaa !1
  store i64 %33, ptr %13, align 4, !tbaa !1
  br i1 %.not, label %fall, label %common.ret.sink.split

common.ret.sink.split:                            ; preds = %fall, %entry
  %.sink = phi i64 [ 72478, %entry ], [ 72562, %fall ]
  store i64 %.sink, ptr %2, align 4, !tbaa !1
  %35 = shufflevector <2 x i64> %15, <2 x i64> poison, <2 x i32> <i32 1, i32 0>
  store <2 x i64> %35, ptr %9, align 4, !tbaa !1
  br label %common.ret

common.ret:                                       ; preds = %common.ret.sink.split, %fall
  %storemerge = phi i64 [ 72516, %fall ], [ 71878, %common.ret.sink.split ]
  %36 = getelementptr i8, ptr %0, i64 512
  store i64 %storemerge, ptr %36, align 4, !tbaa !1
  ret i64 4294967298

fall:                                             ; preds = %entry
  %37 = and i64 %17, 4294967295
  store i64 %8, ptr %23, align 4, !tbaa !4
  %38 = add i64 %37, %8
  %.not.i = icmp ult i64 %8, %38
  store i64 %38, ptr %16, align 4, !tbaa !1
  br i1 %.not.i, label %common.ret, label %common.ret.sink.split
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(readwrite, inaccessiblemem: none)
define noundef i64 @tb_11b72(ptr captures(none) initializes((104, 128), (512, 520)) %0, i64 %1) local_unnamed_addr #4 {
entry:
  %2 = getelementptr i8, ptr %0, i64 16
  %3 = load i64, ptr %2, align 4, !tbaa !1
  %4 = getelementptr i8, ptr %0, i64 64
  %5 = load i64, ptr %4, align 4, !tbaa !1
  %6 = getelementptr i8, ptr %0, i64 72
  %7 = load i64, ptr %6, align 4, !tbaa !1
  %8 = getelementptr i8, ptr %0, i64 80
  %9 = load i64, ptr %8, align 4, !tbaa !1
  %10 = getelementptr i8, ptr %0, i64 104
  %11 = getelementptr i8, ptr %0, i64 112
  %12 = getelementptr i8, ptr %0, i64 120
  %13 = getelementptr i8, ptr %0, i64 152
  %14 = load <2 x i64>, ptr %13, align 4, !tbaa !1
  %15 = getelementptr i8, ptr %0, i64 192
  %16 = load i64, ptr %15, align 4, !tbaa !1
  %17 = shl i64 %9, 2
  %18 = and i64 %17, 17179869180
  %19 = add i64 %18, %5
  %20 = add i64 %1, 8
  %21 = add i64 %20, %3
  %22 = inttoptr i64 %21 to ptr
  %23 = load i64, ptr %22, align 4, !tbaa !4
  %24 = add i64 %19, %1
  %25 = inttoptr i64 %24 to ptr
  %26 = load i32, ptr %25, align 4, !tbaa !4
  %27 = sext i32 %26 to i64
  %28 = add i64 %23, %1
  %29 = inttoptr i64 %28 to ptr
  %30 = load i8, ptr %29, align 1, !tbaa !4
  %31 = zext i8 %30 to i64
  %32 = add nsw i64 %27, 1
  %33 = trunc i64 %32 to i32
  store i32 %33, ptr %25, align 4, !tbaa !4
  %.not = icmp eq i8 %30, 0
  store i64 %31, ptr %10, align 4, !tbaa !1
  store i64 %19, ptr %11, align 4, !tbaa !1
  store i64 %32, ptr %12, align 4, !tbaa !1
  br i1 %.not, label %fall, label %L0

common.ret:                                       ; preds = %fall, %L0
  %storemerge = phi i64 [ %..i, %fall ], [ 71878, %L0 ]
  %34 = getelementptr i8, ptr %0, i64 512
  store i64 %storemerge, ptr %34, align 4, !tbaa !1
  ret i64 4294967298

L0:                                               ; preds = %entry
  %35 = getelementptr i8, ptr %0, i64 8
  store i64 72562, ptr %35, align 4, !tbaa !1
  %36 = shufflevector <2 x i64> %14, <2 x i64> poison, <2 x i32> <i32 1, i32 0>
  store <2 x i64> %36, ptr %8, align 4, !tbaa !1
  br label %common.ret

fall:                                             ; preds = %entry
  %.not.i = icmp ult i64 %7, %16
  store i64 44, ptr %10, align 4, !tbaa !1
  %..i = select i1 %.not.i, i64 72594, i64 72616
  br label %common.ret
}

; Function Attrs: nofree nosync nounwind memory(readwrite, inaccessiblemem: none)
define range(i64 2, 4294967299) i64 @tb_11bb4(ptr initializes((8, 16), (48, 56), (88, 144), (224, 256), (512, 520)) %0, i64 %1) local_unnamed_addr #1 {
entry:
  %2 = getelementptr i8, ptr %0, i64 8
  %3 = getelementptr i8, ptr %0, i64 72
  %4 = load i64, ptr %3, align 4, !tbaa !1
  %5 = getelementptr i8, ptr %0, i64 80
  %6 = load i64, ptr %5, align 4, !tbaa !1
  %7 = getelementptr i8, ptr %0, i64 88
  %8 = getelementptr i8, ptr %0, i64 512
  %9 = add i64 %4, %1
  %10 = inttoptr i64 %9 to ptr
  %11 = load i32, ptr %10, align 4, !tbaa !4
  %12 = sext i32 %11 to i64
  %13 = add i64 %4, 4
  store i64 72638, ptr %2, align 4, !tbaa !1
  store i64 %13, ptr %3, align 4, !tbaa !1
  store i64 %12, ptr %5, align 4, !tbaa !1
  store i64 %6, ptr %7, align 4, !tbaa !1
  store i64 73664, ptr %8, align 4, !tbaa !1
  %14 = musttail call i64 @tb_11fc0(ptr %0, i64 %1)
  ret i64 %14
}

; Function Attrs: nofree nosync nounwind memory(readwrite, inaccessiblemem: none)
define range(i64 2, 4294967299) i64 @tb_11bbe(ptr initializes((8, 16), (144, 152), (512, 520)) %0, i64 %1) local_unnamed_addr #1 {
entry:
  %2 = getelementptr i8, ptr %0, i64 64
  %3 = load i64, ptr %2, align 4, !tbaa !1
  %4 = getelementptr i8, ptr %0, i64 80
  %5 = load i64, ptr %4, align 4, !tbaa !1
  %6 = getelementptr i8, ptr %0, i64 144
  %7 = getelementptr i8, ptr %0, i64 152
  %8 = load i64, ptr %7, align 4, !tbaa !1
  %9 = getelementptr i8, ptr %0, i64 512
  %.not = icmp eq i64 %8, %3
  store i64 %5, ptr %6, align 4, !tbaa !1
  %10 = getelementptr i8, ptr %0, i64 8
  br i1 %.not, label %fall, label %L0

L0:                                               ; preds = %entry
  %11 = getelementptr i8, ptr %0, i64 88
  %12 = add i64 %3, %1
  %13 = inttoptr i64 %12 to ptr
  %14 = load i32, ptr %13, align 4, !tbaa !4
  %15 = sext i32 %14 to i64
  %16 = add i64 %3, 4
  store i64 72628, ptr %10, align 4, !tbaa !1
  store i64 %16, ptr %2, align 4, !tbaa !1
  store i64 %15, ptr %4, align 4, !tbaa !1
  store i64 %5, ptr %11, align 4, !tbaa !1
  store i64 73664, ptr %9, align 4, !tbaa !1
  %17 = musttail call range(i64 2, 4294967299) i64 @tb_11fc0(ptr nonnull %0, i64 %1)
  ret i64 %17

fall:                                             ; preds = %entry
  %18 = getelementptr i8, ptr %0, i64 16
  %19 = load i64, ptr %18, align 4, !tbaa !1
  %20 = getelementptr i8, ptr %0, i64 72
  %21 = getelementptr i8, ptr %0, i64 160
  %22 = getelementptr i8, ptr %0, i64 168
  %23 = getelementptr i8, ptr %0, i64 176
  %24 = getelementptr i8, ptr %0, i64 184
  %25 = getelementptr i8, ptr %0, i64 192
  %26 = add i64 %19, %1
  %27 = add i64 %26, 152
  %28 = inttoptr i64 %27 to ptr
  %29 = load i64, ptr %28, align 4, !tbaa !4
  %30 = add i64 %26, 144
  %31 = inttoptr i64 %30 to ptr
  %32 = load i64, ptr %31, align 4, !tbaa !4
  %33 = add i64 %26, 136
  %34 = inttoptr i64 %33 to ptr
  %35 = load i64, ptr %34, align 4, !tbaa !4
  %36 = add i64 %26, 128
  %37 = inttoptr i64 %36 to ptr
  %38 = load i64, ptr %37, align 4, !tbaa !4
  %39 = add i64 %26, 120
  %40 = inttoptr i64 %39 to ptr
  %41 = load i64, ptr %40, align 4, !tbaa !4
  %42 = add i64 %26, 112
  %43 = inttoptr i64 %42 to ptr
  %44 = load i64, ptr %43, align 4, !tbaa !4
  %45 = add i64 %26, 104
  %46 = inttoptr i64 %45 to ptr
  %47 = load i64, ptr %46, align 4, !tbaa !4
  %48 = add i64 %26, 96
  %49 = inttoptr i64 %48 to ptr
  %50 = load i64, ptr %49, align 4, !tbaa !4
  %51 = add i64 %26, 88
  %52 = inttoptr i64 %51 to ptr
  %53 = load i64, ptr %52, align 4, !tbaa !4
  %54 = add i64 %26, 80
  %55 = inttoptr i64 %54 to ptr
  %56 = load i64, ptr %55, align 4, !tbaa !4
  %57 = add i64 %19, 160
  %58 = and i64 %29, -2
  store i64 %29, ptr %10, align 4, !tbaa !1
  store i64 %57, ptr %18, align 4, !tbaa !1
  store i64 %32, ptr %2, align 4, !tbaa !1
  store i64 %35, ptr %20, align 4, !tbaa !1
  store i64 %38, ptr %6, align 4, !tbaa !1
  store i64 %41, ptr %7, align 4, !tbaa !1
  store i64 %44, ptr %21, align 4, !tbaa !1
  store i64 %47, ptr %22, align 4, !tbaa !1
  store i64 %50, ptr %23, align 4, !tbaa !1
  store i64 %53, ptr %24, align 4, !tbaa !1
  store i64 %56, ptr %25, align 4, !tbaa !1
  store i64 %58, ptr %9, align 4, !tbaa !1
  %59 = musttail call range(i64 2, 4294967299) i64 @aot_dispatch(ptr nonnull %0, i64 %1)
  ret i64 %59
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(readwrite, inaccessiblemem: none)
define noundef i64 @tb_10be0(ptr captures(none) initializes((72, 80), (120, 144), (512, 520)) %0, i64 %1) local_unnamed_addr #4 {
entry:
  %2 = getelementptr i8, ptr %0, i64 16
  %3 = load i64, ptr %2, align 4, !tbaa !1
  %4 = getelementptr i8, ptr %0, i64 72
  %5 = getelementptr i8, ptr %0, i64 80
  %6 = load i64, ptr %5, align 4, !tbaa !1
  %7 = getelementptr i8, ptr %0, i64 128
  %8 = getelementptr i8, ptr %0, i64 136
  %9 = add i64 %3, %1
  %10 = inttoptr i64 %9 to ptr
  %11 = load i64, ptr %10, align 4, !tbaa !4
  %12 = add i64 %9, 8
  %13 = inttoptr i64 %12 to ptr
  %14 = load i64, ptr %13, align 4, !tbaa !4
  %15 = add i64 %11, %1
  %16 = add i64 %15, 102
  %17 = inttoptr i64 %16 to ptr
  %18 = load i16, ptr %17, align 2, !tbaa !4
  %19 = shl i64 %6, 48
  %20 = ashr exact i64 %19, 48
  %.not = icmp eq i16 %18, 0
  store i64 %20, ptr %4, align 4, !tbaa !1
  store i64 %14, ptr %7, align 4, !tbaa !1
  store i64 %11, ptr %8, align 4, !tbaa !1
  br i1 %.not, label %fall, label %common.ret

common.ret:                                       ; preds = %entry, %fall
  %21 = getelementptr i8, ptr %0, i64 120
  %22 = getelementptr i8, ptr %0, i64 512
  %storemerge.in.in.in = add i64 %15, 96
  %storemerge.in.in = inttoptr i64 %storemerge.in.in.in to ptr
  %storemerge.in = load i16, ptr %storemerge.in.in, align 2, !tbaa !4
  %storemerge = zext i16 %storemerge.in to i64
  store i64 %storemerge, ptr %21, align 4, !tbaa !1
  store i64 68480, ptr %22, align 4, !tbaa !1
  ret i64 4294967298

fall:                                             ; preds = %entry
  %23 = trunc i64 %6 to i16
  store i16 %23, ptr %17, align 2, !tbaa !4
  br label %common.ret
}

; Function Attrs: nofree nosync nounwind memory(readwrite, inaccessiblemem: none)
define range(i64 2, 4294967299) i64 @tb_10f12(ptr initializes((64, 72), (88, 128), (136, 160), (512, 520)) %0, i64 %1) local_unnamed_addr #1 {
entry:
  %2 = getelementptr i8, ptr %0, i64 16
  %3 = load i64, ptr %2, align 4, !tbaa !1
  %4 = getelementptr i8, ptr %0, i64 64
  %5 = getelementptr i8, ptr %0, i64 80
  %6 = load i64, ptr %5, align 4, !tbaa !1
  %7 = getelementptr i8, ptr %0, i64 88
  %8 = getelementptr i8, ptr %0, i64 96
  %9 = getelementptr i8, ptr %0, i64 104
  %10 = getelementptr i8, ptr %0, i64 112
  %11 = getelementptr i8, ptr %0, i64 120
  %12 = getelementptr i8, ptr %0, i64 136
  %13 = getelementptr i8, ptr %0, i64 144
  %14 = getelementptr i8, ptr %0, i64 152
  %15 = getelementptr i8, ptr %0, i64 512
  %16 = add i64 %1, 8
  %17 = add i64 %3, %16
  %18 = inttoptr i64 %17 to ptr
  %19 = load i64, ptr %18, align 4, !tbaa !4
  %20 = add i64 %3, %1
  %21 = inttoptr i64 %20 to ptr
  %22 = load i64, ptr %21, align 4, !tbaa !4
  %23 = add i64 %6, %1
  %24 = inttoptr i64 %23 to ptr
  %25 = load i64, ptr %24, align 4, !tbaa !4
  %26 = add i64 %25, %1
  %27 = inttoptr i64 %26 to ptr
  %28 = load i64, ptr %27, align 4, !tbaa !4
  %29 = add i64 %25, %16
  %30 = inttoptr i64 %29 to ptr
  %31 = load i64, ptr %30, align 4, !tbaa !4
  %32 = add i64 %28, %16
  %33 = inttoptr i64 %32 to ptr
  %34 = load i64, ptr %33, align 4, !tbaa !4
  %35 = add i64 %28, %1
  %36 = inttoptr i64 %35 to ptr
  %37 = load i64, ptr %36, align 4, !tbaa !4
  store i64 %34, ptr %30, align 4, !tbaa !4
  store i64 %31, ptr %33, align 4, !tbaa !4
  store i64 %37, ptr %27, align 4, !tbaa !4
  store i64 0, ptr %36, align 4, !tbaa !4
  %38 = icmp sgt i64 %22, -1
  store i64 %6, ptr %4, align 4, !tbaa !1
  store i64 %34, ptr %7, align 4, !tbaa !1
  store i64 %37, ptr %8, align 4, !tbaa !1
  store i64 %22, ptr %9, align 4, !tbaa !1
  store i64 %31, ptr %10, align 4, !tbaa !1
  store i64 %25, ptr %11, align 4, !tbaa !1
  store i64 %19, ptr %12, align 4, !tbaa !1
  store i64 %6, ptr %13, align 4, !tbaa !1
  store i64 %28, ptr %14, align 4, !tbaa !1
  br i1 %38, label %L0, label %common.ret

L0:                                               ; preds = %entry
  %39 = add i64 %6, %16
  %40 = inttoptr i64 %39 to ptr
  %41 = load i64, ptr %40, align 4, !tbaa !4
  %42 = add i64 %1, 2
  %43 = add i64 %42, %41
  %44 = inttoptr i64 %43 to ptr
  %45 = load i16, ptr %44, align 2, !tbaa !4
  %46 = sext i16 %45 to i64
  %.not.i = icmp eq i64 %22, %46
  store i64 %46, ptr %11, align 4, !tbaa !1
  br i1 %.not.i, label %fall.i, label %common.ret

fall.i:                                           ; preds = %L0
  %47 = getelementptr i8, ptr %0, i64 8
  %48 = getelementptr i8, ptr %0, i64 72
  %49 = load i64, ptr %48, align 4, !tbaa !1
  %50 = add i64 %41, %1
  %51 = inttoptr i64 %50 to ptr
  %52 = load i16, ptr %51, align 2, !tbaa !4
  %53 = sext i16 %52 to i64
  store i64 69468, ptr %47, align 4, !tbaa !1
  store i64 %53, ptr %5, align 4, !tbaa !1
  store i64 %49, ptr %7, align 4, !tbaa !1
  store i64 %41, ptr %11, align 4, !tbaa !1
  store i64 74642, ptr %15, align 4, !tbaa !1
  %54 = musttail call range(i64 2, 4294967299) i64 @tb_12392(ptr nonnull %0, i64 %1)
  ret i64 %54

common.ret:                                       ; preds = %entry, %L0
  %storemerge = phi i64 [ 69436, %L0 ], [ 69576, %entry ]
  store i64 %storemerge, ptr %15, align 4, !tbaa !1
  ret i64 4294967298
}

; Function Attrs: nofree nosync nounwind memory(readwrite, inaccessiblemem: none)
define range(i64 2, 4294967299) i64 @tb_10fea(ptr initializes((8, 16), (72, 80), (88, 128), (512, 520)) %0, i64 %1) local_unnamed_addr #1 {
entry:
  %2 = getelementptr i8, ptr %0, i64 72
  %3 = getelementptr i8, ptr %0, i64 80
  %4 = load i64, ptr %3, align 4, !tbaa !1
  %5 = getelementptr i8, ptr %0, i64 88
  %6 = getelementptr i8, ptr %0, i64 120
  %7 = getelementptr i8, ptr %0, i64 144
  %8 = load i64, ptr %7, align 4, !tbaa !1
  %9 = getelementptr i8, ptr %0, i64 152
  %10 = load i64, ptr %9, align 4, !tbaa !1
  %11 = getelementptr i8, ptr %0, i64 512
  %12 = add i64 %8, %1
  %13 = inttoptr i64 %12 to ptr
  %14 = load i64, ptr %13, align 4, !tbaa !4
  %.not = icmp eq i64 %14, 0
  store i64 %4, ptr %2, align 4, !tbaa !1
  br i1 %.not, label %fall, label %L0

L0:                                               ; preds = %entry
  %15 = getelementptr i8, ptr %0, i64 8
  %16 = getelementptr i8, ptr %0, i64 64
  %17 = load i64, ptr %16, align 4, !tbaa !1
  %18 = add i64 %1, 8
  %19 = add i64 %18, %17
  %20 = inttoptr i64 %19 to ptr
  %21 = load i64, ptr %20, align 4, !tbaa !4
  %22 = add i64 %21, %1
  %23 = inttoptr i64 %22 to ptr
  %24 = load i16, ptr %23, align 2, !tbaa !4
  %25 = sext i16 %24 to i64
  store i64 69468, ptr %15, align 4, !tbaa !1
  store i64 %25, ptr %3, align 4, !tbaa !1
  store i64 %4, ptr %5, align 4, !tbaa !1
  store i64 %21, ptr %6, align 4, !tbaa !1
  store i64 %14, ptr %7, align 4, !tbaa !1
  store i64 74642, ptr %11, align 4, !tbaa !1
  %26 = musttail call range(i64 2, 4294967299) i64 @tb_12392(ptr nonnull %0, i64 %1)
  ret i64 %26

fall:                                             ; preds = %entry
  %27 = getelementptr i8, ptr %0, i64 112
  %28 = getelementptr i8, ptr %0, i64 104
  %29 = getelementptr i8, ptr %0, i64 96
  %30 = getelementptr i8, ptr %0, i64 8
  %31 = getelementptr i8, ptr %0, i64 64
  %32 = load i64, ptr %31, align 4, !tbaa !1
  %33 = add i64 %32, %1
  %34 = inttoptr i64 %33 to ptr
  %35 = load i64, ptr %34, align 4, !tbaa !4
  %36 = add i64 %1, 8
  %37 = add i64 %10, %36
  %38 = inttoptr i64 %37 to ptr
  %39 = load i64, ptr %38, align 4, !tbaa !4
  %40 = add i64 %35, %36
  %41 = inttoptr i64 %40 to ptr
  %42 = load i64, ptr %41, align 4, !tbaa !4
  %43 = add i64 %35, %1
  %44 = inttoptr i64 %43 to ptr
  %45 = load i64, ptr %44, align 4, !tbaa !4
  store i64 %42, ptr %38, align 4, !tbaa !4
  store i64 %39, ptr %41, align 4, !tbaa !4
  %46 = add i64 %10, %1
  %47 = inttoptr i64 %46 to ptr
  store i64 %45, ptr %47, align 4, !tbaa !4
  store i64 %10, ptr %44, align 4, !tbaa !4
  store i64 69526, ptr %30, align 4, !tbaa !1
  store i64 %32, ptr %3, align 4, !tbaa !1
  store i64 68302, ptr %5, align 4, !tbaa !1
  store i64 0, ptr %29, align 4, !tbaa !1
  store i64 %42, ptr %28, align 4, !tbaa !1
  store i64 %39, ptr %27, align 4, !tbaa !1
  store i64 %45, ptr %6, align 4, !tbaa !1
  store i64 %35, ptr %7, align 4, !tbaa !1
  store i64 68848, ptr %11, align 4, !tbaa !1
  ret i64 4294967298
}

; Function Attrs: nofree nosync nounwind memory(readwrite, inaccessiblemem: none)
define range(i64 2, 4294967299) i64 @tb_10f5c(ptr initializes((8, 16), (72, 80), (88, 128), (512, 520)) %0, i64 %1) local_unnamed_addr #1 {
entry:
  %2 = getelementptr i8, ptr %0, i64 72
  %3 = getelementptr i8, ptr %0, i64 80
  %4 = load i64, ptr %3, align 4, !tbaa !1
  %5 = getelementptr i8, ptr %0, i64 88
  %6 = getelementptr i8, ptr %0, i64 120
  %7 = getelementptr i8, ptr %0, i64 144
  %8 = load i64, ptr %7, align 4, !tbaa !1
  %9 = getelementptr i8, ptr %0, i64 152
  %10 = load i64, ptr %9, align 4, !tbaa !1
  %11 = getelementptr i8, ptr %0, i64 512
  %12 = add i64 %8, %1
  %13 = inttoptr i64 %12 to ptr
  %14 = load i64, ptr %13, align 4, !tbaa !4
  %.not = icmp eq i64 %14, 0
  store i64 %4, ptr %2, align 4, !tbaa !1
  br i1 %.not, label %fall, label %L0

L0:                                               ; preds = %entry
  %15 = getelementptr i8, ptr %0, i64 8
  %16 = getelementptr i8, ptr %0, i64 64
  %17 = load i64, ptr %16, align 4, !tbaa !1
  %18 = add i64 %1, 8
  %19 = add i64 %18, %17
  %20 = inttoptr i64 %19 to ptr
  %21 = load i64, ptr %20, align 4, !tbaa !4
  %22 = add i64 %21, %1
  %23 = inttoptr i64 %22 to ptr
  %24 = load i16, ptr %23, align 2, !tbaa !4
  %25 = sext i16 %24 to i64
  store i64 69468, ptr %15, align 4, !tbaa !1
  store i64 %25, ptr %3, align 4, !tbaa !1
  store i64 %4, ptr %5, align 4, !tbaa !1
  store i64 %21, ptr %6, align 4, !tbaa !1
  store i64 %14, ptr %7, align 4, !tbaa !1
  store i64 74642, ptr %11, align 4, !tbaa !1
  %26 = musttail call range(i64 2, 4294967299) i64 @tb_12392(ptr nonnull %0, i64 %1)
  ret i64 %26

fall:                                             ; preds = %entry
  %27 = getelementptr i8, ptr %0, i64 112
  %28 = getelementptr i8, ptr %0, i64 104
  %29 = getelementptr i8, ptr %0, i64 96
  %30 = getelementptr i8, ptr %0, i64 8
  %31 = getelementptr i8, ptr %0, i64 64
  %32 = load i64, ptr %31, align 4, !tbaa !1
  %33 = add i64 %32, %1
  %34 = inttoptr i64 %33 to ptr
  %35 = load i64, ptr %34, align 4, !tbaa !4
  %36 = add i64 %1, 8
  %37 = add i64 %10, %36
  %38 = inttoptr i64 %37 to ptr
  %39 = load i64, ptr %38, align 4, !tbaa !4
  %40 = add i64 %35, %36
  %41 = inttoptr i64 %40 to ptr
  %42 = load i64, ptr %41, align 4, !tbaa !4
  %43 = add i64 %35, %1
  %44 = inttoptr i64 %43 to ptr
  %45 = load i64, ptr %44, align 4, !tbaa !4
  store i64 %42, ptr %38, align 4, !tbaa !4
  store i64 %39, ptr %41, align 4, !tbaa !4
  %46 = add i64 %10, %1
  %47 = inttoptr i64 %46 to ptr
  store i64 %45, ptr %47, align 4, !tbaa !4
  store i64 %10, ptr %44, align 4, !tbaa !4
  store i64 69526, ptr %30, align 4, !tbaa !1
  store i64 %32, ptr %3, align 4, !tbaa !1
  store i64 68302, ptr %5, align 4, !tbaa !1
  store i64 0, ptr %29, align 4, !tbaa !1
  store i64 %42, ptr %28, align 4, !tbaa !1
  store i64 %39, ptr %27, align 4, !tbaa !1
  store i64 %45, ptr %6, align 4, !tbaa !1
  store i64 %35, ptr %7, align 4, !tbaa !1
  store i64 68848, ptr %11, align 4, !tbaa !1
  ret i64 4294967298
}

; Function Attrs: nofree nosync nounwind memory(readwrite, inaccessiblemem: none)
define range(i64 2, 4294967299) i64 @tb_10f96(ptr initializes((8, 16), (64, 72), (144, 152), (512, 520)) %0, i64 %1) local_unnamed_addr #1 {
entry:
  %2 = getelementptr i8, ptr %0, i64 64
  %3 = getelementptr i8, ptr %0, i64 72
  %4 = load i64, ptr %3, align 4, !tbaa !1
  %5 = getelementptr i8, ptr %0, i64 80
  %6 = load i64, ptr %5, align 4, !tbaa !1
  %7 = getelementptr i8, ptr %0, i64 144
  %8 = getelementptr i8, ptr %0, i64 512
  %9 = add i64 %6, %1
  %10 = inttoptr i64 %9 to ptr
  %11 = load i64, ptr %10, align 4, !tbaa !4
  %12 = icmp eq i64 %11, 0
  store i64 %11, ptr %2, align 4, !tbaa !1
  store i64 %6, ptr %7, align 4, !tbaa !1
  br i1 %12, label %L0, label %fall

L0:                                               ; preds = %entry
  %13 = getelementptr i8, ptr %0, i64 8
  %14 = getelementptr i8, ptr %0, i64 16
  %15 = load i64, ptr %14, align 4, !tbaa !1
  %16 = getelementptr i8, ptr %0, i64 152
  %17 = add i64 %15, %1
  %18 = add i64 %17, 56
  %19 = inttoptr i64 %18 to ptr
  %20 = load i64, ptr %19, align 4, !tbaa !4
  %21 = add i64 %17, 48
  %22 = inttoptr i64 %21 to ptr
  %23 = load i64, ptr %22, align 4, !tbaa !4
  %24 = add i64 %17, 32
  %25 = inttoptr i64 %24 to ptr
  %26 = load i64, ptr %25, align 4, !tbaa !4
  %27 = add i64 %17, 24
  %28 = inttoptr i64 %27 to ptr
  %29 = load i64, ptr %28, align 4, !tbaa !4
  %30 = add i64 %17, 40
  %31 = inttoptr i64 %30 to ptr
  %32 = load i64, ptr %31, align 4, !tbaa !4
  %33 = add i64 %15, 64
  %34 = and i64 %20, -2
  store i64 %20, ptr %13, align 4, !tbaa !1
  store i64 %33, ptr %14, align 4, !tbaa !1
  store i64 %23, ptr %2, align 4, !tbaa !1
  store i64 %32, ptr %3, align 4, !tbaa !1
  store i64 %4, ptr %5, align 4, !tbaa !1
  store i64 %26, ptr %7, align 4, !tbaa !1
  store i64 %29, ptr %16, align 4, !tbaa !1
  store i64 %34, ptr %8, align 4, !tbaa !1
  %35 = musttail call range(i64 2, 4294967299) i64 @aot_dispatch(ptr nonnull %0, i64 %1)
  ret i64 %35

fall:                                             ; preds = %entry
  %36 = getelementptr i8, ptr %0, i64 120
  %37 = getelementptr i8, ptr %0, i64 88
  %38 = getelementptr i8, ptr %0, i64 8
  %39 = add i64 %9, 8
  %40 = inttoptr i64 %39 to ptr
  %41 = load i64, ptr %40, align 4, !tbaa !4
  %42 = add i64 %41, %1
  %43 = inttoptr i64 %42 to ptr
  %44 = load i16, ptr %43, align 2, !tbaa !4
  %45 = sext i16 %44 to i64
  store i64 69546, ptr %38, align 4, !tbaa !1
  store i64 %45, ptr %5, align 4, !tbaa !1
  store i64 %4, ptr %37, align 4, !tbaa !1
  store i64 %41, ptr %36, align 4, !tbaa !1
  store i64 74642, ptr %8, align 4, !tbaa !1
  %46 = musttail call range(i64 2, 4294967299) i64 @tb_12392(ptr nonnull %0, i64 %1)
  ret i64 %46
}

; Function Attrs: nofree nosync nounwind memory(readwrite, inaccessiblemem: none)
define range(i64 2, 4294967299) i64 @tb_10faa(ptr initializes((8, 16), (72, 80), (512, 520)) %0, i64 %1) local_unnamed_addr #1 {
entry:
  %2 = getelementptr i8, ptr %0, i64 64
  %3 = load i64, ptr %2, align 4, !tbaa !1
  %4 = getelementptr i8, ptr %0, i64 72
  %5 = getelementptr i8, ptr %0, i64 80
  %6 = load i64, ptr %5, align 4, !tbaa !1
  %7 = getelementptr i8, ptr %0, i64 144
  %8 = load i64, ptr %7, align 4, !tbaa !1
  %9 = getelementptr i8, ptr %0, i64 512
  %10 = add i64 %3, %1
  %11 = inttoptr i64 %10 to ptr
  %12 = load i64, ptr %11, align 4, !tbaa !4
  %.not = icmp eq i64 %12, 0
  store i64 %12, ptr %2, align 4, !tbaa !1
  br i1 %.not, label %fall, label %L0

L0:                                               ; preds = %entry
  %13 = getelementptr i8, ptr %0, i64 120
  %14 = getelementptr i8, ptr %0, i64 88
  %15 = getelementptr i8, ptr %0, i64 8
  %16 = add i64 %1, 8
  %17 = add i64 %16, %8
  %18 = inttoptr i64 %17 to ptr
  %19 = load i64, ptr %18, align 4, !tbaa !4
  %20 = add i64 %19, %1
  %21 = inttoptr i64 %20 to ptr
  %22 = load i16, ptr %21, align 2, !tbaa !4
  %23 = sext i16 %22 to i64
  store i64 69546, ptr %15, align 4, !tbaa !1
  store i64 %6, ptr %4, align 4, !tbaa !1
  store i64 %23, ptr %5, align 4, !tbaa !1
  store i64 %6, ptr %14, align 4, !tbaa !1
  store i64 %19, ptr %13, align 4, !tbaa !1
  store i64 74642, ptr %9, align 4, !tbaa !1
  %24 = musttail call range(i64 2, 4294967299) i64 @tb_12392(ptr nonnull %0, i64 %1)
  ret i64 %24

fall:                                             ; preds = %entry
  %25 = getelementptr i8, ptr %0, i64 8
  %26 = getelementptr i8, ptr %0, i64 16
  %27 = load i64, ptr %26, align 4, !tbaa !1
  %28 = getelementptr i8, ptr %0, i64 152
  %29 = add i64 %27, %1
  %30 = add i64 %29, 56
  %31 = inttoptr i64 %30 to ptr
  %32 = load i64, ptr %31, align 4, !tbaa !4
  %33 = add i64 %29, 48
  %34 = inttoptr i64 %33 to ptr
  %35 = load i64, ptr %34, align 4, !tbaa !4
  %36 = add i64 %29, 32
  %37 = inttoptr i64 %36 to ptr
  %38 = load i64, ptr %37, align 4, !tbaa !4
  %39 = add i64 %29, 24
  %40 = inttoptr i64 %39 to ptr
  %41 = load i64, ptr %40, align 4, !tbaa !4
  %42 = add i64 %29, 40
  %43 = inttoptr i64 %42 to ptr
  %44 = load i64, ptr %43, align 4, !tbaa !4
  %45 = add i64 %27, 64
  %46 = and i64 %32, -2
  store i64 %32, ptr %25, align 4, !tbaa !1
  store i64 %45, ptr %26, align 4, !tbaa !1
  store i64 %35, ptr %2, align 4, !tbaa !1
  store i64 %44, ptr %4, align 4, !tbaa !1
  store i64 %38, ptr %7, align 4, !tbaa !1
  store i64 %41, ptr %28, align 4, !tbaa !1
  store i64 %46, ptr %9, align 4, !tbaa !1
  %47 = musttail call range(i64 2, 4294967299) i64 @aot_dispatch(ptr nonnull %0, i64 %1)
  ret i64 %47
}

; Function Attrs: nofree nosync nounwind memory(readwrite, inaccessiblemem: none)
define range(i64 2, 4294967299) i64 @tb_1116c(ptr initializes((8, 16), (48, 56), (88, 144), (224, 248), (512, 520)) %0, i64 %1) local_unnamed_addr #1 {
entry:
  %2 = getelementptr i8, ptr %0, i64 8
  %3 = getelementptr i8, ptr %0, i64 64
  %4 = load i64, ptr %3, align 4, !tbaa !1
  %5 = getelementptr i8, ptr %0, i64 88
  %6 = getelementptr i8, ptr %0, i64 512
  %7 = add i64 %1, 96
  %8 = add i64 %7, %4
  %9 = inttoptr i64 %8 to ptr
  %10 = load i16, ptr %9, align 2, !tbaa !4
  %11 = zext i16 %10 to i64
  store i64 70004, ptr %2, align 4, !tbaa !1
  store i64 %11, ptr %5, align 4, !tbaa !1
  store i64 73184, ptr %6, align 4, !tbaa !1
  %12 = musttail call i64 @tb_11de0(ptr %0, i64 %1)
  ret i64 %12
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(readwrite, inaccessiblemem: none)
define noundef i64 @tb_11174(ptr captures(none) initializes((8, 16), (48, 56), (88, 96), (512, 520)) %0, i64 %1) local_unnamed_addr #4 {
entry:
  %2 = getelementptr i8, ptr %0, i64 8
  %3 = getelementptr i8, ptr %0, i64 16
  %4 = load i64, ptr %3, align 4, !tbaa !1
  %5 = getelementptr i8, ptr %0, i64 48
  %6 = getelementptr i8, ptr %0, i64 64
  %7 = load i64, ptr %6, align 4, !tbaa !1
  %8 = getelementptr i8, ptr %0, i64 72
  %9 = load i64, ptr %8, align 4, !tbaa !1
  %10 = getelementptr i8, ptr %0, i64 80
  %11 = load i64, ptr %10, align 4, !tbaa !1
  %12 = getelementptr i8, ptr %0, i64 88
  %13 = getelementptr i8, ptr %0, i64 144
  %14 = load i64, ptr %13, align 4, !tbaa !1
  %15 = getelementptr i8, ptr %0, i64 152
  %16 = load i64, ptr %15, align 4, !tbaa !1
  %17 = getelementptr i8, ptr %0, i64 512
  %18 = add i64 %7, %1
  %19 = add i64 %18, 96
  %20 = inttoptr i64 %19 to ptr
  %21 = trunc i64 %11 to i16
  store i16 %21, ptr %20, align 2, !tbaa !4
  store i64 70016, ptr %2, align 4, !tbaa !1
  store i64 %7, ptr %10, align 4, !tbaa !1
  store i64 -1, ptr %12, align 4, !tbaa !1
  %22 = add i64 %18, 4
  %23 = inttoptr i64 %22 to ptr
  %24 = load i16, ptr %23, align 2, !tbaa !4
  %25 = sext i16 %24 to i64
  %26 = add i64 %4, -64
  %27 = add i64 %4, %1
  %28 = add i64 %27, -16
  %29 = inttoptr i64 %28 to ptr
  store i64 %7, ptr %29, align 4, !tbaa !4
  %30 = add i64 %27, -8
  %31 = inttoptr i64 %30 to ptr
  store i64 70016, ptr %31, align 4, !tbaa !4
  %32 = add i64 %27, -24
  %33 = inttoptr i64 %32 to ptr
  store i64 %9, ptr %33, align 4, !tbaa !4
  %34 = add i64 %27, -32
  %35 = inttoptr i64 %34 to ptr
  store i64 %14, ptr %35, align 4, !tbaa !4
  %36 = add i64 %27, -40
  %37 = inttoptr i64 %36 to ptr
  store i64 %16, ptr %37, align 4, !tbaa !4
  %38 = add i64 %18, 56
  %39 = inttoptr i64 %38 to ptr
  %40 = load i64, ptr %39, align 4, !tbaa !4
  %41 = icmp slt i16 %24, 1
  store i64 %26, ptr %3, align 4, !tbaa !1
  store i64 %25, ptr %5, align 4, !tbaa !1
  store i64 %40, ptr %6, align 4, !tbaa !1
  %..i = select i1 %41, i64 69622, i64 69136
  store i64 %..i, ptr %17, align 4, !tbaa !1
  ret i64 4294967298
}

; Function Attrs: nofree nosync nounwind memory(readwrite, inaccessiblemem: none)
define range(i64 2, 4294967299) i64 @tb_11180(ptr initializes((8, 16), (48, 56), (88, 144), (224, 248), (512, 520)) %0, i64 %1) local_unnamed_addr #1 {
entry:
  %2 = getelementptr i8, ptr %0, i64 8
  %3 = getelementptr i8, ptr %0, i64 64
  %4 = load i64, ptr %3, align 4, !tbaa !1
  %5 = getelementptr i8, ptr %0, i64 88
  %6 = getelementptr i8, ptr %0, i64 512
  %7 = add i64 %1, 96
  %8 = add i64 %7, %4
  %9 = inttoptr i64 %8 to ptr
  %10 = load i16, ptr %9, align 2, !tbaa !4
  %11 = zext i16 %10 to i64
  store i64 70024, ptr %2, align 4, !tbaa !1
  store i64 %11, ptr %5, align 4, !tbaa !1
  store i64 73184, ptr %6, align 4, !tbaa !1
  %12 = musttail call i64 @tb_11de0(ptr %0, i64 %1)
  ret i64 %12
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(write, argmem: readwrite, inaccessiblemem: none)
define noundef i64 @tb_11188(ptr captures(none) initializes((512, 520)) %0, i64 %1) local_unnamed_addr #2 {
entry:
  %2 = getelementptr i8, ptr %0, i64 64
  %3 = load i64, ptr %2, align 4, !tbaa !1
  %4 = getelementptr i8, ptr %0, i64 72
  %5 = load i64, ptr %4, align 4, !tbaa !1
  %6 = getelementptr i8, ptr %0, i64 80
  %7 = load i64, ptr %6, align 4, !tbaa !1
  %8 = getelementptr i8, ptr %0, i64 144
  %9 = load i64, ptr %8, align 4, !tbaa !1
  %10 = add i64 %3, %1
  %11 = add i64 %10, 96
  %12 = inttoptr i64 %11 to ptr
  %13 = trunc i64 %7 to i16
  store i16 %13, ptr %12, align 2, !tbaa !4
  %.not = icmp eq i64 %5, 0
  br i1 %.not, label %fall, label %L0

common.ret:                                       ; preds = %fall, %L0
  %.sink112 = phi i64 [ 1, %fall ], [ %15, %L0 ]
  %.not.i109 = icmp eq i64 %9, %.sink112
  %..i110 = select i1 %.not.i109, i64 70040, i64 69988
  %14 = getelementptr i8, ptr %0, i64 512
  store i64 %.sink112, ptr %4, align 4, !tbaa !1
  store i64 %..i110, ptr %14, align 4, !tbaa !1
  ret i64 4294967298

L0:                                               ; preds = %entry
  %15 = add i64 %5, 1
  br label %common.ret

fall:                                             ; preds = %entry
  %16 = add i64 %10, 98
  %17 = inttoptr i64 %16 to ptr
  store i16 %13, ptr %17, align 2, !tbaa !4
  br label %common.ret
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(readwrite, inaccessiblemem: none)
define noundef i64 @tb_12e32(ptr captures(none) initializes((48, 56), (224, 232), (512, 520)) %0, i64 %1) local_unnamed_addr #4 {
entry:
  %2 = getelementptr i8, ptr %0, i64 8
  %3 = load i64, ptr %2, align 4, !tbaa !1
  %4 = getelementptr i8, ptr %0, i64 16
  %5 = load i64, ptr %4, align 4, !tbaa !1
  %6 = getelementptr i8, ptr %0, i64 48
  %7 = getelementptr i8, ptr %0, i64 72
  %8 = load i64, ptr %7, align 4, !tbaa !1
  %9 = getelementptr i8, ptr %0, i64 80
  %10 = load i64, ptr %9, align 4, !tbaa !1
  %11 = getelementptr i8, ptr %0, i64 88
  %12 = load i64, ptr %11, align 4, !tbaa !1
  %13 = getelementptr i8, ptr %0, i64 96
  %14 = load i64, ptr %13, align 4, !tbaa !1
  %15 = getelementptr i8, ptr %0, i64 104
  %16 = load i64, ptr %15, align 4, !tbaa !1
  %17 = getelementptr i8, ptr %0, i64 112
  %18 = load i64, ptr %17, align 4, !tbaa !1
  %19 = getelementptr i8, ptr %0, i64 120
  %20 = load i64, ptr %19, align 4, !tbaa !1
  %21 = getelementptr i8, ptr %0, i64 128
  %22 = load i64, ptr %21, align 4, !tbaa !1
  %23 = getelementptr i8, ptr %0, i64 136
  %24 = load i64, ptr %23, align 4, !tbaa !1
  %25 = getelementptr i8, ptr %0, i64 224
  %26 = getelementptr i8, ptr %0, i64 512
  %27 = add i64 %1, 510984
  %28 = inttoptr i64 %27 to ptr
  %29 = load i64, ptr %28, align 4, !tbaa !4
  %30 = add i64 %5, -96
  %31 = add i64 %29, %1
  %32 = inttoptr i64 %31 to ptr
  %33 = load i64, ptr %32, align 4, !tbaa !4
  %34 = add i64 %5, -56
  %35 = add i64 %34, %1
  %36 = inttoptr i64 %35 to ptr
  store i64 %12, ptr %36, align 4, !tbaa !4
  %37 = add i64 %5, %1
  %38 = add i64 %37, -48
  %39 = inttoptr i64 %38 to ptr
  store i64 %14, ptr %39, align 4, !tbaa !4
  %40 = add i64 %37, -40
  %41 = inttoptr i64 %40 to ptr
  store i64 %16, ptr %41, align 4, !tbaa !4
  %42 = add i64 %37, -72
  %43 = inttoptr i64 %42 to ptr
  store i64 %3, ptr %43, align 4, !tbaa !4
  %44 = add i64 %37, -32
  %45 = inttoptr i64 %44 to ptr
  store i64 %18, ptr %45, align 4, !tbaa !4
  %46 = add i64 %37, -24
  %47 = inttoptr i64 %46 to ptr
  store i64 %20, ptr %47, align 4, !tbaa !4
  %48 = add i64 %37, -16
  %49 = inttoptr i64 %48 to ptr
  store i64 %22, ptr %49, align 4, !tbaa !4
  %50 = add i64 %37, -8
  %51 = inttoptr i64 %50 to ptr
  store i64 %24, ptr %51, align 4, !tbaa !4
  %52 = add i64 %37, -88
  %53 = inttoptr i64 %52 to ptr
  store i64 %34, ptr %53, align 4, !tbaa !4
  store i64 77410, ptr %2, align 4, !tbaa !1
  store i64 %34, ptr %6, align 4, !tbaa !1
  store i64 %33, ptr %9, align 4, !tbaa !1
  store i64 %10, ptr %11, align 4, !tbaa !1
  store i64 %34, ptr %13, align 4, !tbaa !1
  store i64 0, ptr %15, align 4, !tbaa !1
  store i64 %10, ptr %25, align 4, !tbaa !1
  %54 = add i64 %1, 192
  %55 = add i64 %54, %33
  %56 = inttoptr i64 %55 to ptr
  %57 = load i32, ptr %56, align 4, !tbaa !4
  %58 = sext i32 %57 to i64
  %59 = add i64 %5, -368
  %60 = add i64 %30, %1
  %61 = add i64 %60, -8
  %62 = inttoptr i64 %61 to ptr
  store i64 77410, ptr %62, align 4, !tbaa !4
  %63 = add i64 %60, -24
  %64 = inttoptr i64 %63 to ptr
  store i64 %8, ptr %64, align 4, !tbaa !4
  %.not.i = icmp eq i32 %57, 0
  store i64 %59, ptr %4, align 4, !tbaa !1
  store i64 %58, ptr %19, align 4, !tbaa !1
  %..i = select i1 %.not.i, i64 89034, i64 89160
  store i64 %..i, ptr %26, align 4, !tbaa !1
  ret i64 4294967298
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(readwrite, inaccessiblemem: none)
define noundef i64 @tb_15cfc(ptr captures(none) initializes((8, 16), (80, 112), (120, 128), (512, 520)) %0, i64 %1) local_unnamed_addr #4 {
entry:
  %2 = getelementptr i8, ptr %0, i64 8
  %3 = getelementptr i8, ptr %0, i64 16
  %4 = load i64, ptr %3, align 4, !tbaa !1
  %5 = getelementptr i8, ptr %0, i64 32
  %6 = load i64, ptr %5, align 4, !tbaa !1
  %7 = getelementptr i8, ptr %0, i64 64
  %8 = load i64, ptr %7, align 4, !tbaa !1
  %9 = getelementptr i8, ptr %0, i64 72
  %10 = load i64, ptr %9, align 4, !tbaa !1
  %11 = getelementptr i8, ptr %0, i64 80
  %12 = getelementptr i8, ptr %0, i64 88
  %13 = getelementptr i8, ptr %0, i64 96
  %14 = getelementptr i8, ptr %0, i64 104
  %15 = getelementptr i8, ptr %0, i64 120
  %16 = getelementptr i8, ptr %0, i64 144
  %17 = load i64, ptr %16, align 4, !tbaa !1
  %18 = getelementptr i8, ptr %0, i64 152
  %19 = load i64, ptr %18, align 4, !tbaa !1
  %20 = getelementptr i8, ptr %0, i64 160
  %21 = load i64, ptr %20, align 4, !tbaa !1
  %22 = getelementptr i8, ptr %0, i64 208
  %23 = load i64, ptr %22, align 4, !tbaa !1
  %24 = getelementptr i8, ptr %0, i64 512
  store i64 %19, ptr %13, align 4, !tbaa !1
  store i64 %21, ptr %14, align 4, !tbaa !1
  %25 = add i64 %1, 511440
  %26 = inttoptr i64 %25 to ptr
  %27 = load i64, ptr %26, align 4, !tbaa !4
  %28 = add i64 %4, -1360
  %29 = add i64 %4, %1
  %30 = add i64 %29, -16
  %31 = inttoptr i64 %30 to ptr
  store i64 %8, ptr %31, align 4, !tbaa !4
  %32 = add i64 %29, -1208
  %33 = inttoptr i64 %32 to ptr
  store i64 %27, ptr %33, align 4, !tbaa !4
  %34 = add i64 %6, %1
  %35 = add i64 %34, %27
  %36 = inttoptr i64 %35 to ptr
  %37 = load i32, ptr %36, align 4, !tbaa !4
  %38 = sext i32 %37 to i64
  %39 = add i64 %29, -32
  %40 = inttoptr i64 %39 to ptr
  store i64 %17, ptr %40, align 4, !tbaa !4
  %41 = add i64 %29, -1224
  %42 = inttoptr i64 %41 to ptr
  store i64 %17, ptr %42, align 4, !tbaa !4
  %43 = add i64 %29, -1240
  %44 = inttoptr i64 %43 to ptr
  store i64 %38, ptr %44, align 4, !tbaa !4
  %45 = add i64 %29, -8
  %46 = inttoptr i64 %45 to ptr
  store i64 89352, ptr %46, align 4, !tbaa !4
  %47 = add i64 %29, -24
  %48 = inttoptr i64 %47 to ptr
  store i64 %10, ptr %48, align 4, !tbaa !4
  %49 = add i64 %29, -40
  %50 = inttoptr i64 %49 to ptr
  store i64 %19, ptr %50, align 4, !tbaa !4
  %51 = add i64 %29, -96
  %52 = inttoptr i64 %51 to ptr
  store i64 %23, ptr %52, align 4, !tbaa !4
  %53 = add i64 %29, -1168
  %54 = inttoptr i64 %53 to ptr
  store i64 %19, ptr %54, align 4, !tbaa !4
  %55 = add i64 %29, -1232
  %56 = inttoptr i64 %55 to ptr
  store i64 %21, ptr %56, align 4, !tbaa !4
  store i64 83278, ptr %2, align 4, !tbaa !1
  store i64 %28, ptr %3, align 4, !tbaa !1
  store i64 %19, ptr %9, align 4, !tbaa !1
  store i64 %17, ptr %11, align 4, !tbaa !1
  store i64 37, ptr %12, align 4, !tbaa !1
  store i64 %38, ptr %15, align 4, !tbaa !1
  store i64 %21, ptr %18, align 4, !tbaa !1
  store i64 130504, ptr %24, align 4, !tbaa !1
  ret i64 4294967298
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(readwrite, inaccessiblemem: none)
define noundef i64 @tb_1454e(ptr captures(none) initializes((8, 16), (88, 104), (120, 128), (208, 216), (512, 520)) %0, i64 %1) local_unnamed_addr #4 {
entry:
  %2 = getelementptr i8, ptr %0, i64 8
  %3 = getelementptr i8, ptr %0, i64 16
  %4 = load i64, ptr %3, align 4, !tbaa !1
  %5 = getelementptr i8, ptr %0, i64 64
  %6 = load i64, ptr %5, align 4, !tbaa !1
  %7 = getelementptr i8, ptr %0, i64 80
  %8 = load i64, ptr %7, align 4, !tbaa !1
  %9 = getelementptr i8, ptr %0, i64 88
  %10 = getelementptr i8, ptr %0, i64 96
  %11 = getelementptr i8, ptr %0, i64 120
  %12 = getelementptr i8, ptr %0, i64 144
  %13 = load i64, ptr %12, align 4, !tbaa !1
  %14 = getelementptr i8, ptr %0, i64 208
  %15 = getelementptr i8, ptr %0, i64 512
  %16 = add i64 %1, 144
  %17 = add i64 %16, %4
  %18 = inttoptr i64 %17 to ptr
  store i64 %8, ptr %18, align 4, !tbaa !4
  %19 = sub i64 %8, %13
  store i64 83294, ptr %2, align 4, !tbaa !1
  store i64 %6, ptr %7, align 4, !tbaa !1
  store i64 %13, ptr %9, align 4, !tbaa !1
  store i64 %19, ptr %10, align 4, !tbaa !1
  store i64 %8, ptr %14, align 4, !tbaa !1
  %20 = add i64 %1, 32
  %21 = add i64 %20, %6
  %22 = inttoptr i64 %21 to ptr
  %23 = load i32, ptr %22, align 4, !tbaa !4
  %24 = sext i32 %23 to i64
  %25 = icmp eq i32 %23, 0
  store i64 %24, ptr %11, align 4, !tbaa !1
  %..i = select i1 %25, i64 199190, i64 199056
  store i64 %..i, ptr %15, align 4, !tbaa !1
  ret i64 4294967298
}

; Function Attrs: nofree nosync nounwind memory(readwrite, inaccessiblemem: none)
define range(i64 2, 4294967299) i64 @tb_20934(ptr initializes((80, 144), (512, 520)) %0, i64 %1) local_unnamed_addr #1 {
entry:
  %2 = getelementptr i8, ptr %0, i64 16
  %3 = load i64, ptr %2, align 4, !tbaa !1
  %4 = getelementptr i8, ptr %0, i64 80
  %5 = getelementptr i8, ptr %0, i64 88
  %6 = getelementptr i8, ptr %0, i64 96
  %7 = getelementptr i8, ptr %0, i64 104
  %8 = getelementptr i8, ptr %0, i64 112
  %9 = getelementptr i8, ptr %0, i64 120
  %10 = getelementptr i8, ptr %0, i64 128
  %11 = getelementptr i8, ptr %0, i64 136
  %12 = getelementptr i8, ptr %0, i64 512
  %13 = add i64 %3, %1
  %14 = add i64 %13, 24
  %15 = inttoptr i64 %14 to ptr
  %16 = load i64, ptr %15, align 4, !tbaa !4
  %17 = add i64 %13, 16
  %18 = inttoptr i64 %17 to ptr
  %19 = load i64, ptr %18, align 4, !tbaa !4
  %20 = add i64 %13, 8
  %21 = inttoptr i64 %20 to ptr
  %22 = load i64, ptr %21, align 4, !tbaa !4
  %23 = inttoptr i64 %13 to ptr
  %24 = load i64, ptr %23, align 4, !tbaa !4
  %25 = and i64 %19, -8
  %26 = add i64 %22, %25
  %27 = add i64 %24, %25
  %28 = and i64 %19, 7
  %29 = xor i64 %26, -1
  %30 = add i64 %27, %29
  %31 = add i64 %26, %28
  %32 = icmp eq i64 %28, 0
  store i64 %31, ptr %5, align 4, !tbaa !1
  store i64 %26, ptr %7, align 4, !tbaa !1
  store i64 %30, ptr %8, align 4, !tbaa !1
  store i64 %26, ptr %9, align 4, !tbaa !1
  store i64 %19, ptr %10, align 4, !tbaa !1
  store i64 %16, ptr %11, align 4, !tbaa !1
  br i1 %32, label %L0, label %fall

L0:                                               ; preds = %entry
  %33 = getelementptr i8, ptr %0, i64 8
  %34 = add i64 %13, 40
  %35 = inttoptr i64 %34 to ptr
  %36 = load i64, ptr %35, align 4, !tbaa !4
  %37 = add i64 %3, 48
  %38 = and i64 %36, -2
  store i64 %36, ptr %33, align 4, !tbaa !1
  store i64 %37, ptr %2, align 4, !tbaa !1
  store i64 %16, ptr %4, align 4, !tbaa !1
  store i64 0, ptr %6, align 4, !tbaa !1
  store i64 %38, ptr %12, align 4, !tbaa !1
  %39 = musttail call range(i64 2, 4294967299) i64 @aot_dispatch(ptr nonnull %0, i64 %1)
  ret i64 %39

fall:                                             ; preds = %entry
  %40 = add i64 %26, %1
  %41 = inttoptr i64 %40 to ptr
  %42 = load i8, ptr %41, align 1, !tbaa !4
  %43 = add i64 %27, %1
  %44 = inttoptr i64 %43 to ptr
  store i8 %42, ptr %44, align 1, !tbaa !4
  %.not129.i = icmp eq i64 %28, 1
  br i1 %.not129.i, label %fall.i, label %L0.preheader.i

L0.preheader.i:                                   ; preds = %fall
  %45 = add i64 %26, 1
  %invariant.op.i = add i64 %30, %1
  br label %L0.i

L0.i:                                             ; preds = %L0.i, %L0.preheader.i
  %46 = phi i64 [ %50, %L0.i ], [ %45, %L0.preheader.i ]
  %47 = add i64 %46, %1
  %48 = inttoptr i64 %47 to ptr
  %49 = load i8, ptr %48, align 1, !tbaa !4
  %50 = add i64 %46, 1
  %.reass.i = add i64 %invariant.op.i, %50
  %51 = inttoptr i64 %.reass.i to ptr
  store i8 %49, ptr %51, align 1, !tbaa !4
  %.not.i = icmp eq i64 %31, %50
  br i1 %.not.i, label %fall.i.loopexit, label %L0.i

fall.i.loopexit:                                  ; preds = %L0.i
  %52 = add i64 %30, %31
  br label %fall.i

fall.i:                                           ; preds = %fall.i.loopexit, %fall
  %.lcssa128.i = phi i64 [ %27, %fall ], [ %52, %fall.i.loopexit ]
  %.lcssa127.in.i = phi i8 [ %42, %fall ], [ %49, %fall.i.loopexit ]
  %.lcssa127.i = zext i8 %.lcssa127.in.i to i64
  %53 = getelementptr i8, ptr %0, i64 8
  %54 = add i64 %13, 40
  %55 = inttoptr i64 %54 to ptr
  %56 = load i64, ptr %55, align 4, !tbaa !4
  %57 = add i64 %3, 48
  %58 = and i64 %56, -2
  store i64 %56, ptr %53, align 4, !tbaa !1
  store i64 %57, ptr %2, align 4, !tbaa !1
  store i64 %16, ptr %4, align 4, !tbaa !1
  store i64 %.lcssa127.i, ptr %6, align 4, !tbaa !1
  store i64 %.lcssa128.i, ptr %7, align 4, !tbaa !1
  store i64 %31, ptr %9, align 4, !tbaa !1
  store i64 %58, ptr %12, align 4, !tbaa !1
  %59 = musttail call range(i64 2, 4294967299) i64 @aot_dispatch(ptr nonnull %0, i64 %1)
  ret i64 %59
}

; Function Attrs: nofree nosync nounwind memory(readwrite, inaccessiblemem: none)
define range(i64 2, 4294967299) i64 @tb_309bc(ptr initializes((80, 88), (512, 520)) %0, i64 %1) local_unnamed_addr #1 {
entry:
  %2 = getelementptr i8, ptr %0, i64 64
  %3 = load i64, ptr %2, align 4, !tbaa !1
  %4 = getelementptr i8, ptr %0, i64 72
  %5 = load i64, ptr %4, align 4, !tbaa !1
  %6 = getelementptr i8, ptr %0, i64 80
  %7 = getelementptr i8, ptr %0, i64 144
  %8 = load i64, ptr %7, align 4, !tbaa !1
  %9 = getelementptr i8, ptr %0, i64 152
  %10 = load i64, ptr %9, align 4, !tbaa !1
  %11 = getelementptr i8, ptr %0, i64 512
  %12 = add i64 %8, %1
  %13 = add i64 %12, 8
  %14 = inttoptr i64 %13 to ptr
  %15 = load i64, ptr %14, align 4, !tbaa !4
  %16 = sub i64 %5, %3
  %17 = add i64 %10, %3
  %18 = add i64 %15, %3
  store i64 %18, ptr %14, align 4, !tbaa !4
  %19 = icmp eq i64 %5, %3
  store i64 %16, ptr %4, align 4, !tbaa !1
  store i64 %18, ptr %6, align 4, !tbaa !1
  store i64 %17, ptr %9, align 4, !tbaa !1
  br i1 %19, label %L0, label %fall

L0:                                               ; preds = %entry
  %20 = getelementptr i8, ptr %0, i64 8
  %21 = getelementptr i8, ptr %0, i64 16
  %22 = load i64, ptr %21, align 4, !tbaa !1
  %23 = add i64 %22, %1
  %24 = add i64 %23, 32
  %25 = inttoptr i64 %24 to ptr
  %26 = load i64, ptr %25, align 4, !tbaa !4
  %27 = add i64 %23, 16
  %28 = inttoptr i64 %27 to ptr
  %29 = load i64, ptr %28, align 4, !tbaa !4
  %30 = add i64 %23, 8
  %31 = inttoptr i64 %30 to ptr
  %32 = load i64, ptr %31, align 4, !tbaa !4
  %33 = add i64 %23, 40
  %34 = inttoptr i64 %33 to ptr
  %35 = load i64, ptr %34, align 4, !tbaa !4
  %36 = add i64 %23, 24
  %37 = inttoptr i64 %36 to ptr
  %38 = load i64, ptr %37, align 4, !tbaa !4
  %39 = add i64 %22, 48
  %40 = and i64 %35, -2
  store i64 %35, ptr %20, align 4, !tbaa !1
  store i64 %39, ptr %21, align 4, !tbaa !1
  store i64 %26, ptr %2, align 4, !tbaa !1
  store i64 %38, ptr %4, align 4, !tbaa !1
  store i64 %29, ptr %7, align 4, !tbaa !1
  store i64 %32, ptr %9, align 4, !tbaa !1
  store i64 %40, ptr %11, align 4, !tbaa !1
  %41 = musttail call range(i64 2, 4294967299) i64 @aot_dispatch(ptr nonnull %0, i64 %1)
  ret i64 %41

fall:                                             ; preds = %entry
  %42 = add i64 %12, 16
  %43 = inttoptr i64 %42 to ptr
  %44 = load i64, ptr %43, align 4, !tbaa !4
  %.not.i = icmp eq i64 %44, %18
  store i64 %44, ptr %2, align 4, !tbaa !1
  %..i = select i1 %.not.i, i64 199124, i64 199080
  store i64 %..i, ptr %11, align 4, !tbaa !1
  ret i64 4294967298
}

; Function Attrs: nofree nosync nounwind memory(readwrite, inaccessiblemem: none)
define range(i64 2, 4294967299) i64 @tb_1455e(ptr initializes((120, 128), (512, 520)) %0, i64 %1) local_unnamed_addr #1 {
entry:
  %2 = getelementptr i8, ptr %0, i64 64
  %3 = load i64, ptr %2, align 4, !tbaa !1
  %4 = getelementptr i8, ptr %0, i64 120
  %5 = getelementptr i8, ptr %0, i64 512
  %6 = add i64 %1, 32
  %7 = add i64 %6, %3
  %8 = inttoptr i64 %7 to ptr
  %9 = load i32, ptr %8, align 4, !tbaa !4
  %10 = sext i32 %9 to i64
  %11 = icmp eq i32 %9, 0
  store i64 %10, ptr %4, align 4, !tbaa !1
  br i1 %11, label %L0, label %fall

L0:                                               ; preds = %entry
  %12 = getelementptr i8, ptr %0, i64 8
  %13 = getelementptr i8, ptr %0, i64 16
  %14 = load i64, ptr %13, align 4, !tbaa !1
  %15 = getelementptr i8, ptr %0, i64 72
  %16 = getelementptr i8, ptr %0, i64 144
  %17 = getelementptr i8, ptr %0, i64 152
  %18 = getelementptr i8, ptr %0, i64 208
  %19 = add i64 %14, %1
  %20 = add i64 %19, 1352
  %21 = inttoptr i64 %20 to ptr
  %22 = load i64, ptr %21, align 4, !tbaa !4
  %23 = add i64 %19, 1344
  %24 = inttoptr i64 %23 to ptr
  %25 = load i64, ptr %24, align 4, !tbaa !4
  %26 = add i64 %19, 1336
  %27 = inttoptr i64 %26 to ptr
  %28 = load i64, ptr %27, align 4, !tbaa !4
  %29 = add i64 %19, 1328
  %30 = inttoptr i64 %29 to ptr
  %31 = load i64, ptr %30, align 4, !tbaa !4
  %32 = add i64 %19, 1320
  %33 = inttoptr i64 %32 to ptr
  %34 = load i64, ptr %33, align 4, !tbaa !4
  %35 = add i64 %19, 1264
  %36 = inttoptr i64 %35 to ptr
  %37 = load i64, ptr %36, align 4, !tbaa !4
  %38 = add i64 %14, 1360
  %39 = and i64 %22, -2
  store i64 %22, ptr %12, align 4, !tbaa !1
  store i64 %38, ptr %13, align 4, !tbaa !1
  store i64 %25, ptr %2, align 4, !tbaa !1
  store i64 %28, ptr %15, align 4, !tbaa !1
  store i64 %31, ptr %16, align 4, !tbaa !1
  store i64 %34, ptr %17, align 4, !tbaa !1
  store i64 %37, ptr %18, align 4, !tbaa !1
  store i64 %39, ptr %5, align 4, !tbaa !1
  %40 = musttail call range(i64 2, 4294967299) i64 @aot_dispatch(ptr nonnull %0, i64 %1)
  ret i64 %40

fall:                                             ; preds = %entry
  %41 = getelementptr i8, ptr %0, i64 208
  %42 = load i64, ptr %41, align 4, !tbaa !1
  %43 = add i64 %42, %1
  %44 = inttoptr i64 %43 to ptr
  %45 = load i8, ptr %44, align 1, !tbaa !4
  %46 = zext i8 %45 to i64
  %47 = icmp eq i8 %45, 0
  store i64 %46, ptr %4, align 4, !tbaa !1
  br i1 %47, label %L0.i, label %tb_14562.exit

L0.i:                                             ; preds = %fall
  %48 = getelementptr i8, ptr %0, i64 8
  %49 = getelementptr i8, ptr %0, i64 16
  %50 = load i64, ptr %49, align 4, !tbaa !1
  %51 = getelementptr i8, ptr %0, i64 72
  %52 = getelementptr i8, ptr %0, i64 144
  %53 = getelementptr i8, ptr %0, i64 152
  %54 = add i64 %50, %1
  %55 = add i64 %54, 1352
  %56 = inttoptr i64 %55 to ptr
  %57 = load i64, ptr %56, align 4, !tbaa !4
  %58 = add i64 %54, 1344
  %59 = inttoptr i64 %58 to ptr
  %60 = load i64, ptr %59, align 4, !tbaa !4
  %61 = add i64 %54, 1336
  %62 = inttoptr i64 %61 to ptr
  %63 = load i64, ptr %62, align 4, !tbaa !4
  %64 = add i64 %54, 1328
  %65 = inttoptr i64 %64 to ptr
  %66 = load i64, ptr %65, align 4, !tbaa !4
  %67 = add i64 %54, 1320
  %68 = inttoptr i64 %67 to ptr
  %69 = load i64, ptr %68, align 4, !tbaa !4
  %70 = add i64 %54, 1264
  %71 = inttoptr i64 %70 to ptr
  %72 = load i64, ptr %71, align 4, !tbaa !4
  %73 = add i64 %50, 1360
  %74 = and i64 %57, -2
  store i64 %57, ptr %48, align 4, !tbaa !1
  store i64 %73, ptr %49, align 4, !tbaa !1
  store i64 %60, ptr %2, align 4, !tbaa !1
  store i64 %63, ptr %51, align 4, !tbaa !1
  store i64 %66, ptr %52, align 4, !tbaa !1
  store i64 %69, ptr %53, align 4, !tbaa !1
  store i64 %72, ptr %41, align 4, !tbaa !1
  store i64 %74, ptr %5, align 4, !tbaa !1
  %75 = musttail call range(i64 2, 4294967299) i64 @aot_dispatch(ptr nonnull %0, i64 %1)
  ret i64 %75

tb_14562.exit:                                    ; preds = %fall
  store i64 83304, ptr %5, align 4, !tbaa !1
  ret i64 4294967298
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(readwrite, inaccessiblemem: none)
define noundef i64 @tb_14910(ptr captures(none) initializes((104, 120), (224, 240), (512, 520)) %0, i64 %1) local_unnamed_addr #4 {
entry:
  %2 = getelementptr i8, ptr %0, i64 16
  %3 = load i64, ptr %2, align 4, !tbaa !1
  %4 = getelementptr i8, ptr %0, i64 32
  %5 = load i64, ptr %4, align 4, !tbaa !1
  %6 = getelementptr i8, ptr %0, i64 80
  %7 = load i64, ptr %6, align 4, !tbaa !1
  %8 = getelementptr i8, ptr %0, i64 104
  %9 = getelementptr i8, ptr %0, i64 112
  %10 = getelementptr i8, ptr %0, i64 160
  %11 = load i64, ptr %10, align 4, !tbaa !1
  %12 = getelementptr i8, ptr %0, i64 200
  %13 = load i64, ptr %12, align 4, !tbaa !1
  %14 = getelementptr i8, ptr %0, i64 224
  %15 = getelementptr i8, ptr %0, i64 232
  %16 = add i64 %3, %1
  %17 = add i64 %16, 104
  %18 = inttoptr i64 %17 to ptr
  %19 = load i64, ptr %18, align 4, !tbaa !4
  %20 = add i64 %3, 1248
  %21 = and i64 %13, %11
  %22 = add i64 %16, 72
  %23 = inttoptr i64 %22 to ptr
  store i64 %7, ptr %23, align 4, !tbaa !4
  %24 = sub i64 %20, %7
  %25 = add i64 %16, 112
  %26 = inttoptr i64 %25 to ptr
  %27 = load i64, ptr %26, align 4, !tbaa !4
  %.not = icmp eq i64 %19, 0
  store i64 %24, ptr %8, align 4, !tbaa !1
  store i64 %20, ptr %9, align 4, !tbaa !1
  store i64 %21, ptr %10, align 4, !tbaa !1
  store i64 %24, ptr %12, align 4, !tbaa !1
  store i64 %19, ptr %14, align 4, !tbaa !1
  store i64 %27, ptr %15, align 4, !tbaa !1
  br i1 %.not, label %fall, label %L0

common.ret:                                       ; preds = %fall, %L0
  %storemerge = phi i64 [ 197932, %L0 ], [ %..i, %fall ]
  %28 = getelementptr i8, ptr %0, i64 512
  store i64 %storemerge, ptr %28, align 4, !tbaa !1
  ret i64 4294967298

L0:                                               ; preds = %entry
  %29 = getelementptr i8, ptr %0, i64 96
  %30 = getelementptr i8, ptr %0, i64 88
  %31 = getelementptr i8, ptr %0, i64 72
  %32 = getelementptr i8, ptr %0, i64 8
  %33 = add i64 %1, 511728
  %34 = inttoptr i64 %33 to ptr
  %35 = load i64, ptr %34, align 4, !tbaa !4
  %36 = add i64 %3, 224
  %37 = add i64 %5, %1
  %38 = add i64 %37, %35
  %39 = inttoptr i64 %38 to ptr
  %40 = load i64, ptr %39, align 4, !tbaa !4
  store i64 %24, ptr %18, align 4, !tbaa !4
  store i64 84758, ptr %32, align 4, !tbaa !1
  store i64 %35, ptr %31, align 4, !tbaa !1
  store i64 %36, ptr %6, align 4, !tbaa !1
  store i64 1, ptr %30, align 4, !tbaa !1
  store i64 %40, ptr %29, align 4, !tbaa !1
  br label %common.ret

fall:                                             ; preds = %entry
  %41 = icmp eq i64 %21, 0
  %..i = select i1 %41, i64 86538, i64 84270
  br label %common.ret
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(argmem: readwrite)
define noundef i64 @tb_14bdc(ptr captures(none) initializes((8, 16), (80, 96), (152, 160), (512, 520)) %0, i64 %1) local_unnamed_addr #3 {
entry:
  %2 = getelementptr i8, ptr %0, i64 512
  %3 = getelementptr i8, ptr %0, i64 8
  %4 = getelementptr i8, ptr %0, i64 80
  %5 = getelementptr i8, ptr %0, i64 88
  %6 = getelementptr i8, ptr %0, i64 152
  %7 = getelementptr i8, ptr %0, i64 208
  %8 = load i64, ptr %7, align 4, !tbaa !1
  %9 = add i64 %8, 1
  store i64 83790, ptr %3, align 4, !tbaa !1
  store i64 %9, ptr %4, align 4, !tbaa !1
  store i64 37, ptr %5, align 4, !tbaa !1
  store i64 %9, ptr %6, align 4, !tbaa !1
  store i64 130504, ptr %2, align 4, !tbaa !1
  ret i64 4294967298
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(read, argmem: readwrite, inaccessiblemem: none)
define noundef i64 @tb_1474e(ptr captures(none) initializes((8, 16), (88, 104), (120, 128), (208, 216), (512, 520)) %0, i64 %1) local_unnamed_addr #0 {
entry:
  %2 = getelementptr i8, ptr %0, i64 8
  %3 = getelementptr i8, ptr %0, i64 64
  %4 = load i64, ptr %3, align 4, !tbaa !1
  %5 = getelementptr i8, ptr %0, i64 80
  %6 = load i64, ptr %5, align 4, !tbaa !1
  %7 = getelementptr i8, ptr %0, i64 88
  %8 = getelementptr i8, ptr %0, i64 96
  %9 = getelementptr i8, ptr %0, i64 120
  %10 = getelementptr i8, ptr %0, i64 152
  %11 = load i64, ptr %10, align 4, !tbaa !1
  %12 = getelementptr i8, ptr %0, i64 208
  %13 = getelementptr i8, ptr %0, i64 512
  %14 = sub i64 %6, %11
  store i64 83804, ptr %2, align 4, !tbaa !1
  store i64 %4, ptr %5, align 4, !tbaa !1
  store i64 %11, ptr %7, align 4, !tbaa !1
  store i64 %14, ptr %8, align 4, !tbaa !1
  store i64 %6, ptr %12, align 4, !tbaa !1
  %15 = add i64 %1, 32
  %16 = add i64 %15, %4
  %17 = inttoptr i64 %16 to ptr
  %18 = load i32, ptr %17, align 4, !tbaa !4
  %19 = sext i32 %18 to i64
  %20 = icmp eq i32 %18, 0
  store i64 %19, ptr %9, align 4, !tbaa !1
  %..i = select i1 %20, i64 199190, i64 199056
  store i64 %..i, ptr %13, align 4, !tbaa !1
  ret i64 4294967298
}

; Function Attrs: nofree nosync nounwind memory(readwrite, inaccessiblemem: none)
define range(i64 2, 4294967299) i64 @tb_1475c(ptr initializes((120, 128), (512, 520)) %0, i64 %1) local_unnamed_addr #1 {
entry:
  %2 = getelementptr i8, ptr %0, i64 120
  %3 = getelementptr i8, ptr %0, i64 168
  %4 = load i64, ptr %3, align 4, !tbaa !1
  %5 = getelementptr i8, ptr %0, i64 208
  %6 = load i64, ptr %5, align 4, !tbaa !1
  %7 = getelementptr i8, ptr %0, i64 512
  %8 = add i64 %6, %1
  %9 = inttoptr i64 %8 to ptr
  %10 = load i8, ptr %9, align 1, !tbaa !4
  %11 = zext i8 %10 to i64
  %12 = add i64 %4, 1
  %13 = icmp eq i8 %10, 0
  store i64 %11, ptr %2, align 4, !tbaa !1
  store i64 %12, ptr %3, align 4, !tbaa !1
  br i1 %13, label %L0, label %fall

L0:                                               ; preds = %entry
  %14 = getelementptr i8, ptr %0, i64 8
  %15 = getelementptr i8, ptr %0, i64 16
  %16 = load i64, ptr %15, align 4, !tbaa !1
  %17 = getelementptr i8, ptr %0, i64 64
  %18 = getelementptr i8, ptr %0, i64 72
  %19 = getelementptr i8, ptr %0, i64 144
  %20 = getelementptr i8, ptr %0, i64 152
  %21 = getelementptr i8, ptr %0, i64 160
  %22 = getelementptr i8, ptr %0, i64 176
  %23 = getelementptr i8, ptr %0, i64 184
  %24 = getelementptr i8, ptr %0, i64 192
  %25 = getelementptr i8, ptr %0, i64 200
  %26 = getelementptr i8, ptr %0, i64 216
  %27 = add i64 %16, %1
  %28 = add i64 %27, 1312
  %29 = inttoptr i64 %28 to ptr
  %30 = load i64, ptr %29, align 4, !tbaa !4
  %31 = add i64 %27, 1304
  %32 = inttoptr i64 %31 to ptr
  %33 = load i64, ptr %32, align 4, !tbaa !4
  %34 = add i64 %27, 1296
  %35 = inttoptr i64 %34 to ptr
  %36 = load i64, ptr %35, align 4, !tbaa !4
  %37 = add i64 %27, 1288
  %38 = inttoptr i64 %37 to ptr
  %39 = load i64, ptr %38, align 4, !tbaa !4
  %40 = add i64 %27, 1280
  %41 = inttoptr i64 %40 to ptr
  %42 = load i64, ptr %41, align 4, !tbaa !4
  %43 = add i64 %27, 1272
  %44 = inttoptr i64 %43 to ptr
  %45 = load i64, ptr %44, align 4, !tbaa !4
  %46 = add i64 %27, 1256
  %47 = inttoptr i64 %46 to ptr
  %48 = load i64, ptr %47, align 4, !tbaa !4
  %49 = add i64 %27, 1352
  %50 = inttoptr i64 %49 to ptr
  %51 = load i64, ptr %50, align 4, !tbaa !4
  %52 = add i64 %27, 1344
  %53 = inttoptr i64 %52 to ptr
  %54 = load i64, ptr %53, align 4, !tbaa !4
  %55 = add i64 %27, 1336
  %56 = inttoptr i64 %55 to ptr
  %57 = load i64, ptr %56, align 4, !tbaa !4
  %58 = add i64 %27, 1328
  %59 = inttoptr i64 %58 to ptr
  %60 = load i64, ptr %59, align 4, !tbaa !4
  %61 = add i64 %27, 1320
  %62 = inttoptr i64 %61 to ptr
  %63 = load i64, ptr %62, align 4, !tbaa !4
  %64 = add i64 %27, 1264
  %65 = inttoptr i64 %64 to ptr
  %66 = load i64, ptr %65, align 4, !tbaa !4
  %67 = add i64 %16, 1360
  %68 = and i64 %51, -2
  store i64 %51, ptr %14, align 4, !tbaa !1
  store i64 %67, ptr %15, align 4, !tbaa !1
  store i64 %54, ptr %17, align 4, !tbaa !1
  store i64 %57, ptr %18, align 4, !tbaa !1
  store i64 %60, ptr %19, align 4, !tbaa !1
  store i64 %63, ptr %20, align 4, !tbaa !1
  store i64 %30, ptr %21, align 4, !tbaa !1
  store i64 %33, ptr %3, align 4, !tbaa !1
  store i64 %36, ptr %22, align 4, !tbaa !1
  store i64 %39, ptr %23, align 4, !tbaa !1
  store i64 %42, ptr %24, align 4, !tbaa !1
  store i64 %45, ptr %25, align 4, !tbaa !1
  store i64 %66, ptr %5, align 4, !tbaa !1
  store i64 %48, ptr %26, align 4, !tbaa !1
  store i64 %68, ptr %7, align 4, !tbaa !1
  %69 = musttail call range(i64 2, 4294967299) i64 @aot_dispatch(ptr nonnull %0, i64 %1)
  ret i64 %69

fall:                                             ; preds = %entry
  store i64 83812, ptr %7, align 4, !tbaa !1
  %70 = musttail call i64 @tb_14764(ptr nonnull %0, i64 %1)
  ret i64 %70
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(read, argmem: readwrite, inaccessiblemem: none)
define noundef i64 @tb_15d08(ptr captures(none) initializes((8, 16), (80, 88), (112, 120), (512, 520)) %0, i64 %1) local_unnamed_addr #0 {
entry:
  %2 = getelementptr i8, ptr %0, i64 8
  %3 = getelementptr i8, ptr %0, i64 64
  %4 = load i64, ptr %3, align 4, !tbaa !1
  %5 = getelementptr i8, ptr %0, i64 80
  %6 = getelementptr i8, ptr %0, i64 112
  %7 = getelementptr i8, ptr %0, i64 512
  store i64 89358, ptr %2, align 4, !tbaa !1
  store i64 %4, ptr %5, align 4, !tbaa !1
  %8 = add i64 %1, 32
  %9 = add i64 %8, %4
  %10 = inttoptr i64 %9 to ptr
  %11 = load i32, ptr %10, align 4, !tbaa !4
  %12 = sext i32 %11 to i64
  %13 = icmp eq i32 %11, 0
  store i64 %12, ptr %6, align 4, !tbaa !1
  %..i = select i1 %13, i64 199048, i64 199024
  store i64 %..i, ptr %7, align 4, !tbaa !1
  ret i64 4294967298
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(read, argmem: readwrite, inaccessiblemem: none)
define noundef i64 @tb_3097c(ptr captures(none) initializes((8, 16), (80, 88), (120, 128), (512, 520)) %0, i64 %1) local_unnamed_addr #0 {
entry:
  %2 = getelementptr i8, ptr %0, i64 8
  %3 = getelementptr i8, ptr %0, i64 16
  %4 = load i64, ptr %3, align 4, !tbaa !1
  %5 = getelementptr i8, ptr %0, i64 64
  %6 = load i64, ptr %5, align 4, !tbaa !1
  %7 = getelementptr i8, ptr %0, i64 80
  %8 = getelementptr i8, ptr %0, i64 120
  %9 = getelementptr i8, ptr %0, i64 512
  %10 = add i64 %4, %1
  %11 = inttoptr i64 %10 to ptr
  %12 = load i64, ptr %11, align 4, !tbaa !4
  %13 = add i64 %10, 8
  %14 = inttoptr i64 %13 to ptr
  %15 = load i64, ptr %14, align 4, !tbaa !4
  %16 = add i64 %4, 16
  store i64 %15, ptr %2, align 4, !tbaa !1
  store i64 %16, ptr %3, align 4, !tbaa !1
  store i64 %12, ptr %5, align 4, !tbaa !1
  store i64 %6, ptr %7, align 4, !tbaa !1
  %17 = add i64 %1, 32
  %18 = add i64 %17, %6
  %19 = inttoptr i64 %18 to ptr
  %20 = load i32, ptr %19, align 4, !tbaa !4
  %21 = sext i32 %20 to i64
  %22 = icmp eq i32 %20, 0
  store i64 %21, ptr %8, align 4, !tbaa !1
  %..i = select i1 %22, i64 198284, i64 198236
  store i64 %..i, ptr %9, align 4, !tbaa !1
  ret i64 4294967298
}

; Function Attrs: nofree nosync nounwind memory(readwrite, inaccessiblemem: none)
define range(i64 2, 4294967299) i64 @tb_15d0e(ptr initializes((8, 16), (64, 80), (144, 168), (512, 520)) %0, i64 %1) local_unnamed_addr #1 {
entry:
  %2 = getelementptr i8, ptr %0, i64 8
  %3 = getelementptr i8, ptr %0, i64 16
  %4 = load i64, ptr %3, align 4, !tbaa !1
  %5 = getelementptr i8, ptr %0, i64 64
  %6 = getelementptr i8, ptr %0, i64 72
  %7 = getelementptr i8, ptr %0, i64 144
  %8 = getelementptr i8, ptr %0, i64 152
  %9 = getelementptr i8, ptr %0, i64 160
  %10 = getelementptr i8, ptr %0, i64 512
  %11 = add i64 %4, %1
  %12 = add i64 %11, 264
  %13 = inttoptr i64 %12 to ptr
  %14 = load i64, ptr %13, align 4, !tbaa !4
  %15 = add i64 %11, 256
  %16 = inttoptr i64 %15 to ptr
  %17 = load i64, ptr %16, align 4, !tbaa !4
  %18 = add i64 %11, 240
  %19 = inttoptr i64 %18 to ptr
  %20 = load i64, ptr %19, align 4, !tbaa !4
  %21 = add i64 %11, 232
  %22 = inttoptr i64 %21 to ptr
  %23 = load i64, ptr %22, align 4, !tbaa !4
  %24 = add i64 %11, 224
  %25 = inttoptr i64 %24 to ptr
  %26 = load i64, ptr %25, align 4, !tbaa !4
  %27 = add i64 %11, 248
  %28 = inttoptr i64 %27 to ptr
  %29 = load i64, ptr %28, align 4, !tbaa !4
  %30 = add i64 %4, 272
  %31 = and i64 %14, -2
  store i64 %14, ptr %2, align 4, !tbaa !1
  store i64 %30, ptr %3, align 4, !tbaa !1
  store i64 %17, ptr %5, align 4, !tbaa !1
  store i64 %29, ptr %6, align 4, !tbaa !1
  store i64 %20, ptr %7, align 4, !tbaa !1
  store i64 %23, ptr %8, align 4, !tbaa !1
  store i64 %26, ptr %9, align 4, !tbaa !1
  store i64 %31, ptr %10, align 4, !tbaa !1
  %32 = musttail call i64 @aot_dispatch(ptr %0, i64 %1)
  ret i64 %32
}

; Function Attrs: nofree nosync nounwind memory(readwrite, inaccessiblemem: none)
define range(i64 2, 4294967299) i64 @tb_12e62(ptr initializes((8, 16), (512, 520)) %0, i64 %1) local_unnamed_addr #1 {
entry:
  %2 = getelementptr i8, ptr %0, i64 8
  %3 = getelementptr i8, ptr %0, i64 16
  %4 = load i64, ptr %3, align 4, !tbaa !1
  %5 = getelementptr i8, ptr %0, i64 512
  %6 = add i64 %1, 24
  %7 = add i64 %6, %4
  %8 = inttoptr i64 %7 to ptr
  %9 = load i64, ptr %8, align 4, !tbaa !4
  %10 = add i64 %4, 96
  %11 = and i64 %9, -2
  store i64 %9, ptr %2, align 4, !tbaa !1
  store i64 %10, ptr %3, align 4, !tbaa !1
  store i64 %11, ptr %5, align 4, !tbaa !1
  %12 = musttail call i64 @aot_dispatch(ptr %0, i64 %1)
  ret i64 %12
}

; Function Attrs: nofree nosync nounwind memory(readwrite, inaccessiblemem: none)
define range(i64 2, 4294967299) i64 @tb_30a7c(ptr initializes((96, 136), (512, 520)) %0, i64 %1) local_unnamed_addr #1 {
entry:
  %2 = getelementptr i8, ptr %0, i64 16
  %3 = load i64, ptr %2, align 4, !tbaa !1
  %4 = getelementptr i8, ptr %0, i64 80
  %5 = load i64, ptr %4, align 4, !tbaa !1
  %6 = getelementptr i8, ptr %0, i64 96
  %7 = getelementptr i8, ptr %0, i64 104
  %8 = getelementptr i8, ptr %0, i64 112
  %9 = getelementptr i8, ptr %0, i64 120
  %10 = getelementptr i8, ptr %0, i64 128
  %11 = getelementptr i8, ptr %0, i64 512
  %12 = add i64 %3, %1
  %13 = inttoptr i64 %12 to ptr
  %14 = load i64, ptr %13, align 4, !tbaa !4
  %15 = add i64 %14, %1
  %16 = add i64 %15, 24
  %17 = inttoptr i64 %16 to ptr
  %18 = load i64, ptr %17, align 4, !tbaa !4
  %19 = add i64 %15, 40
  %20 = inttoptr i64 %19 to ptr
  %21 = load i64, ptr %20, align 4, !tbaa !4
  %22 = add i64 %15, 48
  %23 = inttoptr i64 %22 to ptr
  %24 = load i64, ptr %23, align 4, !tbaa !4
  %25 = sub i64 %18, %21
  %26 = shl i64 %25, 3
  %27 = add i64 %24, %1
  %28 = add i64 %27, %26
  %29 = inttoptr i64 %28 to ptr
  store i64 %5, ptr %29, align 4, !tbaa !4
  %30 = load i64, ptr %20, align 4, !tbaa !4
  %31 = inttoptr i64 %27 to ptr
  %32 = load i64, ptr %31, align 4, !tbaa !4
  store i64 %30, ptr %17, align 4, !tbaa !4
  %33 = icmp eq i64 %30, 0
  store i64 %32, ptr %6, align 4, !tbaa !1
  store i64 %21, ptr %7, align 4, !tbaa !1
  store i64 %30, ptr %9, align 4, !tbaa !1
  store i64 %14, ptr %10, align 4, !tbaa !1
  br i1 %33, label %L0, label %fall

L0:                                               ; preds = %entry
  %34 = getelementptr i8, ptr %0, i64 8
  %35 = add i64 %1, 24
  %36 = add i64 %3, %35
  %37 = inttoptr i64 %36 to ptr
  %38 = load i64, ptr %37, align 4, !tbaa !4
  %39 = add i64 %32, 48
  %40 = and i64 %39, 255
  %41 = add i64 %14, %35
  %42 = inttoptr i64 %41 to ptr
  store i64 1, ptr %42, align 4, !tbaa !4
  %43 = add i64 %3, 32
  %44 = and i64 %38, -2
  store i64 %38, ptr %34, align 4, !tbaa !1
  store i64 %43, ptr %2, align 4, !tbaa !1
  store i64 %40, ptr %4, align 4, !tbaa !1
  store i64 1, ptr %8, align 4, !tbaa !1
  store i64 %40, ptr %9, align 4, !tbaa !1
  store i64 %44, ptr %11, align 4, !tbaa !1
  %45 = musttail call range(i64 2, 4294967299) i64 @aot_dispatch(ptr nonnull %0, i64 %1)
  ret i64 %45

fall:                                             ; preds = %entry
  %46 = getelementptr i8, ptr %0, i64 88
  %47 = add i64 %15, 16
  %48 = inttoptr i64 %47 to ptr
  %49 = load i64, ptr %48, align 4, !tbaa !4
  %50 = shl i64 %30, 3
  %51 = add i64 %49, %50
  store i64 %49, ptr %46, align 4, !tbaa !1
  store i64 %51, ptr %8, align 4, !tbaa !1
  store i64 199344, ptr %11, align 4, !tbaa !1
  ret i64 4294967298
}

; Function Attrs: nofree nosync nounwind memory(readwrite, inaccessiblemem: none)
define range(i64 2, 4294967299) i64 @tb_30a48(ptr initializes((8, 16), (120, 128), (512, 520)) %0, i64 %1) local_unnamed_addr #1 {
entry:
  %2 = getelementptr i8, ptr %0, i64 16
  %3 = load i64, ptr %2, align 4, !tbaa !1
  %4 = getelementptr i8, ptr %0, i64 80
  %5 = load i64, ptr %4, align 4, !tbaa !1
  %6 = getelementptr i8, ptr %0, i64 120
  %7 = getelementptr i8, ptr %0, i64 512
  %8 = add i64 %3, %1
  %9 = inttoptr i64 %8 to ptr
  %10 = load i64, ptr %9, align 4, !tbaa !4
  %11 = icmp eq i64 %5, 0
  store i64 %10, ptr %6, align 4, !tbaa !1
  %12 = getelementptr i8, ptr %0, i64 8
  br i1 %11, label %L0, label %fall

L0:                                               ; preds = %entry
  %13 = add i64 %8, 24
  %14 = inttoptr i64 %13 to ptr
  %15 = load i64, ptr %14, align 4, !tbaa !4
  %16 = add i64 %3, 32
  %17 = and i64 %15, -2
  store i64 %15, ptr %12, align 4, !tbaa !1
  store i64 %16, ptr %2, align 4, !tbaa !1
  store i64 %10, ptr %4, align 4, !tbaa !1
  store i64 %17, ptr %7, align 4, !tbaa !1
  %18 = musttail call range(i64 2, 4294967299) i64 @aot_dispatch(ptr nonnull %0, i64 %1)
  ret i64 %18

fall:                                             ; preds = %entry
  %19 = getelementptr i8, ptr %0, i64 96
  %20 = getelementptr i8, ptr %0, i64 104
  %21 = getelementptr i8, ptr %0, i64 112
  %22 = getelementptr i8, ptr %0, i64 128
  %23 = add i64 %8, 8
  %24 = inttoptr i64 %23 to ptr
  %25 = load i64, ptr %24, align 4, !tbaa !4
  %26 = add i64 %1, 24
  %27 = add i64 %25, %26
  %28 = inttoptr i64 %27 to ptr
  %29 = load i64, ptr %28, align 4, !tbaa !4
  %30 = add i64 %1, 16
  %31 = add i64 %30, %25
  %32 = inttoptr i64 %31 to ptr
  %33 = load i64, ptr %32, align 4, !tbaa !4
  %34 = add i64 %29, 1
  %35 = shl i64 %29, 3
  store i64 %34, ptr %28, align 4, !tbaa !4
  %36 = add i64 %35, %33
  %37 = add i64 %36, %1
  %38 = inttoptr i64 %37 to ptr
  store i64 %5, ptr %38, align 4, !tbaa !4
  %39 = add i64 %3, %26
  %40 = inttoptr i64 %39 to ptr
  %41 = load i64, ptr %40, align 4, !tbaa !4
  %42 = add i64 %3, 32
  %43 = and i64 %41, -2
  store i64 %41, ptr %12, align 4, !tbaa !1
  store i64 %42, ptr %2, align 4, !tbaa !1
  store i64 %10, ptr %4, align 4, !tbaa !1
  store i64 %34, ptr %19, align 4, !tbaa !1
  store i64 %33, ptr %20, align 4, !tbaa !1
  store i64 %36, ptr %21, align 4, !tbaa !1
  store i64 %25, ptr %22, align 4, !tbaa !1
  store i64 %43, ptr %7, align 4, !tbaa !1
  %44 = musttail call range(i64 2, 4294967299) i64 @aot_dispatch(ptr nonnull %0, i64 %1)
  ret i64 %44
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(readwrite, inaccessiblemem: none)
define noundef i64 @tb_316dc(ptr captures(none) initializes((512, 520)) %0, i64 %1) local_unnamed_addr #4 {
entry:
  %2 = getelementptr i8, ptr %0, i64 64
  %3 = load i64, ptr %2, align 4, !tbaa !1
  %4 = getelementptr i8, ptr %0, i64 80
  %5 = load i64, ptr %4, align 4, !tbaa !1
  %6 = getelementptr i8, ptr %0, i64 144
  %7 = load i64, ptr %6, align 4, !tbaa !1
  %8 = getelementptr i8, ptr %0, i64 152
  %9 = load i64, ptr %8, align 4, !tbaa !1
  %10 = getelementptr i8, ptr %0, i64 176
  %11 = load i64, ptr %10, align 4, !tbaa !1
  %12 = getelementptr i8, ptr %0, i64 184
  %13 = load i64, ptr %12, align 4, !tbaa !1
  %14 = add i64 %13, 1
  %15 = add i64 %1, -1
  %16 = add i64 %15, %7
  %17 = inttoptr i64 %16 to ptr
  %18 = trunc i64 %5 to i8
  store i8 %18, ptr %17, align 1, !tbaa !4
  %19 = icmp slt i64 %14, %11
  store i64 %14, ptr %12, align 4, !tbaa !1
  br i1 %19, label %L0, label %fall

common.ret:                                       ; preds = %fall, %L0
  %storemerge = phi i64 [ 199192, %L0 ], [ 201536, %fall ]
  %20 = getelementptr i8, ptr %0, i64 512
  store i64 %storemerge, ptr %20, align 4, !tbaa !1
  ret i64 4294967298

L0:                                               ; preds = %entry
  %21 = getelementptr i8, ptr %0, i64 8
  %22 = add i64 %7, 1
  store i64 202460, ptr %21, align 4, !tbaa !1
  store i64 %9, ptr %4, align 4, !tbaa !1
  store i64 %22, ptr %6, align 4, !tbaa !1
  br label %common.ret

fall:                                             ; preds = %entry
  %23 = getelementptr i8, ptr %0, i64 120
  %24 = add i64 %1, -232
  %25 = add i64 %24, %3
  %26 = inttoptr i64 %25 to ptr
  %27 = load i64, ptr %26, align 4, !tbaa !4
  %28 = add i64 %27, 1
  store i64 %28, ptr %23, align 4, !tbaa !1
  br label %common.ret
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(write, argmem: readwrite, inaccessiblemem: none)
define noundef i64 @tb_3138a(ptr captures(none) initializes((104, 112), (512, 520)) %0, i64 %1) local_unnamed_addr #2 {
entry:
  %2 = getelementptr i8, ptr %0, i64 80
  %3 = load i64, ptr %2, align 4, !tbaa !1
  %4 = getelementptr i8, ptr %0, i64 104
  %5 = getelementptr i8, ptr %0, i64 144
  %6 = load i64, ptr %5, align 4, !tbaa !1
  %7 = getelementptr i8, ptr %0, i64 208
  %8 = getelementptr i8, ptr %0, i64 216
  %9 = trunc i64 %3 to i8
  %10 = add i64 %3, -48
  %.not = icmp eq i64 %10, 0
  store i64 %10, ptr %4, align 4, !tbaa !1
  %11 = load i64, ptr %8, align 4, !tbaa !1
  %12 = load <2 x i64>, ptr %7, align 4, !tbaa !1
  %13 = add i64 %11, %1
  %14 = inttoptr i64 %13 to ptr
  store i8 %9, ptr %14, align 1, !tbaa !4
  %15 = add <2 x i64> %12, splat (i64 1)
  store <2 x i64> %15, ptr %7, align 4, !tbaa !1
  %16 = icmp eq i64 %6, 0
  %or.cond = select i1 %.not, i1 %16, i1 false
  br i1 %or.cond, label %common.ret, label %common.ret.sink.split

common.ret.sink.split:                            ; preds = %entry
  store i64 1, ptr %5, align 4, !tbaa !1
  br label %common.ret

common.ret:                                       ; preds = %entry, %common.ret.sink.split
  %storemerge = phi i64 [ 201586, %common.ret.sink.split ], [ 201636, %entry ]
  %17 = getelementptr i8, ptr %0, i64 512
  store i64 %storemerge, ptr %17, align 4, !tbaa !1
  ret i64 4294967298
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(read, argmem: readwrite, inaccessiblemem: none)
define hidden noundef i64 @tb_4ec72(ptr captures(none) initializes((512, 520)) %0, i64 %1) local_unnamed_addr #0 {
entry:
  %2 = getelementptr i8, ptr %0, i64 64
  %3 = load i64, ptr %2, align 4, !tbaa !1
  %4 = getelementptr i8, ptr %0, i64 208
  %5 = load i64, ptr %4, align 4, !tbaa !1
  %6 = getelementptr i8, ptr %0, i64 512
  %7 = and i64 %3, 4294967295
  %8 = add nuw nsw i64 %7, 4
  %9 = add i64 %8, %5
  %10 = add i64 %9, %1
  %11 = inttoptr i64 %10 to ptr
  %12 = load i32, ptr %11, align 4, !tbaa !4
  %13 = sext i32 %12 to i64
  %.not = icmp eq i32 %12, 0
  store i64 %13, ptr %2, align 4, !tbaa !1
  store i64 %9, ptr %4, align 4, !tbaa !1
  %. = select i1 %.not, i64 322688, i64 322520
  store i64 %., ptr %6, align 4, !tbaa !1
  ret i64 4294967298
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(readwrite, inaccessiblemem: none)
define hidden noundef i64 @tb_10bf2(ptr captures(none) initializes((120, 128), (512, 520)) %0, i64 %1) local_unnamed_addr #4 {
entry:
  %2 = getelementptr i8, ptr %0, i64 80
  %3 = load i64, ptr %2, align 4, !tbaa !1
  %4 = getelementptr i8, ptr %0, i64 120
  %5 = getelementptr i8, ptr %0, i64 136
  %6 = load i64, ptr %5, align 4, !tbaa !1
  %7 = getelementptr i8, ptr %0, i64 512
  %8 = add i64 %6, %1
  %9 = add i64 %8, 102
  %10 = inttoptr i64 %9 to ptr
  %11 = trunc i64 %3 to i16
  store i16 %11, ptr %10, align 2, !tbaa !4
  %12 = add i64 %8, 96
  %13 = inttoptr i64 %12 to ptr
  %14 = load i16, ptr %13, align 2, !tbaa !4
  %15 = zext i16 %14 to i64
  store i64 %15, ptr %4, align 4, !tbaa !1
  store i64 68480, ptr %7, align 4, !tbaa !1
  ret i64 4294967298
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(argmem: write)
define hidden noundef i64 @tb_4ecc2(ptr writeonly captures(none) initializes((104, 112), (120, 128), (512, 520)) %0, i64 %1) local_unnamed_addr #6 {
entry:
  %2 = getelementptr i8, ptr %0, i64 104
  %3 = getelementptr i8, ptr %0, i64 120
  %4 = getelementptr i8, ptr %0, i64 512
  store i64 65535, ptr %2, align 4, !tbaa !1
  store i64 65536, ptr %3, align 4, !tbaa !1
  store i64 322626, ptr %4, align 4, !tbaa !1
  ret i64 4294967298
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(argmem: readwrite)
define hidden noundef i64 @tb_4ecd8(ptr captures(none) initializes((512, 520)) %0, i64 %1) local_unnamed_addr #3 {
entry:
  %2 = getelementptr i8, ptr %0, i64 104
  %3 = load i64, ptr %2, align 4, !tbaa !1
  %4 = getelementptr i8, ptr %0, i64 184
  %5 = load i64, ptr %4, align 4, !tbaa !1
  %.not = icmp ult i64 %5, %3
  %. = select i1 %.not, i64 322780, i64 322784
  %6 = getelementptr i8, ptr %0, i64 512
  store i64 %., ptr %6, align 4, !tbaa !1
  ret i64 4294967298
}

; Function Attrs: nofree nosync nounwind memory(readwrite, inaccessiblemem: none)
define hidden range(i64 2, 4294967299) i64 @tb_30a08(ptr initializes((8, 16), (64, 80), (144, 160), (512, 520)) %0, i64 %1) local_unnamed_addr #1 {
entry:
  %2 = getelementptr i8, ptr %0, i64 8
  %3 = getelementptr i8, ptr %0, i64 16
  %4 = load i64, ptr %3, align 4, !tbaa !1
  %5 = getelementptr i8, ptr %0, i64 64
  %6 = getelementptr i8, ptr %0, i64 72
  %7 = getelementptr i8, ptr %0, i64 144
  %8 = getelementptr i8, ptr %0, i64 152
  %9 = getelementptr i8, ptr %0, i64 512
  %10 = add i64 %4, %1
  %11 = add i64 %10, 32
  %12 = inttoptr i64 %11 to ptr
  %13 = load i64, ptr %12, align 4, !tbaa !4
  %14 = add i64 %10, 16
  %15 = inttoptr i64 %14 to ptr
  %16 = load i64, ptr %15, align 4, !tbaa !4
  %17 = add i64 %10, 8
  %18 = inttoptr i64 %17 to ptr
  %19 = load i64, ptr %18, align 4, !tbaa !4
  %20 = add i64 %10, 40
  %21 = inttoptr i64 %20 to ptr
  %22 = load i64, ptr %21, align 4, !tbaa !4
  %23 = add i64 %10, 24
  %24 = inttoptr i64 %23 to ptr
  %25 = load i64, ptr %24, align 4, !tbaa !4
  %26 = add i64 %4, 48
  %27 = and i64 %22, -2
  store i64 %22, ptr %2, align 4, !tbaa !1
  store i64 %26, ptr %3, align 4, !tbaa !1
  store i64 %13, ptr %5, align 4, !tbaa !1
  store i64 %25, ptr %6, align 4, !tbaa !1
  store i64 %16, ptr %7, align 4, !tbaa !1
  store i64 %19, ptr %8, align 4, !tbaa !1
  store i64 %27, ptr %9, align 4, !tbaa !1
  %28 = musttail call i64 @aot_dispatch(ptr %0, i64 %1)
  ret i64 %28
}

; Function Attrs: nofree nosync nounwind memory(readwrite, inaccessiblemem: none)
define hidden range(i64 2, 4294967299) i64 @tb_14562(ptr initializes((120, 128), (512, 520)) %0, i64 %1) local_unnamed_addr #1 {
entry:
  %2 = getelementptr i8, ptr %0, i64 120
  %3 = getelementptr i8, ptr %0, i64 208
  %4 = load i64, ptr %3, align 4, !tbaa !1
  %5 = getelementptr i8, ptr %0, i64 512
  %6 = add i64 %4, %1
  %7 = inttoptr i64 %6 to ptr
  %8 = load i8, ptr %7, align 1, !tbaa !4
  %9 = zext i8 %8 to i64
  %10 = icmp eq i8 %8, 0
  store i64 %9, ptr %2, align 4, !tbaa !1
  br i1 %10, label %L0, label %fall

L0:                                               ; preds = %entry
  %11 = getelementptr i8, ptr %0, i64 8
  %12 = getelementptr i8, ptr %0, i64 16
  %13 = load i64, ptr %12, align 4, !tbaa !1
  %14 = getelementptr i8, ptr %0, i64 64
  %15 = getelementptr i8, ptr %0, i64 72
  %16 = getelementptr i8, ptr %0, i64 144
  %17 = getelementptr i8, ptr %0, i64 152
  %18 = add i64 %13, %1
  %19 = add i64 %18, 1352
  %20 = inttoptr i64 %19 to ptr
  %21 = load i64, ptr %20, align 4, !tbaa !4
  %22 = add i64 %18, 1344
  %23 = inttoptr i64 %22 to ptr
  %24 = load i64, ptr %23, align 4, !tbaa !4
  %25 = add i64 %18, 1336
  %26 = inttoptr i64 %25 to ptr
  %27 = load i64, ptr %26, align 4, !tbaa !4
  %28 = add i64 %18, 1328
  %29 = inttoptr i64 %28 to ptr
  %30 = load i64, ptr %29, align 4, !tbaa !4
  %31 = add i64 %18, 1320
  %32 = inttoptr i64 %31 to ptr
  %33 = load i64, ptr %32, align 4, !tbaa !4
  %34 = add i64 %18, 1264
  %35 = inttoptr i64 %34 to ptr
  %36 = load i64, ptr %35, align 4, !tbaa !4
  %37 = add i64 %13, 1360
  %38 = and i64 %21, -2
  store i64 %21, ptr %11, align 4, !tbaa !1
  store i64 %37, ptr %12, align 4, !tbaa !1
  store i64 %24, ptr %14, align 4, !tbaa !1
  store i64 %27, ptr %15, align 4, !tbaa !1
  store i64 %30, ptr %16, align 4, !tbaa !1
  store i64 %33, ptr %17, align 4, !tbaa !1
  store i64 %36, ptr %3, align 4, !tbaa !1
  store i64 %38, ptr %5, align 4, !tbaa !1
  %39 = musttail call range(i64 2, 4294967299) i64 @aot_dispatch(ptr nonnull %0, i64 %1)
  ret i64 %39

fall:                                             ; preds = %entry
  store i64 83304, ptr %5, align 4, !tbaa !1
  ret i64 4294967298
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(readwrite, inaccessiblemem: none)
define hidden noundef i64 @tb_14afc(ptr captures(none) initializes((8, 16), (72, 104), (512, 520)) %0, i64 %1) local_unnamed_addr #4 {
entry:
  %2 = getelementptr i8, ptr %0, i64 8
  %3 = getelementptr i8, ptr %0, i64 16
  %4 = load i64, ptr %3, align 4, !tbaa !1
  %5 = getelementptr i8, ptr %0, i64 32
  %6 = load i64, ptr %5, align 4, !tbaa !1
  %7 = getelementptr i8, ptr %0, i64 72
  %8 = getelementptr i8, ptr %0, i64 80
  %9 = getelementptr i8, ptr %0, i64 88
  %10 = getelementptr i8, ptr %0, i64 96
  %11 = getelementptr i8, ptr %0, i64 104
  %12 = load i64, ptr %11, align 4, !tbaa !1
  %13 = getelementptr i8, ptr %0, i64 232
  %14 = load i64, ptr %13, align 4, !tbaa !1
  %15 = getelementptr i8, ptr %0, i64 512
  %16 = add i64 %1, 511728
  %17 = inttoptr i64 %16 to ptr
  %18 = load i64, ptr %17, align 4, !tbaa !4
  %19 = add i64 %4, 224
  %20 = add i64 %6, %1
  %21 = add i64 %20, %18
  %22 = inttoptr i64 %21 to ptr
  %23 = load i64, ptr %22, align 4, !tbaa !4
  %24 = add i64 %4, %1
  %25 = add i64 %24, 112
  %26 = inttoptr i64 %25 to ptr
  store i64 %14, ptr %26, align 4, !tbaa !4
  %27 = add i64 %24, 104
  %28 = inttoptr i64 %27 to ptr
  store i64 %12, ptr %28, align 4, !tbaa !4
  store i64 84758, ptr %2, align 4, !tbaa !1
  store i64 %18, ptr %7, align 4, !tbaa !1
  store i64 %19, ptr %8, align 4, !tbaa !1
  store i64 1, ptr %9, align 4, !tbaa !1
  store i64 %23, ptr %10, align 4, !tbaa !1
  store i64 197932, ptr %15, align 4, !tbaa !1
  ret i64 4294967298
}

; Function Attrs: nofree nosync nounwind memory(readwrite, inaccessiblemem: none)
define hidden range(i64 2, 4294967299) i64 @tb_10f50(ptr initializes((8, 16), (48, 56), (80, 144), (224, 248), (512, 520)) %0, i64 %1) local_unnamed_addr #1 {
entry:
  %2 = getelementptr i8, ptr %0, i64 8
  %3 = getelementptr i8, ptr %0, i64 64
  %4 = load i64, ptr %3, align 4, !tbaa !1
  %5 = getelementptr i8, ptr %0, i64 72
  %6 = load i64, ptr %5, align 4, !tbaa !1
  %7 = getelementptr i8, ptr %0, i64 80
  %8 = getelementptr i8, ptr %0, i64 88
  %9 = getelementptr i8, ptr %0, i64 120
  %10 = getelementptr i8, ptr %0, i64 512
  %11 = add i64 %1, 8
  %12 = add i64 %11, %4
  %13 = inttoptr i64 %12 to ptr
  %14 = load i64, ptr %13, align 4, !tbaa !4
  %15 = add i64 %14, %1
  %16 = inttoptr i64 %15 to ptr
  %17 = load i16, ptr %16, align 2, !tbaa !4
  %18 = sext i16 %17 to i64
  store i64 69468, ptr %2, align 4, !tbaa !1
  store i64 %18, ptr %7, align 4, !tbaa !1
  store i64 %6, ptr %8, align 4, !tbaa !1
  store i64 %14, ptr %9, align 4, !tbaa !1
  store i64 74642, ptr %10, align 4, !tbaa !1
  %19 = musttail call i64 @tb_12392(ptr %0, i64 %1)
  ret i64 %19
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(argmem: readwrite)
define hidden noundef i64 @tb_113f8(ptr captures(none) initializes((40, 48), (56, 64), (224, 232), (512, 520)) %0, i64 %1) local_unnamed_addr #3 {
entry:
  %2 = getelementptr i8, ptr %0, i64 40
  %3 = getelementptr i8, ptr %0, i64 56
  %4 = getelementptr i8, ptr %0, i64 80
  %5 = load i64, ptr %4, align 4, !tbaa !1
  %6 = getelementptr i8, ptr %0, i64 224
  %7 = getelementptr i8, ptr %0, i64 512
  %8 = icmp eq i64 %5, 0
  store i64 0, ptr %2, align 4, !tbaa !1
  store i64 0, ptr %3, align 4, !tbaa !1
  store i64 %5, ptr %6, align 4, !tbaa !1
  %. = select i1 %8, i64 70778, i64 70656
  store i64 %., ptr %7, align 4, !tbaa !1
  ret i64 4294967298
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(argmem: readwrite)
define hidden noundef i64 @tb_11b16(ptr captures(none) initializes((8, 16), (80, 96), (512, 520)) %0, i64 %1) local_unnamed_addr #3 {
entry:
  %2 = getelementptr i8, ptr %0, i64 8
  %3 = getelementptr i8, ptr %0, i64 80
  %4 = getelementptr i8, ptr %0, i64 152
  %5 = getelementptr i8, ptr %0, i64 512
  store i64 72478, ptr %2, align 4, !tbaa !1
  %6 = load <2 x i64>, ptr %4, align 4, !tbaa !1
  %7 = shufflevector <2 x i64> %6, <2 x i64> poison, <2 x i32> <i32 1, i32 0>
  store <2 x i64> %7, ptr %3, align 4, !tbaa !1
  store i64 71878, ptr %5, align 4, !tbaa !1
  ret i64 4294967298
}

; Function Attrs: nofree nosync nounwind memory(readwrite, inaccessiblemem: none)
define hidden range(i64 2, 4294967299) i64 @tb_11bc4(ptr initializes((8, 16), (64, 80), (144, 200), (512, 520)) %0, i64 %1) local_unnamed_addr #1 {
entry:
  %2 = getelementptr i8, ptr %0, i64 8
  %3 = getelementptr i8, ptr %0, i64 16
  %4 = load i64, ptr %3, align 4, !tbaa !1
  %5 = getelementptr i8, ptr %0, i64 64
  %6 = getelementptr i8, ptr %0, i64 72
  %7 = getelementptr i8, ptr %0, i64 144
  %8 = getelementptr i8, ptr %0, i64 152
  %9 = getelementptr i8, ptr %0, i64 160
  %10 = getelementptr i8, ptr %0, i64 168
  %11 = getelementptr i8, ptr %0, i64 176
  %12 = getelementptr i8, ptr %0, i64 184
  %13 = getelementptr i8, ptr %0, i64 192
  %14 = getelementptr i8, ptr %0, i64 512
  %15 = add i64 %4, %1
  %16 = add i64 %15, 152
  %17 = inttoptr i64 %16 to ptr
  %18 = load i64, ptr %17, align 4, !tbaa !4
  %19 = add i64 %15, 144
  %20 = inttoptr i64 %19 to ptr
  %21 = load i64, ptr %20, align 4, !tbaa !4
  %22 = add i64 %15, 136
  %23 = inttoptr i64 %22 to ptr
  %24 = load i64, ptr %23, align 4, !tbaa !4
  %25 = add i64 %15, 128
  %26 = inttoptr i64 %25 to ptr
  %27 = load i64, ptr %26, align 4, !tbaa !4
  %28 = add i64 %15, 120
  %29 = inttoptr i64 %28 to ptr
  %30 = load i64, ptr %29, align 4, !tbaa !4
  %31 = add i64 %15, 112
  %32 = inttoptr i64 %31 to ptr
  %33 = load i64, ptr %32, align 4, !tbaa !4
  %34 = add i64 %15, 104
  %35 = inttoptr i64 %34 to ptr
  %36 = load i64, ptr %35, align 4, !tbaa !4
  %37 = add i64 %15, 96
  %38 = inttoptr i64 %37 to ptr
  %39 = load i64, ptr %38, align 4, !tbaa !4
  %40 = add i64 %15, 88
  %41 = inttoptr i64 %40 to ptr
  %42 = load i64, ptr %41, align 4, !tbaa !4
  %43 = add i64 %15, 80
  %44 = inttoptr i64 %43 to ptr
  %45 = load i64, ptr %44, align 4, !tbaa !4
  %46 = add i64 %4, 160
  %47 = and i64 %18, -2
  store i64 %18, ptr %2, align 4, !tbaa !1
  store i64 %46, ptr %3, align 4, !tbaa !1
  store i64 %21, ptr %5, align 4, !tbaa !1
  store i64 %24, ptr %6, align 4, !tbaa !1
  store i64 %27, ptr %7, align 4, !tbaa !1
  store i64 %30, ptr %8, align 4, !tbaa !1
  store i64 %33, ptr %9, align 4, !tbaa !1
  store i64 %36, ptr %10, align 4, !tbaa !1
  store i64 %39, ptr %11, align 4, !tbaa !1
  store i64 %42, ptr %12, align 4, !tbaa !1
  store i64 %45, ptr %13, align 4, !tbaa !1
  store i64 %47, ptr %14, align 4, !tbaa !1
  %48 = musttail call i64 @aot_dispatch(ptr %0, i64 %1)
  ret i64 %48
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(argmem: readwrite)
define hidden noundef i64 @tb_316d4(ptr captures(none) initializes((8, 16), (80, 88), (512, 520)) %0, i64 %1) local_unnamed_addr #3 {
entry:
  %2 = getelementptr i8, ptr %0, i64 8
  %3 = getelementptr i8, ptr %0, i64 80
  %4 = getelementptr i8, ptr %0, i64 144
  %5 = load i64, ptr %4, align 4, !tbaa !1
  %6 = getelementptr i8, ptr %0, i64 152
  %7 = load i64, ptr %6, align 4, !tbaa !1
  %8 = getelementptr i8, ptr %0, i64 512
  %9 = add i64 %5, 1
  store i64 202460, ptr %2, align 4, !tbaa !1
  store i64 %7, ptr %3, align 4, !tbaa !1
  store i64 %9, ptr %4, align 4, !tbaa !1
  store i64 199192, ptr %8, align 4, !tbaa !1
  ret i64 4294967298
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(read, argmem: readwrite, inaccessiblemem: none)
define hidden noundef i64 @tb_4e36a(ptr captures(none) initializes((112, 128), (512, 520)) %0, i64 %1) local_unnamed_addr #0 {
entry:
  %2 = getelementptr i8, ptr %0, i64 72
  %3 = load i64, ptr %2, align 4, !tbaa !1
  %4 = getelementptr i8, ptr %0, i64 112
  %5 = getelementptr i8, ptr %0, i64 120
  %6 = getelementptr i8, ptr %0, i64 512
  %7 = add i64 %1, 9
  %8 = add i64 %7, %3
  %9 = inttoptr i64 %8 to ptr
  %10 = load i8, ptr %9, align 1, !tbaa !4
  %11 = zext i8 %10 to i64
  %12 = icmp eq i8 %10, 122
  store i64 %11, ptr %4, align 4, !tbaa !1
  store i64 122, ptr %5, align 4, !tbaa !1
  %. = select i1 %12, i64 320386, i64 320374
  store i64 %., ptr %6, align 4, !tbaa !1
  ret i64 4294967298
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(argmem: write)
define hidden noundef i64 @tb_313a0(ptr writeonly captures(none) initializes((144, 152), (512, 520)) %0, i64 %1) local_unnamed_addr #6 {
entry:
  %2 = getelementptr i8, ptr %0, i64 144
  %3 = getelementptr i8, ptr %0, i64 512
  store i64 1, ptr %2, align 4, !tbaa !1
  store i64 201586, ptr %3, align 4, !tbaa !1
  ret i64 4294967298
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(argmem: readwrite)
define hidden noundef i64 @tb_20970(ptr captures(none) initializes((112, 128), (512, 520)) %0, i64 %1) local_unnamed_addr #3 {
entry:
  %2 = getelementptr i8, ptr %0, i64 80
  %3 = load i64, ptr %2, align 4, !tbaa !1
  %4 = getelementptr i8, ptr %0, i64 88
  %5 = load i64, ptr %4, align 4, !tbaa !1
  %6 = getelementptr i8, ptr %0, i64 96
  %7 = load i64, ptr %6, align 4, !tbaa !1
  %8 = getelementptr i8, ptr %0, i64 112
  %9 = getelementptr i8, ptr %0, i64 120
  %10 = getelementptr i8, ptr %0, i64 512
  %11 = xor i64 %5, -1
  %12 = add i64 %3, %11
  %13 = add i64 %7, %5
  %14 = icmp eq i64 %7, 0
  store i64 %13, ptr %4, align 4, !tbaa !1
  store i64 %12, ptr %8, align 4, !tbaa !1
  store i64 %5, ptr %9, align 4, !tbaa !1
  %. = select i1 %14, i64 133518, i64 133500
  store i64 %., ptr %10, align 4, !tbaa !1
  ret i64 4294967298
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(read, argmem: readwrite, inaccessiblemem: none)
define hidden noundef i64 @tb_3096c(ptr captures(none) initializes((112, 120), (512, 520)) %0, i64 %1) local_unnamed_addr #0 {
entry:
  %2 = getelementptr i8, ptr %0, i64 80
  %3 = load i64, ptr %2, align 4, !tbaa !1
  %4 = getelementptr i8, ptr %0, i64 112
  %5 = getelementptr i8, ptr %0, i64 512
  %6 = add i64 %1, 32
  %7 = add i64 %6, %3
  %8 = inttoptr i64 %7 to ptr
  %9 = load i32, ptr %8, align 4, !tbaa !4
  %10 = sext i32 %9 to i64
  %11 = icmp eq i32 %9, 0
  store i64 %10, ptr %4, align 4, !tbaa !1
  %. = select i1 %11, i64 199048, i64 199024
  store i64 %., ptr %5, align 4, !tbaa !1
  ret i64 4294967298
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(read, argmem: readwrite, inaccessiblemem: none)
define hidden noundef i64 @tb_30658(ptr captures(none) initializes((120, 128), (512, 520)) %0, i64 %1) local_unnamed_addr #0 {
entry:
  %2 = getelementptr i8, ptr %0, i64 80
  %3 = load i64, ptr %2, align 4, !tbaa !1
  %4 = getelementptr i8, ptr %0, i64 120
  %5 = getelementptr i8, ptr %0, i64 512
  %6 = add i64 %1, 32
  %7 = add i64 %6, %3
  %8 = inttoptr i64 %7 to ptr
  %9 = load i32, ptr %8, align 4, !tbaa !4
  %10 = sext i32 %9 to i64
  %11 = icmp eq i32 %9, 0
  store i64 %10, ptr %4, align 4, !tbaa !1
  %. = select i1 %11, i64 198284, i64 198236
  store i64 %., ptr %5, align 4, !tbaa !1
  ret i64 4294967298
}

; Function Attrs: nofree nosync nounwind memory(readwrite, inaccessiblemem: none)
define hidden range(i64 2, 4294967299) i64 @tb_11fc0(ptr initializes((48, 56), (96, 144), (224, 256), (512, 520)) %0, i64 %1) local_unnamed_addr #1 {
entry:
  %2 = getelementptr i8, ptr %0, i64 8
  %3 = load i64, ptr %2, align 4, !tbaa !1
  %4 = getelementptr i8, ptr %0, i64 48
  %5 = getelementptr i8, ptr %0, i64 80
  %6 = load i64, ptr %5, align 4, !tbaa !1
  %7 = getelementptr i8, ptr %0, i64 88
  %8 = load i64, ptr %7, align 4, !tbaa !1
  %9 = getelementptr i8, ptr %0, i64 96
  %10 = getelementptr i8, ptr %0, i64 104
  %11 = getelementptr i8, ptr %0, i64 112
  %12 = getelementptr i8, ptr %0, i64 120
  %13 = getelementptr i8, ptr %0, i64 128
  %14 = getelementptr i8, ptr %0, i64 136
  %15 = getelementptr i8, ptr %0, i64 224
  %16 = getelementptr i8, ptr %0, i64 232
  %17 = getelementptr i8, ptr %0, i64 248
  %18 = getelementptr i8, ptr %0, i64 512
  %19 = shl i64 %8, 8
  %20 = and i64 %19, 65280
  %21 = lshr i64 %8, 8
  %22 = or i64 %20, %21
  %23 = lshr i64 %22, 4
  %24 = and i64 %23, 268435215
  %25 = shl nuw nsw i64 %22, 4
  %26 = and i64 %25, 61680
  %27 = or i64 %26, %24
  %28 = lshr i64 %27, 2
  %29 = and i64 %28, 67105587
  %30 = shl nuw nsw i64 %27, 2
  %31 = and i64 %30, 52428
  %32 = or i64 %31, %29
  %33 = shl nuw nsw i64 %32, 1
  %34 = and i64 %33, 43690
  %35 = lshr i64 %32, 1
  %36 = and i64 %35, 33543509
  %37 = or i64 %34, %36
  %38 = lshr i64 %37, 8
  %trunc = trunc i64 %6 to i8
  %rev = tail call i8 @llvm.bitreverse.i8(i8 %trunc)
  %39 = zext i8 %rev to i64
  %40 = xor i64 %38, %39
  %41 = shl nuw nsw i64 %40, 1
  %42 = add i64 %1, 361072
  %43 = add i64 %42, %41
  %44 = inttoptr i64 %43 to ptr
  %45 = load i16, ptr %44, align 2, !tbaa !4
  %46 = zext i16 %45 to i64
  %47 = shl i64 %37, 56
  %48 = shl nuw i64 %46, 48
  %49 = xor i64 %47, %48
  %50 = lshr i64 %49, 56
  %51 = shl nuw nsw i64 %46, 8
  %52 = and i64 %51, 65280
  %53 = or disjoint i64 %50, %52
  %54 = lshr i64 %53, 4
  %55 = and i64 %54, 3855
  %56 = shl nuw nsw i64 %53, 4
  %57 = and i64 %56, 61680
  %58 = or disjoint i64 %57, %55
  %59 = lshr i64 %58, 2
  %60 = and i64 %59, 13107
  %61 = shl nuw nsw i64 %58, 2
  %62 = and i64 %61, 52428
  %63 = or disjoint i64 %62, %60
  %64 = lshr i64 %63, 1
  %65 = and i64 %64, 21845
  %66 = shl nuw nsw i64 %63, 1
  %67 = and i64 %66, 43690
  %68 = or disjoint i64 %67, %65
  %69 = shl nuw nsw i64 %68, 8
  %70 = and i64 %69, 65280
  %71 = lshr i64 %68, 8
  %72 = or disjoint i64 %70, %71
  %73 = lshr i64 %72, 4
  %74 = and i64 %73, 3855
  %75 = shl nuw nsw i64 %72, 4
  %76 = and i64 %75, 61680
  %77 = or disjoint i64 %76, %74
  %78 = lshr i64 %6, 12
  %79 = and i64 %78, 15
  %80 = lshr i64 %6, 4
  %81 = and i64 %80, 240
  %82 = or disjoint i64 %81, %79
  %83 = lshr i64 %77, 2
  %84 = and i64 %83, 13107
  %85 = shl nuw nsw i64 %77, 2
  %86 = and i64 %85, 52428
  %87 = or disjoint i64 %86, %84
  %88 = lshr i64 %82, 2
  %89 = and i64 %88, 51
  %90 = shl nuw nsw i64 %82, 2
  %91 = and i64 %90, 204
  %92 = or disjoint i64 %91, %89
  %93 = shl nuw nsw i64 %87, 1
  %94 = and i64 %93, 43690
  %95 = lshr i64 %87, 1
  %96 = and i64 %95, 21845
  %97 = or disjoint i64 %94, %96
  %98 = shl nuw nsw i64 %92, 1
  %99 = and i64 %98, 170
  %100 = lshr i64 %92, 1
  %101 = and i64 %100, 85
  %102 = lshr i64 %97, 8
  %103 = or disjoint i64 %99, %101
  %104 = xor i64 %102, %103
  %105 = shl nuw nsw i64 %104, 1
  %106 = add i64 %42, %105
  %107 = inttoptr i64 %106 to ptr
  %108 = load i16, ptr %107, align 2, !tbaa !4
  %109 = zext i16 %108 to i64
  %110 = shl i64 %97, 56
  %111 = shl nuw i64 %109, 48
  %112 = xor i64 %110, %111
  %113 = lshr i64 %112, 56
  %114 = shl nuw nsw i64 %109, 8
  %115 = and i64 %114, 65280
  %116 = or disjoint i64 %113, %115
  %117 = lshr i64 %116, 4
  %118 = and i64 %117, 3855
  %119 = shl nuw nsw i64 %116, 4
  %120 = and i64 %119, 61680
  %121 = or disjoint i64 %120, %118
  %122 = lshr i64 %121, 2
  %123 = and i64 %122, 13107
  %124 = shl nuw nsw i64 %121, 2
  %125 = and i64 %124, 52428
  %126 = or disjoint i64 %125, %123
  %127 = lshr i64 %126, 1
  %128 = and i64 %127, 21845
  %129 = shl nuw nsw i64 %126, 1
  %130 = and i64 %129, 43690
  %131 = or disjoint i64 %130, %128
  %132 = shl nuw nsw i64 %131, 8
  %133 = and i64 %132, 65280
  %134 = lshr i64 %131, 8
  %135 = or disjoint i64 %133, %134
  %136 = lshr i64 %135, 4
  %137 = and i64 %136, 3855
  %138 = shl nuw nsw i64 %135, 4
  %139 = and i64 %138, 61680
  %140 = or disjoint i64 %139, %137
  %141 = lshr i64 %6, 20
  %142 = and i64 %141, 15
  %143 = and i64 %78, 240
  %144 = or disjoint i64 %143, %142
  %145 = lshr i64 %140, 2
  %146 = and i64 %145, 13107
  %147 = shl nuw nsw i64 %140, 2
  %148 = and i64 %147, 52428
  %149 = or disjoint i64 %148, %146
  %150 = lshr i64 %144, 2
  %151 = and i64 %150, 51
  %152 = shl nuw nsw i64 %144, 2
  %153 = and i64 %152, 204
  %154 = or disjoint i64 %153, %151
  %155 = lshr i64 %149, 1
  %156 = and i64 %155, 21845
  %157 = shl nuw nsw i64 %149, 1
  %158 = and i64 %157, 43690
  %159 = or disjoint i64 %158, %156
  %160 = shl nuw nsw i64 %154, 1
  %161 = and i64 %160, 170
  %162 = lshr i64 %154, 1
  %163 = and i64 %162, 85
  %164 = lshr i64 %159, 8
  %165 = or disjoint i64 %161, %163
  %166 = xor i64 %164, %165
  %167 = shl nuw nsw i64 %166, 1
  %168 = add i64 %42, %167
  %169 = inttoptr i64 %168 to ptr
  %170 = load i16, ptr %169, align 2, !tbaa !4
  %171 = zext i16 %170 to i64
  %172 = shl i64 %159, 56
  %173 = shl nuw i64 %171, 48
  %174 = xor i64 %172, %173
  %175 = lshr i64 %174, 56
  %176 = shl nuw nsw i64 %171, 8
  %177 = and i64 %176, 65280
  %178 = or disjoint i64 %175, %177
  %179 = lshr i64 %178, 4
  %180 = and i64 %179, 3855
  %181 = shl nuw nsw i64 %178, 4
  %182 = and i64 %181, 61680
  %183 = or disjoint i64 %182, %180
  %184 = lshr i64 %183, 2
  %185 = and i64 %184, 13107
  %186 = shl nuw nsw i64 %183, 2
  %187 = and i64 %186, 52428
  %188 = or disjoint i64 %187, %185
  %189 = lshr i64 %188, 1
  %190 = and i64 %189, 21845
  %191 = shl nuw nsw i64 %188, 1
  %192 = and i64 %191, 43690
  %193 = or disjoint i64 %192, %190
  %194 = shl nuw nsw i64 %193, 8
  %195 = and i64 %194, 65280
  %196 = lshr i64 %193, 8
  %197 = or disjoint i64 %195, %196
  %198 = shl nuw nsw i64 %197, 4
  %199 = and i64 %3, -2
  store i64 -21846, ptr %7, align 4, !tbaa !1
  store i64 21845, ptr %10, align 4, !tbaa !1
  store i64 -13108, ptr %13, align 4, !tbaa !1
  store i64 -3856, ptr %14, align 4, !tbaa !1
  store i64 361072, ptr %15, align 4, !tbaa !1
  %200 = insertelement <2 x i64> poison, i64 %198, i64 0
  %201 = insertelement <2 x i64> %200, i64 %141, i64 1
  %202 = and <2 x i64> %201, <i64 61680, i64 240>
  %203 = insertelement <2 x i64> poison, i64 %197, i64 0
  %204 = insertelement <2 x i64> %203, i64 %6, i64 1
  %205 = lshr <2 x i64> %204, <i64 4, i64 28>
  %206 = and <2 x i64> %205, <i64 3855, i64 15>
  %207 = or disjoint <2 x i64> %206, %202
  %208 = lshr <2 x i64> %207, splat (i64 2)
  %209 = and <2 x i64> %208, <i64 13107, i64 51>
  %210 = shl nuw nsw <2 x i64> %207, splat (i64 2)
  %211 = and <2 x i64> %210, <i64 52428, i64 204>
  %212 = or disjoint <2 x i64> %211, %209
  %213 = lshr <2 x i64> %212, splat (i64 1)
  %214 = and <2 x i64> %213, <i64 21845, i64 85>
  %215 = shl nuw nsw <2 x i64> %212, splat (i64 1)
  %216 = and <2 x i64> %215, <i64 43690, i64 170>
  %217 = or disjoint <2 x i64> %216, %214
  %218 = extractelement <2 x i64> %217, i64 0
  %219 = lshr i64 %218, 8
  %220 = extractelement <2 x i64> %217, i64 1
  %221 = xor i64 %219, %220
  %222 = shl nuw nsw i64 %221, 1
  %223 = add i64 %42, %222
  %224 = inttoptr i64 %223 to ptr
  %225 = load i16, ptr %224, align 2, !tbaa !4
  %226 = zext i16 %225 to i64
  %227 = shl i64 %218, 56
  %228 = shl nuw i64 %226, 48
  %229 = xor i64 %227, %228
  %230 = lshr i64 %229, 56
  %231 = shl nuw nsw i64 %226, 8
  %232 = and i64 %231, 65280
  %233 = or disjoint i64 %230, %232
  %234 = lshr i64 %233, 4
  %235 = and i64 %234, 3855
  %236 = shl nuw nsw i64 %233, 4
  %237 = and i64 %236, 61680
  %238 = or disjoint i64 %237, %235
  %239 = lshr i64 %238, 2
  %240 = and i64 %239, 13107
  %241 = shl nuw nsw i64 %238, 2
  %242 = and i64 %241, 52428
  %243 = or disjoint i64 %242, %240
  %244 = lshr i64 %243, 1
  %245 = and i64 %244, 21845
  %246 = shl nuw nsw i64 %243, 1
  %247 = and i64 %246, 43690
  %248 = or disjoint i64 %247, %245
  store i64 %229, ptr %4, align 4, !tbaa !1
  store i64 %248, ptr %5, align 4, !tbaa !1
  store i64 %235, ptr %9, align 4, !tbaa !1
  store i64 %240, ptr %11, align 4, !tbaa !1
  store i64 %245, ptr %12, align 4, !tbaa !1
  store <2 x i64> %217, ptr %16, align 4, !tbaa !1
  store i64 %161, ptr %17, align 4, !tbaa !1
  store i64 %199, ptr %18, align 4, !tbaa !1
  %249 = musttail call i64 @aot_dispatch(ptr %0, i64 %1)
  ret i64 %249
}

; Function Attrs: nofree nosync nounwind memory(readwrite, inaccessiblemem: none)
define hidden range(i64 2, 4294967299) i64 @tb_14614(ptr initializes((8, 16), (64, 80), (144, 160), (208, 216), (512, 520)) %0, i64 %1) local_unnamed_addr #1 {
entry:
  %2 = getelementptr i8, ptr %0, i64 8
  %3 = getelementptr i8, ptr %0, i64 16
  %4 = load i64, ptr %3, align 4, !tbaa !1
  %5 = getelementptr i8, ptr %0, i64 64
  %6 = getelementptr i8, ptr %0, i64 72
  %7 = getelementptr i8, ptr %0, i64 144
  %8 = getelementptr i8, ptr %0, i64 152
  %9 = getelementptr i8, ptr %0, i64 208
  %10 = getelementptr i8, ptr %0, i64 512
  %11 = add i64 %4, %1
  %12 = add i64 %11, 1352
  %13 = inttoptr i64 %12 to ptr
  %14 = load i64, ptr %13, align 4, !tbaa !4
  %15 = add i64 %11, 1344
  %16 = inttoptr i64 %15 to ptr
  %17 = load i64, ptr %16, align 4, !tbaa !4
  %18 = add i64 %11, 1336
  %19 = inttoptr i64 %18 to ptr
  %20 = load i64, ptr %19, align 4, !tbaa !4
  %21 = add i64 %11, 1328
  %22 = inttoptr i64 %21 to ptr
  %23 = load i64, ptr %22, align 4, !tbaa !4
  %24 = add i64 %11, 1320
  %25 = inttoptr i64 %24 to ptr
  %26 = load i64, ptr %25, align 4, !tbaa !4
  %27 = add i64 %11, 1264
  %28 = inttoptr i64 %27 to ptr
  %29 = load i64, ptr %28, align 4, !tbaa !4
  %30 = add i64 %4, 1360
  %31 = and i64 %14, -2
  store i64 %14, ptr %2, align 4, !tbaa !1
  store i64 %30, ptr %3, align 4, !tbaa !1
  store i64 %17, ptr %5, align 4, !tbaa !1
  store i64 %20, ptr %6, align 4, !tbaa !1
  store i64 %23, ptr %7, align 4, !tbaa !1
  store i64 %26, ptr %8, align 4, !tbaa !1
  store i64 %29, ptr %9, align 4, !tbaa !1
  store i64 %31, ptr %10, align 4, !tbaa !1
  %32 = musttail call i64 @aot_dispatch(ptr %0, i64 %1)
  ret i64 %32
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(argmem: readwrite)
define hidden noundef i64 @tb_1133c(ptr captures(none) initializes((512, 520)) %0, i64 %1) local_unnamed_addr #3 {
entry:
  %2 = getelementptr i8, ptr %0, i64 80
  %3 = load i64, ptr %2, align 4, !tbaa !1
  %4 = icmp eq i64 %3, 0
  %. = select i1 %4, i64 70534, i64 70462
  %5 = getelementptr i8, ptr %0, i64 512
  store i64 %., ptr %5, align 4, !tbaa !1
  ret i64 4294967298
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(read, argmem: readwrite, inaccessiblemem: none)
define hidden noundef i64 @tb_2314c(ptr captures(none) initializes((112, 120), (512, 520)) %0, i64 %1) local_unnamed_addr #0 {
entry:
  %2 = getelementptr i8, ptr %0, i64 104
  %3 = load i64, ptr %2, align 4, !tbaa !1
  %4 = getelementptr i8, ptr %0, i64 112
  %5 = getelementptr i8, ptr %0, i64 128
  %6 = load i64, ptr %5, align 4, !tbaa !1
  %7 = getelementptr i8, ptr %0, i64 512
  %8 = add i64 %1, 88
  %9 = add i64 %8, %3
  %10 = inttoptr i64 %9 to ptr
  %11 = load i64, ptr %10, align 4, !tbaa !4
  %12 = icmp eq i64 %6, 0
  store i64 %11, ptr %4, align 4, !tbaa !1
  %. = select i1 %12, i64 143724, i64 143698
  store i64 %., ptr %7, align 4, !tbaa !1
  ret i64 4294967298
}

; Function Attrs: nofree nosync nounwind memory(readwrite, inaccessiblemem: none)
define hidden range(i64 2, 4294967299) i64 @tb_10adc(ptr initializes((104, 128), (512, 520)) %0, i64 %1) local_unnamed_addr #1 {
entry:
  %2 = getelementptr i8, ptr %0, i64 8
  %3 = load i64, ptr %2, align 4, !tbaa !1
  %4 = getelementptr i8, ptr %0, i64 80
  %5 = load i64, ptr %4, align 4, !tbaa !1
  %6 = getelementptr i8, ptr %0, i64 88
  %7 = load i64, ptr %6, align 4, !tbaa !1
  %8 = getelementptr i8, ptr %0, i64 104
  %9 = getelementptr i8, ptr %0, i64 112
  %10 = getelementptr i8, ptr %0, i64 120
  %11 = getelementptr i8, ptr %0, i64 512
  %12 = add i64 %5, %1
  %13 = inttoptr i64 %12 to ptr
  %14 = load i16, ptr %13, align 2, !tbaa !4
  %15 = and i16 %14, -256
  %16 = lshr i16 %14, 8
  %17 = or disjoint i16 %16, %15
  store i16 %17, ptr %13, align 2, !tbaa !4
  %18 = add i64 %7, %1
  %19 = inttoptr i64 %18 to ptr
  %20 = load i16, ptr %19, align 2, !tbaa !4
  %21 = sext i16 %20 to i64
  %22 = add i64 %1, 2
  %23 = add i64 %22, %5
  %24 = inttoptr i64 %23 to ptr
  %25 = load i16, ptr %24, align 2, !tbaa !4
  %26 = sext i16 %25 to i64
  %27 = shl nsw i64 %21, 48
  %28 = and i64 %21, -256
  %29 = lshr i64 %27, 56
  %30 = or disjoint i64 %29, %28
  %31 = trunc nsw i64 %30 to i16
  store i16 %31, ptr %19, align 2, !tbaa !4
  %32 = add i64 %22, %7
  %33 = inttoptr i64 %32 to ptr
  %34 = load i16, ptr %33, align 2, !tbaa !4
  %35 = sext i16 %34 to i64
  %36 = sub nsw i64 %26, %35
  %37 = and i64 %3, -2
  store i64 %36, ptr %4, align 4, !tbaa !1
  store i64 %27, ptr %8, align 4, !tbaa !1
  store i64 %28, ptr %9, align 4, !tbaa !1
  store i64 %35, ptr %10, align 4, !tbaa !1
  store i64 %37, ptr %11, align 4, !tbaa !1
  %38 = musttail call i64 @aot_dispatch(ptr nonnull %0, i64 %1)
  ret i64 %38
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(readwrite, inaccessiblemem: none)
define hidden noundef i64 @tb_1450a(ptr captures(none) initializes((120, 128), (512, 520)) %0, i64 %1) local_unnamed_addr #4 {
entry:
  %2 = getelementptr i8, ptr %0, i64 8
  %3 = load i64, ptr %2, align 4, !tbaa !1
  %4 = getelementptr i8, ptr %0, i64 16
  %5 = load i64, ptr %4, align 4, !tbaa !1
  %6 = getelementptr i8, ptr %0, i64 32
  %7 = load i64, ptr %6, align 4, !tbaa !1
  %8 = getelementptr i8, ptr %0, i64 64
  %9 = load i64, ptr %8, align 4, !tbaa !1
  %10 = getelementptr i8, ptr %0, i64 72
  %11 = load i64, ptr %10, align 4, !tbaa !1
  %12 = getelementptr i8, ptr %0, i64 80
  %13 = load i64, ptr %12, align 4, !tbaa !1
  %14 = getelementptr i8, ptr %0, i64 88
  %15 = load i64, ptr %14, align 4, !tbaa !1
  %16 = getelementptr i8, ptr %0, i64 96
  %17 = load i64, ptr %16, align 4, !tbaa !1
  %18 = getelementptr i8, ptr %0, i64 104
  %19 = load i64, ptr %18, align 4, !tbaa !1
  %20 = getelementptr i8, ptr %0, i64 120
  %21 = getelementptr i8, ptr %0, i64 144
  %22 = load i64, ptr %21, align 4, !tbaa !1
  %23 = getelementptr i8, ptr %0, i64 152
  %24 = load i64, ptr %23, align 4, !tbaa !1
  %25 = getelementptr i8, ptr %0, i64 208
  %26 = load i64, ptr %25, align 4, !tbaa !1
  %27 = getelementptr i8, ptr %0, i64 512
  %28 = add i64 %1, 511440
  %29 = inttoptr i64 %28 to ptr
  %30 = load i64, ptr %29, align 4, !tbaa !4
  %31 = add i64 %5, -1360
  %32 = add i64 %5, %1
  %33 = add i64 %32, -16
  %34 = inttoptr i64 %33 to ptr
  store i64 %9, ptr %34, align 4, !tbaa !4
  %35 = add i64 %32, -1208
  %36 = inttoptr i64 %35 to ptr
  store i64 %30, ptr %36, align 4, !tbaa !4
  %37 = add i64 %7, %1
  %38 = add i64 %37, %30
  %39 = inttoptr i64 %38 to ptr
  %40 = load i32, ptr %39, align 4, !tbaa !4
  %41 = sext i32 %40 to i64
  %42 = add i64 %32, -32
  %43 = inttoptr i64 %42 to ptr
  store i64 %22, ptr %43, align 4, !tbaa !4
  %44 = add i64 %32, -1224
  %45 = inttoptr i64 %44 to ptr
  store i64 %15, ptr %45, align 4, !tbaa !4
  %46 = add i64 %32, -1240
  %47 = inttoptr i64 %46 to ptr
  store i64 %41, ptr %47, align 4, !tbaa !4
  %48 = add i64 %32, -8
  %49 = inttoptr i64 %48 to ptr
  store i64 %3, ptr %49, align 4, !tbaa !4
  %50 = add i64 %32, -24
  %51 = inttoptr i64 %50 to ptr
  store i64 %11, ptr %51, align 4, !tbaa !4
  %52 = add i64 %32, -40
  %53 = inttoptr i64 %52 to ptr
  store i64 %24, ptr %53, align 4, !tbaa !4
  %54 = add i64 %32, -96
  %55 = inttoptr i64 %54 to ptr
  store i64 %26, ptr %55, align 4, !tbaa !4
  %56 = add i64 %32, -1168
  %57 = inttoptr i64 %56 to ptr
  store i64 %17, ptr %57, align 4, !tbaa !4
  %58 = add i64 %32, -1232
  %59 = inttoptr i64 %58 to ptr
  store i64 %19, ptr %59, align 4, !tbaa !4
  store i64 83278, ptr %2, align 4, !tbaa !1
  store i64 %31, ptr %4, align 4, !tbaa !1
  store i64 %13, ptr %8, align 4, !tbaa !1
  store i64 %17, ptr %10, align 4, !tbaa !1
  store i64 %15, ptr %12, align 4, !tbaa !1
  store i64 37, ptr %14, align 4, !tbaa !1
  store i64 %41, ptr %20, align 4, !tbaa !1
  store i64 %15, ptr %21, align 4, !tbaa !1
  store i64 %19, ptr %23, align 4, !tbaa !1
  store i64 130504, ptr %27, align 4, !tbaa !1
  ret i64 4294967298
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(argmem: readwrite)
define hidden noundef i64 @tb_11388(ptr captures(none) initializes((40, 48), (56, 64), (224, 232), (512, 520)) %0, i64 %1) local_unnamed_addr #3 {
entry:
  %2 = getelementptr i8, ptr %0, i64 40
  %3 = getelementptr i8, ptr %0, i64 56
  %4 = getelementptr i8, ptr %0, i64 80
  %5 = load i64, ptr %4, align 4, !tbaa !1
  %6 = getelementptr i8, ptr %0, i64 224
  %7 = getelementptr i8, ptr %0, i64 512
  %8 = icmp eq i64 %5, 0
  store i64 0, ptr %2, align 4, !tbaa !1
  store i64 0, ptr %3, align 4, !tbaa !1
  store i64 %5, ptr %6, align 4, !tbaa !1
  %. = select i1 %8, i64 70646, i64 70544
  store i64 %., ptr %7, align 4, !tbaa !1
  ret i64 4294967298
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(argmem: readwrite)
define hidden noundef i64 @tb_1492a(ptr captures(none) initializes((512, 520)) %0, i64 %1) local_unnamed_addr #3 {
entry:
  %2 = getelementptr i8, ptr %0, i64 160
  %3 = load i64, ptr %2, align 4, !tbaa !1
  %4 = icmp eq i64 %3, 0
  %. = select i1 %4, i64 86538, i64 84270
  %5 = getelementptr i8, ptr %0, i64 512
  store i64 %., ptr %5, align 4, !tbaa !1
  ret i64 4294967298
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(read, argmem: readwrite, inaccessiblemem: none)
define hidden noundef i64 @tb_10bf6(ptr captures(none) initializes((120, 128), (512, 520)) %0, i64 %1) local_unnamed_addr #0 {
entry:
  %2 = getelementptr i8, ptr %0, i64 120
  %3 = getelementptr i8, ptr %0, i64 136
  %4 = load i64, ptr %3, align 4, !tbaa !1
  %5 = getelementptr i8, ptr %0, i64 512
  %6 = add i64 %1, 96
  %7 = add i64 %6, %4
  %8 = inttoptr i64 %7 to ptr
  %9 = load i16, ptr %8, align 2, !tbaa !4
  %10 = zext i16 %9 to i64
  store i64 %10, ptr %2, align 4, !tbaa !1
  store i64 68480, ptr %5, align 4, !tbaa !1
  ret i64 4294967298
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(read, argmem: readwrite, inaccessiblemem: none)
define hidden noundef i64 @tb_10dbe(ptr captures(none) initializes((112, 120), (512, 520)) %0, i64 %1) local_unnamed_addr #0 {
entry:
  %2 = getelementptr i8, ptr %0, i64 64
  %3 = load i64, ptr %2, align 4, !tbaa !1
  %4 = getelementptr i8, ptr %0, i64 72
  %5 = load i64, ptr %4, align 4, !tbaa !1
  %6 = getelementptr i8, ptr %0, i64 112
  %7 = getelementptr i8, ptr %0, i64 152
  %8 = load i64, ptr %7, align 4, !tbaa !1
  %9 = getelementptr i8, ptr %0, i64 512
  %10 = add i64 %8, %1
  %11 = inttoptr i64 %10 to ptr
  %12 = load i64, ptr %11, align 4, !tbaa !4
  %13 = add i64 %3, -1
  %14 = icmp eq i64 %5, 0
  store i64 %13, ptr %2, align 4, !tbaa !1
  store i64 %12, ptr %6, align 4, !tbaa !1
  %. = select i1 %14, i64 69072, i64 69062
  store i64 %., ptr %9, align 4, !tbaa !1
  ret i64 4294967298
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(read, argmem: readwrite, inaccessiblemem: none)
define hidden noundef i64 @tb_10b14(ptr captures(none) initializes((112, 136), (512, 520)) %0, i64 %1) local_unnamed_addr #0 {
entry:
  %2 = getelementptr i8, ptr %0, i64 80
  %3 = load i64, ptr %2, align 4, !tbaa !1
  %4 = getelementptr i8, ptr %0, i64 112
  %5 = getelementptr i8, ptr %0, i64 120
  %6 = getelementptr i8, ptr %0, i64 128
  %7 = getelementptr i8, ptr %0, i64 512
  %8 = add i64 %3, %1
  %9 = inttoptr i64 %8 to ptr
  %10 = load i16, ptr %9, align 2, !tbaa !4
  %11 = sext i16 %10 to i64
  %12 = shl i64 %11, 56
  %13 = lshr i64 %12, 63
  %14 = icmp sgt i64 %12, -1
  store i64 %12, ptr %4, align 4, !tbaa !1
  store i64 %13, ptr %5, align 4, !tbaa !1
  store i64 %11, ptr %6, align 4, !tbaa !1
  %. = select i1 %14, i64 68392, i64 68386
  store i64 %., ptr %7, align 4, !tbaa !1
  ret i64 4294967298
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(read, argmem: readwrite, inaccessiblemem: none)
define hidden noundef i64 @tb_1159e(ptr captures(none) initializes((88, 96), (104, 112), (512, 520)) %0, i64 %1) local_unnamed_addr #0 {
entry:
  %2 = getelementptr i8, ptr %0, i64 64
  %3 = load i64, ptr %2, align 4, !tbaa !1
  %4 = getelementptr i8, ptr %0, i64 80
  %5 = load i64, ptr %4, align 4, !tbaa !1
  %6 = getelementptr i8, ptr %0, i64 88
  %7 = getelementptr i8, ptr %0, i64 96
  %8 = load i64, ptr %7, align 4, !tbaa !1
  %9 = getelementptr i8, ptr %0, i64 104
  %10 = getelementptr i8, ptr %0, i64 112
  %11 = load i64, ptr %10, align 4, !tbaa !1
  %12 = getelementptr i8, ptr %0, i64 120
  %13 = load i64, ptr %12, align 4, !tbaa !1
  %14 = getelementptr i8, ptr %0, i64 168
  %15 = load i64, ptr %14, align 4, !tbaa !1
  %16 = getelementptr i8, ptr %0, i64 512
  %17 = shl i64 %13, 2
  %18 = and i64 %17, 17179869180
  %19 = add i64 %3, %1
  %20 = add i64 %19, %18
  %21 = inttoptr i64 %20 to ptr
  %22 = load i32, ptr %21, align 4, !tbaa !4
  %23 = sext i32 %22 to i64
  %24 = add i64 %5, 10
  %25 = add i64 %8, %23
  %26 = icmp slt i64 %11, %23
  %27 = zext i1 %26 to i64
  %.not = icmp slt i64 %15, %25
  store i64 %27, ptr %6, align 4, !tbaa !1
  store i64 %25, ptr %7, align 4, !tbaa !1
  store i64 %24, ptr %9, align 4, !tbaa !1
  store i64 %23, ptr %10, align 4, !tbaa !1
  %. = select i1 %.not, i64 71098, i64 71052
  store i64 %., ptr %16, align 4, !tbaa !1
  ret i64 4294967298
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(argmem: readwrite)
define hidden noundef i64 @tb_4ec32(ptr captures(none) initializes((512, 520)) %0, i64 %1) local_unnamed_addr #3 {
entry:
  %2 = getelementptr i8, ptr %0, i64 88
  %3 = load i64, ptr %2, align 4, !tbaa !1
  %4 = getelementptr i8, ptr %0, i64 168
  %5 = load i64, ptr %4, align 4, !tbaa !1
  %.not = icmp ult i64 %5, %3
  %. = select i1 %.not, i64 322614, i64 322860
  %6 = getelementptr i8, ptr %0, i64 512
  store i64 %., ptr %6, align 4, !tbaa !1
  ret i64 4294967298
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(read, argmem: readwrite, inaccessiblemem: none)
define hidden noundef i64 @tb_4e432(ptr captures(none) initializes((80, 88), (112, 128), (512, 520)) %0, i64 %1) local_unnamed_addr #0 {
entry:
  %2 = getelementptr i8, ptr %0, i64 80
  %3 = getelementptr i8, ptr %0, i64 96
  %4 = load i64, ptr %3, align 4, !tbaa !1
  %5 = getelementptr i8, ptr %0, i64 112
  %6 = getelementptr i8, ptr %0, i64 120
  %7 = getelementptr i8, ptr %0, i64 512
  %8 = add i64 %4, %1
  %9 = inttoptr i64 %8 to ptr
  %10 = load i8, ptr %9, align 1, !tbaa !4
  %11 = zext i8 %10 to i64
  %.not = icmp eq i8 %10, 8
  store i64 255, ptr %2, align 4, !tbaa !1
  store i64 %11, ptr %5, align 4, !tbaa !1
  store i64 8, ptr %6, align 4, !tbaa !1
  %. = select i1 %.not, i64 320576, i64 320376
  store i64 %., ptr %7, align 4, !tbaa !1
  ret i64 4294967298
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(readwrite, inaccessiblemem: none)
define hidden noundef i64 @tb_15bbe(ptr captures(none) initializes((120, 128), (512, 520)) %0, i64 %1) local_unnamed_addr #4 {
entry:
  %2 = getelementptr i8, ptr %0, i64 8
  %3 = load i64, ptr %2, align 4, !tbaa !1
  %4 = getelementptr i8, ptr %0, i64 16
  %5 = load i64, ptr %4, align 4, !tbaa !1
  %6 = getelementptr i8, ptr %0, i64 72
  %7 = load i64, ptr %6, align 4, !tbaa !1
  %8 = getelementptr i8, ptr %0, i64 80
  %9 = load i64, ptr %8, align 4, !tbaa !1
  %10 = getelementptr i8, ptr %0, i64 120
  %11 = getelementptr i8, ptr %0, i64 512
  %12 = add i64 %1, 192
  %13 = add i64 %12, %9
  %14 = inttoptr i64 %13 to ptr
  %15 = load i32, ptr %14, align 4, !tbaa !4
  %16 = sext i32 %15 to i64
  %17 = add i64 %5, -272
  %18 = add i64 %5, %1
  %19 = add i64 %18, -8
  %20 = inttoptr i64 %19 to ptr
  store i64 %3, ptr %20, align 4, !tbaa !4
  %21 = add i64 %18, -24
  %22 = inttoptr i64 %21 to ptr
  store i64 %7, ptr %22, align 4, !tbaa !4
  %.not = icmp eq i32 %15, 0
  store i64 %17, ptr %4, align 4, !tbaa !1
  store i64 %16, ptr %10, align 4, !tbaa !1
  %. = select i1 %.not, i64 89034, i64 89160
  store i64 %., ptr %11, align 4, !tbaa !1
  ret i64 4294967298
}

; Function Attrs: nofree nosync nounwind memory(readwrite, inaccessiblemem: none)
define hidden range(i64 2, 4294967299) i64 @tb_30a64(ptr initializes((8, 16), (80, 88), (512, 520)) %0, i64 %1) local_unnamed_addr #1 {
entry:
  %2 = getelementptr i8, ptr %0, i64 8
  %3 = getelementptr i8, ptr %0, i64 16
  %4 = load i64, ptr %3, align 4, !tbaa !1
  %5 = getelementptr i8, ptr %0, i64 80
  %6 = getelementptr i8, ptr %0, i64 120
  %7 = load i64, ptr %6, align 4, !tbaa !1
  %8 = getelementptr i8, ptr %0, i64 512
  %9 = add i64 %1, 24
  %10 = add i64 %9, %4
  %11 = inttoptr i64 %10 to ptr
  %12 = load i64, ptr %11, align 4, !tbaa !4
  %13 = add i64 %4, 32
  %14 = and i64 %12, -2
  store i64 %12, ptr %2, align 4, !tbaa !1
  store i64 %13, ptr %3, align 4, !tbaa !1
  store i64 %7, ptr %5, align 4, !tbaa !1
  store i64 %14, ptr %8, align 4, !tbaa !1
  %15 = musttail call i64 @aot_dispatch(ptr %0, i64 %1)
  ret i64 %15
}

; Function Attrs: nofree nosync nounwind memory(readwrite, inaccessiblemem: none)
define hidden range(i64 2, 4294967299) i64 @tb_30a4c(ptr initializes((8, 16), (96, 120), (128, 136), (512, 520)) %0, i64 %1) local_unnamed_addr #1 {
entry:
  %2 = getelementptr i8, ptr %0, i64 8
  %3 = getelementptr i8, ptr %0, i64 16
  %4 = load i64, ptr %3, align 4, !tbaa !1
  %5 = getelementptr i8, ptr %0, i64 80
  %6 = load i64, ptr %5, align 4, !tbaa !1
  %7 = getelementptr i8, ptr %0, i64 96
  %8 = getelementptr i8, ptr %0, i64 104
  %9 = getelementptr i8, ptr %0, i64 112
  %10 = getelementptr i8, ptr %0, i64 120
  %11 = load i64, ptr %10, align 4, !tbaa !1
  %12 = getelementptr i8, ptr %0, i64 128
  %13 = getelementptr i8, ptr %0, i64 512
  %14 = add i64 %4, %1
  %15 = add i64 %14, 8
  %16 = inttoptr i64 %15 to ptr
  %17 = load i64, ptr %16, align 4, !tbaa !4
  %18 = add i64 %1, 24
  %19 = add i64 %18, %17
  %20 = inttoptr i64 %19 to ptr
  %21 = load i64, ptr %20, align 4, !tbaa !4
  %22 = add i64 %17, %1
  %23 = add i64 %22, 16
  %24 = inttoptr i64 %23 to ptr
  %25 = load i64, ptr %24, align 4, !tbaa !4
  %26 = add i64 %21, 1
  %27 = shl i64 %21, 3
  store i64 %26, ptr %20, align 4, !tbaa !4
  %28 = add i64 %27, %25
  %29 = add i64 %28, %1
  %30 = inttoptr i64 %29 to ptr
  store i64 %6, ptr %30, align 4, !tbaa !4
  %31 = add i64 %18, %4
  %32 = inttoptr i64 %31 to ptr
  %33 = load i64, ptr %32, align 4, !tbaa !4
  %34 = add i64 %4, 32
  %35 = and i64 %33, -2
  store i64 %33, ptr %2, align 4, !tbaa !1
  store i64 %34, ptr %3, align 4, !tbaa !1
  store i64 %11, ptr %5, align 4, !tbaa !1
  store i64 %26, ptr %7, align 4, !tbaa !1
  store i64 %25, ptr %8, align 4, !tbaa !1
  store i64 %28, ptr %9, align 4, !tbaa !1
  store i64 %17, ptr %12, align 4, !tbaa !1
  store i64 %35, ptr %13, align 4, !tbaa !1
  %36 = musttail call i64 @aot_dispatch(ptr nonnull %0, i64 %1)
  ret i64 %36
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(read, argmem: readwrite, inaccessiblemem: none)
define hidden noundef i64 @tb_316e6(ptr captures(none) initializes((120, 128), (512, 520)) %0, i64 %1) local_unnamed_addr #0 {
entry:
  %2 = getelementptr i8, ptr %0, i64 64
  %3 = load i64, ptr %2, align 4, !tbaa !1
  %4 = getelementptr i8, ptr %0, i64 120
  %5 = getelementptr i8, ptr %0, i64 512
  %6 = add i64 %1, -232
  %7 = add i64 %6, %3
  %8 = inttoptr i64 %7 to ptr
  %9 = load i64, ptr %8, align 4, !tbaa !4
  %10 = add i64 %9, 1
  store i64 %10, ptr %4, align 4, !tbaa !1
  store i64 201536, ptr %5, align 4, !tbaa !1
  ret i64 4294967298
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(argmem: readwrite)
define hidden noundef i64 @tb_4ec68(ptr captures(none) initializes((512, 520)) %0, i64 %1) local_unnamed_addr #3 {
entry:
  %2 = getelementptr i8, ptr %0, i64 96
  %3 = load i64, ptr %2, align 4, !tbaa !1
  %.not = icmp eq i64 %3, 0
  %. = select i1 %.not, i64 322666, i64 322784
  %4 = getelementptr i8, ptr %0, i64 512
  store i64 %., ptr %4, align 4, !tbaa !1
  ret i64 4294967298
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(readwrite, inaccessiblemem: none)
define hidden noundef i64 @tb_10dfa(ptr captures(none) initializes((48, 56), (512, 520)) %0, i64 %1) local_unnamed_addr #4 {
entry:
  %2 = getelementptr i8, ptr %0, i64 8
  %3 = load i64, ptr %2, align 4, !tbaa !1
  %4 = getelementptr i8, ptr %0, i64 16
  %5 = load i64, ptr %4, align 4, !tbaa !1
  %6 = getelementptr i8, ptr %0, i64 48
  %7 = getelementptr i8, ptr %0, i64 64
  %8 = load i64, ptr %7, align 4, !tbaa !1
  %9 = getelementptr i8, ptr %0, i64 72
  %10 = load i64, ptr %9, align 4, !tbaa !1
  %11 = getelementptr i8, ptr %0, i64 80
  %12 = load i64, ptr %11, align 4, !tbaa !1
  %13 = getelementptr i8, ptr %0, i64 144
  %14 = load i64, ptr %13, align 4, !tbaa !1
  %15 = getelementptr i8, ptr %0, i64 152
  %16 = load i64, ptr %15, align 4, !tbaa !1
  %17 = getelementptr i8, ptr %0, i64 512
  %18 = add i64 %12, %1
  %19 = add i64 %18, 4
  %20 = inttoptr i64 %19 to ptr
  %21 = load i16, ptr %20, align 2, !tbaa !4
  %22 = sext i16 %21 to i64
  %23 = add i64 %5, -64
  %24 = add i64 %5, %1
  %25 = add i64 %24, -16
  %26 = inttoptr i64 %25 to ptr
  store i64 %8, ptr %26, align 4, !tbaa !4
  %27 = add i64 %24, -8
  %28 = inttoptr i64 %27 to ptr
  store i64 %3, ptr %28, align 4, !tbaa !4
  %29 = add i64 %24, -24
  %30 = inttoptr i64 %29 to ptr
  store i64 %10, ptr %30, align 4, !tbaa !4
  %31 = add i64 %24, -32
  %32 = inttoptr i64 %31 to ptr
  store i64 %14, ptr %32, align 4, !tbaa !4
  %33 = add i64 %24, -40
  %34 = inttoptr i64 %33 to ptr
  store i64 %16, ptr %34, align 4, !tbaa !4
  %35 = add i64 %18, 56
  %36 = inttoptr i64 %35 to ptr
  %37 = load i64, ptr %36, align 4, !tbaa !4
  %38 = icmp slt i16 %21, 1
  store i64 %23, ptr %4, align 4, !tbaa !1
  store i64 %22, ptr %6, align 4, !tbaa !1
  store i64 %37, ptr %7, align 4, !tbaa !1
  %. = select i1 %38, i64 69622, i64 69136
  store i64 %., ptr %17, align 4, !tbaa !1
  ret i64 4294967298
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(read, argmem: readwrite, inaccessiblemem: none)
define hidden noundef i64 @tb_1167c(ptr captures(none) initializes((88, 96), (120, 128), (512, 520)) %0, i64 %1) local_unnamed_addr #0 {
entry:
  %2 = getelementptr i8, ptr %0, i64 64
  %3 = load i64, ptr %2, align 4, !tbaa !1
  %4 = getelementptr i8, ptr %0, i64 80
  %5 = load i64, ptr %4, align 4, !tbaa !1
  %6 = getelementptr i8, ptr %0, i64 88
  %7 = getelementptr i8, ptr %0, i64 96
  %8 = load i64, ptr %7, align 4, !tbaa !1
  %9 = getelementptr i8, ptr %0, i64 104
  %10 = load i64, ptr %9, align 4, !tbaa !1
  %11 = getelementptr i8, ptr %0, i64 112
  %12 = load i64, ptr %11, align 4, !tbaa !1
  %13 = getelementptr i8, ptr %0, i64 120
  %14 = getelementptr i8, ptr %0, i64 128
  %15 = load i64, ptr %14, align 4, !tbaa !1
  %16 = getelementptr i8, ptr %0, i64 168
  %17 = load i64, ptr %16, align 4, !tbaa !1
  %18 = getelementptr i8, ptr %0, i64 512
  %19 = add i64 %15, %10
  %20 = shl i64 %19, 2
  %21 = and i64 %20, 17179869180
  %22 = add i64 %3, %1
  %23 = add i64 %22, %21
  %24 = inttoptr i64 %23 to ptr
  %25 = load i32, ptr %24, align 4, !tbaa !4
  %26 = sext i32 %25 to i64
  %27 = add i64 %5, 10
  %28 = add i64 %8, %26
  %29 = icmp slt i64 %12, %26
  %30 = zext i1 %29 to i64
  %.not = icmp slt i64 %17, %28
  store i64 %30, ptr %6, align 4, !tbaa !1
  store i64 %28, ptr %7, align 4, !tbaa !1
  store i64 %26, ptr %11, align 4, !tbaa !1
  store i64 %27, ptr %13, align 4, !tbaa !1
  %. = select i1 %.not, i64 71324, i64 71274
  store i64 %., ptr %18, align 4, !tbaa !1
  ret i64 4294967298
}

; Function Attrs: nofree nosync nounwind memory(readwrite, inaccessiblemem: none)
define hidden range(i64 2, 4294967299) i64 @tb_11de0(ptr initializes((48, 56), (96, 144), (224, 248), (512, 520)) %0, i64 %1) local_unnamed_addr #1 {
entry:
  %2 = getelementptr i8, ptr %0, i64 8
  %3 = load i64, ptr %2, align 4, !tbaa !1
  %4 = getelementptr i8, ptr %0, i64 48
  %5 = getelementptr i8, ptr %0, i64 80
  %6 = load i64, ptr %5, align 4, !tbaa !1
  %7 = getelementptr i8, ptr %0, i64 88
  %8 = load i64, ptr %7, align 4, !tbaa !1
  %9 = getelementptr i8, ptr %0, i64 96
  %10 = getelementptr i8, ptr %0, i64 104
  %11 = getelementptr i8, ptr %0, i64 112
  %12 = getelementptr i8, ptr %0, i64 120
  %13 = getelementptr i8, ptr %0, i64 128
  %14 = getelementptr i8, ptr %0, i64 136
  %15 = getelementptr i8, ptr %0, i64 224
  %16 = getelementptr i8, ptr %0, i64 232
  %17 = getelementptr i8, ptr %0, i64 512
  %18 = shl i64 %8, 8
  %19 = and i64 %18, 65280
  %20 = lshr i64 %8, 8
  %21 = or i64 %19, %20
  %22 = lshr i64 %21, 4
  %23 = and i64 %22, 268435215
  %24 = shl nuw nsw i64 %21, 4
  %25 = and i64 %24, 61680
  %26 = or i64 %25, %23
  %27 = lshr i64 %26, 2
  %28 = and i64 %27, 67105587
  %29 = shl nuw nsw i64 %26, 2
  %30 = and i64 %29, 52428
  %31 = or i64 %30, %28
  %32 = shl nuw nsw i64 %31, 1
  %33 = and i64 %32, 43690
  %34 = lshr i64 %31, 1
  %35 = and i64 %34, 33543509
  %36 = or i64 %33, %35
  %37 = lshr i64 %36, 8
  %trunc = trunc i64 %6 to i8
  %rev = tail call i8 @llvm.bitreverse.i8(i8 %trunc)
  %38 = zext i8 %rev to i64
  %39 = xor i64 %37, %38
  %40 = shl nuw nsw i64 %39, 1
  %41 = add i64 %1, 361072
  %42 = add i64 %41, %40
  %43 = inttoptr i64 %42 to ptr
  %44 = load i16, ptr %43, align 2, !tbaa !4
  %45 = zext i16 %44 to i64
  %46 = shl i64 %36, 56
  %47 = shl nuw i64 %45, 48
  %48 = xor i64 %46, %47
  %49 = lshr i64 %48, 56
  %50 = shl nuw nsw i64 %45, 8
  %51 = and i64 %50, 65280
  %52 = or disjoint i64 %49, %51
  %53 = lshr i64 %52, 4
  %54 = and i64 %53, 3855
  %55 = shl nuw nsw i64 %52, 4
  %56 = and i64 %55, 61680
  %57 = or disjoint i64 %56, %54
  %58 = lshr i64 %57, 2
  %59 = and i64 %58, 13107
  %60 = shl nuw nsw i64 %57, 2
  %61 = and i64 %60, 52428
  %62 = or disjoint i64 %61, %59
  %63 = lshr i64 %62, 1
  %64 = and i64 %63, 21845
  %65 = shl nuw nsw i64 %62, 1
  %66 = and i64 %65, 43690
  %67 = or disjoint i64 %66, %64
  %68 = shl nuw nsw i64 %67, 8
  %69 = and i64 %68, 65280
  %70 = lshr i64 %67, 8
  %71 = or disjoint i64 %69, %70
  %72 = shl nuw nsw i64 %71, 4
  %73 = lshr i64 %6, 12
  %74 = and i64 %3, -2
  store i64 -3856, ptr %4, align 4, !tbaa !1
  store i64 21845, ptr %7, align 4, !tbaa !1
  store i64 -21846, ptr %13, align 4, !tbaa !1
  store i64 -13108, ptr %14, align 4, !tbaa !1
  store i64 361072, ptr %15, align 4, !tbaa !1
  %75 = insertelement <2 x i64> poison, i64 %72, i64 0
  %76 = insertelement <2 x i64> %75, i64 %73, i64 1
  %77 = and <2 x i64> %76, <i64 61680, i64 1048575>
  %78 = insertelement <2 x i64> poison, i64 %71, i64 0
  %79 = insertelement <2 x i64> %78, i64 %6, i64 1
  %80 = lshr <2 x i64> %79, splat (i64 4)
  %81 = and <2 x i64> %80, <i64 3855, i64 240>
  %82 = or <2 x i64> %81, %77
  %83 = lshr <2 x i64> %82, splat (i64 2)
  %84 = and <2 x i64> %83, <i64 13107, i64 51>
  %85 = shl nuw nsw <2 x i64> %82, splat (i64 2)
  %86 = and <2 x i64> %85, <i64 52428, i64 204>
  %87 = or disjoint <2 x i64> %86, %84
  %88 = lshr <2 x i64> %87, splat (i64 1)
  %89 = and <2 x i64> %88, <i64 21845, i64 85>
  %90 = shl nuw nsw <2 x i64> %87, splat (i64 1)
  %91 = and <2 x i64> %90, <i64 43690, i64 170>
  %92 = or disjoint <2 x i64> %91, %89
  %93 = extractelement <2 x i64> %92, i64 0
  %94 = lshr i64 %93, 8
  %95 = extractelement <2 x i64> %92, i64 1
  %96 = xor i64 %94, %95
  %97 = shl nuw nsw i64 %96, 1
  %98 = add i64 %41, %97
  %99 = inttoptr i64 %98 to ptr
  %100 = load i16, ptr %99, align 2, !tbaa !4
  %101 = zext i16 %100 to i64
  %102 = shl i64 %93, 56
  %103 = shl nuw i64 %101, 48
  %104 = xor i64 %102, %103
  %105 = lshr i64 %104, 56
  %106 = shl nuw nsw i64 %101, 8
  %107 = and i64 %106, 65280
  %108 = or disjoint i64 %105, %107
  %109 = lshr i64 %108, 4
  %110 = and i64 %109, 3855
  %111 = shl nuw nsw i64 %108, 4
  %112 = and i64 %111, 61680
  %113 = or disjoint i64 %112, %110
  %114 = lshr i64 %113, 2
  %115 = and i64 %114, 13107
  %116 = shl nuw nsw i64 %113, 2
  %117 = and i64 %116, 52428
  %118 = or disjoint i64 %117, %115
  %119 = lshr i64 %118, 1
  %120 = and i64 %119, 21845
  %121 = shl nuw nsw i64 %118, 1
  %122 = and i64 %121, 43690
  %123 = or disjoint i64 %122, %120
  store i64 %123, ptr %5, align 4, !tbaa !1
  store i64 %105, ptr %9, align 4, !tbaa !1
  store i64 %110, ptr %10, align 4, !tbaa !1
  store i64 %115, ptr %11, align 4, !tbaa !1
  store i64 %120, ptr %12, align 4, !tbaa !1
  store <2 x i64> %92, ptr %16, align 4, !tbaa !1
  store i64 %74, ptr %17, align 4, !tbaa !1
  %124 = musttail call i64 @aot_dispatch(ptr %0, i64 %1)
  ret i64 %124
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(write, argmem: readwrite, inaccessiblemem: none)
define hidden noundef i64 @tb_1118e(ptr captures(none) initializes((512, 520)) %0, i64 %1) local_unnamed_addr #2 {
entry:
  %2 = getelementptr i8, ptr %0, i64 64
  %3 = load i64, ptr %2, align 4, !tbaa !1
  %4 = getelementptr i8, ptr %0, i64 72
  %5 = load i64, ptr %4, align 4, !tbaa !1
  %6 = getelementptr i8, ptr %0, i64 80
  %7 = load i64, ptr %6, align 4, !tbaa !1
  %8 = getelementptr i8, ptr %0, i64 144
  %9 = load i64, ptr %8, align 4, !tbaa !1
  %10 = getelementptr i8, ptr %0, i64 512
  %11 = add i64 %1, 98
  %12 = add i64 %11, %3
  %13 = inttoptr i64 %12 to ptr
  %14 = trunc i64 %7 to i16
  store i16 %14, ptr %13, align 2, !tbaa !4
  %15 = add i64 %5, 1
  %.not = icmp eq i64 %9, %15
  store i64 %15, ptr %4, align 4, !tbaa !1
  %. = select i1 %.not, i64 70040, i64 69988
  store i64 %., ptr %10, align 4, !tbaa !1
  ret i64 4294967298
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(argmem: readwrite)
define hidden noundef i64 @tb_11b8a(ptr captures(none) initializes((104, 112), (512, 520)) %0, i64 %1) local_unnamed_addr #3 {
entry:
  %2 = getelementptr i8, ptr %0, i64 72
  %3 = load i64, ptr %2, align 4, !tbaa !1
  %4 = getelementptr i8, ptr %0, i64 104
  %5 = getelementptr i8, ptr %0, i64 192
  %6 = load i64, ptr %5, align 4, !tbaa !1
  %7 = getelementptr i8, ptr %0, i64 512
  %.not = icmp ult i64 %3, %6
  store i64 44, ptr %4, align 4, !tbaa !1
  %. = select i1 %.not, i64 72594, i64 72616
  store i64 %., ptr %7, align 4, !tbaa !1
  ret i64 4294967298
}

; Function Attrs: nofree nosync nounwind memory(readwrite, inaccessiblemem: none)
define hidden range(i64 2, 4294967299) i64 @tb_10fb0(ptr initializes((8, 16), (64, 72), (80, 88), (144, 160), (512, 520)) %0, i64 %1) local_unnamed_addr #1 {
entry:
  %2 = getelementptr i8, ptr %0, i64 8
  %3 = getelementptr i8, ptr %0, i64 16
  %4 = load i64, ptr %3, align 4, !tbaa !1
  %5 = getelementptr i8, ptr %0, i64 64
  %6 = getelementptr i8, ptr %0, i64 72
  %7 = load i64, ptr %6, align 4, !tbaa !1
  %8 = getelementptr i8, ptr %0, i64 80
  %9 = getelementptr i8, ptr %0, i64 144
  %10 = getelementptr i8, ptr %0, i64 152
  %11 = getelementptr i8, ptr %0, i64 512
  %12 = add i64 %4, %1
  %13 = add i64 %12, 56
  %14 = inttoptr i64 %13 to ptr
  %15 = load i64, ptr %14, align 4, !tbaa !4
  %16 = add i64 %12, 48
  %17 = inttoptr i64 %16 to ptr
  %18 = load i64, ptr %17, align 4, !tbaa !4
  %19 = add i64 %12, 32
  %20 = inttoptr i64 %19 to ptr
  %21 = load i64, ptr %20, align 4, !tbaa !4
  %22 = add i64 %12, 24
  %23 = inttoptr i64 %22 to ptr
  %24 = load i64, ptr %23, align 4, !tbaa !4
  %25 = add i64 %12, 40
  %26 = inttoptr i64 %25 to ptr
  %27 = load i64, ptr %26, align 4, !tbaa !4
  %28 = add i64 %4, 64
  %29 = and i64 %15, -2
  store i64 %15, ptr %2, align 4, !tbaa !1
  store i64 %28, ptr %3, align 4, !tbaa !1
  store i64 %18, ptr %5, align 4, !tbaa !1
  store i64 %27, ptr %6, align 4, !tbaa !1
  store i64 %7, ptr %8, align 4, !tbaa !1
  store i64 %21, ptr %9, align 4, !tbaa !1
  store i64 %24, ptr %10, align 4, !tbaa !1
  store i64 %29, ptr %11, align 4, !tbaa !1
  %30 = musttail call i64 @aot_dispatch(ptr %0, i64 %1)
  ret i64 %30
}

; Function Attrs: nofree nosync nounwind memory(readwrite, inaccessiblemem: none)
define hidden range(i64 2, 4294967299) i64 @tb_10f44(ptr initializes((120, 128), (512, 520)) %0, i64 %1) local_unnamed_addr #1 {
entry:
  %2 = getelementptr i8, ptr %0, i64 104
  %3 = load i64, ptr %2, align 4, !tbaa !1
  %4 = getelementptr i8, ptr %0, i64 120
  %5 = getelementptr i8, ptr %0, i64 144
  %6 = load i64, ptr %5, align 4, !tbaa !1
  %7 = getelementptr i8, ptr %0, i64 512
  %8 = add i64 %1, 8
  %9 = add i64 %8, %6
  %10 = inttoptr i64 %9 to ptr
  %11 = load i64, ptr %10, align 4, !tbaa !4
  %12 = add i64 %1, 2
  %13 = add i64 %12, %11
  %14 = inttoptr i64 %13 to ptr
  %15 = load i16, ptr %14, align 2, !tbaa !4
  %16 = sext i16 %15 to i64
  %.not = icmp eq i64 %3, %16
  store i64 %16, ptr %4, align 4, !tbaa !1
  br i1 %.not, label %fall, label %L0

L0:                                               ; preds = %entry
  store i64 69436, ptr %7, align 4, !tbaa !1
  ret i64 4294967298

fall:                                             ; preds = %entry
  %17 = getelementptr i8, ptr %0, i64 8
  %18 = getelementptr i8, ptr %0, i64 64
  %19 = load i64, ptr %18, align 4, !tbaa !1
  %20 = getelementptr i8, ptr %0, i64 72
  %21 = load i64, ptr %20, align 4, !tbaa !1
  %22 = getelementptr i8, ptr %0, i64 80
  %23 = getelementptr i8, ptr %0, i64 88
  %24 = add i64 %8, %19
  %25 = inttoptr i64 %24 to ptr
  %26 = load i64, ptr %25, align 4, !tbaa !4
  %27 = add i64 %26, %1
  %28 = inttoptr i64 %27 to ptr
  %29 = load i16, ptr %28, align 2, !tbaa !4
  %30 = sext i16 %29 to i64
  store i64 69468, ptr %17, align 4, !tbaa !1
  store i64 %30, ptr %22, align 4, !tbaa !1
  store i64 %21, ptr %23, align 4, !tbaa !1
  store i64 %26, ptr %4, align 4, !tbaa !1
  store i64 74642, ptr %7, align 4, !tbaa !1
  %31 = musttail call range(i64 2, 4294967299) i64 @tb_12392(ptr nonnull %0, i64 %1)
  ret i64 %31
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(argmem: readwrite)
define hidden noundef i64 @tb_11192(ptr captures(none) initializes((512, 520)) %0, i64 %1) local_unnamed_addr #3 {
entry:
  %2 = getelementptr i8, ptr %0, i64 72
  %3 = load i64, ptr %2, align 4, !tbaa !1
  %4 = getelementptr i8, ptr %0, i64 144
  %5 = load i64, ptr %4, align 4, !tbaa !1
  %6 = getelementptr i8, ptr %0, i64 512
  %7 = add i64 %3, 1
  %.not = icmp eq i64 %5, %7
  store i64 %7, ptr %2, align 4, !tbaa !1
  %. = select i1 %.not, i64 70040, i64 69988
  store i64 %., ptr %6, align 4, !tbaa !1
  ret i64 4294967298
}

; Function Attrs: nofree norecurse nosync nounwind memory(readwrite, inaccessiblemem: none)
define hidden noundef i64 @tb_116c2(ptr captures(none) initializes((112, 128)) %0, i64 %1) local_unnamed_addr #5 {
entry:
  %2 = getelementptr i8, ptr %0, i64 72
  %3 = load i64, ptr %2, align 4, !tbaa !1
  %4 = getelementptr i8, ptr %0, i64 88
  %5 = load i64, ptr %4, align 4, !tbaa !1
  %6 = getelementptr i8, ptr %0, i64 96
  %7 = load i64, ptr %6, align 4, !tbaa !1
  %8 = getelementptr i8, ptr %0, i64 104
  %9 = getelementptr i8, ptr %0, i64 152
  %10 = load i64, ptr %9, align 4, !tbaa !1
  %11 = getelementptr i8, ptr %0, i64 160
  %12 = load i64, ptr %11, align 4, !tbaa !1
  %invariant.op = add i64 %10, %1
  %.promoted = load i64, ptr %8, align 4, !tbaa !1
  %13 = add i64 %.promoted, %7
  %14 = shl i64 %13, 1
  %15 = and i64 %14, 8589934590
  %.reass135 = add i64 %15, %invariant.op
  %16 = inttoptr i64 %.reass135 to ptr
  %17 = load i16, ptr %16, align 2, !tbaa !4
  %18 = zext i16 %17 to i64
  %19 = add i64 %.promoted, 1
  %20 = sub i64 %18, %12
  %21 = trunc i64 %20 to i16
  store i16 %21, ptr %16, align 2, !tbaa !4
  %22 = icmp ult i64 %19, %3
  br i1 %22, label %L0, label %fall

L0:                                               ; preds = %entry, %L0
  %23 = phi i64 [ %30, %L0 ], [ %19, %entry ]
  %24 = add i64 %23, %7
  %25 = shl i64 %24, 1
  %26 = and i64 %25, 8589934590
  %.reass = add i64 %26, %invariant.op
  %27 = inttoptr i64 %.reass to ptr
  %28 = load i16, ptr %27, align 2, !tbaa !4
  %29 = zext i16 %28 to i64
  %30 = add nuw i64 %23, 1
  %31 = sub i64 %29, %12
  %32 = trunc i64 %31 to i16
  store i16 %32, ptr %27, align 2, !tbaa !4
  %exitcond.not = icmp eq i64 %30, %3
  br i1 %exitcond.not, label %fall, label %L0

fall:                                             ; preds = %L0, %entry
  %.pn = phi i64 [ %15, %entry ], [ %26, %L0 ]
  %.lcssa133 = phi i64 [ %20, %entry ], [ %31, %L0 ]
  %.lcssa132 = phi i64 [ %19, %entry ], [ %3, %L0 ]
  %33 = getelementptr i8, ptr %0, i64 512
  %.lcssa134 = add i64 %.pn, %10
  %34 = getelementptr i8, ptr %0, i64 120
  %35 = getelementptr i8, ptr %0, i64 112
  store i64 %.lcssa132, ptr %8, align 4, !tbaa !1
  store i64 %.lcssa133, ptr %35, align 4, !tbaa !1
  store i64 %.lcssa134, ptr %34, align 4, !tbaa !1
  %36 = add i64 %5, 1
  %37 = add i64 %7, %3
  %38 = icmp ult i64 %36, %3
  store i64 %36, ptr %4, align 4, !tbaa !1
  store i64 %37, ptr %6, align 4, !tbaa !1
  %..i = select i1 %38, i64 71360, i64 71402
  store i64 %..i, ptr %33, align 4, !tbaa !1
  ret i64 4294967298
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(argmem: readwrite)
define hidden noundef i64 @tb_3139c(ptr captures(none) initializes((512, 520)) %0, i64 %1) local_unnamed_addr #3 {
entry:
  %2 = getelementptr i8, ptr %0, i64 144
  %3 = load i64, ptr %2, align 4, !tbaa !1
  %4 = icmp eq i64 %3, 0
  br i1 %4, label %common.ret, label %fall

common.ret:                                       ; preds = %entry, %fall
  %storemerge = phi i64 [ 201586, %fall ], [ 201636, %entry ]
  %5 = getelementptr i8, ptr %0, i64 512
  store i64 %storemerge, ptr %5, align 4, !tbaa !1
  ret i64 4294967298

fall:                                             ; preds = %entry
  store i64 1, ptr %2, align 4, !tbaa !1
  br label %common.ret
}

; Function Attrs: nofree nosync nounwind memory(readwrite, inaccessiblemem: none)
define hidden range(i64 2, 4294967299) i64 @tb_30b08(ptr initializes((8, 16), (80, 88), (112, 128), (512, 520)) %0, i64 %1) local_unnamed_addr #1 {
entry:
  %2 = getelementptr i8, ptr %0, i64 8
  %3 = getelementptr i8, ptr %0, i64 16
  %4 = load i64, ptr %3, align 4, !tbaa !1
  %5 = getelementptr i8, ptr %0, i64 80
  %6 = getelementptr i8, ptr %0, i64 96
  %7 = load i64, ptr %6, align 4, !tbaa !1
  %8 = getelementptr i8, ptr %0, i64 112
  %9 = getelementptr i8, ptr %0, i64 120
  %10 = getelementptr i8, ptr %0, i64 128
  %11 = load i64, ptr %10, align 4, !tbaa !1
  %12 = getelementptr i8, ptr %0, i64 512
  %13 = add i64 %1, 24
  %14 = add i64 %13, %4
  %15 = inttoptr i64 %14 to ptr
  %16 = load i64, ptr %15, align 4, !tbaa !4
  %17 = add i64 %7, 48
  %18 = and i64 %17, 255
  %19 = add i64 %13, %11
  %20 = inttoptr i64 %19 to ptr
  store i64 1, ptr %20, align 4, !tbaa !4
  %21 = add i64 %4, 32
  %22 = and i64 %16, -2
  store i64 %16, ptr %2, align 4, !tbaa !1
  store i64 %21, ptr %3, align 4, !tbaa !1
  store i64 %18, ptr %5, align 4, !tbaa !1
  store i64 1, ptr %8, align 4, !tbaa !1
  store i64 %18, ptr %9, align 4, !tbaa !1
  store i64 %22, ptr %12, align 4, !tbaa !1
  %23 = musttail call i64 @aot_dispatch(ptr nonnull %0, i64 %1)
  ret i64 %23
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(argmem: readwrite)
define hidden noundef i64 @tb_4db12(ptr captures(none) initializes((512, 520)) %0, i64 %1) local_unnamed_addr #3 {
entry:
  %2 = getelementptr i8, ptr %0, i64 120
  %3 = load i64, ptr %2, align 4, !tbaa !1
  %4 = icmp eq i64 %3, 0
  %. = select i1 %4, i64 318254, i64 318228
  %5 = getelementptr i8, ptr %0, i64 512
  store i64 %., ptr %5, align 4, !tbaa !1
  ret i64 4294967298
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(argmem: readwrite)
define hidden noundef i64 @tb_211fc(ptr captures(none) initializes((120, 128), (512, 520)) %0, i64 %1) local_unnamed_addr #3 {
entry:
  %2 = getelementptr i8, ptr %0, i64 80
  %3 = load i64, ptr %2, align 4, !tbaa !1
  %4 = getelementptr i8, ptr %0, i64 120
  %5 = getelementptr i8, ptr %0, i64 512
  %6 = icmp eq i64 %3, -38
  store i64 -38, ptr %4, align 4, !tbaa !1
  %. = select i1 %6, i64 135724, i64 135684
  store i64 %., ptr %5, align 4, !tbaa !1
  ret i64 4294967298
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(argmem: write)
define hidden noundef i64 @tb_10f3a(ptr writeonly captures(none) initializes((512, 520)) %0, i64 %1) local_unnamed_addr #6 {
entry:
  %2 = getelementptr i8, ptr %0, i64 512
  store i64 69576, ptr %2, align 4, !tbaa !1
  ret i64 4294967298
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(readwrite, inaccessiblemem: none)
define hidden noundef i64 @tb_10f66(ptr captures(none) initializes((8, 16), (80, 128), (144, 152), (512, 520)) %0, i64 %1) local_unnamed_addr #4 {
entry:
  %2 = getelementptr i8, ptr %0, i64 8
  %3 = getelementptr i8, ptr %0, i64 64
  %4 = load i64, ptr %3, align 4, !tbaa !1
  %5 = getelementptr i8, ptr %0, i64 80
  %6 = getelementptr i8, ptr %0, i64 88
  %7 = getelementptr i8, ptr %0, i64 96
  %8 = getelementptr i8, ptr %0, i64 104
  %9 = getelementptr i8, ptr %0, i64 112
  %10 = getelementptr i8, ptr %0, i64 120
  %11 = getelementptr i8, ptr %0, i64 144
  %12 = getelementptr i8, ptr %0, i64 152
  %13 = load i64, ptr %12, align 4, !tbaa !1
  %14 = getelementptr i8, ptr %0, i64 512
  %15 = add i64 %4, %1
  %16 = inttoptr i64 %15 to ptr
  %17 = load i64, ptr %16, align 4, !tbaa !4
  %18 = add i64 %1, 8
  %19 = add i64 %18, %13
  %20 = inttoptr i64 %19 to ptr
  %21 = load i64, ptr %20, align 4, !tbaa !4
  %22 = add i64 %18, %17
  %23 = inttoptr i64 %22 to ptr
  %24 = load i64, ptr %23, align 4, !tbaa !4
  %25 = add i64 %17, %1
  %26 = inttoptr i64 %25 to ptr
  %27 = load i64, ptr %26, align 4, !tbaa !4
  store i64 %24, ptr %20, align 4, !tbaa !4
  store i64 %21, ptr %23, align 4, !tbaa !4
  %28 = add i64 %13, %1
  %29 = inttoptr i64 %28 to ptr
  store i64 %27, ptr %29, align 4, !tbaa !4
  store i64 %13, ptr %26, align 4, !tbaa !4
  store i64 69526, ptr %2, align 4, !tbaa !1
  store i64 %4, ptr %5, align 4, !tbaa !1
  store i64 68302, ptr %6, align 4, !tbaa !1
  store i64 0, ptr %7, align 4, !tbaa !1
  store i64 %24, ptr %8, align 4, !tbaa !1
  store i64 %21, ptr %9, align 4, !tbaa !1
  store i64 %27, ptr %10, align 4, !tbaa !1
  store i64 %17, ptr %11, align 4, !tbaa !1
  store i64 68848, ptr %14, align 4, !tbaa !1
  ret i64 4294967298
}

; Function Attrs: nofree nosync nounwind memory(readwrite, inaccessiblemem: none)
define hidden range(i64 2, 4294967299) i64 @tb_1476a(ptr initializes((8, 16), (64, 80), (144, 224), (512, 520)) %0, i64 %1) local_unnamed_addr #1 {
entry:
  %2 = getelementptr i8, ptr %0, i64 8
  %3 = getelementptr i8, ptr %0, i64 16
  %4 = load i64, ptr %3, align 4, !tbaa !1
  %5 = getelementptr i8, ptr %0, i64 64
  %6 = getelementptr i8, ptr %0, i64 72
  %7 = getelementptr i8, ptr %0, i64 144
  %8 = getelementptr i8, ptr %0, i64 152
  %9 = getelementptr i8, ptr %0, i64 160
  %10 = getelementptr i8, ptr %0, i64 168
  %11 = getelementptr i8, ptr %0, i64 176
  %12 = getelementptr i8, ptr %0, i64 184
  %13 = getelementptr i8, ptr %0, i64 192
  %14 = getelementptr i8, ptr %0, i64 200
  %15 = getelementptr i8, ptr %0, i64 208
  %16 = getelementptr i8, ptr %0, i64 216
  %17 = getelementptr i8, ptr %0, i64 512
  %18 = add i64 %4, %1
  %19 = add i64 %18, 1312
  %20 = inttoptr i64 %19 to ptr
  %21 = load i64, ptr %20, align 4, !tbaa !4
  %22 = add i64 %18, 1304
  %23 = inttoptr i64 %22 to ptr
  %24 = load i64, ptr %23, align 4, !tbaa !4
  %25 = add i64 %18, 1296
  %26 = inttoptr i64 %25 to ptr
  %27 = load i64, ptr %26, align 4, !tbaa !4
  %28 = add i64 %18, 1288
  %29 = inttoptr i64 %28 to ptr
  %30 = load i64, ptr %29, align 4, !tbaa !4
  %31 = add i64 %18, 1280
  %32 = inttoptr i64 %31 to ptr
  %33 = load i64, ptr %32, align 4, !tbaa !4
  %34 = add i64 %18, 1272
  %35 = inttoptr i64 %34 to ptr
  %36 = load i64, ptr %35, align 4, !tbaa !4
  %37 = add i64 %18, 1256
  %38 = inttoptr i64 %37 to ptr
  %39 = load i64, ptr %38, align 4, !tbaa !4
  %40 = add i64 %18, 1352
  %41 = inttoptr i64 %40 to ptr
  %42 = load i64, ptr %41, align 4, !tbaa !4
  %43 = add i64 %18, 1344
  %44 = inttoptr i64 %43 to ptr
  %45 = load i64, ptr %44, align 4, !tbaa !4
  %46 = add i64 %18, 1336
  %47 = inttoptr i64 %46 to ptr
  %48 = load i64, ptr %47, align 4, !tbaa !4
  %49 = add i64 %18, 1328
  %50 = inttoptr i64 %49 to ptr
  %51 = load i64, ptr %50, align 4, !tbaa !4
  %52 = add i64 %18, 1320
  %53 = inttoptr i64 %52 to ptr
  %54 = load i64, ptr %53, align 4, !tbaa !4
  %55 = add i64 %18, 1264
  %56 = inttoptr i64 %55 to ptr
  %57 = load i64, ptr %56, align 4, !tbaa !4
  %58 = add i64 %4, 1360
  %59 = and i64 %42, -2
  store i64 %42, ptr %2, align 4, !tbaa !1
  store i64 %58, ptr %3, align 4, !tbaa !1
  store i64 %45, ptr %5, align 4, !tbaa !1
  store i64 %48, ptr %6, align 4, !tbaa !1
  store i64 %51, ptr %7, align 4, !tbaa !1
  store i64 %54, ptr %8, align 4, !tbaa !1
  store i64 %21, ptr %9, align 4, !tbaa !1
  store i64 %24, ptr %10, align 4, !tbaa !1
  store i64 %27, ptr %11, align 4, !tbaa !1
  store i64 %30, ptr %12, align 4, !tbaa !1
  store i64 %33, ptr %13, align 4, !tbaa !1
  store i64 %36, ptr %14, align 4, !tbaa !1
  store i64 %57, ptr %15, align 4, !tbaa !1
  store i64 %39, ptr %16, align 4, !tbaa !1
  store i64 %59, ptr %17, align 4, !tbaa !1
  %60 = musttail call i64 @aot_dispatch(ptr %0, i64 %1)
  ret i64 %60
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(read, argmem: readwrite, inaccessiblemem: none)
define hidden noundef i64 @tb_30a9e(ptr captures(none) initializes((88, 96), (112, 120), (512, 520)) %0, i64 %1) local_unnamed_addr #0 {
entry:
  %2 = getelementptr i8, ptr %0, i64 88
  %3 = getelementptr i8, ptr %0, i64 112
  %4 = getelementptr i8, ptr %0, i64 120
  %5 = load i64, ptr %4, align 4, !tbaa !1
  %6 = getelementptr i8, ptr %0, i64 128
  %7 = load i64, ptr %6, align 4, !tbaa !1
  %8 = getelementptr i8, ptr %0, i64 512
  %9 = add i64 %1, 16
  %10 = add i64 %9, %7
  %11 = inttoptr i64 %10 to ptr
  %12 = load i64, ptr %11, align 4, !tbaa !4
  %13 = shl i64 %5, 3
  %14 = add i64 %12, %13
  store i64 %12, ptr %2, align 4, !tbaa !1
  store i64 %14, ptr %3, align 4, !tbaa !1
  store i64 199344, ptr %8, align 4, !tbaa !1
  ret i64 4294967298
}

; Function Attrs: nofree nosync nounwind memory(readwrite, inaccessiblemem: none)
define hidden range(i64 2, 4294967299) i64 @tb_2093c(ptr initializes((88, 104), (112, 128), (512, 520)) %0, i64 %1) local_unnamed_addr #1 {
entry:
  %2 = getelementptr i8, ptr %0, i64 80
  %3 = load i64, ptr %2, align 4, !tbaa !1
  %4 = getelementptr i8, ptr %0, i64 88
  %5 = getelementptr i8, ptr %0, i64 96
  %6 = getelementptr i8, ptr %0, i64 104
  %7 = load i64, ptr %6, align 4, !tbaa !1
  %8 = getelementptr i8, ptr %0, i64 112
  %9 = getelementptr i8, ptr %0, i64 120
  %10 = getelementptr i8, ptr %0, i64 128
  %11 = load i64, ptr %10, align 4, !tbaa !1
  %12 = getelementptr i8, ptr %0, i64 136
  %13 = load i64, ptr %12, align 4, !tbaa !1
  %14 = getelementptr i8, ptr %0, i64 512
  %15 = and i64 %11, -8
  %16 = add i64 %15, %7
  %17 = add i64 %15, %3
  %18 = and i64 %11, 7
  %19 = xor i64 %16, -1
  %20 = add i64 %17, %19
  %21 = add i64 %16, %18
  %22 = icmp eq i64 %18, 0
  store i64 %21, ptr %4, align 4, !tbaa !1
  store i64 %16, ptr %6, align 4, !tbaa !1
  store i64 %20, ptr %8, align 4, !tbaa !1
  store i64 %16, ptr %9, align 4, !tbaa !1
  br i1 %22, label %L0, label %fall

L0:                                               ; preds = %entry
  %23 = getelementptr i8, ptr %0, i64 8
  %24 = getelementptr i8, ptr %0, i64 16
  %25 = load i64, ptr %24, align 4, !tbaa !1
  %26 = add i64 %1, 40
  %27 = add i64 %26, %25
  %28 = inttoptr i64 %27 to ptr
  %29 = load i64, ptr %28, align 4, !tbaa !4
  %30 = add i64 %25, 48
  %31 = and i64 %29, -2
  store i64 %29, ptr %23, align 4, !tbaa !1
  store i64 %30, ptr %24, align 4, !tbaa !1
  store i64 %13, ptr %2, align 4, !tbaa !1
  store i64 0, ptr %5, align 4, !tbaa !1
  store i64 %31, ptr %14, align 4, !tbaa !1
  %32 = musttail call range(i64 2, 4294967299) i64 @aot_dispatch(ptr nonnull %0, i64 %1)
  ret i64 %32

fall:                                             ; preds = %entry
  %33 = getelementptr i8, ptr %0, i64 16
  %34 = load i64, ptr %33, align 4, !tbaa !1
  %35 = add i64 %16, %1
  %36 = inttoptr i64 %35 to ptr
  %37 = load i8, ptr %36, align 1, !tbaa !4
  %38 = add i64 %17, %1
  %39 = inttoptr i64 %38 to ptr
  store i8 %37, ptr %39, align 1, !tbaa !4
  %.not129.i = icmp eq i64 %18, 1
  br i1 %.not129.i, label %fall.i, label %L0.preheader.i

L0.preheader.i:                                   ; preds = %fall
  %40 = add i64 %16, 1
  %invariant.op.i = add i64 %20, %1
  br label %L0.i

L0.i:                                             ; preds = %L0.i, %L0.preheader.i
  %41 = phi i64 [ %45, %L0.i ], [ %40, %L0.preheader.i ]
  %42 = add i64 %41, %1
  %43 = inttoptr i64 %42 to ptr
  %44 = load i8, ptr %43, align 1, !tbaa !4
  %45 = add i64 %41, 1
  %.reass.i = add i64 %invariant.op.i, %45
  %46 = inttoptr i64 %.reass.i to ptr
  store i8 %44, ptr %46, align 1, !tbaa !4
  %.not.i = icmp eq i64 %21, %45
  br i1 %.not.i, label %fall.i.loopexit, label %L0.i

fall.i.loopexit:                                  ; preds = %L0.i
  %47 = add i64 %21, %20
  br label %fall.i

fall.i:                                           ; preds = %fall.i.loopexit, %fall
  %.lcssa128.i = phi i64 [ %17, %fall ], [ %47, %fall.i.loopexit ]
  %.lcssa127.in.i = phi i8 [ %37, %fall ], [ %44, %fall.i.loopexit ]
  %.lcssa127.i = zext i8 %.lcssa127.in.i to i64
  %48 = getelementptr i8, ptr %0, i64 8
  %49 = add i64 %1, 40
  %50 = add i64 %49, %34
  %51 = inttoptr i64 %50 to ptr
  %52 = load i64, ptr %51, align 4, !tbaa !4
  %53 = add i64 %34, 48
  %54 = and i64 %52, -2
  store i64 %52, ptr %48, align 4, !tbaa !1
  store i64 %53, ptr %33, align 4, !tbaa !1
  store i64 %13, ptr %2, align 4, !tbaa !1
  store i64 %.lcssa127.i, ptr %5, align 4, !tbaa !1
  store i64 %.lcssa128.i, ptr %6, align 4, !tbaa !1
  store i64 %21, ptr %9, align 4, !tbaa !1
  store i64 %54, ptr %14, align 4, !tbaa !1
  %55 = musttail call range(i64 2, 4294967299) i64 @aot_dispatch(ptr nonnull %0, i64 %1)
  ret i64 %55
}

; Function Attrs: nofree nosync nounwind memory(readwrite, inaccessiblemem: none)
define hidden range(i64 2, 4294967299) i64 @tb_11baa(ptr initializes((8, 16), (48, 56), (80, 144), (224, 256), (512, 520)) %0, i64 %1) local_unnamed_addr #1 {
entry:
  %2 = getelementptr i8, ptr %0, i64 8
  %3 = getelementptr i8, ptr %0, i64 64
  %4 = load i64, ptr %3, align 4, !tbaa !1
  %5 = getelementptr i8, ptr %0, i64 80
  %6 = getelementptr i8, ptr %0, i64 88
  %7 = getelementptr i8, ptr %0, i64 144
  %8 = load i64, ptr %7, align 4, !tbaa !1
  %9 = getelementptr i8, ptr %0, i64 512
  %10 = add i64 %4, %1
  %11 = inttoptr i64 %10 to ptr
  %12 = load i32, ptr %11, align 4, !tbaa !4
  %13 = sext i32 %12 to i64
  %14 = add i64 %4, 4
  store i64 72628, ptr %2, align 4, !tbaa !1
  store i64 %14, ptr %3, align 4, !tbaa !1
  store i64 %13, ptr %5, align 4, !tbaa !1
  store i64 %8, ptr %6, align 4, !tbaa !1
  store i64 73664, ptr %9, align 4, !tbaa !1
  %15 = musttail call i64 @tb_11fc0(ptr %0, i64 %1)
  ret i64 %15
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(read, argmem: readwrite, inaccessiblemem: none)
define hidden noundef i64 @tb_1160e(ptr captures(none) initializes((88, 96), (120, 128), (512, 520)) %0, i64 %1) local_unnamed_addr #0 {
entry:
  %2 = getelementptr i8, ptr %0, i64 64
  %3 = load i64, ptr %2, align 4, !tbaa !1
  %4 = getelementptr i8, ptr %0, i64 80
  %5 = load i64, ptr %4, align 4, !tbaa !1
  %6 = getelementptr i8, ptr %0, i64 88
  %7 = getelementptr i8, ptr %0, i64 96
  %8 = load i64, ptr %7, align 4, !tbaa !1
  %9 = getelementptr i8, ptr %0, i64 104
  %10 = load i64, ptr %9, align 4, !tbaa !1
  %11 = getelementptr i8, ptr %0, i64 112
  %12 = load i64, ptr %11, align 4, !tbaa !1
  %13 = getelementptr i8, ptr %0, i64 120
  %14 = getelementptr i8, ptr %0, i64 128
  %15 = load i64, ptr %14, align 4, !tbaa !1
  %16 = getelementptr i8, ptr %0, i64 168
  %17 = load i64, ptr %16, align 4, !tbaa !1
  %18 = getelementptr i8, ptr %0, i64 512
  %19 = add i64 %15, %12
  %20 = shl i64 %19, 2
  %21 = and i64 %20, 17179869180
  %22 = add i64 %3, %1
  %23 = add i64 %22, %21
  %24 = inttoptr i64 %23 to ptr
  %25 = load i32, ptr %24, align 4, !tbaa !4
  %26 = sext i32 %25 to i64
  %27 = add i64 %5, 10
  %28 = add i64 %8, %26
  %29 = icmp slt i64 %10, %26
  %30 = zext i1 %29 to i64
  %.not = icmp slt i64 %17, %28
  store i64 %30, ptr %6, align 4, !tbaa !1
  store i64 %28, ptr %7, align 4, !tbaa !1
  store i64 %26, ptr %9, align 4, !tbaa !1
  store i64 %27, ptr %13, align 4, !tbaa !1
  %. = select i1 %.not, i64 71214, i64 71164
  store i64 %., ptr %18, align 4, !tbaa !1
  ret i64 4294967298
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(read, argmem: readwrite, inaccessiblemem: none)
define hidden noundef i64 @tb_3098c(ptr captures(none) initializes((120, 128), (512, 520)) %0, i64 %1) local_unnamed_addr #0 {
entry:
  %2 = getelementptr i8, ptr %0, i64 80
  %3 = load i64, ptr %2, align 4, !tbaa !1
  %4 = getelementptr i8, ptr %0, i64 120
  %5 = getelementptr i8, ptr %0, i64 512
  %6 = add i64 %1, 32
  %7 = add i64 %6, %3
  %8 = inttoptr i64 %7 to ptr
  %9 = load i32, ptr %8, align 4, !tbaa !4
  %10 = sext i32 %9 to i64
  %11 = icmp eq i32 %9, 0
  store i64 %10, ptr %4, align 4, !tbaa !1
  %. = select i1 %11, i64 199190, i64 199056
  store i64 %., ptr %5, align 4, !tbaa !1
  ret i64 4294967298
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(argmem: readwrite)
define hidden noundef i64 @tb_4ecba(ptr captures(none) initializes((88, 96), (512, 520)) %0, i64 %1) local_unnamed_addr #3 {
entry:
  %2 = getelementptr i8, ptr %0, i64 88
  %3 = getelementptr i8, ptr %0, i64 168
  %4 = load i64, ptr %3, align 4, !tbaa !1
  %5 = getelementptr i8, ptr %0, i64 192
  %6 = load i64, ptr %5, align 4, !tbaa !1
  %7 = and i64 %6, 7
  %.not = icmp eq i64 %7, %4
  store i64 %7, ptr %2, align 4, !tbaa !1
  br i1 %.not, label %fall, label %L0

common.ret:                                       ; preds = %fall, %L0
  %storemerge = phi i64 [ %..i, %L0 ], [ 322626, %fall ]
  %8 = getelementptr i8, ptr %0, i64 512
  store i64 %storemerge, ptr %8, align 4, !tbaa !1
  ret i64 4294967298

L0:                                               ; preds = %entry
  %.not.i = icmp ult i64 %4, %7
  %..i = select i1 %.not.i, i64 322614, i64 322860
  br label %common.ret

fall:                                             ; preds = %entry
  %9 = getelementptr i8, ptr %0, i64 120
  %10 = getelementptr i8, ptr %0, i64 104
  store i64 65535, ptr %10, align 4, !tbaa !1
  store i64 65536, ptr %9, align 4, !tbaa !1
  br label %common.ret
}

; Function Attrs: nofree nosync nounwind memory(readwrite, inaccessiblemem: none)
define hidden range(i64 2, 4294967299) i64 @tb_10ad0(ptr initializes((120, 128), (512, 520)) %0, i64 %1) local_unnamed_addr #1 {
entry:
  %2 = getelementptr i8, ptr %0, i64 8
  %3 = load i64, ptr %2, align 4, !tbaa !1
  %4 = getelementptr i8, ptr %0, i64 80
  %5 = load i64, ptr %4, align 4, !tbaa !1
  %6 = getelementptr i8, ptr %0, i64 88
  %7 = load i64, ptr %6, align 4, !tbaa !1
  %8 = getelementptr i8, ptr %0, i64 120
  %9 = getelementptr i8, ptr %0, i64 512
  %10 = add i64 %1, 2
  %11 = add i64 %10, %5
  %12 = inttoptr i64 %11 to ptr
  %13 = load i16, ptr %12, align 2, !tbaa !4
  %14 = sext i16 %13 to i64
  %15 = add i64 %10, %7
  %16 = inttoptr i64 %15 to ptr
  %17 = load i16, ptr %16, align 2, !tbaa !4
  %18 = sext i16 %17 to i64
  %19 = sub nsw i64 %14, %18
  %20 = and i64 %3, -2
  store i64 %19, ptr %4, align 4, !tbaa !1
  store i64 %18, ptr %8, align 4, !tbaa !1
  store i64 %20, ptr %9, align 4, !tbaa !1
  %21 = musttail call i64 @aot_dispatch(ptr %0, i64 %1)
  ret i64 %21
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(readwrite, inaccessiblemem: none)
define hidden noundef i64 @tb_10ff4(ptr captures(none) initializes((8, 16), (80, 128), (144, 152), (512, 520)) %0, i64 %1) local_unnamed_addr #4 {
entry:
  %2 = getelementptr i8, ptr %0, i64 512
  %3 = getelementptr i8, ptr %0, i64 8
  %4 = getelementptr i8, ptr %0, i64 64
  %5 = load i64, ptr %4, align 4, !tbaa !1
  %6 = getelementptr i8, ptr %0, i64 80
  %7 = getelementptr i8, ptr %0, i64 88
  %8 = getelementptr i8, ptr %0, i64 96
  %9 = getelementptr i8, ptr %0, i64 104
  %10 = getelementptr i8, ptr %0, i64 112
  %11 = getelementptr i8, ptr %0, i64 120
  %12 = getelementptr i8, ptr %0, i64 144
  %13 = getelementptr i8, ptr %0, i64 152
  %14 = load i64, ptr %13, align 4, !tbaa !1
  %15 = add i64 %5, %1
  %16 = inttoptr i64 %15 to ptr
  %17 = load i64, ptr %16, align 4, !tbaa !4
  %18 = add i64 %1, 8
  %19 = add i64 %14, %18
  %20 = inttoptr i64 %19 to ptr
  %21 = load i64, ptr %20, align 4, !tbaa !4
  %22 = add i64 %17, %18
  %23 = inttoptr i64 %22 to ptr
  %24 = load i64, ptr %23, align 4, !tbaa !4
  %25 = add i64 %17, %1
  %26 = inttoptr i64 %25 to ptr
  %27 = load i64, ptr %26, align 4, !tbaa !4
  store i64 %24, ptr %20, align 4, !tbaa !4
  store i64 %21, ptr %23, align 4, !tbaa !4
  %28 = add i64 %14, %1
  %29 = inttoptr i64 %28 to ptr
  store i64 %27, ptr %29, align 4, !tbaa !4
  store i64 %14, ptr %26, align 4, !tbaa !4
  store i64 69526, ptr %3, align 4, !tbaa !1
  store i64 %5, ptr %6, align 4, !tbaa !1
  store i64 68302, ptr %7, align 4, !tbaa !1
  store i64 0, ptr %8, align 4, !tbaa !1
  store i64 %24, ptr %9, align 4, !tbaa !1
  store i64 %21, ptr %10, align 4, !tbaa !1
  store i64 %27, ptr %11, align 4, !tbaa !1
  store i64 %17, ptr %12, align 4, !tbaa !1
  store i64 68848, ptr %2, align 4, !tbaa !1
  ret i64 4294967298
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(read, argmem: readwrite, inaccessiblemem: none)
define hidden noundef i64 @tb_10d6e(ptr captures(none) initializes((120, 128), (512, 520)) %0, i64 %1) local_unnamed_addr #0 {
entry:
  %2 = getelementptr i8, ptr %0, i64 72
  %3 = load i64, ptr %2, align 4, !tbaa !1
  %4 = getelementptr i8, ptr %0, i64 120
  %5 = getelementptr i8, ptr %0, i64 144
  %6 = load i64, ptr %5, align 4, !tbaa !1
  %7 = getelementptr i8, ptr %0, i64 160
  %8 = load i64, ptr %7, align 4, !tbaa !1
  %9 = getelementptr i8, ptr %0, i64 512
  %10 = add i64 %8, -1
  %11 = add i64 %6, %1
  %12 = inttoptr i64 %11 to ptr
  %13 = load i64, ptr %12, align 4, !tbaa !4
  %.not = icmp eq i64 %3, 0
  store i64 %6, ptr %2, align 4, !tbaa !1
  store i64 %3, ptr %4, align 4, !tbaa !1
  store i64 %13, ptr %5, align 4, !tbaa !1
  store i64 %10, ptr %7, align 4, !tbaa !1
  %. = select i1 %.not, i64 68986, i64 68940
  store i64 %., ptr %9, align 4, !tbaa !1
  ret i64 4294967298
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(argmem: readwrite)
define hidden noundef i64 @tb_23170(ptr captures(none) initializes((512, 520)) %0, i64 %1) local_unnamed_addr #3 {
entry:
  %2 = getelementptr i8, ptr %0, i64 112
  %3 = load i64, ptr %2, align 4, !tbaa !1
  %4 = getelementptr i8, ptr %0, i64 120
  %5 = load i64, ptr %4, align 4, !tbaa !1
  %6 = getelementptr i8, ptr %0, i64 512
  %7 = add i64 %3, 88
  %8 = add i64 %7, %5
  store i64 %8, ptr %2, align 4, !tbaa !1
  store i64 143698, ptr %6, align 4, !tbaa !1
  ret i64 4294967298
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(write, argmem: readwrite, inaccessiblemem: none)
define hidden noundef i64 @tb_208f0(ptr captures(none) initializes((48, 56), (128, 136), (512, 520)) %0, i64 %1) local_unnamed_addr #2 {
entry:
  %2 = getelementptr i8, ptr %0, i64 8
  %3 = load i64, ptr %2, align 4, !tbaa !1
  %4 = getelementptr i8, ptr %0, i64 16
  %5 = load i64, ptr %4, align 4, !tbaa !1
  %6 = getelementptr i8, ptr %0, i64 48
  %7 = getelementptr i8, ptr %0, i64 80
  %8 = load i64, ptr %7, align 4, !tbaa !1
  %9 = getelementptr i8, ptr %0, i64 96
  %10 = load i64, ptr %9, align 4, !tbaa !1
  %11 = getelementptr i8, ptr %0, i64 128
  %12 = getelementptr i8, ptr %0, i64 512
  %13 = add i64 %5, -48
  %14 = sub i64 0, %8
  %15 = and i64 %14, 7
  %16 = add i64 %1, -8
  %17 = add i64 %16, %5
  %18 = inttoptr i64 %17 to ptr
  store i64 %3, ptr %18, align 4, !tbaa !4
  %19 = sub i64 %10, %15
  %20 = icmp eq i64 %15, 0
  store i64 %13, ptr %4, align 4, !tbaa !1
  store i64 %15, ptr %6, align 4, !tbaa !1
  store i64 %19, ptr %11, align 4, !tbaa !1
  %. = select i1 %20, i64 133404, i64 133380
  store i64 %., ptr %12, align 4, !tbaa !1
  ret i64 4294967298
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(argmem: readwrite)
define hidden noundef i64 @tb_116e2(ptr captures(none) initializes((512, 520)) %0, i64 %1) local_unnamed_addr #3 {
entry:
  %2 = getelementptr i8, ptr %0, i64 72
  %3 = load i64, ptr %2, align 4, !tbaa !1
  %4 = getelementptr i8, ptr %0, i64 88
  %5 = load i64, ptr %4, align 4, !tbaa !1
  %6 = getelementptr i8, ptr %0, i64 96
  %7 = load i64, ptr %6, align 4, !tbaa !1
  %8 = getelementptr i8, ptr %0, i64 512
  %9 = add i64 %5, 1
  %10 = add i64 %7, %3
  %11 = icmp ult i64 %9, %3
  store i64 %9, ptr %4, align 4, !tbaa !1
  store i64 %10, ptr %6, align 4, !tbaa !1
  %. = select i1 %11, i64 71360, i64 71402
  store i64 %., ptr %8, align 4, !tbaa !1
  ret i64 4294967298
}

; Function Attrs: nofree nosync nounwind memory(readwrite, inaccessiblemem: none)
define hidden range(i64 2, 4294967299) i64 @tb_211fa(ptr initializes((512, 520)) %0, i64 %1) local_unnamed_addr #1 {
entry:
  %2 = getelementptr i8, ptr %0, i64 8
  %3 = load i64, ptr %2, align 4, !tbaa !1
  %4 = getelementptr i8, ptr %0, i64 512
  %5 = and i64 %3, -2
  store i64 %5, ptr %4, align 4, !tbaa !1
  %6 = musttail call i64 @aot_dispatch(ptr %0, i64 %1)
  ret i64 %6
}

; Function Attrs: nofree nosync nounwind memory(readwrite, inaccessiblemem: none)
define hidden range(i64 2, 4294967299) i64 @tb_20956(ptr initializes((96, 112)) %0, i64 %1) local_unnamed_addr #1 {
entry:
  %2 = getelementptr i8, ptr %0, i64 16
  %3 = load i64, ptr %2, align 4, !tbaa !1
  %4 = getelementptr i8, ptr %0, i64 88
  %5 = load i64, ptr %4, align 4, !tbaa !1
  %6 = getelementptr i8, ptr %0, i64 112
  %7 = load i64, ptr %6, align 4, !tbaa !1
  %8 = getelementptr i8, ptr %0, i64 120
  %9 = getelementptr i8, ptr %0, i64 136
  %10 = load i64, ptr %9, align 4, !tbaa !1
  %.promoted = load i64, ptr %8, align 4, !tbaa !1
  %11 = add i64 %.promoted, %1
  %12 = inttoptr i64 %11 to ptr
  %13 = load i8, ptr %12, align 1, !tbaa !4
  %14 = add i64 %.promoted, 1
  %15 = add i64 %14, %7
  %16 = add i64 %15, %1
  %17 = inttoptr i64 %16 to ptr
  store i8 %13, ptr %17, align 1, !tbaa !4
  %.not129 = icmp eq i64 %5, %14
  br i1 %.not129, label %fall, label %L0.preheader

L0.preheader:                                     ; preds = %entry
  %invariant.op = add i64 %7, %1
  br label %L0

L0:                                               ; preds = %L0.preheader, %L0
  %18 = phi i64 [ %22, %L0 ], [ %14, %L0.preheader ]
  %19 = add i64 %18, %1
  %20 = inttoptr i64 %19 to ptr
  %21 = load i8, ptr %20, align 1, !tbaa !4
  %22 = add i64 %18, 1
  %.reass = add i64 %22, %invariant.op
  %23 = inttoptr i64 %.reass to ptr
  store i8 %21, ptr %23, align 1, !tbaa !4
  %.not = icmp eq i64 %5, %22
  br i1 %.not, label %fall.loopexit, label %L0

fall.loopexit:                                    ; preds = %L0
  %24 = add i64 %22, %7
  br label %fall

fall:                                             ; preds = %fall.loopexit, %entry
  %.lcssa128 = phi i64 [ %15, %entry ], [ %24, %fall.loopexit ]
  %.lcssa127.in = phi i8 [ %13, %entry ], [ %21, %fall.loopexit ]
  %25 = getelementptr i8, ptr %0, i64 512
  %.lcssa127 = zext i8 %.lcssa127.in to i64
  %26 = getelementptr i8, ptr %0, i64 104
  %27 = getelementptr i8, ptr %0, i64 96
  %28 = getelementptr i8, ptr %0, i64 80
  %29 = getelementptr i8, ptr %0, i64 8
  %30 = add i64 %1, 40
  %31 = add i64 %30, %3
  %32 = inttoptr i64 %31 to ptr
  %33 = load i64, ptr %32, align 4, !tbaa !4
  %34 = add i64 %3, 48
  %35 = and i64 %33, -2
  store i64 %33, ptr %29, align 4, !tbaa !1
  store i64 %34, ptr %2, align 4, !tbaa !1
  store i64 %10, ptr %28, align 4, !tbaa !1
  store i64 %.lcssa127, ptr %27, align 4, !tbaa !1
  store i64 %.lcssa128, ptr %26, align 4, !tbaa !1
  store i64 %5, ptr %8, align 4, !tbaa !1
  store i64 %35, ptr %25, align 4, !tbaa !1
  %36 = musttail call range(i64 2, 4294967299) i64 @aot_dispatch(ptr nonnull %0, i64 %1)
  ret i64 %36
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(argmem: write)
define hidden noundef i64 @tb_4ed42(ptr writeonly captures(none) initializes((160, 168), (512, 520)) %0, i64 %1) local_unnamed_addr #6 {
entry:
  %2 = getelementptr i8, ptr %0, i64 160
  %3 = getelementptr i8, ptr %0, i64 512
  store i64 -1, ptr %2, align 4, !tbaa !1
  store i64 322688, ptr %3, align 4, !tbaa !1
  ret i64 4294967298
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(read, argmem: readwrite, inaccessiblemem: none)
define hidden noundef i64 @tb_309cc(ptr captures(none) initializes((64, 72), (512, 520)) %0, i64 %1) local_unnamed_addr #0 {
entry:
  %2 = getelementptr i8, ptr %0, i64 64
  %3 = getelementptr i8, ptr %0, i64 80
  %4 = load i64, ptr %3, align 4, !tbaa !1
  %5 = getelementptr i8, ptr %0, i64 144
  %6 = load i64, ptr %5, align 4, !tbaa !1
  %7 = getelementptr i8, ptr %0, i64 512
  %8 = add i64 %1, 16
  %9 = add i64 %8, %6
  %10 = inttoptr i64 %9 to ptr
  %11 = load i64, ptr %10, align 4, !tbaa !4
  %.not = icmp eq i64 %11, %4
  store i64 %11, ptr %2, align 4, !tbaa !1
  %. = select i1 %.not, i64 199124, i64 199080
  store i64 %., ptr %7, align 4, !tbaa !1
  ret i64 4294967298
}

; Function Attrs: nofree nosync nounwind memory(readwrite, inaccessiblemem: none)
define hidden range(i64 2, 4294967299) i64 @tb_14764(ptr initializes((120, 128), (512, 520)) %0, i64 %1) local_unnamed_addr #1 {
entry:
  %2 = getelementptr i8, ptr %0, i64 64
  %3 = load i64, ptr %2, align 4, !tbaa !1
  %4 = getelementptr i8, ptr %0, i64 120
  %5 = getelementptr i8, ptr %0, i64 512
  %6 = add i64 %1, 32
  %7 = add i64 %6, %3
  %8 = inttoptr i64 %7 to ptr
  %9 = load i32, ptr %8, align 4, !tbaa !4
  %10 = sext i32 %9 to i64
  %.not = icmp eq i32 %9, 0
  store i64 %10, ptr %4, align 4, !tbaa !1
  br i1 %.not, label %fall, label %L0

L0:                                               ; preds = %entry
  store i64 83386, ptr %5, align 4, !tbaa !1
  ret i64 4294967298

fall:                                             ; preds = %entry
  %11 = getelementptr i8, ptr %0, i64 8
  %12 = getelementptr i8, ptr %0, i64 16
  %13 = load i64, ptr %12, align 4, !tbaa !1
  %14 = getelementptr i8, ptr %0, i64 72
  %15 = getelementptr i8, ptr %0, i64 144
  %16 = getelementptr i8, ptr %0, i64 152
  %17 = getelementptr i8, ptr %0, i64 160
  %18 = getelementptr i8, ptr %0, i64 168
  %19 = getelementptr i8, ptr %0, i64 176
  %20 = getelementptr i8, ptr %0, i64 184
  %21 = getelementptr i8, ptr %0, i64 192
  %22 = getelementptr i8, ptr %0, i64 200
  %23 = getelementptr i8, ptr %0, i64 208
  %24 = getelementptr i8, ptr %0, i64 216
  %25 = add i64 %13, %1
  %26 = add i64 %25, 1312
  %27 = inttoptr i64 %26 to ptr
  %28 = load i64, ptr %27, align 4, !tbaa !4
  %29 = add i64 %25, 1304
  %30 = inttoptr i64 %29 to ptr
  %31 = load i64, ptr %30, align 4, !tbaa !4
  %32 = add i64 %25, 1296
  %33 = inttoptr i64 %32 to ptr
  %34 = load i64, ptr %33, align 4, !tbaa !4
  %35 = add i64 %25, 1288
  %36 = inttoptr i64 %35 to ptr
  %37 = load i64, ptr %36, align 4, !tbaa !4
  %38 = add i64 %25, 1280
  %39 = inttoptr i64 %38 to ptr
  %40 = load i64, ptr %39, align 4, !tbaa !4
  %41 = add i64 %25, 1272
  %42 = inttoptr i64 %41 to ptr
  %43 = load i64, ptr %42, align 4, !tbaa !4
  %44 = add i64 %25, 1256
  %45 = inttoptr i64 %44 to ptr
  %46 = load i64, ptr %45, align 4, !tbaa !4
  %47 = add i64 %25, 1352
  %48 = inttoptr i64 %47 to ptr
  %49 = load i64, ptr %48, align 4, !tbaa !4
  %50 = add i64 %25, 1344
  %51 = inttoptr i64 %50 to ptr
  %52 = load i64, ptr %51, align 4, !tbaa !4
  %53 = add i64 %25, 1336
  %54 = inttoptr i64 %53 to ptr
  %55 = load i64, ptr %54, align 4, !tbaa !4
  %56 = add i64 %25, 1328
  %57 = inttoptr i64 %56 to ptr
  %58 = load i64, ptr %57, align 4, !tbaa !4
  %59 = add i64 %25, 1320
  %60 = inttoptr i64 %59 to ptr
  %61 = load i64, ptr %60, align 4, !tbaa !4
  %62 = add i64 %25, 1264
  %63 = inttoptr i64 %62 to ptr
  %64 = load i64, ptr %63, align 4, !tbaa !4
  %65 = add i64 %13, 1360
  %66 = and i64 %49, -2
  store i64 %49, ptr %11, align 4, !tbaa !1
  store i64 %65, ptr %12, align 4, !tbaa !1
  store i64 %52, ptr %2, align 4, !tbaa !1
  store i64 %55, ptr %14, align 4, !tbaa !1
  store i64 %58, ptr %15, align 4, !tbaa !1
  store i64 %61, ptr %16, align 4, !tbaa !1
  store i64 %28, ptr %17, align 4, !tbaa !1
  store i64 %31, ptr %18, align 4, !tbaa !1
  store i64 %34, ptr %19, align 4, !tbaa !1
  store i64 %37, ptr %20, align 4, !tbaa !1
  store i64 %40, ptr %21, align 4, !tbaa !1
  store i64 %43, ptr %22, align 4, !tbaa !1
  store i64 %64, ptr %23, align 4, !tbaa !1
  store i64 %46, ptr %24, align 4, !tbaa !1
  store i64 %66, ptr %5, align 4, !tbaa !1
  %67 = musttail call range(i64 2, 4294967299) i64 @aot_dispatch(ptr nonnull %0, i64 %1)
  ret i64 %67
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(argmem: readwrite)
define hidden noundef i64 @tb_4ebfa(ptr captures(none) initializes((96, 112), (192, 200), (512, 520)) %0, i64 %1) local_unnamed_addr #3 {
entry:
  %2 = getelementptr i8, ptr %0, i64 80
  %3 = load i64, ptr %2, align 4, !tbaa !1
  %4 = getelementptr i8, ptr %0, i64 96
  %5 = getelementptr i8, ptr %0, i64 104
  %6 = getelementptr i8, ptr %0, i64 192
  %7 = getelementptr i8, ptr %0, i64 512
  %8 = and i64 %3, 112
  %9 = and i64 %3, 255
  %10 = icmp eq i64 %8, 32
  store i64 32, ptr %4, align 4, !tbaa !1
  store i64 %8, ptr %5, align 4, !tbaa !1
  store i64 %9, ptr %6, align 4, !tbaa !1
  %. = select i1 %10, i64 322844, i64 322570
  store i64 %., ptr %7, align 4, !tbaa !1
  ret i64 4294967298
}

; Function Attrs: nofree nosync nounwind memory(readwrite, inaccessiblemem: none)
define hidden range(i64 2, 4294967299) i64 @tb_10f9c(ptr initializes((8, 16), (48, 56), (80, 144), (224, 248), (512, 520)) %0, i64 %1) local_unnamed_addr #1 {
entry:
  %2 = getelementptr i8, ptr %0, i64 8
  %3 = getelementptr i8, ptr %0, i64 72
  %4 = load i64, ptr %3, align 4, !tbaa !1
  %5 = getelementptr i8, ptr %0, i64 80
  %6 = getelementptr i8, ptr %0, i64 88
  %7 = getelementptr i8, ptr %0, i64 120
  %8 = getelementptr i8, ptr %0, i64 144
  %9 = load i64, ptr %8, align 4, !tbaa !1
  %10 = getelementptr i8, ptr %0, i64 512
  %11 = add i64 %1, 8
  %12 = add i64 %11, %9
  %13 = inttoptr i64 %12 to ptr
  %14 = load i64, ptr %13, align 4, !tbaa !4
  %15 = add i64 %14, %1
  %16 = inttoptr i64 %15 to ptr
  %17 = load i16, ptr %16, align 2, !tbaa !4
  %18 = sext i16 %17 to i64
  store i64 69546, ptr %2, align 4, !tbaa !1
  store i64 %18, ptr %5, align 4, !tbaa !1
  store i64 %4, ptr %6, align 4, !tbaa !1
  store i64 %14, ptr %7, align 4, !tbaa !1
  store i64 74642, ptr %10, align 4, !tbaa !1
  %19 = musttail call i64 @tb_12392(ptr %0, i64 %1)
  ret i64 %19
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(argmem: readwrite)
define hidden noundef i64 @tb_11b6a(ptr captures(none) initializes((8, 16), (80, 96), (512, 520)) %0, i64 %1) local_unnamed_addr #3 {
entry:
  %2 = getelementptr i8, ptr %0, i64 8
  %3 = getelementptr i8, ptr %0, i64 80
  %4 = getelementptr i8, ptr %0, i64 152
  %5 = getelementptr i8, ptr %0, i64 512
  store i64 72562, ptr %2, align 4, !tbaa !1
  %6 = load <2 x i64>, ptr %4, align 4, !tbaa !1
  %7 = shufflevector <2 x i64> %6, <2 x i64> poison, <2 x i32> <i32 1, i32 0>
  store <2 x i64> %7, ptr %3, align 4, !tbaa !1
  store i64 71878, ptr %5, align 4, !tbaa !1
  ret i64 4294967298
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(write, argmem: readwrite, inaccessiblemem: none)
define hidden noundef i64 @tb_11b36(ptr captures(none) initializes((512, 520)) %0, i64 %1) local_unnamed_addr #2 {
entry:
  %2 = getelementptr i8, ptr %0, i64 16
  %3 = load i64, ptr %2, align 4, !tbaa !1
  %4 = getelementptr i8, ptr %0, i64 72
  %5 = load i64, ptr %4, align 4, !tbaa !1
  %6 = getelementptr i8, ptr %0, i64 152
  %7 = load <2 x i64>, ptr %6, align 4, !tbaa !1
  %8 = getelementptr i8, ptr %0, i64 192
  %9 = load i64, ptr %8, align 4, !tbaa !1
  %10 = and i64 %9, 4294967295
  %11 = add i64 %1, 8
  %12 = add i64 %11, %3
  %13 = inttoptr i64 %12 to ptr
  store i64 %5, ptr %13, align 4, !tbaa !4
  %14 = add i64 %10, %5
  %.not = icmp ult i64 %5, %14
  store i64 %14, ptr %8, align 4, !tbaa !1
  br i1 %.not, label %common.ret, label %L0

common.ret:                                       ; preds = %entry, %L0
  %storemerge = phi i64 [ 71878, %L0 ], [ 72516, %entry ]
  %15 = getelementptr i8, ptr %0, i64 512
  store i64 %storemerge, ptr %15, align 4, !tbaa !1
  ret i64 4294967298

L0:                                               ; preds = %entry
  %16 = getelementptr i8, ptr %0, i64 80
  %17 = getelementptr i8, ptr %0, i64 8
  store i64 72562, ptr %17, align 4, !tbaa !1
  %18 = shufflevector <2 x i64> %7, <2 x i64> poison, <2 x i32> <i32 1, i32 0>
  store <2 x i64> %18, ptr %16, align 4, !tbaa !1
  br label %common.ret
}

; Function Attrs: nofree nosync nounwind memory(readwrite, inaccessiblemem: none)
define hidden range(i64 2, 4294967299) i64 @tb_20968(ptr initializes((8, 16), (80, 88), (512, 520)) %0, i64 %1) local_unnamed_addr #1 {
entry:
  %2 = getelementptr i8, ptr %0, i64 8
  %3 = getelementptr i8, ptr %0, i64 16
  %4 = load i64, ptr %3, align 4, !tbaa !1
  %5 = getelementptr i8, ptr %0, i64 80
  %6 = getelementptr i8, ptr %0, i64 136
  %7 = load i64, ptr %6, align 4, !tbaa !1
  %8 = getelementptr i8, ptr %0, i64 512
  %9 = add i64 %1, 40
  %10 = add i64 %9, %4
  %11 = inttoptr i64 %10 to ptr
  %12 = load i64, ptr %11, align 4, !tbaa !4
  %13 = add i64 %4, 48
  %14 = and i64 %12, -2
  store i64 %12, ptr %2, align 4, !tbaa !1
  store i64 %13, ptr %3, align 4, !tbaa !1
  store i64 %7, ptr %5, align 4, !tbaa !1
  store i64 %14, ptr %8, align 4, !tbaa !1
  %15 = musttail call i64 @aot_dispatch(ptr %0, i64 %1)
  ret i64 %15
}

; Function Attrs: mustprogress nofree norecurse nosync nounwind willreturn memory(argmem: readwrite)
define hidden noundef i64 @tb_14740(ptr captures(none) initializes((8, 16), (80, 96), (152, 160), (512, 520)) %0, i64 %1) local_unnamed_addr #3 {
entry:
  %2 = getelementptr i8, ptr %0, i64 8
  %3 = getelementptr i8, ptr %0, i64 80
  %4 = getelementptr i8, ptr %0, i64 88
  %5 = getelementptr i8, ptr %0, i64 152
  %6 = getelementptr i8, ptr %0, i64 208
  %7 = load i64, ptr %6, align 4, !tbaa !1
  %8 = getelementptr i8, ptr %0, i64 512
  %9 = add i64 %7, 1
  store i64 83790, ptr %2, align 4, !tbaa !1
  store i64 %9, ptr %3, align 4, !tbaa !1
  store i64 37, ptr %4, align 4, !tbaa !1
  store i64 %9, ptr %5, align 4, !tbaa !1
  store i64 130504, ptr %8, align 4, !tbaa !1
  ret i64 4294967298
}

; Function Attrs: nofree nosync nounwind memory(readwrite, inaccessiblemem: none)
define hidden range(i64 2, 4294967299) i64 @tb_10b78(ptr initializes((8, 16), (48, 56), (88, 128), (224, 248), (512, 520)) %0, i64 %1) local_unnamed_addr #1 {
entry:
  %2 = getelementptr i8, ptr %0, i64 8
  %3 = getelementptr i8, ptr %0, i64 16
  %4 = load i64, ptr %3, align 4, !tbaa !1
  %5 = getelementptr i8, ptr %0, i64 80
  %6 = load i64, ptr %5, align 4, !tbaa !1
  %7 = getelementptr i8, ptr %0, i64 88
  %8 = getelementptr i8, ptr %0, i64 120
  %9 = getelementptr i8, ptr %0, i64 128
  %10 = load i64, ptr %9, align 4, !tbaa !1
  %11 = getelementptr i8, ptr %0, i64 136
  %12 = load i64, ptr %11, align 4, !tbaa !1
  %13 = getelementptr i8, ptr %0, i64 512
  %14 = add i64 %12, %1
  %15 = add i64 %14, 96
  %16 = inttoptr i64 %15 to ptr
  %17 = load i16, ptr %16, align 2, !tbaa !4
  %18 = zext i16 %17 to i64
  %19 = add i64 %14, 100
  %20 = inttoptr i64 %19 to ptr
  %21 = trunc i64 %6 to i16
  store i16 %21, ptr %20, align 2, !tbaa !4
  %22 = add i64 %4, %1
  %23 = add i64 %22, 8
  %24 = inttoptr i64 %23 to ptr
  store i64 %12, ptr %24, align 4, !tbaa !4
  %25 = inttoptr i64 %22 to ptr
  store i64 %10, ptr %25, align 4, !tbaa !4
  store i64 68490, ptr %2, align 4, !tbaa !1
  store i64 %18, ptr %7, align 4, !tbaa !1
  store i64 %18, ptr %8, align 4, !tbaa !1
  store i64 73184, ptr %13, align 4, !tbaa !1
  %26 = musttail call i64 @tb_11de0(ptr nonnull %0, i64 %1)
  ret i64 %26
}

; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
declare i8 @llvm.bitreverse.i8(i8) #7

; Function Attrs: nocallback nofree nounwind willreturn memory(argmem: write)
declare void @llvm.memset.p0.i64(ptr writeonly captures(none), i8, i64, i1 immarg) #8

attributes #0 = { mustprogress nofree norecurse nosync nounwind willreturn memory(read, argmem: readwrite, inaccessiblemem: none) }
attributes #1 = { nofree nosync nounwind memory(readwrite, inaccessiblemem: none) }
attributes #2 = { mustprogress nofree norecurse nosync nounwind willreturn memory(write, argmem: readwrite, inaccessiblemem: none) }
attributes #3 = { mustprogress nofree norecurse nosync nounwind willreturn memory(argmem: readwrite) }
attributes #4 = { mustprogress nofree norecurse nosync nounwind willreturn memory(readwrite, inaccessiblemem: none) }
attributes #5 = { nofree norecurse nosync nounwind memory(readwrite, inaccessiblemem: none) }
attributes #6 = { mustprogress nofree norecurse nosync nounwind willreturn memory(argmem: write) }
attributes #7 = { nocallback nofree nosync nounwind speculatable willreturn memory(none) }
attributes #8 = { nocallback nofree nounwind willreturn memory(argmem: write) }

!llvm.module.flags = !{!0}

!0 = !{i32 8, !"PIC Level", i32 2}
!1 = !{!2, !2, i64 0}
!2 = !{!"cpustate", !3, i64 0}
!3 = !{!"tcg-rs tbaa"}
!4 = !{!5, !5, i64 0}
!5 = !{!"guest", !3, i64 0}
