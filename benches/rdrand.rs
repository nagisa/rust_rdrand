use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use rand_core::RngCore;

fn bench_rdrand(c: &mut Criterion) {
    let mut gen = match rdrand::RdRand::new() {
        Ok(g) => g,
        Err(_) => return,
    };
    let mut group = c.benchmark_group("rdrand");

    group
        .throughput(Throughput::Bytes(2))
        .bench_function("try_next/u16", move |b| {
            b.iter(move || gen.try_next_u16().unwrap())
        });
    group
        .throughput(Throughput::Bytes(4))
        .bench_function("try_next/u32", move |b| {
            b.iter(move || gen.try_next_u32().unwrap())
        });
    group
        .throughput(Throughput::Bytes(4))
        .bench_function("next/u32", move |b| b.iter(move || gen.next_u32()));
    group
        .throughput(Throughput::Bytes(8))
        .bench_function("try_next/u64", move |b| {
            b.iter(move || gen.try_next_u64().unwrap())
        });
    group
        .throughput(Throughput::Bytes(8))
        .bench_function("next/u64", move |b| b.iter(move || gen.next_u64()));
    let mut buffer = [0; 128];
    group
        .throughput(Throughput::Bytes(128))
        .bench_function("try_next/fill128", |b| {
            b.iter(|| gen.try_fill_bytes(&mut buffer).unwrap())
        });
    group
        .throughput(Throughput::Bytes(128))
        .bench_function("next/fill128", |b| b.iter(|| gen.fill_bytes(&mut buffer)));

    group.finish();
}

fn bench_rdseed(c: &mut Criterion) {
    let mut gen = match rdrand::RdSeed::new() {
        Ok(g) => g,
        Err(_) => return,
    };
    let mut group = c.benchmark_group("rdseed");

    group
        .throughput(Throughput::Bytes(2))
        .bench_function("try_next/u16", move |b| {
            b.iter(move || gen.try_next_u16().unwrap())
        });
    group
        .throughput(Throughput::Bytes(4))
        .bench_function("try_next/u32", move |b| {
            b.iter(move || gen.try_next_u32().unwrap())
        });
    group
        .throughput(Throughput::Bytes(4))
        .bench_function("next/u32", move |b| b.iter(move || gen.next_u32()));
    group
        .throughput(Throughput::Bytes(8))
        .bench_function("try_next/u64", move |b| {
            b.iter(move || gen.try_next_u64().unwrap())
        });
    group
        .throughput(Throughput::Bytes(8))
        .bench_function("next/u64", move |b| b.iter(move || gen.next_u64()));
    let mut buffer = [0; 128];
    group
        .throughput(Throughput::Bytes(128))
        .bench_function("try_next/fill128", |b| {
            b.iter(|| gen.try_fill_bytes(&mut buffer).unwrap())
        });
    group
        .throughput(Throughput::Bytes(128))
        .bench_function("next/fill128", |b| b.iter(|| gen.fill_bytes(&mut buffer)));

    group.finish();
}

criterion_group!(benches, bench_rdrand, bench_rdseed);
criterion_main!(benches);
