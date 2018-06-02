// Copyright © 2014, Simonas Kazlauskas <rdrand@kazlauskas.me>
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

#![no_std]
#![feature(slice_align_to)]

extern crate rand_core;

use rand_core::{RngCore, CryptoRng, Error, ErrorKind};
use core::slice;

const RETRY_LIMIT: u8 = 127;

#[cold]
#[inline(never)]
pub fn busy_loop_fail() -> ! {
    panic!("hardware generator failure");
}

/// A cryptographically secure statistically uniform, non-periodic and non-deterministic random bit
/// generator.
///
/// Note that this generator may be implemented using a deterministic algorithm that is reseeded
/// routinely from a non-deterministic entropy source to achieve the desirable properties.
///
/// This generator is a viable replacement to any generator, however, since nobody has audited
/// Intel or AMD hardware yet, the usual disclaimers as to their suitability apply.
///
/// It is much faster than `OsRng`, but is only supported on more recent Intel
/// (Ivy Bridge and later) and AMD (Ryzen and later) processors.
#[derive(Clone, Copy)]
pub struct RdRand(());

/// A cryptographically secure non-deterministic random bit generator.
///
/// This generator produces high-entropy output and is suited to seed other pseudo-random
/// generators.
///
/// This instruction currently is only available in Intel Broadwell (and later) and AMD Ryzen
/// processors.
///
/// This generator is not intended for general random number generation purposes and should be used
/// to seed other generators implementing [SeedableRng].
#[derive(Clone, Copy)]
pub struct RdSeed(());

impl CryptoRng for RdRand {}
impl CryptoRng for RdSeed {}

mod arch {
    #[cfg(target_arch = "x86_64")]
    pub use core::arch::x86_64::*;
    #[cfg(target_arch = "x86")]
    pub use core::arch::x86::*;

    #[cfg(target_arch = "x86")]
    pub(crate) unsafe fn _rdrand64_step(dest: &mut u64) -> i32 {
        let mut ret1: u32 = ::core::mem::uninitialized();
        let mut ret2: u32 = ::core::mem::uninitialized();
        if _rdrand32_step(&mut ret1) != 0 && _rdrand32_step(&mut ret2) != 0 {
            *dest = (ret1 as u64) << 32 | (ret2 as u64);
            1
        } else {
            0
        }
    }

    #[cfg(target_arch = "x86")]
    pub(crate) unsafe fn _rdseed64_step(dest: &mut u64) -> i32 {
        let mut ret1: u32 = ::core::mem::uninitialized();
        let mut ret2: u32 = ::core::mem::uninitialized();
        if _rdseed32_step(&mut ret1) != 0 && _rdseed32_step(&mut ret2) != 0 {
            *dest = (ret1 as u64) << 32 | (ret2 as u64);
            1
        } else {
            0
        }
    }
}

macro_rules! check_cpuid {
    ("rdrand") => { {
        const FLAG : u32 = 1 << 30;
        ::arch::__cpuid(1).ecx & FLAG == FLAG
    } };
    ("rdseed") => { {
        const FLAG : u32 = 1 << 18;
        ::arch::__cpuid(7).ebx & FLAG == FLAG
    } };
}

macro_rules! loop_rand {
    ($feat: tt, $el: ty, $step: path) => { {
        #[target_feature(enable = $feat)]
        unsafe fn imp() -> Option<$el> {
            let mut ret: $el = ::core::mem::uninitialized();
            for _ in 0..RETRY_LIMIT {
                if $step(&mut ret) != 0 {
                    return Some(ret);
                }
            }
            return None;
        }
        unsafe { imp() }
    } }
}

