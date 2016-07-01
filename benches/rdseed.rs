#![feature(test)]
extern crate rand;
extern crate rdrand;
extern crate test;

use rand::Rng;
use test::Bencher;

#[bench]
fn bench_rdseed_u16(b : &mut Bencher) {
    if let Ok(gen) = rdrand::RdSeed::new() {
        b.bytes = 2;
        b.iter(|| {
            gen.next_u16()
        });
    }
}

#[bench]
fn bench_rdseed_u32(b : &mut Bencher) {
    if let Ok(mut gen) = rdrand::RdSeed::new() {
        b.iter(|| {
            gen.next_u32()
        });
    }
}

#[bench]
fn bench_rdseed_u64(b : &mut Bencher) {
    if let Ok(mut gen) = rdrand::RdSeed::new() {
        b.iter(|| {
            gen.next_u64()
        });
    }
}
