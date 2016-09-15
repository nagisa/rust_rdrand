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

#![feature(asm)]

extern crate rand;

use rand::Rng;
use std::result::Result;
mod util;


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
        if util::is_intel() && util::has_rdrand() {
            return Ok(RdRand(()));
        } else {
            return Err(Error::UnsupportedProcessor);
        }
    }

    /// Generate a value.
    #[inline]
    fn gen_value<T>(&self) -> T {
        let var;
        unsafe {
            asm!("1: rdrand $0; jnc 1b;" : "=r"(var));
        }
        var
    }

    /// Generate a u16 value.
    pub fn next_u16(&self) -> u16 {
        self.gen_value()
    }
}

impl Rng for RdRand {
    fn next_u32(&mut self) -> u32 {
        self.gen_value()
    }

    fn next_u64(&mut self) -> u64 {
        self.gen_value()
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
        if util::is_intel() && util::has_rdseed() {
            return Ok(RdSeed(()));
        } else {
            return Err(Error::UnsupportedProcessor);
        }
    }

    /// Generate a value.
    #[inline]
    fn gen_value<T>(&self) -> T {
        let var;
        unsafe {
            asm!("1: rdseed $0; jnc 1b;" : "=r"(var));
        }
        var
    }

    /// Generate a u16 value.
    pub fn next_u16(&self) -> u16 {
        self.gen_value()
    }
}

impl Rng for RdSeed {
    fn next_u32(&mut self) -> u32 {
        self.gen_value()
    }

    fn next_u64(&mut self) -> u64 {
        self.gen_value()
    }
}

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
