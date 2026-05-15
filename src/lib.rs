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
#![no_std]
pub mod changelog;
mod errors;

use core::hint::spin_loop;
pub use errors::ErrorCode;
use rand_core::{TryCryptoRng, TryRng};

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
/// This instruction is only supported by recent architectures such as Intel Broadwell, AMD Zen,
/// and as an optional feature on AArch64 Armv8.1 and newer.
///
/// This generator is not intended for general random number generation purposes and should be used
/// to seed other generators implementing [rand_core::SeedableRng].
#[derive(Clone, Copy)]
pub struct RdSeed(());

impl TryCryptoRng for RdRand {}
impl TryCryptoRng for RdSeed {}

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

    #[cfg(target_arch = "aarch64")]
    pub(crate) unsafe fn rand(out: &mut u64) -> i32 {
        let value: u64;
        let success: u64;

        unsafe {
            core::arch::asm!(
                "mrs {0}, S3_3_C2_C4_0", // RNDR
                "cset {1:w}, cs",  // Set w{1} to 1 if carry flag is set, else 0
                out(reg) value,
                lateout(reg) success,
                options(nostack)
            );
        }
        *out = value;
        // From ARM spec:
        // If the hardware returns a genuine random number, PSTATE.NZCV is set to 0b0000.
        //
        // If the instruction cannot return a genuine random number in a reasonable period of
        // time, PSTATE.NZCV is set to 0b0100 and the data value returned is 0.
        // So the assembly code returns 0 for success and nonzero for failure, but loop_rand expects
        // the opposite.
        (success == 0) as i32 // Returns 1 for success, 0 for failure
    }

    #[cfg(target_arch = "aarch64")]
    pub(crate) unsafe fn rand32(out: &mut u32) -> i32 {
        let mut out64 = 0u64;
        let status = unsafe { rand(&mut out64) };
        *out = out64 as u32;
        status
    }

    #[cfg(target_arch = "aarch64")]
    pub(crate) unsafe fn seed(out: &mut u64) -> i32 {
        let value: u64;
        let success: u64;

        unsafe {
            core::arch::asm!(
                "mrs {0}, S3_3_C2_C4_1", // RNDRRS
                "cset {1:w}, cs",  // Set w{1} to 1 if carry flag is set, else 0
                out(reg) value,
                lateout(reg) success,
                options(nostack)
            );
        }

        *out = value;
        (success == 0) as i32 // See rand() above for note on the inverted status.
    }

    #[cfg(target_arch = "aarch64")]
    pub(crate) unsafe fn seed32(out: &mut u32) -> i32 {
        let mut out64 = 0u64;
        let status = unsafe { seed(&mut out64) };
        *out = out64 as u32;
        status
    }
}

