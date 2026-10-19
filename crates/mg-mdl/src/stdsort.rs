//! libstdc++'s `std::sort`, step for step: the order the game's compiler
//! leaves a node's children in when it sorts them (nwn-mdl-spec
//! MDL-NODE-012). The sort is not stable, so past 16 elements only the
//! same algorithm gives the same order: introsort (a median-of-three
//! pivot, partitions down to runs of 16, heap sort past 2·⌊log2 n⌋
//! levels), then an insertion sort.

/// Sorts `v` as libstdc++'s `std::sort` does with the comparison `less`.
pub(crate) fn sort<T: Copy>(v: &mut [T], less: &dyn Fn(&T, &T) -> bool) {
    if v.len() < 2 {
        return;
    }
    let depth = 2 * (usize::BITS - 1 - v.len().leading_zeros()) as usize;
    introsort_loop(v, 0, v.len(), depth, less);
    final_insertion_sort(v, less);
}

const THRESHOLD: usize = 16;

fn introsort_loop<T: Copy>(
    v: &mut [T],
    first: usize,
    mut last: usize,
    mut depth: usize,
    less: &dyn Fn(&T, &T) -> bool,
) {
    while last - first > THRESHOLD {
        if depth == 0 {
            heap_sort(&mut v[first..last], less);
            return;
        }
        depth -= 1;
        let cut = partition_pivot(v, first, last, less);
        introsort_loop(v, cut, last, depth, less);
        last = cut;
    }
}

fn partition_pivot<T: Copy>(
    v: &mut [T],
    first: usize,
    last: usize,
    less: &dyn Fn(&T, &T) -> bool,
) -> usize {
    let mid = first + (last - first) / 2;
    move_median_to_first(v, first, first + 1, mid, last - 1, less);
    unguarded_partition(v, first + 1, last, first, less)
}

fn move_median_to_first<T: Copy>(
    v: &mut [T],
    result: usize,
    a: usize,
    b: usize,
    c: usize,
    less: &dyn Fn(&T, &T) -> bool,
) {
    let pick = if less(&v[a], &v[b]) {
        if less(&v[b], &v[c]) {
            b
        } else if less(&v[a], &v[c]) {
            c
        } else {
            a
        }
    } else if less(&v[a], &v[c]) {
        a
    } else if less(&v[b], &v[c]) {
        c
    } else {
        b
    };
    v.swap(result, pick);
}

fn unguarded_partition<T: Copy>(
    v: &mut [T],
    mut first: usize,
    mut last: usize,
    pivot: usize,
    less: &dyn Fn(&T, &T) -> bool,
) -> usize {
    loop {
        while less(&v[first], &v[pivot]) {
            first += 1;
        }
        last -= 1;
        while less(&v[pivot], &v[last]) {
            last -= 1;
        }
        if first >= last {
            return first;
        }
        v.swap(first, last);
        first += 1;
    }
}

fn final_insertion_sort<T: Copy>(v: &mut [T], less: &dyn Fn(&T, &T) -> bool) {
    if v.len() > THRESHOLD {
        insertion_sort(&mut v[..THRESHOLD], less);
        for i in THRESHOLD..v.len() {
            unguarded_linear_insert(v, i, less);
        }
    } else {
        insertion_sort(v, less);
    }
}

fn insertion_sort<T: Copy>(v: &mut [T], less: &dyn Fn(&T, &T) -> bool) {
    for i in 1..v.len() {
        if less(&v[i], &v[0]) {
            let value = v[i];
            v.copy_within(0..i, 1);
            v[0] = value;
        } else {
            unguarded_linear_insert(v, i, less);
        }
    }
}

fn unguarded_linear_insert<T: Copy>(v: &mut [T], mut last: usize, less: &dyn Fn(&T, &T) -> bool) {
    let value = v[last];
    while less(&value, &v[last - 1]) {
        v[last] = v[last - 1];
        last -= 1;
    }
    v[last] = value;
}

fn heap_sort<T: Copy>(v: &mut [T], less: &dyn Fn(&T, &T) -> bool) {
    let len = v.len();
    if len >= 2 {
        let mut parent = (len - 2) / 2;
        loop {
            let value = v[parent];
            adjust_heap(v, parent, len, value, less);
            if parent == 0 {
                break;
            }
            parent -= 1;
        }
    }
    let mut last = len;
    while last > 1 {
        last -= 1;
        let value = v[last];
        v[last] = v[0];
        adjust_heap(&mut v[..last], 0, last, value, less);
    }
}

fn adjust_heap<T: Copy>(
    v: &mut [T],
    mut hole: usize,
    len: usize,
    value: T,
    less: &dyn Fn(&T, &T) -> bool,
) {
    let top = hole;
    let mut second = hole;
    while second < (len - 1) / 2 {
        second = 2 * (second + 1);
        if less(&v[second], &v[second - 1]) {
            second -= 1;
        }
        v[hole] = v[second];
        hole = second;
    }
    if len & 1 == 0 && second == (len - 2) / 2 {
        second = 2 * (second + 1);
        v[hole] = v[second - 1];
        hole = second - 1;
    }
    // (Pushed back up.)
    let mut parent = hole.wrapping_sub(1) / 2;
    while hole > top && less(&v[parent], &value) {
        v[hole] = v[parent];
        hole = parent;
        parent = hole.wrapping_sub(1) / 2;
    }
    v[hole] = value;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// It sorts, by any measure; and as a sort of equal keys past 16 it is
    /// not stable, the property that makes the game's order its own.
    #[test]
    fn it_sorts_and_is_not_stable_past_sixteen() {
        let mut seed = 7u32;
        let mut next = || {
            seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12345);
            seed >> 16
        };
        for n in [0, 1, 2, 5, 16, 17, 40, 100, 1000] {
            let mut v: Vec<(u32, usize)> = (0..n).map(|i| (next() % 4, i)).collect();
            sort(&mut v, &|a, b| a.0 < b.0);
            assert!(v.windows(2).all(|w| w[0].0 <= w[1].0), "{n}");
        }
        let mut v: Vec<(u32, usize)> = (0..40).map(|i| ((i % 3) as u32, i)).collect();
        sort(&mut v, &|a, b| a.0 < b.0);
        let stable = v.windows(2).all(|w| w[0].0 != w[1].0 || w[0].1 < w[1].1);
        assert!(!stable);
        // (Heap sort, where the depth runs out: a long run of one order.)
        let mut v: Vec<u32> = (0..500).rev().collect();
        sort(&mut v, &|a, b| a < b);
        assert!(v.windows(2).all(|w| w[0] <= w[1]));
    }
}
