/*!
This library provides heavily optimized routines for string search primitives.
# Overview
This section gives a brief high level overview of what this crate offers.
* The top-level module provides routines for searching for 1, 2 or 3 bytes
  in the forward or reverse direction. When searching for more than one byte,
  positions are considered a match if the byte at that position matches any
  of the bytes.
* The [`memmem`] sub-module provides forward and reverse substring search
  routines.
In all such cases, routines operate on `&[u8]` without regard to encoding. This
is exactly what you want when searching either UTF-8 or arbitrary bytes.
# Example: using `memchr`
This example shows how to use `memchr` to find the first occurrence of `z` in
a haystack:
```
use memchr::memchr;
let haystack = b"foo bar baz quuz";
assert_eq!(Some(10), memchr(b'z', haystack));
```
# Example: matching one of three possible bytes
This examples shows how to use `memrchr3` to find occurrences of `a`, `b` or
`c`, starting at the end of the haystack.
```
use memchr::memchr3_iter;
let haystack = b"xyzaxyzbxyzc";
let mut it = memchr3_iter(b'a', b'b', b'c', haystack).rev();
assert_eq!(Some(11), it.next());
assert_eq!(Some(7), it.next());
assert_eq!(Some(3), it.next());
assert_eq!(None, it.next());
```
# Example: iterating over substring matches
This example shows how to use the [`memmem`] sub-module to find occurrences of
a substring in a haystack.
```
use memchr::memmem;
let haystack = b"foo bar foo baz foo";
let mut it = memmem::find_iter(haystack, "foo");
assert_eq!(Some(0), it.next());
assert_eq!(Some(8), it.next());
assert_eq!(Some(16), it.next());
assert_eq!(None, it.next());
```
# Example: repeating a search for the same needle
It may be possible for the overhead of constructing a substring searcher to be
measurable in some workloads. In cases where the same needle is used to search
many haystacks, it is possible to do construction once and thus to avoid it for
subsequent searches. This can be done with a [`memmem::Finder`]:
```
use memchr::memmem;
let finder = memmem::Finder::new("foo");
assert_eq!(Some(4), finder.find(b"baz foo quux"));
assert_eq!(None, finder.find(b"quux baz bar"));
```
# Why use this crate?
At first glance, the APIs provided by this crate might seem weird. Why provide
a dedicated routine like `memchr` for something that could be implemented
clearly and trivially in one line:
```
fn memchr(needle: u8, haystack: &[u8]) -> Option<usize> {
    haystack.iter().position(|&b| b == needle)
}
```
Or similarly, why does this crate provide substring search routines when Rust's
core library already provides them?
```
fn search(haystack: &str, needle: &str) -> Option<usize> {
    haystack.find(needle)
}
```
The primary reason for both of them to exist is performance. When it comes to
performance, at a high level at least, there are two primary ways to look at
it:
* **Throughput**: For this, think about it as, "given some very large haystack
  and a byte that never occurs in that haystack, how long does it take to
  search through it and determine that it, in fact, does not occur?"
* **Latency**: For this, think about it as, "given a tiny haystack---just a
  few bytes---how long does it take to determine if a byte is in it?"
The `memchr` routine in this crate has _slightly_ worse latency than the
solution presented above, however, its throughput can easily be over an
order of magnitude faster. This is a good general purpose trade off to make.
You rarely lose, but often gain big.
**NOTE:** The name `memchr` comes from the corresponding routine in libc. A key
advantage of using this library is that its performance is not tied to its
quality of implementation in the libc you happen to be using, which can vary
greatly from platform to platform.
But what about substring search? This one is a bit more complicated. The
primary reason for its existence is still indeed performance, but it's also
useful because Rust's core library doesn't actually expose any substring
search routine on arbitrary bytes. The only substring search routine that
exists works exclusively on valid UTF-8.
So if you have valid UTF-8, is there a reason to use this over the standard
library substring search routine? Yes. This routine is faster on almost every
metric, including latency. The natural question then, is why isn't this
implementation in the standard library, even if only for searching on UTF-8?
The reason is that the implementation details for using SIMD in the standard
library haven't quite been worked out yet.
**NOTE:** Currently, only `x86_64` targets have highly accelerated
implementations of substring search. For `memchr`, all targets have
somewhat-accelerated implementations, while only `x86_64` targets have highly
accelerated implementations. This limitation is expected to be lifted once the
standard library exposes a platform independent SIMD API.
# Crate features
* **std** - When enabled (the default), this will permit this crate to use
  features specific to the standard library. Currently, the only thing used
  from the standard library is runtime SIMD CPU feature detection. This means
  that this feature must be enabled to get AVX accelerated routines. When
  `std` is not enabled, this crate will still attempt to use SSE2 accelerated
  routines on `x86_64`.
* **libc** - When enabled (**not** the default), this library will use your
  platform's libc implementation of `memchr` (and `memrchr` on Linux). This
  can be useful on non-`x86_64` targets where the fallback implementation in
  this crate is not as good as the one found in your libc. All other routines
  (e.g., `memchr[23]` and substring search) unconditionally use the
  implementation in this crate.
*/

