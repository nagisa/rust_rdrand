extern crate rdrand;
extern crate test;

use test::Bencher;
use std::rand::Rng;
use std::rand::StdRng;
use std::rand::OsRng;

#[bench]
fn bench_u16(b : &mut Bencher) {
    if let Ok(mut gen) = rdrand::RdRand::new() {
        b.iter(|| {
            gen.next_u16();
        });
    } else {
        panic!("rdrand instruction is not supported!");
    }
}

#[bench]
fn bench_u32(b : &mut Bencher) {
    if let Ok(mut gen) = rdrand::RdRand::new() {
        b.iter(|| {
            gen.next_u32();
        });
    } else {
        panic!("rdrand instruction is not supported!");
    }
}

#[bench]
fn bench_u64(b : &mut Bencher) {
    if let Ok(mut gen) = rdrand::RdRand::new() {
        b.iter(|| {
            gen.next_u64();
        });
    } else {
        panic!("rdrand instruction is not supported!");
    }
}

// StdRng is the default for non-crypto uses.
#[bench]
fn bench_stdrng_u64(b : &mut Bencher) {
    if let Ok(mut gen) = StdRng::new() {
        b.iter(|| {
            gen.next_u64();
        });
    } else {
        panic!("couldn’t create StdRng");
    }
}

// OsRng is supposed to be the default for crypto uses.
#[bench]
fn bench_osrng_u64(b : &mut Bencher) {
    if let Ok(mut gen) = OsRng::new() {
        b.iter(|| {
            gen.next_u64();
        });
    } else {
        panic!("couldn’t create OsRng");
    }
}
