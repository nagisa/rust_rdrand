declare {i16, i32} @llvm.x86.rdrand.16()
declare {i32, i32} @llvm.x86.rdrand.32()
declare {i64, i32} @llvm.x86.rdrand.64()
declare {i16, i32} @llvm.x86.rdseed.16()
declare {i32, i32} @llvm.x86.rdseed.32()
declare {i64, i32} @llvm.x86.rdseed.64()

define i64 @librdrand_rust_rand_64() {
    br label %body
body:
    %result = tail call {i64, i32} @llvm.x86.rdrand.64() nounwind
    %flag = extractvalue {i64, i32} %result, 1
    %boolflag = icmp eq i32 %flag, 0
    br i1 %boolflag, label %body, label %done
done:
    %val = extractvalue {i64, i32} %result, 0
    ret i64 %val
}

define i32 @librdrand_rust_rand_32() {
    br label %body
body:
    %result = tail call {i32, i32} @llvm.x86.rdrand.32() nounwind
    %flag = extractvalue {i32, i32} %result, 1
    %boolflag = icmp eq i32 %flag, 0
    br i1 %boolflag, label %body, label %done
done:
    %val = extractvalue {i32, i32} %result, 0
    ret i32 %val
}

define i16 @librdrand_rust_rand_16() {
    br label %body
body:
    %result = tail call {i16, i32} @llvm.x86.rdrand.16() nounwind
    %flag = extractvalue {i16, i32} %result, 1
    %boolflag = icmp eq i32 %flag, 0
    br i1 %boolflag, label %body, label %done
done:
    %val = extractvalue {i16, i32} %result, 0
    ret i16 %val
}

define i64 @librdrand_rust_seed_64() {
    br label %body
body:
    %result = tail call {i64, i32} @llvm.x86.rdseed.64() nounwind
    %flag = extractvalue {i64, i32} %result, 1
    %boolflag = icmp eq i32 %flag, 0
    br i1 %boolflag, label %body, label %done
done:
    %val = extractvalue {i64, i32} %result, 0
    ret i64 %val
}

define i32 @librdrand_rust_seed_32() {
    br label %body
body:
    %result = tail call {i32, i32} @llvm.x86.rdseed.32() nounwind
    %flag = extractvalue {i32, i32} %result, 1
    %boolflag = icmp eq i32 %flag, 0
    br i1 %boolflag, label %body, label %done
done:
    %val = extractvalue {i32, i32} %result, 0
    ret i32 %val
}

define i16 @librdrand_rust_seed_16() {
    br label %body
body:
    %result = tail call {i16, i32} @llvm.x86.rdseed.16() nounwind
    %flag = extractvalue {i16, i32} %result, 1
    %boolflag = icmp eq i32 %flag, 0
    br i1 %boolflag, label %body, label %done
done:
    %val = extractvalue {i16, i32} %result, 0
    ret i16 %val
}

define zeroext i1 @librdrand_rust_has_rdrand() unnamed_addr #0 {
entry-block:
  %0 = tail call { i32, i32, i32, i32 } asm "cpuid", "={eax},={ebx},={ecx},={edx},0,2"(i32 1, i32 0)
  %1 = extractvalue { i32, i32, i32, i32 } %0, 2
  %2 = and i32 %1, 1073741824
  %3 = icmp ne i32 %2, 0
  ret i1 %3
}

define zeroext i1 @librdrand_rust_has_rdseed() unnamed_addr #0 {
entry-block:
  %0 = tail call { i32, i32, i32, i32 } asm "cpuid", "={eax},={ebx},={ecx},={edx},0,2"(i32 7, i32 0)
  %1 = extractvalue { i32, i32, i32, i32 } %0, 1
  %2 = and i32 %1, 262144
  %3 = icmp ne i32 %2, 0
  ret i1 %3
}