// See the following documentation for usage (in particular wrt retries) recommendations:
//
// https://software.intel.com/content/www/us/en/develop/articles/intel-digital-random-number-generator-drng-software-implementation-guide.html
macro_rules! loop_rand {
    ("rdrand", $el: ty, $step: path) => {{
        let mut idx = 0;
        #[allow(unused_unsafe)]
        loop {
            let mut el: $el = 0;
            if unsafe { $step(&mut el) } != 0 {
                break Ok(el);
            } else if idx == 10 {
                break Err(ErrorCode::HardwareFailure);
            }
            idx += 1;
        }
    }};
    ("rdseed", $el: ty, $step: path) => {{
        let mut idx = 0;
        #[allow(unused_unsafe)]
        loop {
            let mut el: $el = 0;
            if unsafe { $step(&mut el) } != 0 {
                break Ok(el);
            } else if idx == 127 {
                break Err(ErrorCode::HardwareFailure);
            }
            idx += 1;
            spin_loop();
        }
    }};
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[allow(unused_unsafe)]
#[inline(always)]
fn authentic_amd() -> bool {
    let cpuid0 = unsafe { arch::__cpuid(0) };
    matches!(
        (cpuid0.ebx, cpuid0.ecx, cpuid0.edx),
        (0x68747541, 0x444D4163, 0x69746E65)
    )
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[inline(always)]
fn amd_family(cpuid1: &arch::CpuidResult) -> u32 {
    ((cpuid1.eax >> 8) & 0xF) + ((cpuid1.eax >> 20) & 0xFF)
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[inline(always)]
fn has_rdrand(cpuid1: &arch::CpuidResult) -> bool {
    const FLAG: u32 = 1 << 30;
    cpuid1.ecx & FLAG == FLAG
}

#[cfg(target_arch = "aarch64")]
#[inline(always)]
fn has_rand() -> bool {
    #[cfg(target_os = "windows")]
    {
        // On Windows, use IsProcessorFeaturePresent
        use core::ffi::c_int;
        const PF_ARM_V81_ATOMIC_INSTRUCTIONS_AVAILABLE: c_int = 33;
        unsafe extern "C" {
            fn IsProcessorFeaturePresent(feature: c_int) -> i32;
        }
        // RNDR requires at least ARMv8.1, so check for atomic instructions
        // as a minimum.
        // FIXME: May falsely report available on rare hardware. Ideally Windows would have
        // a check specifically for RNDR.
        unsafe { IsProcessorFeaturePresent(PF_ARM_V81_ATOMIC_INSTRUCTIONS_AVAILABLE) != 0 }
    }
    #[cfg(any(target_os = "macos"))]
    {
        let mut value: u32 = 0;
        let mut size = core::mem::size_of::<u32>();
        let name = b"hw.optional.arm.FEAT_RNG\0";
        unsafe extern "C" {
            fn sysctlbyname(
                name: *const u8,
                oldp: *mut u32,
                oldlenp: *mut usize,
                newp: *const core::ffi::c_void,
                newlen: usize,
            ) -> core::ffi::c_int;
        }
        unsafe {
            sysctlbyname(name.as_ptr(), &mut value, &mut size, core::ptr::null(), 0) == 0
                && value != 0
        }
    }
    #[cfg(any(
        target_os = "linux",
        target_os = "android",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "none",
        target_os = "uefi"))]
    {
        let value: u64;
        unsafe {
            // MRS is a privileged instruction (EL1),
            // but it's emulated on Linux, FreeBSD and NetBSD.
            core::arch::asm!(
                "mrs {0}, ID_AA64ISAR0_EL1", // feature register
                out(reg) value,
                options(nostack)
            );
        }
        (value & 0xF000_0000_0000_0000) != 0
    }
    #[cfg(not(any(
        target_os = "linux",
        target_os = "android",
        target_os = "windows",
        target_os = "macos",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "uefi",
        target_os = "none"
    )))]
    {
        #[cfg(feature = "std")]
        {
            extern crate std;
            std::arch::is_aarch64_feature_detected!("rand")
        }

        #[cfg(not(feature = "std"))]
        {
            #[cfg(target_os = "openbsd")]
            {
                // FIXME: Not tested.
                const CTL_MACHDEP: c_int = 7;
                const CPU_ID_AA64ISAR0: c_int = 2;

                let mib = [CTL_MACHDEP, CPU_ID_AA64ISAR0];
                let mut isar0: u64 = 0;
                let mut len = core::mem::size_of_val(&isar0);

                let result = unsafe {
                    libc::sysctl(
                        mib.as_ptr(),
                        mib.len() as u32,
                        &mut isar0 as *mut _ as *mut c_void,
                        &mut len,
                        core::ptr::null_mut(),
                        0,
                    )
                };

                if result == 0 {
                    // Extract the RND field (bits 60-63)
                    ((isar0 >> 60) & 0xF) >= 1
                } else {
                    false
                }
            }
            #[cfg(not(target_os = "openbsd"))] {
                // When we can't detect the feature, assume it's unavailable unless compiling with
                // `-Ctarget-feature=+rand` (in which case `has_rand` is bypassed altogether).
                // FIXME: Detection on iOS should be possible, but no known method is future-proof.
                false
            }
        }
    }
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[allow(unused_unsafe)]
#[inline(always)]
fn has_rdseed() -> bool {
    const FLAG: u32 = 1 << 18;
    (unsafe { arch::__cpuid(7) }.ebx & FLAG) == FLAG
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
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
const FIRST_GOOD_AMD_FAMILY: u32 = 0x17;

macro_rules! is_available {
    ("rdrand") => {{
        #[allow(unused_unsafe)]
        if authentic_amd() {
            let cpuid1 = unsafe { arch::__cpuid(1) };
            has_rdrand(&cpuid1) && amd_family(&cpuid1) >= FIRST_GOOD_AMD_FAMILY
        } else {
            cfg!(target_feature = "rdrand") || has_rdrand(&unsafe { arch::__cpuid(1) })
        }
    }};
    ("rand") => {{
        #[cfg(target_arch = "aarch64")]
        {
            cfg!(target_feature = "rand") || has_rand()
        }
        #[cfg(not(target_arch = "aarch64"))]
        {
            unreachable!()
        }
    }};
    ("rdseed") => {{
        #[allow(unused_unsafe)]
        if authentic_amd() {
            amd_family(&unsafe { arch::__cpuid(1) }) >= FIRST_GOOD_AMD_FAMILY && has_rdseed()
        } else {
            cfg!(target_feature = "rdrand") || has_rdseed()
        }
    }};
}

macro_rules! impl_rand {
    ($gen:ident, $feat:tt, $loop_mode:tt, $step32:path, $step64:path,
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
        }
        impl TryRng for $gen {
            type Error = ErrorCode;
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
            #[allow(unused_unsafe)]
            fn try_next_u32(&mut self) -> Result<u32, ErrorCode> {
                #[target_feature(enable = $feat)]
                #[allow(unused_unsafe)]
                unsafe fn imp() -> Result<u32, ErrorCode> {
                    loop_rand!($loop_mode, u32, $step32)
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
            fn try_next_u64(&mut self) -> Result<u64, ErrorCode> {
                #[target_feature(enable = $feat)]
                #[allow(unused_unsafe)]
                unsafe fn imp() -> Result<u64, ErrorCode> {
                    loop_rand!($loop_mode, u64, $step64)
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
            fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), ErrorCode> {
                #[target_feature(enable = $feat)]
                #[allow(unused_unsafe)]
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
                            #[allow(unused_unsafe)]
                            if buffer.is_empty() {
                                word = unsafe { loop_rand!($loop_mode, $maxty, $maxstep) }?
                                    .to_ne_bytes();
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
                        let (left, mid, right) = unsafe { dest.align_to_mut() };
                        for el in mid {
                            *el = loop_rand!($loop_mode, $maxty, $maxstep)?;
                        }

                        slow_fill_bytes(left, right)
                    } else {
                        slow_fill_bytes(dest, &mut [])
                    }
                }
                unsafe { imp(dest) }
            }
        }
    };
}

#[cfg(target_arch = "x86_64")]
impl_rand!(
    RdRand,
    "rdrand",
    "rdrand",
    arch::_rdrand32_step,
    arch::_rdrand64_step,
    maxstep = arch::_rdrand64_step,
    maxty = u64
);
#[cfg(target_arch = "x86_64")]
impl_rand!(
    RdSeed,
    "rdseed",
    "rdseed",
    arch::_rdseed32_step,
    arch::_rdseed64_step,
    maxstep = arch::_rdseed64_step,
    maxty = u64
);
#[cfg(target_arch = "x86")]
impl_rand!(
    RdRand,
    "rdrand",
    "rdrand",
    arch::_rdrand32_step,
    arch::_rdrand64_step,
    maxstep = arch::_rdrand32_step,
    maxty = u32
);
#[cfg(target_arch = "x86")]
impl_rand!(
    RdSeed,
    "rdseed",
    "rdseed",
    arch::_rdseed32_step,
    arch::_rdseed64_step,
    maxstep = arch::_rdseed32_step,
    maxty = u32
);
#[cfg(target_arch = "aarch64")]
impl_rand!(
    RdRand,
    "rand",
    "rdrand",
    arch::rand32,
    arch::rand,
    maxstep = arch::rand,
    maxty = u64
);
#[cfg(target_arch = "aarch64")]
impl_rand!(
    RdSeed,
    "rand",
    "rdseed",
    arch::seed32,
    arch::seed,
    maxstep = arch::seed,
    maxty = u64
);

#[cfg(not(any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")))]
impl RdRand {
    fn new() -> Result<Self, ErrorCode> {
        Err(ErrorCode::UnsupportedInstruction)
    }
}

#[cfg(not(any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")))]
impl TryRng for RdRand {
    type Error = ErrorCode;
    fn try_next_u32(&mut self) -> Result<u32, ErrorCode> {
        Err(ErrorCode::UnsupportedInstruction)
    }
    fn try_next_u64(&mut self) -> Result<u64, ErrorCode> {
        Err(ErrorCode::UnsupportedInstruction)
    }
    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), ErrorCode> {
        Err(ErrorCode::UnsupportedInstruction)
    }
}

#[cfg(not(any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")))]
impl RdSeed {
    fn new() -> Result<Self, ErrorCode> {
        Err(ErrorCode::UnsupportedInstruction)
    }
}

#[cfg(not(any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")))]
impl TryRng for RdSeed {
    type Error = ErrorCode;
    fn try_next_u32(&mut self) -> Result<u32, ErrorCode> {
        Err(ErrorCode::UnsupportedInstruction)
    }
    fn try_next_u64(&mut self) -> Result<u64, ErrorCode> {
        Err(ErrorCode::UnsupportedInstruction)
    }
    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), ErrorCode> {
        Err(ErrorCode::UnsupportedInstruction)
    }
}

