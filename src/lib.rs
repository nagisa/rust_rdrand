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
//!
//! The random number generators provided by this crate are fairly slow (the latency for these
//! instructions is pretty high), but provide high quality random bits. Caveat is: neither AMD’s
//! nor Intel’s designs are public and therefore are not verifiable for lack of backdoors.
//!
//! Unless you know what you are doing, use the random number generators provided by the `rand`
//! crate (such as `OsRng`) instead.
//!
//! Here are a measurements for select processor architectures. Check [Agner’s instruction tables]
//! for up-to-date listings.
//!
//! <table>
//!   <tr>
//!     <th>Architecture</th>
//!     <th colspan="3">Latency (cycles)</th>
//!     <th>Maximum throughput (per core)</th>
//!   </tr>
//!   <tr>
//!     <td></td>
//!     <td>u16</td>
//!     <td>u32</td>
//!     <td>u64</td>
//!     <td></td>
//!   </tr>
//!   <tr>
//!     <td>AMD Ryzen</td>
//!     <td>~1200</td>
//!     <td>~1200</td>
//!     <td>~2500</td>
//!     <td>~12MB/s @ 3.7GHz</td>
//!   </tr>
//!   <tr>
//!     <td>Intel Skylake</td>
//!     <td>460</td>
//!     <td>460</td>
//!     <td>460</td>
//!     <td>~72MB/s @ 4.2GHz</td>
//!   </tr>
//!   <tr>
//!     <td>Intel Haswell</td>
//!     <td>320</td>
//!     <td>320</td>
//!     <td>320</td>
//!     <td>~110MB/s @ 4.4GHz</td>
//!   </tr>
//! </table>
//!
//! [Agner’s instruction tables]: http://agner.org/optimize/
#![cfg_attr(not(feature = "std"), no_std)]

pub mod changelog;
mod errors;

pub use errors::ErrorCode;
use rand_core::{CryptoRng, Error, RngCore};

#[cold]
#[inline(never)]
pub(crate) fn busy_loop_fail(code: ErrorCode) -> ! {
    panic!("{}", code);
}

/// A cryptographically secure statistically uniform, non-periodic and non-deterministic random bit
/// generator.
///
/// Note that this generator may be implemented using a deterministic algorithm that is reseeded
/// routinely from a non-deterministic entropy source to achieve the desirable properties.
///
/// This generator is a viable replacement to any generator, however, since nobody has audited
/// this hardware implementation yet, the usual disclaimers as to their suitability apply.
///
/// It is potentially faster than `OsRng`, but is only supported by more recent architectures such
/// as Intel Ivy Bridge and AMD Zen.
#[derive(Clone, Copy)]
pub struct RdRand(());

/// A cryptographically secure non-deterministic random bit generator.
///
/// This generator produces high-entropy output and is suited to seed other pseudo-random
/// generators.
///
/// This instruction is only supported by recent architectures such as Intel Broadwell and AMD Zen.
///
/// This generator is not intended for general random number generation purposes and should be used
/// to seed other generators implementing [rand_core::SeedableRng].
#[derive(Clone, Copy)]
pub struct RdSeed(());

impl CryptoRng for RdRand {}
impl CryptoRng for RdSeed {}

mod arch {
    #[cfg(target_arch = "x86")]
    pub use core::arch::x86::*;
    #[cfg(target_arch = "x86_64")]
    pub use core::arch::x86_64::*;

    #[cfg(target_arch = "x86")]
    pub(crate) unsafe fn _rdrand64_step(dest: &mut u64) -> i32 {
        let mut ret1: u32 = 0;
        let mut ret2: u32 = 0;
        let ok = _rdrand32_step(&mut ret1) & _rdrand32_step(&mut ret2);
        *dest = (ret1 as u64) << 32 | (ret2 as u64);
        ok
    }

