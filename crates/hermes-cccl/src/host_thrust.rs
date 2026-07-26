//! Host-side Thrust-shaped algorithms (CPU).
//!
//! These mirror CCCL Thrust algorithm *names* and semantics for host execution
//! policies. They are the first CUDA-compat layer that does not require a
//! device kernel compiler — device offload attaches later via hermes-cuda.

use core::cmp::Ordering;

/// thrust::fill
pub fn hermes_fill<T: Copy>(slice: &mut [T], value: T) {
    for x in slice.iter_mut() {
        *x = value;
    }
}

/// thrust::copy
pub fn hermes_copy<T: Copy>(src: &[T], dst: &mut [T]) -> usize {
    let n = core::cmp::min(src.len(), dst.len());
    dst[..n].copy_from_slice(&src[..n]);
    n
}

/// thrust::count
pub fn hermes_count<T: PartialEq>(slice: &[T], value: &T) -> usize {
    slice.iter().filter(|x| *x == value).count()
}

/// thrust::for_each
pub fn hermes_for_each<T, F: FnMut(&mut T)>(slice: &mut [T], mut f: F) {
    for x in slice.iter_mut() {
        f(x);
    }
}

/// thrust::transform (unary)
pub fn hermes_transform<T, U, F: FnMut(&T) -> U>(src: &[T], dst: &mut [U], mut f: F) -> usize {
    let n = core::cmp::min(src.len(), dst.len());
    for i in 0..n {
        dst[i] = f(&src[i]);
    }
    n
}

/// thrust::reduce
pub fn hermes_reduce<T: Copy, F: FnMut(T, T) -> T>(slice: &[T], init: T, mut f: F) -> T {
    let mut acc = init;
    for x in slice {
        acc = f(acc, *x);
    }
    acc
}

/// thrust::sequence — fill with consecutive values starting at `init`.
pub fn hermes_sequence(slice: &mut [i64], init: i64) {
    let mut v = init;
    for x in slice.iter_mut() {
        *x = v;
        v = v.wrapping_add(1);
    }
}

/// thrust::equal
pub fn hermes_equal<T: PartialEq>(a: &[T], b: &[T]) -> bool {
    a == b
}

/// thrust::find — index of first match, or None.
pub fn hermes_find<T: PartialEq>(slice: &[T], value: &T) -> Option<usize> {
    slice.iter().position(|x| x == value)
}

/// thrust::replace
pub fn hermes_replace<T: PartialEq + Copy>(slice: &mut [T], old: &T, new: T) -> usize {
    let mut n = 0;
    for x in slice.iter_mut() {
        if *x == *old {
            *x = new;
            n += 1;
        }
    }
    n
}

/// thrust::sort (unstable, Ord)
pub fn hermes_sort<T: Ord>(slice: &mut [T]) {
    slice.sort_unstable();
}

/// thrust::unique — compact unique prefix; returns new length.
pub fn hermes_unique<T: PartialEq + Copy>(slice: &mut [T]) -> usize {
    if slice.is_empty() {
        return 0;
    }
    let mut w = 1;
    for r in 1..slice.len() {
        if slice[r] != slice[w - 1] {
            slice[w] = slice[r];
            w += 1;
        }
    }
    w
}

/// thrust::inclusive_scan
pub fn hermes_scan_inclusive<T: Copy, F: FnMut(T, T) -> T>(
    src: &[T],
    dst: &mut [T],
    mut f: F,
) -> usize {
    let n = core::cmp::min(src.len(), dst.len());
    if n == 0 {
        return 0;
    }
    dst[0] = src[0];
    for i in 1..n {
        dst[i] = f(dst[i - 1], src[i]);
    }
    n
}

/// Comparison helper matching thrust binary predicates style.
pub fn hermes_cmp<T: Ord>(a: &T, b: &T) -> Ordering {
    a.cmp(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fill_copy_count_transform_reduce() {
        let mut a = [0u32; 8];
        hermes_fill(&mut a, 3);
        assert_eq!(hermes_count(&a, &3), 8);
        let mut b = [0u32; 8];
        assert_eq!(hermes_copy(&a, &mut b), 8);
        let mut c = [0u32; 8];
        hermes_transform(&b, &mut c, |x| x * 2);
        assert_eq!(hermes_reduce(&c, 0, |x, y| x + y), 48);
    }

    #[test]
    fn sort_unique_scan_sequence() {
        let mut v = [3i64, 1, 2, 1, 3];
        hermes_sort(&mut v);
        assert_eq!(&v, &[1, 1, 2, 3, 3]);
        let n = hermes_unique(&mut v);
        assert_eq!(&v[..n], &[1, 2, 3]);
        let mut s = [0i64; 5];
        hermes_sequence(&mut s, 10);
        assert_eq!(&s, &[10, 11, 12, 13, 14]);
        let mut scan = [0i64; 5];
        hermes_scan_inclusive(&s, &mut scan, |a, b| a + b);
        assert_eq!(scan[4], 10 + 11 + 12 + 13 + 14);
    }

    #[test]
    fn find_replace_equal_foreach() {
        let mut a = [1, 2, 3, 2];
        assert_eq!(hermes_find(&a, &2), Some(1));
        assert_eq!(hermes_replace(&mut a, &2, 9), 2);
        assert_eq!(&a, &[1, 9, 3, 9]);
        assert!(hermes_equal(&[1, 2], &[1, 2]));
        hermes_for_each(&mut a, |x| *x += 1);
        assert_eq!(a[0], 2);
    }
}
