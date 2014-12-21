extern crate rdrand;
extern crate test;

use test::Bencher;
use std::rand::Rng;
use std::rand::StdRng;
use std::rand::OsRng;

use std::sync::{Once, ONCE_INIT};
static NORDRAND: Once = ONCE_INIT;
static NORDSEED: Once = ONCE_INIT;

fn report_rdrand() {
    NORDRAND.doit(|| {
        println!("rdrand instruction is not supported!");
    });
}

fn report_rdseed() {
    NORDSEED.doit(|| {
        println!("rdseed instruction is not supported!");
    });
}

#[bench]
fn bench_u16(b : &mut Bencher) {
    if let Ok(gen) = rdrand::RdRand::new() {
        b.bytes = 2;
        b.iter(|| {
            gen.next_u16();
        });
    } else {
        report_rdrand();
    }
}

#[bench]
fn bench_u32(b : &mut Bencher) {
    if let Ok(mut gen) = rdrand::RdRand::new() {
        b.bytes = 4;
        b.iter(|| {
            gen.next_u32();
        });
    } else {
        report_rdrand();
    }
}

#[bench]
fn bench_u64(b : &mut Bencher) {
    if let Ok(mut gen) = rdrand::RdRand::new() {
        b.bytes = 8;
        b.iter(|| {
            gen.next_u64();
        });
    } else {
        report_rdrand();
    }
}

#[bench]
fn bench_rdseed_u16(b : &mut Bencher) {
    if let Ok(gen) = rdrand::RdSeed::new() {
        b.bytes = 2;
        b.iter(|| {
            gen.next_u16();
        });
    } else {
        report_rdseed();
    }
}

#[bench]
fn bench_rdseed_u32(b : &mut Bencher) {
    if let Ok(mut gen) = rdrand::RdSeed::new() {
        b.iter(|| {
            gen.next_u32();
        });
    } else {
        report_rdseed();
    }
}

#[bench]
fn bench_rdseed_u64(b : &mut Bencher) {
    if let Ok(mut gen) = rdrand::RdSeed::new() {
        b.iter(|| {
            gen.next_u64();
        });
    } else {
        report_rdseed();
    }
}

// StdRng is the default for non-crypto uses.
#[bench]
fn bench_stdrng_u64(b : &mut Bencher) {
    if let Ok(mut gen) = StdRng::new() {
        b.bytes = 8;
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
        b.bytes = 8;
        b.iter(|| {
            gen.next_u64();
        });
    } else {
        panic!("couldn’t create OsRng");
    }
}