    #[cfg(target_arch = "x86")]
    pub(crate) unsafe fn _rdseed64_step(dest: &mut u64) -> i32 {
        let mut ret1: u32 = 0;
        let mut ret2: u32 = 0;
        let ok = _rdseed32_step(&mut ret1) & _rdseed32_step(&mut ret2);
        *dest = (ret1 as u64) << 32 | (ret2 as u64);
        ok
    }
}

// See the following documentation for usage (in particular wrt retries) recommendations:
//
// https://software.intel.com/content/www/us/en/develop/articles/intel-digital-random-number-generator-drng-software-implementation-guide.html
macro_rules! loop_rand {
    ("rdrand", $el: ty, $step: path) => {{
        let mut idx = 0;
        loop {
            let mut el: $el = 0;
            if $step(&mut el) != 0 {
                break Ok(el);
            } else if idx == 10 {
                break Err(ErrorCode::HardwareFailure);
            }
            idx += 1;
        }
    }};
    ("rdseed", $el: ty, $step: path) => {{
        let mut idx = 0;
        loop {
            let mut el: $el = 0;
            if $step(&mut el) != 0 {
                break Ok(el);
            } else if idx == 127 {
                break Err(ErrorCode::HardwareFailure);
            }
            idx += 1;
            arch::_mm_pause();
        }
    }};
}

#[inline(always)]
fn authentic_amd() -> bool {
    let cpuid0 = unsafe { arch::__cpuid(0) };
    matches!(
        (cpuid0.ebx, cpuid0.ecx, cpuid0.edx),
        (0x68747541, 0x444D4163, 0x69746E65)
    )
}

#[inline(always)]
fn amd_family(cpuid1: &arch::CpuidResult) -> u32 {
    ((cpuid1.eax >> 8) & 0xF) + ((cpuid1.eax >> 20) & 0xFF)
}

#[inline(always)]
fn has_rdrand(cpuid1: &arch::CpuidResult) -> bool {
    const FLAG: u32 = 1 << 30;
    cpuid1.ecx & FLAG == FLAG
}

#[inline(always)]
fn has_rdseed() -> bool {
    const FLAG: u32 = 1 << 18;
    unsafe { arch::__cpuid(7).ebx & FLAG == FLAG }
}

/// NB: On AMD processor families < 0x17, we want to unconditionally disable RDRAND
/// and RDSEED. Executing these instructions on these processors can return
/// non-random data (0) while also reporting a success.
///
/// See:
/// * https://github.com/systemd/systemd/issues/11810
/// * https://lore.kernel.org/all/776cb5c2d33e7fd0d2893904724c0e52b394f24a.1565817448.git.thomas.lendacky@amd.com/
///
/// We take extra care to do so even if `-Ctarget-features=+rdrand` have been
/// specified, in order to prevent users from shooting themselves in their feet.
const FIRST_GOOD_AMD_FAMILY: u32 = 0x17;

macro_rules! is_available {
    ("rdrand") => {{
        if authentic_amd() {
            let cpuid1 = unsafe { arch::__cpuid(1) };
            has_rdrand(&cpuid1) && amd_family(&cpuid1) >= FIRST_GOOD_AMD_FAMILY
        } else {
            cfg!(target_feature = "rdrand") || has_rdrand(&unsafe { arch::__cpuid(1) })
        }
    }};
    ("rdseed") => {{
        if authentic_amd() {
            amd_family(&unsafe { arch::__cpuid(1) }) >= FIRST_GOOD_AMD_FAMILY && has_rdseed()
        } else {
            cfg!(target_feature = "rdrand") || has_rdseed()
        }
    }};
}