#[cfg(test)]
mod test {
    use super::{RdRand, RdSeed};
    use rand_core::{Rng, TryRng, UnwrapErr};

    #[test]
    fn rdrand_works() {
        extern crate std;
        use std::eprintln;
        eprintln!("Checking RdRand::new()...");
        match RdRand::new() {
            Ok(mut r) => {
                eprintln!("RdRand created successfully, calling try_next_u32()");
                match r.try_next_u32() {
                    Ok(val) => eprintln!("Got random value: {}", val),
                    Err(e) => eprintln!("try_next_u32 failed: {:?}", e),
                }
            }
            Err(e) => {
                eprintln!("RdRand::new() failed with: {:?}", e);
                eprintln!("This is expected on CPUs without RDRAND support");
            }
        }
    }

    #[repr(C, align(8))]
    struct FillBuffer([u8; 64]);

    #[test]
    fn fill_fills_all_bytes() {
        let _status = RdRand::new().map(|r| {
            let mut r = UnwrapErr(r);
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
                (0, 8),  // 1 word-worth of data, aligned.
                (1, 9),  // 1 word-worth of data, misaligned.
                (0, 7),  // less than 1 word of data.
                (1, 7),  // less than 1 word of data.
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
        #[cfg(any(
            all(target_feature = "rand", target_arch = "aarch64"),
            target_arch = "x86_64",
            target_arch = "x86"
        ))]
        _status.unwrap();
    }

    #[test]
    fn rdseed_works() {
        let _status = RdSeed::new().map(|mut r| {
            r.try_next_u32().unwrap();
            r.try_next_u64().unwrap();
        });
        #[cfg(any(
            all(target_feature = "rand", target_arch = "aarch64"),
            target_arch = "x86_64",
            target_arch = "x86"
        ))]
        _status.unwrap();
    }
}
