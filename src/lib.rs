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

//! An implementation of random number generator based on `rdrand` instruction.
//!
//! `rdrand` is claimed to be a cryptographically secure PRNG. It is much faster than `OsRng` (and
//! slower than `StdRng`), but is only supported on more recent Intel processors.
//!
//! The generator provided by this crate is a viable replacement to any
//! [std::rand](http://doc.rust-lang.org/std/rand/index.html) generator, however, since nobody has
//! audited Intel hardware yet, the usual disclaimers apply.

#![feature(asm)]
use std::rand::Rng;
use std::result::Result;


struct PrivateInner;
pub struct RdRand(PrivateInner);


#[deriving(Copy)]
#[stable]
pub enum Error {
    /// The processor does not support the `rdrand` instruction.
    UnsupportedProcessor
}


/// Check whether RDRAND instruction is supported
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
fn is_supported() -> bool {
    const FLAG : u32 = 1 << 30;
    let (mut b, mut c, mut d) : (u32, u32, u32);
    unsafe {
        asm!("
             mov eax, 0;
             cpuid;
             mov $0, ebx;
             mov $1, ecx;
             mov $2, edx;
            "
            : "=r"(b), "=r"(c), "=r"(d)
            :
            : "eax", "ebx", "ecx", "edx"
            : "intel");
    }
    // Genuine Intel
    if b != 0x756E6547 || d != 0x49656e69 || c != 0x6C65746E {
        return false;
    }
    // 30th bit of 1st cpuid function and 0th subfunction is set
    unsafe {
        asm!("
             mov eax, 1;
             mov ecx, 0;
             cpuid;
             mov $0, ecx;
            "
            : "=r"(c)
            :
            : "eax", "ebx", "ecx", "edx"
            : "intel");
    }
    return c & FLAG == FLAG
}


impl RdRand {
    /// Build a generator object. The function will only succeed if `rdrand` instruction can be
    /// successfully used.
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    pub fn new() -> Result<RdRand, Error> {
        if is_supported() {
            return Ok(RdRand(PrivateInner));
        } else {
            return Err(Error::UnsupportedProcessor);
        }
    }

    #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
    pub fn new() -> Result<RdRand, Error> {
        Err(Error::UnsupportedProcessor)
    }

    /// Generate a value.
    #[inline]
    fn gen_value<T>(&self) -> T {
        let mut var;
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