#![deny(missing_docs)]
#![cfg_attr(not(feature = "std"), no_std)]
// It's not worth trying to gate all code on just miri, so turn off relevant
// dead code warnings.
#![cfg_attr(miri, allow(dead_code, unused_macros))]

// Supporting 8-bit (or others) would be fine. If you need it, please submit a
// bug report at https://github.com/BurntSushi/rust-memchr
#[cfg(not(any(
    target_pointer_width = "16",
    target_pointer_width = "32",
    target_pointer_width = "64"
)))]
compile_error!("memchr currently not supported on non-{16,32,64}");

pub use crate::memchr::{
    memchr, memchr2, memchr2_iter, memchr3, memchr3_iter, memchr_iter,
    memrchr, memrchr2, memrchr2_iter, memrchr3, memrchr3_iter, memrchr_iter,
    Memchr, Memchr2, Memchr3,
};

<<<<<<< HEAD
mod cow;
mod memchr;
pub mod memmem;
#[cfg(test)]
mod tests;
=======
#[cfg(test)]
#[macro_use]
extern crate quickcheck;

use core::iter::Rev;

pub use iter::{Memchr, Memchr2, Memchr3};

// N.B. If you're looking for the cfg knobs for libc, see build.rs.
#[cfg(memchr_libc)]
mod c;
#[allow(dead_code)]
mod fallback;
mod iter;
mod naive;
#[cfg(all(
    not(all(target_arch = "x86_64", memchr_runtime_simd)),
    feature = "nightly"
))]
mod simd;
#[cfg(test)]
mod tests;
#[cfg(all(target_arch = "x86_64", memchr_runtime_simd))]
mod x86;

/// An iterator over all occurrences of the needle in a haystack.
#[inline]
pub fn memchr_iter(needle: u8, haystack: &[u8]) -> Memchr {
    Memchr::new(needle, haystack)
}

/// An iterator over all occurrences of the needles in a haystack.
#[inline]
pub fn memchr2_iter(needle1: u8, needle2: u8, haystack: &[u8]) -> Memchr2 {
    Memchr2::new(needle1, needle2, haystack)
}

/// An iterator over all occurrences of the needles in a haystack.
#[inline]
pub fn memchr3_iter(
    needle1: u8,
    needle2: u8,
    needle3: u8,
    haystack: &[u8],
) -> Memchr3 {
    Memchr3::new(needle1, needle2, needle3, haystack)
}

/// An iterator over all occurrences of the needle in a haystack, in reverse.
#[inline]
pub fn memrchr_iter(needle: u8, haystack: &[u8]) -> Rev<Memchr> {
    Memchr::new(needle, haystack).rev()
}

/// An iterator over all occurrences of the needles in a haystack, in reverse.
#[inline]
pub fn memrchr2_iter(
    needle1: u8,
    needle2: u8,
    haystack: &[u8],
) -> Rev<Memchr2> {
    Memchr2::new(needle1, needle2, haystack).rev()
}