macro_rules! impl_rand {
    ($gen:ident, $feat:tt, $step16: path, $step32:path, $step64:path,
     maxstep = $maxstep:path, maxty = $maxty: ty) => {
        impl $gen {
            /// Create a new instance of the random number generator.
            ///
            /// This constructor checks whether the CPU the program is running on supports the
            /// instruction necessary for this generator to operate. If the instruction is not
            /// supported, an error is returned.
            pub fn new() -> Result<Self, ErrorCode> {
                if cfg!(target_env = "sgx") {
                    if cfg!(target_feature = $feat) {
                        Ok($gen(()))
                    } else {
                        Err(ErrorCode::UnsupportedInstruction)
                    }
                } else if is_available!($feat) {
                    Ok($gen(()))
                } else {
                    Err(ErrorCode::UnsupportedInstruction)
                }
            }

            /// Create a new instance of the random number generator.
            ///
            /// # Safety
            ///
            /// This constructor is unsafe because it doesn't check that the CPU supports the
            /// instruction, but devolves this responsibility to the caller.
            pub unsafe fn new_unchecked() -> Self {
                $gen(())
            }

            /// Generate a single random `u16` value.
            ///
            /// The underlying instruction may fail for variety reasons (such as actual hardware
            /// failure or exhausted entropy), however the exact reason for the failure is not
            /// usually exposed.
            ///
            /// This method will retry calling the instruction a few times, however if all the
            /// attempts fail, it will return `None`.
            ///
            /// In case `Err` is returned, the caller should assume that a non-recoverable failure
            /// has occured and use another random number genrator instead.
            #[inline(always)]
            pub fn try_next_u16(&self) -> Result<u16, ErrorCode> {
                #[target_feature(enable = $feat)]
                unsafe fn imp() -> Result<u16, ErrorCode> {
                    loop_rand!($feat, u16, $step16)
                }
                unsafe { imp() }
            }

            /// Generate a single random `u32` value.
            ///
            /// The underlying instruction may fail for variety reasons (such as actual hardware
            /// failure or exhausted entropy), however the exact reason for the failure is not
            /// usually exposed.
            ///
            /// This method will retry calling the instruction a few times, however if all the
            /// attempts fail, it will return `None`.
            ///
            /// In case `Err` is returned, the caller should assume that a non-recoverable failure
            /// has occured and use another random number genrator instead.
            #[inline(always)]
            pub fn try_next_u32(&self) -> Result<u32, ErrorCode> {
                #[target_feature(enable = $feat)]
                unsafe fn imp() -> Result<u32, ErrorCode> {
                    loop_rand!($feat, u32, $step32)
                }
                unsafe { imp() }
            }

            /// Generate a single random `u64` value.
            ///
            /// The underlying instruction may fail for variety reasons (such as actual hardware
            /// failure or exhausted entropy), however the exact reason for the failure is not
            /// usually exposed.
            ///
            /// This method will retry calling the instruction a few times, however if all the
            /// attempts fail, it will return `None`.
            ///
            /// In case `Err` is returned, the caller should assume that a non-recoverable failure
            /// has occured and use another random number genrator instead.
            ///
            /// Note, that on 32-bit targets, there’s no underlying instruction to generate a
            /// 64-bit number, so it is emulated with the 32-bit version of the instruction.
            #[inline(always)]
            pub fn try_next_u64(&self) -> Result<u64, ErrorCode> {
                #[target_feature(enable = $feat)]
                unsafe fn imp() -> Result<u64, ErrorCode> {
                    loop_rand!($feat, u64, $step64)
                }
                unsafe { imp() }
            }

            /// Fill a buffer `dest` with random data.
            ///
            /// This method will use the most appropriate variant of the instruction available on
            /// the machine to achieve the greatest single-core throughput, however it has a
            /// slightly higher setup cost than the plain `next_u32` or `next_u64` methods.
            ///
            /// The underlying instruction may fail for variety reasons (such as actual hardware
            /// failure or exhausted entropy), however the exact reason for the failure is not
            /// usually exposed.
            ///
            /// This method will retry calling the instruction a few times, however if all the
            /// attempts fail, it will return an error.
            ///
            /// If an error is returned, the caller should assume that an non-recoverable hardware
            /// failure has occured and use another random number genrator instead.
            #[inline(always)]
            pub fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), ErrorCode> {
                #[target_feature(enable = $feat)]
                unsafe fn imp(dest: &mut [u8]) -> Result<(), ErrorCode> {
                    fn slow_fill_bytes<'a>(
                        mut left: &'a mut [u8],
                        mut right: &'a mut [u8],
                    ) -> Result<(), ErrorCode> {
                        let mut word;
                        let mut buffer: &[u8] = &[];
                        loop {
                            if left.is_empty() {
                                if right.is_empty() {
                                    break;
                                }
                                ::core::mem::swap(&mut left, &mut right);
                            }
                            if buffer.is_empty() {
                                word =
                                    unsafe { loop_rand!($feat, $maxty, $maxstep) }?.to_ne_bytes();
                                buffer = &word[..];
                            }
                            let len = left.len().min(buffer.len());
                            let (copy_src, leftover) = buffer.split_at(len);
                            let (copy_dest, dest_leftover) = { left }.split_at_mut(len);
                            buffer = leftover;
                            left = dest_leftover;
                            copy_dest.copy_from_slice(copy_src);
                        }
                        Ok(())
                    }

                    let destlen = dest.len();
                    if destlen > ::core::mem::size_of::<$maxty>() {
                        let (left, mid, right) = dest.align_to_mut();
                        for el in mid {
                            *el = loop_rand!($feat, $maxty, $maxstep)?;
                        }

                        slow_fill_bytes(left, right)
                    } else {
                        slow_fill_bytes(dest, &mut [])
                    }
                }
                unsafe { imp(dest) }
            }
        }

        impl RngCore for $gen {
            /// Generate a single random `u32` value.
            ///
            /// The underlying instruction may fail for variety reasons (such as actual hardware
            /// failure or exhausted entropy), however the exact reason for the failure is not
            /// usually exposed.
            ///
            /// # Panic
            ///
            /// This method will retry calling the instruction a few times, however if all the
            /// attempts fail, it will `panic`.
            ///
            /// In case `panic` occurs, the caller should assume that an non-recoverable
            /// hardware failure has occured and use another random number genrator instead.
            #[inline(always)]
            fn next_u32(&mut self) -> u32 {
                match self.try_next_u32() {
                    Ok(result) => result,
                    Err(c) => busy_loop_fail(c),
                }
            }

            /// Generate a single random `u64` value.
            ///
            /// The underlying instruction may fail for variety reasons (such as actual hardware
            /// failure or exhausted entropy), however the exact reason for the failure is not
            /// usually exposed.
            ///
            /// Note, that on 32-bit targets, there’s no underlying instruction to generate a
            /// 64-bit number, so it is emulated with the 32-bit version of the instruction.
            ///
            /// # Panic
            ///
            /// This method will retry calling the instruction a few times, however if all the
            /// attempts fail, it will `panic`.
            ///
            /// In case `panic` occurs, the caller should assume that an non-recoverable
            /// hardware failure has occured and use another random number genrator instead.
            #[inline(always)]
            fn next_u64(&mut self) -> u64 {
                match self.try_next_u64() {
                    Ok(result) => result,
                    Err(c) => busy_loop_fail(c),
                }
            }

            /// Fill a buffer `dest` with random data.
            ///
            /// See `try_fill_bytes` for a more extensive documentation.
            ///
            /// # Panic
            ///
            /// This method will panic any time `try_fill_bytes` would return an error.
            #[inline(always)]
            fn fill_bytes(&mut self, dest: &mut [u8]) {
                match self.try_fill_bytes(dest) {
                    Ok(result) => result,
                    Err(c) => busy_loop_fail(c),
                }
            }

            /// Fill a buffer `dest` with random data.
            ///
            /// This method will use the most appropriate variant of the instruction available on
            /// the machine to achieve the greatest single-core throughput, however it has a
            /// slightly higher setup cost than the plain `next_u32` or `next_u64` methods.
            ///
            /// The underlying instruction may fail for variety reasons (such as actual hardware
            /// failure or exhausted entropy), however the exact reason for the failure is not
            /// usually exposed.
            ///
            /// This method will retry calling the instruction a few times, however if all the
            /// attempts fail, it will return an error.
            ///
            /// If an error is returned, the caller should assume that an non-recoverable hardware
            /// failure has occured and use another random number genrator instead.
            #[inline(always)]
            fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), Error> {
                self.try_fill_bytes(dest).map_err(Into::into)
            }
        }
    };
}

