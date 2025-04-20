use std::ops::Sub;

#[allow(dead_code)]
pub fn dist_to_range<T: PartialOrd + Sub<Output = T> + Default>(
    x: T,
    start_inclusive: T,
    end_inclusive: T,
) -> T {
    if x < start_inclusive {
        x - start_inclusive
    } else if x > end_inclusive {
        x - end_inclusive
    } else {
        T::default()
    }
}

#[allow(dead_code)]
pub fn get_in_range<T: PartialOrd + Sub<Output = T> + Default>(
    x: T,
    start_inclusive: T,
    end_inclusive: T,
) -> Option<T> {
    if x < start_inclusive || x > end_inclusive {
        None
    } else {
        Some(x)
    }
}