/// An iterator over all occurrences of the needles in a haystack, in reverse.
#[inline]
pub fn memrchr3_iter(
    needle1: u8,
    needle2: u8,
    needle3: u8,
    haystack: &[u8],
) -> Rev<Memchr3> {
    Memchr3::new(needle1, needle2, needle3, haystack).rev()
}

/// Search for the first occurrence of a byte in a slice.
///
/// This returns the index corresponding to the first occurrence of `needle` in
/// `haystack`, or `None` if one is not found.
///
/// While this is operationally the same as something like
/// `haystack.iter().position(|&b| b == needle)`, `memchr` will use a highly
/// optimized routine that can be up to an order of magnitude faster in some
/// cases.
///
/// # Example
///
/// This shows how to find the first position of a byte in a byte string.
///
/// ```
/// use memchr::memchr;
///
/// let haystack = b"the quick brown fox";
/// assert_eq!(memchr(b'k', haystack), Some(8));
/// ```
#[inline]
pub fn memchr(needle: u8, haystack: &[u8]) -> Option<usize> {
    #[cfg(all(target_arch = "x86_64", memchr_runtime_simd))]
    #[inline(always)]
    fn imp(n1: u8, haystack: &[u8]) -> Option<usize> {
        x86::memchr(n1, haystack)
    }

    #[cfg(all(
        not(all(target_arch = "x86_64", memchr_runtime_simd)),
        feature = "nightly"
    ))]
    #[inline(always)]
    fn imp(n1: u8, haystack: &[u8]) -> Option<usize> {
        simd::memchr(n1, haystack)
    }

    #[cfg(all(
        memchr_libc,
        not(all(target_arch = "x86_64", memchr_runtime_simd))
    ))]
    #[inline(always)]
    fn imp(n1: u8, haystack: &[u8]) -> Option<usize> {
        c::memchr(n1, haystack)
    }

    #[cfg(all(
        not(memchr_libc),
        not(all(target_arch = "x86_64", memchr_runtime_simd)),
        not(feature = "nightly")
    ))]
    #[inline(always)]
    fn imp(n1: u8, haystack: &[u8]) -> Option<usize> {
        fallback::memchr(n1, haystack)
    }

    if haystack.is_empty() {
        None
    } else {
        imp(needle, haystack)
    }
}

/// Like `memchr`, but searches for either of two bytes instead of just one.
///
/// This returns the index corresponding to the first occurrence of `needle1`
/// or the first occurrence of `needle2` in `haystack` (whichever occurs
/// earlier), or `None` if neither one is found.
///
/// While this is operationally the same as something like
/// `haystack.iter().position(|&b| b == needle1 || b == needle2)`, `memchr2`
/// will use a highly optimized routine that can be up to an order of magnitude
/// faster in some cases.
///
/// # Example
///
/// This shows how to find the first position of either of two bytes in a byte
/// string.
///
/// ```
/// use memchr::memchr2;
///
/// let haystack = b"the quick brown fox";
/// assert_eq!(memchr2(b'k', b'q', haystack), Some(4));
/// ```
#[inline]
pub fn memchr2(needle1: u8, needle2: u8, haystack: &[u8]) -> Option<usize> {
    #[cfg(all(target_arch = "x86_64", memchr_runtime_simd))]
    #[inline(always)]
    fn imp(n1: u8, n2: u8, haystack: &[u8]) -> Option<usize> {
        x86::memchr2(n1, n2, haystack)
    }

    #[cfg(all(
        not(all(target_arch = "x86_64", memchr_runtime_simd)),
        feature = "nightly"
    ))]
    #[inline(always)]
    fn imp(n1: u8, n2: u8, haystack: &[u8]) -> Option<usize> {
        simd::memchr2(n1, n2, haystack)
    }

    #[cfg(all(
        not(all(target_arch = "x86_64", memchr_runtime_simd)),
        not(feature = "nightly")
    ))]
    #[inline(always)]
    fn imp(n1: u8, n2: u8, haystack: &[u8]) -> Option<usize> {
        fallback::memchr2(n1, n2, haystack)
    }

    if haystack.is_empty() {
        None
    } else {
        imp(needle1, needle2, haystack)
    }
}

