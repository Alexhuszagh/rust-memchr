mod private {

use core::cmp;
use core::mem::size_of;
use core::ops::BitOr;
use core::slice;
use packed_simd_2::*;

const VECTOR_SIZE: usize = size_of::<i8x16>();
const VECTOR_ALIGN: usize = VECTOR_SIZE - 1;

// The number of bytes to loop at in one iteration of memchr/memrchr.
const LOOP_SIZE: usize = 4 * VECTOR_SIZE;

// The number of bytes to loop at in one iteration of memchr2/memrchr2 and
// memchr3/memrchr3. There was no observable difference between 64 and 32 bytes
// in benchmarks. memchr3 in particular only gets a very slight speed up from
// the loop unrolling.
const LOOP_SIZE2: usize = 2 * VECTOR_SIZE;


pub unsafe fn memchr(n1: u8, haystack: &[u8]) -> Option<usize> {
    let vn1 = i8x16::splat(n1 as i8);
    let len = haystack.len();
    let loop_size = cmp::min(LOOP_SIZE, len);
    let start_ptr = haystack.as_ptr();
    let end_ptr = haystack[haystack.len()..].as_ptr();
    let mut ptr = start_ptr;

    if haystack.len() < VECTOR_SIZE {
        while ptr < end_ptr {
            if *ptr == n1 {
                return Some(sub(ptr, start_ptr));
            }
            ptr = ptr.offset(1);
        }
        return None;
    }

    if let Some(i) = forward_search1(start_ptr, end_ptr, ptr, vn1) {
        return Some(i);
    }

    ptr = ptr.add(VECTOR_SIZE - (start_ptr as usize & VECTOR_ALIGN));
    debug_assert!(ptr > start_ptr && end_ptr.sub(VECTOR_SIZE) >= start_ptr);
    while loop_size == LOOP_SIZE && ptr <= end_ptr.sub(loop_size) {
        debug_assert_eq!(0, (ptr as usize) % VECTOR_SIZE);

        let a = i8x16_load(ptr);
        let b = i8x16_load(ptr.add(VECTOR_SIZE));
        let c = i8x16_load(ptr.add(2 * VECTOR_SIZE));
        let d = i8x16_load(ptr.add(3 * VECTOR_SIZE));
        let eqa = vn1.eq(a);
        let eqb = vn1.eq(b);
        let eqc = vn1.eq(c);
        let eqd = vn1.eq(d);
        let or1 = eqa.bitor(eqb);
        let or2 = eqc.bitor(eqd);
        let or3 = or1.bitor(or2);
        if or3.any() {
            let mut at = sub(ptr, start_ptr);
            if eqa.any() {
                return Some(at + forward_pos(eqa.bitmask()))
            }

            at += VECTOR_SIZE;
            if eqb.any() {
                return Some(at + forward_pos(eqb.bitmask()))
            }

            at += VECTOR_SIZE;
            if eqc.any() {
                return Some(at + forward_pos(eqc.bitmask()))
            }

            at += VECTOR_SIZE;
            debug_assert!(eqd.bitmask() != 0);
            return Some(at + forward_pos(eqd.bitmask()));
        }
        ptr = ptr.add(loop_size);
    }
    while ptr <= end_ptr.sub(VECTOR_SIZE) {
        debug_assert!(sub(end_ptr, ptr) >= VECTOR_SIZE);

        if let Some(i) = forward_search1(start_ptr, end_ptr, ptr, vn1) {
            return Some(i);
        }
        ptr = ptr.add(VECTOR_SIZE);
    }
    if ptr < end_ptr {
        debug_assert!(sub(end_ptr, ptr) < VECTOR_SIZE);
        ptr = ptr.sub(VECTOR_SIZE - sub(end_ptr, ptr));
        debug_assert_eq!(sub(end_ptr, ptr), VECTOR_SIZE);

        return forward_search1(start_ptr, end_ptr, ptr, vn1);
    }
    None
}

pub unsafe fn memchr2(n1: u8, n2: u8, haystack: &[u8]) -> Option<usize> {
    let vn1 = i8x16::splat(n1 as i8);
    let vn2 = i8x16::splat(n2 as i8);
    let len = haystack.len();
    let loop_size = cmp::min(LOOP_SIZE2, len);
    let start_ptr = haystack.as_ptr();
    let end_ptr = haystack[haystack.len()..].as_ptr();
    let mut ptr = start_ptr;

    if haystack.len() < VECTOR_SIZE {
        while ptr < end_ptr {
            if *ptr == n1 || *ptr == n2 {
                return Some(sub(ptr, start_ptr));
            }
            ptr = ptr.offset(1);
        }
        return None;
    }

    if let Some(i) = forward_search2(start_ptr, end_ptr, ptr, vn1, vn2) {
        return Some(i);
    }

    ptr = ptr.add(VECTOR_SIZE - (start_ptr as usize & VECTOR_ALIGN));
    debug_assert!(ptr > start_ptr && end_ptr.sub(VECTOR_SIZE) >= start_ptr);
    while loop_size == LOOP_SIZE2 && ptr <= end_ptr.sub(loop_size) {
        debug_assert_eq!(0, (ptr as usize) % VECTOR_SIZE);

        let a = i8x16_load(ptr);
        let b = i8x16_load(ptr.add(VECTOR_SIZE));
        let eqa1 = vn1.eq(a);
        let eqb1 = vn1.eq(b);
        let eqa2 = vn2.eq(a);
        let eqb2 = vn2.eq(b);
        let or1 = eqa1.bitor(eqb1);
        let or2 = eqa2.bitor(eqb2);
        let or3 = or1.bitor(or2);
        if or3.any() {
            let mut at = sub(ptr, start_ptr);
            if eqa1.any() || eqa2.any() {
                return Some(at + forward_pos2(eqa1.bitmask(), eqa2.bitmask()))
            }

            at += VECTOR_SIZE;
            if eqb1.any() || eqb2.any() {
                return Some(at + forward_pos2(eqb1.bitmask(), eqb2.bitmask()))
            }
        }
        ptr = ptr.add(loop_size);
    }
    while ptr <= end_ptr.sub(VECTOR_SIZE) {
        if let Some(i) = forward_search2(start_ptr, end_ptr, ptr, vn1, vn2) {
            return Some(i);
        }
        ptr = ptr.add(VECTOR_SIZE);
    }
    if ptr < end_ptr {
        debug_assert!(sub(end_ptr, ptr) < VECTOR_SIZE);
        ptr = ptr.sub(VECTOR_SIZE - sub(end_ptr, ptr));
        debug_assert_eq!(sub(end_ptr, ptr), VECTOR_SIZE);

        return forward_search2(start_ptr, end_ptr, ptr, vn1, vn2);
    }
    None
}

pub unsafe fn memchr3(n1: u8, n2: u8, n3: u8, haystack: &[u8]) -> Option<usize> {
    let vn1 = i8x16::splat(n1 as i8);
    let vn2 = i8x16::splat(n2 as i8);
    let vn3 = i8x16::splat(n3 as i8);
    let len = haystack.len();
    let loop_size = cmp::min(LOOP_SIZE2, len);
    let start_ptr = haystack.as_ptr();
    let end_ptr = haystack[haystack.len()..].as_ptr();
    let mut ptr = start_ptr;

    if haystack.len() < VECTOR_SIZE {
        while ptr < end_ptr {
            if *ptr == n1 || *ptr == n2 || *ptr == n3 {
                return Some(sub(ptr, start_ptr));
            }
            ptr = ptr.offset(1);
        }
        return None;
    }

    if let Some(i) = forward_search3(start_ptr, end_ptr, ptr, vn1, vn2, vn3) {
        return Some(i);
    }

    ptr = ptr.add(VECTOR_SIZE - (start_ptr as usize & VECTOR_ALIGN));
    debug_assert!(ptr > start_ptr && end_ptr.sub(VECTOR_SIZE) >= start_ptr);
    while loop_size == LOOP_SIZE2 && ptr <= end_ptr.sub(loop_size) {
        debug_assert_eq!(0, (ptr as usize) % VECTOR_SIZE);

        let a = i8x16_load(ptr);
        let b = i8x16_load(ptr.add(VECTOR_SIZE));
        let eqa1 = vn1.eq(a);
        let eqb1 = vn1.eq(b);
        let eqa2 = vn2.eq(a);
        let eqb2 = vn2.eq(b);
        let eqa3 = vn3.eq(a);
        let eqb3 = vn3.eq(b);
        let or1 = eqa1.bitor(eqb1);
        let or2 = eqa2.bitor(eqb2);
        let or3 = eqa3.bitor(eqb3);
        let or4 = or1.bitor(or2);
        let or5 = or3.bitor(or4);
        if or5.any() {
            let mut at = sub(ptr, start_ptr);
            if eqa1.any() || eqa2.any() || eqa3.any() {
                return Some(at + forward_pos3(eqa1.bitmask(), eqa2.bitmask(), eqa3.bitmask()))
            }

            at += VECTOR_SIZE;
            if eqb1.any() || eqb2.any() || eqb3.any() {
                return Some(at + forward_pos3(eqb1.bitmask(), eqb2.bitmask(), eqb3.bitmask()))
            }
        }
        ptr = ptr.add(loop_size);
    }
    while ptr <= end_ptr.sub(VECTOR_SIZE) {
        if let Some(i) = forward_search3(start_ptr, end_ptr, ptr, vn1, vn2, vn3) {
            return Some(i);
        }
        ptr = ptr.add(VECTOR_SIZE);
    }
    if ptr < end_ptr {
        debug_assert!(sub(end_ptr, ptr) < VECTOR_SIZE);
        ptr = ptr.sub(VECTOR_SIZE - sub(end_ptr, ptr));
        debug_assert_eq!(sub(end_ptr, ptr), VECTOR_SIZE);

        return forward_search3(start_ptr, end_ptr, ptr, vn1, vn2, vn3);
    }
    None
}

pub unsafe fn memrchr(n1: u8, haystack: &[u8]) -> Option<usize> {
    let vn1 = i8x16::splat(n1 as i8);
    let len = haystack.len();
    let loop_size = cmp::min(LOOP_SIZE, len);
    let start_ptr = haystack.as_ptr();
    let end_ptr = haystack[haystack.len()..].as_ptr();
    let mut ptr = end_ptr;

    if haystack.len() < VECTOR_SIZE {
        while ptr > start_ptr {
            ptr = ptr.offset(-1);
            if *ptr == n1 {
                return Some(sub(ptr, start_ptr));
            }
        }
        return None;
    }

    ptr = ptr.sub(VECTOR_SIZE);
    if let Some(i) = reverse_search1(start_ptr, end_ptr, ptr, vn1) {
        return Some(i);
    }

    ptr = (end_ptr as usize & !VECTOR_ALIGN) as *const u8;
    debug_assert!(start_ptr <= ptr && ptr <= end_ptr);
    while loop_size == LOOP_SIZE && ptr >= start_ptr.add(loop_size) {
        debug_assert_eq!(0, (ptr as usize) % VECTOR_SIZE);

        ptr = ptr.sub(loop_size);
        let a = i8x16_load(ptr);
        let b = i8x16_load(ptr.add(VECTOR_SIZE));
        let c = i8x16_load(ptr.add(2 * VECTOR_SIZE));
        let d = i8x16_load(ptr.add(3 * VECTOR_SIZE));
        let eqa = vn1.eq(a);
        let eqb = vn1.eq(b);
        let eqc = vn1.eq(c);
        let eqd = vn1.eq(d);
        let or1 = eqa.bitor(eqb);
        let or2 = eqc.bitor(eqd);
        let or3 = or1.bitor(or2);
        if or3.any() {
            let mut at = sub(ptr.add(3 * VECTOR_SIZE), start_ptr);
            if eqd.any() {
                return Some(at + reverse_pos(eqd.bitmask()))
            }

            at -= VECTOR_SIZE;
            if eqc.any() {
                return Some(at + reverse_pos(eqc.bitmask()))
            }

            at -= VECTOR_SIZE;
            if eqb.any() {
                return Some(at + reverse_pos(eqb.bitmask()))
            }

            at -= VECTOR_SIZE;
            debug_assert!(eqa.bitmask() != 0);
            return Some(at + reverse_pos(eqa.bitmask()));
        }
    }
    while ptr >= start_ptr.add(VECTOR_SIZE) {
        ptr = ptr.sub(VECTOR_SIZE);
        if let Some(i) = reverse_search1(start_ptr, end_ptr, ptr, vn1) {
            return Some(i);
        }
    }
    if ptr > start_ptr {
        debug_assert!(sub(ptr, start_ptr) < VECTOR_SIZE);
        return reverse_search1(start_ptr, end_ptr, start_ptr, vn1);
    }
    None
}

pub unsafe fn memrchr2(n1: u8, n2: u8, haystack: &[u8]) -> Option<usize> {
    let vn1 = i8x16::splat(n1 as i8);
    let vn2 = i8x16::splat(n2 as i8);
    let len = haystack.len();
    let loop_size = cmp::min(LOOP_SIZE2, len);
    let start_ptr = haystack.as_ptr();
    let end_ptr = haystack[haystack.len()..].as_ptr();
    let mut ptr = end_ptr;

    if haystack.len() < VECTOR_SIZE {
        while ptr > start_ptr {
            ptr = ptr.offset(-1);
            if *ptr == n1 || *ptr == n2 {
                return Some(sub(ptr, start_ptr));
            }
        }
        return None;
    }

    ptr = ptr.sub(VECTOR_SIZE);
    if let Some(i) = reverse_search2(start_ptr, end_ptr, ptr, vn1, vn2) {
        return Some(i);
    }

    ptr = (end_ptr as usize & !VECTOR_ALIGN) as *const u8;
    debug_assert!(start_ptr <= ptr && ptr <= end_ptr);
    while loop_size == LOOP_SIZE && ptr >= start_ptr.add(loop_size) {
        debug_assert_eq!(0, (ptr as usize) % VECTOR_SIZE);

        ptr = ptr.sub(loop_size);
        let a = i8x16_load(ptr);
        let b = i8x16_load(ptr.add(VECTOR_SIZE));
        let eqa1 = vn1.eq(a);
        let eqb1 = vn1.eq(b);
        let eqa2 = vn2.eq(a);
        let eqb2 = vn2.eq(b);
        let or1 = eqa1.bitor(eqb1);
        let or2 = eqa2.bitor(eqb2);
        let or3 = or1.bitor(or2);
        if or3.any() {
            let mut at = sub(ptr.add(VECTOR_SIZE), start_ptr);
            if eqb1.any() || eqb2.any() {
                return Some(at + reverse_pos2(eqb1.bitmask(), eqb2.bitmask()))
            }

            at -= VECTOR_SIZE;
            return Some(at + reverse_pos2(eqa1.bitmask(), eqa2.bitmask()));
        }
    }
    while ptr >= start_ptr.add(VECTOR_SIZE) {
        ptr = ptr.sub(VECTOR_SIZE);
        if let Some(i) = reverse_search2(start_ptr, end_ptr, ptr, vn1, vn2) {
            return Some(i);
        }
    }
    if ptr > start_ptr {
        debug_assert!(sub(ptr, start_ptr) < VECTOR_SIZE);
        return reverse_search2(start_ptr, end_ptr, start_ptr, vn1, vn2);
    }
    None
}

pub unsafe fn memrchr3(n1: u8, n2: u8, n3: u8, haystack: &[u8]) -> Option<usize> {
    let vn1 = i8x16::splat(n1 as i8);
    let vn2 = i8x16::splat(n2 as i8);
    let vn3 = i8x16::splat(n3 as i8);
    let len = haystack.len();
    let loop_size = cmp::min(LOOP_SIZE2, len);
    let start_ptr = haystack.as_ptr();
    let end_ptr = haystack[haystack.len()..].as_ptr();
    let mut ptr = end_ptr;

    if haystack.len() < VECTOR_SIZE {
        while ptr > start_ptr {
            ptr = ptr.offset(-1);
            if *ptr == n1 || *ptr == n2 || *ptr == n3 {
                return Some(sub(ptr, start_ptr));
            }
        }
        return None;
    }

    ptr = ptr.sub(VECTOR_SIZE);
    if let Some(i) = reverse_search3(start_ptr, end_ptr, ptr, vn1, vn2, vn3) {
        return Some(i);
    }

    ptr = (end_ptr as usize & !VECTOR_ALIGN) as *const u8;
    debug_assert!(start_ptr <= ptr && ptr <= end_ptr);
    while loop_size == LOOP_SIZE && ptr >= start_ptr.add(loop_size) {
        debug_assert_eq!(0, (ptr as usize) % VECTOR_SIZE);

        ptr = ptr.sub(loop_size);
        let a = i8x16_load(ptr);
        let b = i8x16_load(ptr.add(VECTOR_SIZE));
        let eqa1 = vn1.eq(a);
        let eqb1 = vn1.eq(b);
        let eqa2 = vn2.eq(a);
        let eqb2 = vn2.eq(b);
        let eqa3 = vn3.eq(a);
        let eqb3 = vn3.eq(b);
        let or1 = eqa1.bitor(eqb1);
        let or2 = eqa2.bitor(eqb2);
        let or3 = eqa3.bitor(eqb3);
        let or4 = or1.bitor(or2);
        let or5 = or3.bitor(or4);
        if or5.any() {
            let mut at = sub(ptr.add(VECTOR_SIZE), start_ptr);
            if eqb1.any() || eqb2.any() || eqb3.any() {
                return Some(at + reverse_pos3(eqb1.bitmask(), eqb2.bitmask(), eqb3.bitmask()))
            }

            at -= VECTOR_SIZE;
            return Some(at + reverse_pos3(eqa1.bitmask(), eqa2.bitmask(), eqa3.bitmask()));
        }
    }
    while ptr >= start_ptr.add(VECTOR_SIZE) {
        ptr = ptr.sub(VECTOR_SIZE);
        if let Some(i) = reverse_search3(start_ptr, end_ptr, ptr, vn1, vn2, vn3) {
            return Some(i);
        }
    }
    if ptr > start_ptr {
        debug_assert!(sub(ptr, start_ptr) < VECTOR_SIZE);
        return reverse_search3(start_ptr, end_ptr, start_ptr, vn1, vn2, vn3);
    }
    None
}

pub unsafe fn forward_search1(
    start_ptr: *const u8,
    end_ptr: *const u8,
    ptr: *const u8,
    vn1: i8x16,
) -> Option<usize> {
    debug_assert!(sub(end_ptr, start_ptr) >= VECTOR_SIZE);
    debug_assert!(start_ptr <= ptr);
    debug_assert!(ptr <= end_ptr.sub(VECTOR_SIZE));

    let chunk = i8x16_load(ptr);
    let eq = vn1.eq(chunk);
    if eq.any() {
        Some(sub(ptr, start_ptr) + forward_pos(eq.bitmask()))
    } else {
        None
    }
}

unsafe fn forward_search2(
    start_ptr: *const u8,
    end_ptr: *const u8,
    ptr: *const u8,
    vn1: i8x16,
    vn2: i8x16,
) -> Option<usize> {
    debug_assert!(sub(end_ptr, start_ptr) >= VECTOR_SIZE);
    debug_assert!(start_ptr <= ptr);
    debug_assert!(ptr <= end_ptr.sub(VECTOR_SIZE));

    let chunk = i8x16_load(ptr);
    let eq1 = vn1.eq(chunk);
    let eq2 = vn2.eq(chunk);
    let or = eq1.bitor(eq2);
    if or.any() {
        Some(sub(ptr, start_ptr) + forward_pos2(eq1.bitmask(), eq2.bitmask()))
    } else {
        None
    }
}

unsafe fn forward_search3(
    start_ptr: *const u8,
    end_ptr: *const u8,
    ptr: *const u8,
    vn1: i8x16,
    vn2: i8x16,
    vn3: i8x16,
) -> Option<usize> {
    debug_assert!(sub(end_ptr, start_ptr) >= VECTOR_SIZE);
    debug_assert!(start_ptr <= ptr);
    debug_assert!(ptr <= end_ptr.sub(VECTOR_SIZE));

    let chunk = i8x16_load(ptr);
    let eq1 = vn1.eq(chunk);
    let eq2 = vn2.eq(chunk);
    let eq3 = vn3.eq(chunk);
    let or1 = eq1.bitor(eq2);
    let or2 = or1.bitor(eq3);
    if or2.any() {
        Some(sub(ptr, start_ptr) + forward_pos3(eq1.bitmask(), eq2.bitmask(), eq3.bitmask()))
    } else {
        None
    }
}

unsafe fn reverse_search1(
    start_ptr: *const u8,
    end_ptr: *const u8,
    ptr: *const u8,
    vn1: i8x16,
) -> Option<usize> {
    debug_assert!(sub(end_ptr, start_ptr) >= VECTOR_SIZE);
    debug_assert!(start_ptr <= ptr);
    debug_assert!(ptr <= end_ptr.sub(VECTOR_SIZE));

    let chunk = i8x16_load(ptr);
    let eq = vn1.eq(chunk);
    if eq.any() {
        Some(sub(ptr, start_ptr) + reverse_pos(eq.bitmask()))
    } else {
        None
    }
}

unsafe fn reverse_search2(
    start_ptr: *const u8,
    end_ptr: *const u8,
    ptr: *const u8,
    vn1: i8x16,
    vn2: i8x16,
) -> Option<usize> {
    debug_assert!(sub(end_ptr, start_ptr) >= VECTOR_SIZE);
    debug_assert!(start_ptr <= ptr);
    debug_assert!(ptr <= end_ptr.sub(VECTOR_SIZE));

    let chunk = i8x16_load(ptr);
    let eq1 = vn1.eq(chunk);
    let eq2 = vn2.eq(chunk);
    let or = eq1.bitor(eq2);
    if or.any() {
        Some(sub(ptr, start_ptr) + reverse_pos2(eq1.bitmask(), eq2.bitmask()))
    } else {
        None
    }
}

unsafe fn reverse_search3(
    start_ptr: *const u8,
    end_ptr: *const u8,
    ptr: *const u8,
    vn1: i8x16,
    vn2: i8x16,
    vn3: i8x16,
) -> Option<usize> {
    debug_assert!(sub(end_ptr, start_ptr) >= VECTOR_SIZE);
    debug_assert!(start_ptr <= ptr);
    debug_assert!(ptr <= end_ptr.sub(VECTOR_SIZE));

    let chunk = i8x16_load(ptr);
    let eq1 = vn1.eq(chunk);
    let eq2 = vn2.eq(chunk);
    let eq3 = vn3.eq(chunk);
    let or1 = eq1.bitor(eq2);
    let or2 = or1.bitor(eq3);
    if or2.any() {
        Some(sub(ptr, start_ptr) + reverse_pos3(eq1.bitmask(), eq2.bitmask(), eq3.bitmask()))
    } else {
        None
    }
}

/// Compute the position of the first matching byte from the given mask. The
/// position returned is always in the range [0, 15].
///
/// The mask given is expected to be the result of _mm_movemask_epi8.
fn forward_pos(mask: u16) -> usize {
    // We are dealing with little endian here, where the most significant byte
    // is at a higher address. That means the least significant bit that is set
    // corresponds to the position of our first matching byte. That position
    // corresponds to the number of zeros after the least significant bit.
    mask.trailing_zeros() as usize
}

/// Compute the position of the first matching byte from the given masks. The
/// position returned is always in the range [0, 15]. Each mask corresponds to
/// the equality comparison of a single byte.
///
/// The masks given are expected to be the result of _mm_movemask_epi8, where
/// at least one of the masks is non-zero (i.e., indicates a match).
fn forward_pos2(mask1: u16, mask2: u16) -> usize {
    debug_assert!(mask1 != 0 || mask2 != 0);

    forward_pos(mask1 | mask2)
}

/// Compute the position of the first matching byte from the given masks. The
/// position returned is always in the range [0, 15]. Each mask corresponds to
/// the equality comparison of a single byte.
///
/// The masks given are expected to be the result of _mm_movemask_epi8, where
/// at least one of the masks is non-zero (i.e., indicates a match).
fn forward_pos3(mask1: u16, mask2: u16, mask3: u16) -> usize {
    debug_assert!(mask1 != 0 || mask2 != 0 || mask3 != 0);

    forward_pos(mask1 | mask2 | mask3)
}

/// Compute the position of the last matching byte from the given mask. The
/// position returned is always in the range [0, 15].
///
/// The mask given is expected to be the result of _mm_movemask_epi8.
fn reverse_pos(mask: u16) -> usize {
    // We are dealing with little endian here, where the most significant byte
    // is at a higher address. That means the most significant bit that is set
    // corresponds to the position of our last matching byte. The position from
    // the end of the mask is therefore the number of leading zeros in a 16
    // bit integer, and the position from the start of the mask is therefore
    // 16 - (leading zeros) - 1.
    VECTOR_SIZE - mask.leading_zeros() as usize - 1
}

/// Compute the position of the last matching byte from the given masks. The
/// position returned is always in the range [0, 15]. Each mask corresponds to
/// the equality comparison of a single byte.
///
/// The masks given are expected to be the result of _mm_movemask_epi8, where
/// at least one of the masks is non-zero (i.e., indicates a match).
fn reverse_pos2(mask1: u16, mask2: u16) -> usize {
    debug_assert!(mask1 != 0 || mask2 != 0);

    reverse_pos(mask1 | mask2)
}

/// Compute the position of the last matching byte from the given masks. The
/// position returned is always in the range [0, 15]. Each mask corresponds to
/// the equality comparison of a single byte.
///
/// The masks given are expected to be the result of _mm_movemask_epi8, where
/// at least one of the masks is non-zero (i.e., indicates a match).
fn reverse_pos3(mask1: u16, mask2: u16, mask3: u16) -> usize {
    debug_assert!(mask1 != 0 || mask2 != 0 || mask3 != 0);

    reverse_pos(mask1 | mask2 | mask3)
}

/// Subtract `b` from `a` and return the difference. `a` should be greater than
/// or equal to `b`.
fn sub(a: *const u8, b: *const u8) -> usize {
    debug_assert!(a >= b);
    (a as usize) - (b as usize)
}

// Load 128 bits from a ptr into i8x16.
unsafe fn i8x16_load(ptr: *const u8) -> i8x16 {
    let slc = slice::from_raw_parts(ptr as *const i8, 16);
    i8x16::from_slice_unaligned_unchecked(slc)
}

}   // private

#[inline(always)]
pub fn memchr(n1: u8, haystack: &[u8]) -> Option<usize> {
    unsafe { private::memchr(n1, haystack) }
}

#[inline(always)]
pub fn memchr2(n1: u8, n2: u8, haystack: &[u8]) -> Option<usize> {
    unsafe { private::memchr2(n1, n2, haystack) }
}

#[inline(always)]
pub fn memchr3(n1: u8, n2: u8, n3: u8, haystack: &[u8]) -> Option<usize> {
    unsafe { private::memchr3(n1, n2, n3, haystack) }
}

#[inline(always)]
pub fn memrchr(n1: u8, haystack: &[u8]) -> Option<usize> {
    unsafe { private::memrchr(n1, haystack) }
}

#[inline(always)]
pub fn memrchr2(n1: u8, n2: u8, haystack: &[u8]) -> Option<usize> {
    unsafe { private::memrchr2(n1, n2, haystack) }
}

#[inline(always)]
pub fn memrchr3(n1: u8, n2: u8, n3: u8, haystack: &[u8]) -> Option<usize> {
    unsafe { private::memrchr3(n1, n2, n3, haystack) }
}