#[cfg(target_arch = "x86_64")]
impl_rand!(
    RdRand,
    "rdrand",
    arch::_rdrand16_step,
    arch::_rdrand32_step,
    arch::_rdrand64_step,
    maxstep = arch::_rdrand64_step,
    maxty = u64
);
#[cfg(target_arch = "x86_64")]
impl_rand!(
    RdSeed,
    "rdseed",
    arch::_rdseed16_step,
    arch::_rdseed32_step,
    arch::_rdseed64_step,
    maxstep = arch::_rdseed64_step,
    maxty = u64
);
#[cfg(target_arch = "x86")]
impl_rand!(
    RdRand,
    "rdrand",
    arch::_rdrand16_step,
    arch::_rdrand32_step,
    arch::_rdrand64_step,
    maxstep = arch::_rdrand32_step,
    maxty = u32
);
#[cfg(target_arch = "x86")]
impl_rand!(
    RdSeed,
    "rdseed",
    arch::_rdseed16_step,
    arch::_rdseed32_step,
    arch::_rdseed64_step,
    maxstep = arch::_rdseed32_step,
    maxty = u32
);

#[cfg(test)]
mod test {
    use super::{RdRand, RdSeed};
    use rand_core::RngCore;

    #[test]
    fn rdrand_works() {
        let _ = RdRand::new().map(|mut r| {
            r.next_u32();
            r.next_u64();
        });
    }