/// Like `memchr`, but searches for any of three bytes instead of just one.
///
/// This returns the index corresponding to the first occurrence of `needle1`,
/// the first occurrence of `needle2`, or the first occurence of `needle3` in
/// `haystack` (whichever occurs earliest), or `None` if none are found.
///
/// While this is operationally the same as something like
/// `haystack.iter().position(|&b| b == needle1 || b == needle2 ||
/// b == needle3)`, `memchr3` will use a highly optimized routine that can be
/// up to an order of magnitude faster in some cases.
///
/// # Example
///
/// This shows how to find the first position of any of three bytes in a byte
/// string.
///
/// ```
/// use memchr::memchr3;
///
/// let haystack = b"the quick brown fox";
/// assert_eq!(memchr3(b'k', b'q', b'e', haystack), Some(2));
/// ```
#[inline]
pub fn memchr3(
    needle1: u8,
    needle2: u8,
    needle3: u8,
    haystack: &[u8],
) -> Option<usize> {
    #[cfg(all(target_arch = "x86_64", memchr_runtime_simd))]
    #[inline(always)]
    fn imp(n1: u8, n2: u8, n3: u8, haystack: &[u8]) -> Option<usize> {
        x86::memchr3(n1, n2, n3, haystack)
    }

    #[cfg(all(
        not(all(target_arch = "x86_64", memchr_runtime_simd)),
        feature = "nightly"
    ))]
    #[inline(always)]
    fn imp(n1: u8, n2: u8, n3: u8, haystack: &[u8]) -> Option<usize> {
        simd::memchr3(n1, n2, n3, haystack)
    }

    #[cfg(all(
        not(all(target_arch = "x86_64", memchr_runtime_simd)),
        not(feature = "nightly")
    ))]
    #[inline(always)]
    fn imp(n1: u8, n2: u8, n3: u8, haystack: &[u8]) -> Option<usize> {
        fallback::memchr3(n1, n2, n3, haystack)
    }

    if haystack.is_empty() {
        None
    } else {
        imp(needle1, needle2, needle3, haystack)
    }
}

/// Search for the last occurrence of a byte in a slice.
///
/// This returns the index corresponding to the last occurrence of `needle` in
/// `haystack`, or `None` if one is not found.
///
/// While this is operationally the same as something like
/// `haystack.iter().rposition(|&b| b == needle)`, `memrchr` will use a highly
/// optimized routine that can be up to an order of magnitude faster in some
/// cases.
///
/// # Example
///
/// This shows how to find the last position of a byte in a byte string.
///
/// ```
/// use memchr::memrchr;
///
/// let haystack = b"the quick brown fox";
/// assert_eq!(memrchr(b'o', haystack), Some(17));
/// ```
#[inline]
pub fn memrchr(needle: u8, haystack: &[u8]) -> Option<usize> {
    #[cfg(all(target_arch = "x86_64", memchr_runtime_simd))]
    #[inline(always)]
    fn imp(n1: u8, haystack: &[u8]) -> Option<usize> {
        x86::memrchr(n1, haystack)
    }

    #[cfg(all(
        not(all(target_arch = "x86_64", memchr_runtime_simd)),
        feature = "nightly"
    ))]
    #[inline(always)]
    fn imp(n1: u8, haystack: &[u8]) -> Option<usize> {
        simd::memrchr(n1, haystack)
    }

    #[cfg(all(
        all(memchr_libc, target_os = "linux"),
        not(all(target_arch = "x86_64", memchr_runtime_simd))
    ))]
    #[inline(always)]
    fn imp(n1: u8, haystack: &[u8]) -> Option<usize> {
        c::memrchr(n1, haystack)
    }

    #[cfg(all(
        not(all(memchr_libc, target_os = "linux")),
        not(all(target_arch = "x86_64", memchr_runtime_simd)),
        not(feature = "nightly")
    ))]
    #[inline(always)]
    fn imp(n1: u8, haystack: &[u8]) -> Option<usize> {
        fallback::memrchr(n1, haystack)
    }

    if haystack.is_empty() {
        None
    } else {
        imp(needle, haystack)
    }
}

