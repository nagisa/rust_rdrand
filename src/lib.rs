// Copyright © 2014, Simonas Kazlauskas <git@kazlauskas.me>
//
// Permission to use, copy, modify, and/or distribute this software for any purpose with or without
// fee is hereby granted, provided that the above copyright notice and this permission notice
// appear in all copies.
//
// THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES WITH REGARD TO THIS
// SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE
// AUTHOR BE LIABLE FOR ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES
// WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS, WHETHER IN AN ACTION OF CONTRACT,
// NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING OUT OF OR IN CONNECTION WITH THE USE OR PERFORMANCE
// OF THIS SOFTWARE.
//! An implementation of random number generators based on `rdrand` and `rdseed` instructions.
extern crate rand;
extern crate unreachable;

use rand::Rng;
use std::result::Result;


#[derive(Copy, Clone, Debug)]
pub enum Error {
    /// The processor does not support the instruction used in the generator.
    UnsupportedProcessor
}

impl ::std::error::Error for Error {
    fn description(&self) -> &str {
        match self {
            &Error::UnsupportedProcessor => "processor does not support the instruction",
        }
    }
}

impl ::std::fmt::Display for Error {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        match self {
            &Error::UnsupportedProcessor => write!(f, "processor does not support the instruction")
        }
    }
}

impl From<Error> for ::std::io::Error {
    fn from(e: Error) -> ::std::io::Error {
        ::std::io::Error::new(::std::io::ErrorKind::Other, format!("{}", e))
    }
}

/// A cryptographically secure pseudo-random number generator.
///
/// This generator is a viable replacement to any [std::rand] generator, however, since nobody has
/// audited Intel hardware yet, the usual disclaimers apply.
///
/// It is much faster than `OsRng` (and slower than `StdRng`), but is only supported on more recent
/// (since Ivy Bridge) Intel processors.
///
/// [std::rand]: http://doc.rust-lang.org/std/rand/index.html
#[allow(missing_copy_implementations)]
#[derive(Clone)]
pub struct RdRand(());

impl RdRand {
    /// Build a generator object. The function will only succeed if `rdrand` instruction can be
    /// successfully used.
    pub fn new() -> Result<RdRand, Error> {
        if unsafe { librdrand_rust_has_rdrand() } {
            return Ok(RdRand(()));
        } else {
            return Err(Error::UnsupportedProcessor);
        }
    }

    /// Generate a u16 value.
    pub fn next_u16(&self) -> u16 {
        unsafe {
            librdrand_rust_rand_16()
        }
    }
}

impl Rng for RdRand {
    fn next_u32(&mut self) -> u32 {
        unsafe {
            librdrand_rust_rand_32()
        }
    }

    fn next_u64(&mut self) -> u64 {
        unsafe {
            librdrand_rust_rand_64()
        }
    }
}


/// A random number generator suited to seed other pseudo-random generators.
///
/// This instruction currently is only available in Intel Broadwell processors.
///
/// Note: The implementation has not been tested due to the lack of hardware supporting the feature
#[allow(missing_copy_implementations)]
#[derive(Clone)]
pub struct RdSeed(());

impl RdSeed {
    pub fn new() -> Result<RdSeed, Error> {
        if unsafe { librdrand_rust_has_rdseed() } {
            Ok(RdSeed(()))
        } else {
            Err(Error::UnsupportedProcessor)
        }
    }

    /// Generate a u16 value.
    pub fn next_u16(&self) -> u16 {
        unsafe {
            librdrand_rust_seed_16()
        }
    }
}

impl Rng for RdSeed {
    fn next_u32(&mut self) -> u32 {
        unsafe {
            librdrand_rust_seed_32()
        }
    }

    fn next_u64(&mut self) -> u64 {
        unsafe {
            librdrand_rust_seed_64()
        }
    }
}

#[cfg(target_arch = "x86_64")]
extern {
    fn librdrand_rust_rand_64() -> u64;
    fn librdrand_rust_rand_32() -> u32;
    fn librdrand_rust_rand_16() -> u16;
    fn librdrand_rust_seed_64() -> u64;
    fn librdrand_rust_seed_32() -> u32;
    fn librdrand_rust_seed_16() -> u16;
    fn librdrand_rust_has_rdrand() -> bool;
    fn librdrand_rust_has_rdseed() -> bool;
}

macro_rules! unreachable {
    ($($name: ident -> $ty: ident)+) => {
        #[inline(always)]
        fn $name() -> $ty { unreachable::unreachable() }
    }
}

#[cfg(not(target_arch = "x86_64"))]
unreachable!(librdrand_rust_rand_16 -> u16,
             librdrand_rust_rand_32 -> u32,
             librdrand_rust_rand_64 -> u64,
             librdrand_rust_seed_16 -> u16,
             librdrand_rust_seed_32 -> u32,
             librdrand_rust_seed_64 -> u64);
#[cfg(not(target_arch = "x86_64"))]
#[inline(always)]
fn librdrand_rust_has_rdrand() -> bool { false }
#[cfg(not(target_arch = "x86_64"))]
#[inline(always)]
fn librdrand_rust_has_rdseed() -> bool { false }

#[test]
fn rdrand_works() {
    let _ = RdRand::new().map(|mut r| {
        r.next_u32();
        r.next_u64();
    });
}

#[test]
fn rdseed_works() {
    let _ = RdSeed::new().map(|mut r| {
        r.next_u32();
        r.next_u64();
    });
}