    #[repr(C, align(8))]
    struct FillBuffer([u8; 64]);

    #[test]
    fn fill_fills_all_bytes() {
        let _ = RdRand::new().map(|mut r| {
            let mut test_buffer;
            let mut fill_buffer = FillBuffer([0; 64]); // make sure buffer is aligned to 8-bytes...
            let test_cases = [
                (0, 64), // well aligned
                (8, 64), // well aligned
                (0, 64), // well aligned
                (5, 64), // left is non-empty, right is empty.
                (0, 63), // left is empty, right is non-empty.
                (5, 63), // left and right both are non-empty.
                (5, 61), // left and right both are non-empty.
                (0, 8),   // 1 word-worth of data, aligned.
                (1, 9),   // 1 word-worth of data, misaligned.
                (0, 7),   // less than 1 word of data.
                (1, 7),   // less than 1 word of data.
            ];
            'outer: for &(start, end) in &test_cases {
                test_buffer = [0; 64];
                for _ in 0..512 {
                    fill_buffer.0 = [0; 64];
                    r.fill_bytes(&mut fill_buffer.0[start..end]);
                    for (b, p) in test_buffer.iter_mut().zip(fill_buffer.0.iter()) {
                        *b = *b | *p;
                    }
                    if (&test_buffer[start..end]).iter().all(|x| *x != 0) {
                        assert!(
                            test_buffer[..start].iter().all(|x| *x == 0),
                            "all other values must be 0"
                        );
                        assert!(
                            test_buffer[end..].iter().all(|x| *x == 0),
                            "all other values must be 0"
                        );
                        continue 'outer;
                    }
                }
                panic!("wow, we broke it? {} {} {:?}", start, end, &test_buffer[..])
            }
        });
    }

    #[test]
    fn rdseed_works() {
        let _ = RdSeed::new().map(|mut r| {
            r.next_u32();
            r.next_u64();
        });
    }
}