macro_rules! impl_rand {
    ($gen:ident, $feat:tt, $step16: path, $step32:path, $step64:path,
     maxstep = $maxstep:path, maxty = $maxty: ty, next = $trynext:ident) => {
        impl $gen {
            pub fn new() -> Result<Self, Error> {
                unsafe {
                    if cfg!(target_feature=$feat) || check_cpuid!($feat) {
                        Ok($gen(()))
                    } else {
                        Err(Error::new(rand_core::ErrorKind::Unavailable,
                                       "the instruction is not supported"))
                    }
                }
            }

            #[inline(always)]
            pub fn try_next_u16(&self) -> Option<u16> {
                loop_rand!($feat, u16, $step16)
            }

            #[inline(always)]
            pub fn try_next_u32(&self) -> Option<u32> {
                loop_rand!($feat, u32, $step32)
            }

            #[inline(always)]
            pub fn try_next_u64(&self) -> Option<u64> {
                loop_rand!($feat, u64, $step64)
            }
        }

        impl RngCore for $gen {
            #[inline(always)]
            fn next_u32(&mut self) -> u32 {
                if let Some(result) = self.try_next_u32() {
                    result
                } else {
                    busy_loop_fail()
                }
            }

            #[inline(always)]
            fn next_u64(&mut self) -> u64 {
                if let Some(result) = self.try_next_u64() {
                    result
                } else {
                    busy_loop_fail()
                }
            }

            #[inline(always)]
            fn fill_bytes(&mut self, dest: &mut [u8]) {
                if let Err(_) = self.try_fill_bytes(dest) {
                    busy_loop_fail()
                }
            }

            fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), Error> {
                #[target_feature(enable = $feat)]
                unsafe fn imp_fast(dest: &mut [$maxty])
                -> Result<(), Error>
                {
                    'outer: for el in dest {
                        for _ in 0..RETRY_LIMIT {
                            if $maxstep(el) != 0 {
                                continue 'outer;
                            }
                        }
                        return Err(Error::new(ErrorKind::Unexpected, "hardware generator failure"));
                    }
                    Ok(())
                }

                unsafe fn imp_slow(this: &mut $gen, mut dest: &mut [u8],
                                   word: &mut $maxty, buffer: &mut &[u8])
                -> Result<(), Error> {
                    while !dest.is_empty() {
                        if buffer.is_empty() {
                            if let Some(w) = $gen::$trynext(&*this) {
                                *word = w;
                                *buffer = slice::from_raw_parts(&word as *const _ as *const u8,
                                                                ::core::mem::size_of::<$maxty>());
                            } else {
                                return Err(Error::new(ErrorKind::Unexpected,
                                                      "hardware generator failure"));
                            }
                        }
                        let len = dest.len().min(buffer.len());
                        let (copy_src, leftover) = buffer.split_at(len);
                        let (copy_dest, dest_leftover) = { dest }.split_at_mut(len);
                        *buffer = leftover;
                        dest = dest_leftover;
                        copy_dest.copy_from_slice(copy_src);
                    }
                    Ok(())
                }

                unsafe {
                    let destlen = dest.len();
                    if destlen > ::core::mem::size_of::<$maxty>() {
                            let (left, mid, right) = dest.align_to_mut();
                            let mut word = 0;
                            let mut buffer: &[u8] = &[];
                            imp_fast(mid)?;
                            imp_slow(self, left, &mut word, &mut buffer)?;
                            imp_slow(self, right, &mut word, &mut buffer)
                    } else {
                        let mut word = 0;
                        let mut buffer: &[u8] = &[];
                        imp_slow(self, dest, &mut word, &mut buffer)
                    }
                }
            }
        }
    }
}

#[cfg(target_arch = "x86_64")]
impl_rand!(RdRand, "rdrand",
           ::arch::_rdrand16_step, ::arch::_rdrand32_step, ::arch::_rdrand64_step,
           maxstep = ::arch::_rdrand64_step, maxty = u64, next = try_next_u64);
#[cfg(target_arch = "x86_64")]
impl_rand!(RdSeed, "rdseed",
           ::arch::_rdseed16_step, ::arch::_rdseed32_step, ::arch::_rdseed64_step,
           maxstep = ::arch::_rdseed64_step, maxty = u64, next = try_next_u64);
#[cfg(target_arch = "x86")]
impl_rand!(RdRand, "rdrand",
           ::arch::_rdrand16_step, ::arch::_rdrand32_step, ::arch::_rdrand64_step,
           maxstep = ::arch::_rdrand32_step, maxty = u32, next = try_next_u32);
#[cfg(target_arch = "x86")]
impl_rand!(RdSeed, "rdseed",
           ::arch::_rdseed16_step, ::arch::_rdseed32_step, ::arch::_rdseed64_step,
           maxstep = ::arch::_rdseed32_step, maxty = u32, next = try_next_u32);

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
