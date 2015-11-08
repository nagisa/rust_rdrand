#![feature(test)]
extern crate rand;
extern crate rdrand;
extern crate test;

use test::Bencher;
use rand::Rng;

#[bench]
fn bench_u16(b : &mut Bencher) {
    if let Ok(gen) = rdrand::RdRand::new() {
        b.bytes = 2;
        b.iter(|| {
            gen.next_u16()
        });
    } else {
        panic!("rdrand instruction is not supported!");
    }
}

#[bench]
fn bench_u32(b : &mut Bencher) {
    if let Ok(mut gen) = rdrand::RdRand::new() {
        b.bytes = 4;
        b.iter(|| {
            gen.next_u32()
        });
    } else {
        panic!("rdrand instruction is not supported!");
    }
}

#[bench]
fn bench_u64(b : &mut Bencher) {
    if let Ok(mut gen) = rdrand::RdRand::new() {
        b.bytes = 8;
        b.iter(|| {
            gen.next_u64()
        });
    } else {
        panic!("rdrand instruction is not supported!");
    }
}


