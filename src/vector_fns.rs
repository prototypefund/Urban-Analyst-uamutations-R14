use ndarray::Array2;

/// Calculates a vector of sequential difference between two arrays of f64 values.
///
/// The distances are calculated in the full multi-dimensional space, so that each value in the
/// first array (`values1`) is matched to the entry in `values2` at the minimal distance. Each
/// element of `values2` is matched to one unique element of `values1`. Unique matching is done
/// with a hash map, meaning that the procedure only works consistently if started from some
/// extreme point of the `values1` distribution. This extreme point is taken as the lowest value of
/// the first column of `values1`. Alternatives to this include calculating a multiple linear
/// regression between the first column of `values1` and all columns of `values2`, and taking the
/// first point of that, but that will by definition be the lowest (or possibly highest) value of
/// `values1` anyway.
///
/// This consideration means that `values1` can first be sorted by the first column, these sorted
/// values used to find closest values of `values2`, and then the original order restored to yield
/// the final desired matching.
///
/// # Arguments
///
/// * `values1` - An Array2 object which provides the reference values against which to sort
/// `values2`.
/// * `values2` - An Array2 object which is to be sorted against `values1`.
/// * `absolute` - A boolean indicating whether to calculate absolute differences.
///
/// # Panics
///
/// This function will panic if `values1` is empty or if `values1` and `values2` have different
/// dimensions.
///
/// # Returns
///
/// A vector of `usize` values matching each consecutive element in `values1` to the closest
/// elements in `values2`.  If `absolute` is true, the differences are absolute values. Otherwise,
/// the differences are differences relative to `values1`.
///
/// # Example
///
/// ```
/// use uamutations::vector_fns::calculate_dists;
/// let values1 = ndarray::array![[1.0, 2.0, 4.0, 5.0]];
/// let values2 = ndarray::array![[7.0, 9.0, 3.0, 2.0]];
/// let result = calculate_dists(&values1, &values2, true);
/// // For each values1, result will be (v2 - v1) for closest values2. So closest value to v1[3] =
/// // 4, for example, is v2 = 3, and (v2 - v1) = 3 - 4 = -1. Or v1[4] = 5, with closest of 3, and
/// // 3 - 5 = -2.
/// assert_eq!(result, vec![1.0, 1.0, 3.0, 4.0]);
/// let result = calculate_dists(&values1, &values2, false);
/// assert_eq!(result, vec![1.0, 0.5, 0.75, 0.8]);
/// ```
pub fn calculate_dists(values1: &Array2<f64>, values2: &Array2<f64>, absolute: bool) -> Vec<f64> {
    assert!(!values1.is_empty(), "values1 must not be empty");
    assert_eq!(
        values1.dim(),
        values2.dim(),
        "values1 and values2 must have the same dimensions."
    );

    let values1_ref_var: Vec<f64> = values1.row(0).to_vec();
    let values2_ref_var: Vec<f64> = values2.row(0).to_vec();

    let sorting_order = get_ordering_index(&values1_ref_var.to_vec(), false, false);

    use std::collections::HashSet;

    // Make a vector of (distances, index) from each `values1` entry to the closest entry of
    // `values2` along first dimensions only. The main iteration is over `sorting_order`, but
    // values are inserted directly in-space in `results`, which then holds closest distances for
    // each sequential `values1` entry to nearest `values2`.
    let mut nearest_dists: Vec<f64> = vec![0.0; sorting_order.len()];
    let mut used_indices = HashSet::new();

    for &i in sorting_order.iter() {
        let v1 = values1_ref_var[i];
        let mut min_dist = f64::MAX;
        let mut min_index = 0;

        // Identify nearest absolute distance:
        for (j, v2) in values2_ref_var.iter().enumerate() {
            if used_indices.contains(&j) {
                continue;
            }
            let dist = (v1 - v2).abs();

            if dist < min_dist {
                min_dist = dist;
                min_index = j;
            }
        }

        used_indices.insert(min_index);
        // But then return signed versions, converted to relative if !absolute:
        nearest_dists[i] = if absolute {
            values2_ref_var[min_index] - v1
        } else {
            (values2_ref_var[min_index] - v1) / v1
        }
    }

    nearest_dists
}

/// Returns a vector of indices that would sort the input vector in ascending or descending order.
///
/// # Arguments
///
/// * `vals` - A slice of f64 values to be sorted.
/// * `desc` - If `true`, sort in descending order; otherwise sort in ascensing order.
/// * `is_abs` - A boolean indicating whether sorting should be based on absolute values.
///
/// # Example
///
/// ```
/// use uamutations::vector_fns::get_ordering_index;
/// let vals = vec![1.0, -2.0, 3.0, -4.0, 5.0];
/// let result = get_ordering_index(&vals, false, false);
/// assert_eq!(result, vec![3, 1, 0, 2, 4]);
/// ```
pub fn get_ordering_index(vals: &[f64], desc: bool, is_abs: bool) -> Vec<usize> {
    let mut pairs: Vec<_> = vals.iter().enumerate().collect();

    if is_abs {
        if desc {
            pairs.sort_by(|&(_, a), &(_, b)| b.abs().partial_cmp(&a.abs()).unwrap());
        } else {
            pairs.sort_by(|&(_, a), &(_, b)| a.abs().partial_cmp(&b.abs()).unwrap());
        }
    } else if desc {
        pairs.sort_by(|&(_, a), &(_, b)| b.partial_cmp(a).unwrap());
    } else {
        pairs.sort_by(|&(_, a), &(_, b)| a.partial_cmp(b).unwrap());
    }

    let index: Vec<_> = pairs.iter().map(|&(index, _)| index).collect();

    index
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_ordering_index() {
        let vals = vec![1.0, -2.0, 3.0, -4.0, 5.0];
        let mut expected = vec![3, 1, 0, 2, 4]; // Indices of vals in ascending order
                                                // bool flags are (desc, is_abs):
        assert_eq!(get_ordering_index(&vals, false, false), expected);
        expected.reverse();
        assert_eq!(get_ordering_index(&vals, true, false), expected);
    }

    #[test]
    fn test_get_ordering_index_abs() {
        let vals = vec![1.0, -2.0, 3.0, -4.0, 5.0];
        let mut expected = vec![0, 1, 2, 3, 4]; // Indices of absolute vals in ascending order
        assert_eq!(get_ordering_index(&vals, false, true), expected);
        expected.reverse();
        assert_eq!(get_ordering_index(&vals, true, true), expected);
    }

    #[test]
    fn test_calculate_dists_absolute() {
        // Note that 2.0 is closest to 2.0, but is matched to 3.0 because of sequential and unique
        // matching.
        let values1 = ndarray::array![[1.0, 2.0, 4.0, 5.0]];
        let values2 = ndarray::array![[7.0, 9.0, 3.0, 2.0]];
        let result = calculate_dists(&values1, &values2, true);
        assert_eq!(result, vec![1.0, 1.0, 3.0, 4.0]);
    }

    #[test]
    fn test_calculate_dists_relative() {
        let values1 = ndarray::array![[1.0, 2.0, 4.0, 5.0]];
        let values2 = ndarray::array![[7.0, 9.0, 3.0, 2.0]];
        let result = calculate_dists(&values1, &values2, false);
        assert_eq!(result, vec![1.0, 0.5, 0.75, 0.8]);
    }
}
