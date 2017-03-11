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
//! Utility functions to detect presence of `rdrand` and `rdseed` instruction support.

#![allow(dead_code)]

pub use self::imp::*;

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
mod imp {
    #[inline]
    fn cpuid(mut a: u32, mut c: u32) -> (u32, u32, u32, u32) {
        let (b, d);
        unsafe {
            asm!("cpuid"
                :"+{eax}"(a), "={ebx}"(b), "+{ecx}"(c), "={edx}"(d)
                );
        }
        (a, b, c, d)
    }

    #[inline]
    pub fn is_intel() -> bool {
        let (_, b, c, d) = cpuid(0, 0);
        // GenuineIntel
        return b == 0x756E6547 && d == 0x49656e69 && c == 0x6C65746E;
    }

    #[inline]
    pub fn has_rdrand() -> bool {
        const FLAG : u32 = 1 << 30;
        let (_, _, c, _) = cpuid(1, 0);
        return c & FLAG == FLAG;
    }

    #[inline]
    pub fn has_rdseed() -> bool {
        const FLAG : u32 = 1 << 18;
        let (_, b, _, _) = cpuid(7, 0);
        return b & FLAG == FLAG;
    }
}

#[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
mod imp {
    #[inline]
    pub fn is_intel() -> bool {
        false
    }

    #[inline]
    pub fn has_rdrand() -> bool {
        false
    }

    #[inline]
    pub fn has_rdseed() -> bool {
        false
    }
}