/// Like `memrchr`, but searches for either of two bytes instead of just one.
///
/// This returns the index corresponding to the last occurrence of `needle1`
/// or the last occurrence of `needle2` in `haystack` (whichever occurs later),
/// or `None` if neither one is found.
///
/// While this is operationally the same as something like
/// `haystack.iter().rposition(|&b| b == needle1 || b == needle2)`, `memrchr2`
/// will use a highly optimized routine that can be up to an order of magnitude
/// faster in some cases.
///
/// # Example
///
/// This shows how to find the last position of either of two bytes in a byte
/// string.
///
/// ```
/// use memchr::memrchr2;
///
/// let haystack = b"the quick brown fox";
/// assert_eq!(memrchr2(b'k', b'q', haystack), Some(8));
/// ```
#[inline]
pub fn memrchr2(needle1: u8, needle2: u8, haystack: &[u8]) -> Option<usize> {
    #[cfg(all(target_arch = "x86_64", memchr_runtime_simd))]
    #[inline(always)]
    fn imp(n1: u8, n2: u8, haystack: &[u8]) -> Option<usize> {
        x86::memrchr2(n1, n2, haystack)
    }

    #[cfg(all(
        not(all(target_arch = "x86_64", memchr_runtime_simd)),
        feature = "nightly"
    ))]
    #[inline(always)]
    fn imp(n1: u8, n2: u8, haystack: &[u8]) -> Option<usize> {
        simd::memrchr2(n1, n2, haystack)
    }

    #[cfg(all(
        not(all(target_arch = "x86_64", memchr_runtime_simd)),
        not(feature = "nightly")
    ))]
    #[inline(always)]
    fn imp(n1: u8, n2: u8, haystack: &[u8]) -> Option<usize> {
        fallback::memrchr2(n1, n2, haystack)
    }

    if haystack.is_empty() {
        None
    } else {
        imp(needle1, needle2, haystack)
    }
}

/// Like `memrchr`, but searches for any of three bytes instead of just one.
///
/// This returns the index corresponding to the last occurrence of `needle1`,
/// the last occurrence of `needle2`, or the last occurence of `needle3` in
/// `haystack` (whichever occurs later), or `None` if none are found.
///
/// While this is operationally the same as something like
/// `haystack.iter().rposition(|&b| b == needle1 || b == needle2 ||
/// b == needle3)`, `memrchr3` will use a highly optimized routine that can be
/// up to an order of magnitude faster in some cases.
///
/// # Example
///
/// This shows how to find the last position of any of three bytes in a byte
/// string.
///
/// ```
/// use memchr::memrchr3;
///
/// let haystack = b"the quick brown fox";
/// assert_eq!(memrchr3(b'k', b'q', b'e', haystack), Some(8));
/// ```
#[inline]
pub fn memrchr3(
    needle1: u8,
    needle2: u8,
    needle3: u8,
    haystack: &[u8],
) -> Option<usize> {
    #[cfg(all(target_arch = "x86_64", memchr_runtime_simd))]
    #[inline(always)]
    fn imp(n1: u8, n2: u8, n3: u8, haystack: &[u8]) -> Option<usize> {
        x86::memrchr3(n1, n2, n3, haystack)
    }

    #[cfg(all(
        not(all(target_arch = "x86_64", memchr_runtime_simd)),
        feature = "nightly"
    ))]
    #[inline(always)]
    fn imp(n1: u8, n2: u8, n3: u8, haystack: &[u8]) -> Option<usize> {
        simd::memrchr3(n1, n2, n3, haystack)
    }

    #[cfg(all(
        not(all(target_arch = "x86_64", memchr_runtime_simd)),
        not(feature = "nightly")
    ))]
    #[inline(always)]
    fn imp(n1: u8, n2: u8, n3: u8, haystack: &[u8]) -> Option<usize> {
        fallback::memrchr3(n1, n2, n3, haystack)
    }

    if haystack.is_empty() {
        None
    } else {
        imp(needle1, needle2, needle3, haystack)
    }
}
>>>>>>> Accidentally deleted naive path.
