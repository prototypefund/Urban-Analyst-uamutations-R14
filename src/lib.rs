//! This is a stand-alone crate which implements the mutation algorithm for [Urban
//! Analyst](https://urbananalyst.city). The algorithm mutates selected properties for one city to
//! become more like those of another selected city.

use nalgebra::DMatrix;
use std::fs::File;
use std::io::BufReader;

pub mod calculate_dists;
pub mod mlr;
pub mod read_write_file;
pub mod transform;

/// This is the main function, which reads data from two JSON files, calculates absolute and
/// relative differences between the two sets of data, and writes the results to an output file.
///
/// # Arguments
///
/// * `fname1` - Path to local JSON file with data which are to be mutated.
/// * `fname2` - Path to local JSON file with data of mutation target towards which first data are
/// to be mutated.
/// * `varname` - Name of variable in both `fname1` and `fname2` to be mutated.
/// * `varextra` - Extra variables to be considered in the mutation.
/// * `nentries` - The number of entries to be read from the JSON files.
///
/// # Returns
///
/// A vector of length equal to number of distinct groups in the input data 'index' column, with
/// each value quantifying the mean distance to the nearest points in the target distribution.
///
/// # Process
///
/// 1. Reads the variable specified by `varname` from the files `fname1` and `fname2`.
/// 2. Calculates the absolute and relative differences between the two sets of data.
/// 3. Orders the relative differences in descending order.
///
/// The following seven vectors of equal length are written to the output file:
/// 1. values: The original values of 'varname' from 'fname1'.
/// 2. dists: The relative degree by which each should be mutated.
///
/// # Panics
///
/// This function will panic if the input files cannot be read, or if the output file cannot be written.

pub fn uamutate(
    reader1: BufReader<File>,
    reader2: BufReader<File>,
    varnames: &Vec<String>,
    nentries: usize,
) -> DMatrix<f64> {
    // Read contents of files:
    let (mut values1, groups1) = read_write_file::readfile(reader1, varnames, nentries);
    let (mut values2, _groups2) = read_write_file::readfile(reader2, varnames, nentries);
    // Adjust `values1` by removing its dependence on varextra, and replacing with the dependnece
    // of values2 on same variables (but only if `varextra` are specified):
    if values1.nrows() > 1 {
        mlr::adj_for_beta(&mut values1, &values2);
    }

    values1 = transform::transform_values(&values1, &varnames[0]);
    values2 = transform::transform_values(&values2, &varnames[0]);

    // Then calculate successive differences between the two sets of values. These are the
    // distances by which `values1` need to be moved in the first dimension only to match the
    // closest equivalent values of `values2`.
    let dists = calculate_dists::calculate_dists(&values1, &values2);
    aggregate_to_groups(&values1, &dists, &groups1)
}

/// Loop over all columns of the `dists` `DMatrix` object, and aggregate groups for each column.
///
/// # Arguments
///
/// * `values1` - The original values used as references for the distances; aggregated versions of
/// these are also returned.
/// * `dists` - A matrix of distances between entries in `values1` and closest values in `values2`.
/// * `groups` - A vector of same length as `dists`, with 1-based indices of group numbers. There
/// will generally be far fewer unique groups as there are entries in `dists`.
///
/// # Returns
///
/// A `DMatrix` object with numbers of rows equal to number of distinct groups in the input data
/// 'index' column, with each value quantifying the mean distance to the nearest points in the
/// target distribution. This return object has four columns:
/// 1. The original value
/// 2. The mutated value
/// 3. The absolute difference between mutate and original values
/// 4. The relative difference between mutate and original values
fn aggregate_to_groups(
    values1: &DMatrix<f64>,
    dists: &DMatrix<f64>,
    groups: &[usize],
) -> DMatrix<f64> {
    let mut result = DMatrix::zeros(groups.len(), dists.ncols() + 2);

    // Aggregate original values first:
    let values1_first_col: Vec<f64> = values1.column(0).iter().cloned().collect();
    let mean_dist = aggregate_to_groups_single_col(&values1_first_col, groups);
    for (i, &value) in mean_dist.iter().enumerate() {
        result[(i, 0)] = value;
    }

    // Then generate absolute transformed value from original value plus  absolute distance:
    let dists_abs: Vec<f64> = dists.column(0).iter().cloned().collect();
    let values1_transformed: Vec<f64> = values1_first_col
        .iter()
        .zip(dists_abs.iter())
        .map(|(&a, &b)| a + b)
        .collect();
    for (i, &value) in values1_transformed.iter().enumerate() {
        result[(i, 1)] = value;
    }

    // Then both absolute and relative distances:
    for col in 0..dists.ncols() {
        let dists_col: Vec<f64> = dists.column(col).iter().cloned().collect();
        let mean_dist = aggregate_to_groups_single_col(&dists_col, groups);
        for (i, &value) in mean_dist.iter().enumerate() {
            result[(i, col + 2)] = value;
        }
    }

    result
}

/// Aggregate a single column of distances within the groups defined in the original `groups`
/// vector.
///
/// # Arguments
///
/// * `dists` - A vector of distances between entries in `values1` and closest values in `values2`.
/// * `groups` - A vector of same length as `dists`, with 1-based indices of group numbers. There
/// will generally be far fewer unique groups as there are entries in `dists`.
///
/// # Returns
///
/// A vector of mean distances within each group to the nearest points in the target distribution.
fn aggregate_to_groups_single_col(dists: &[f64], groups: &[usize]) -> Vec<f64> {
    let groups_out: Vec<_> = groups.to_vec();
    let max_group = *groups_out.iter().max().unwrap();
    let mut counts = vec![0u32; max_group + 1];
    let mut sums = vec![0f64; max_group + 1];

    for (i, &group) in groups_out.iter().enumerate() {
        counts[group] += 1;
        sums[group] += dists[i];
    }

    // Then convert sums to mean values by dividing by counts:
    for (sum, count) in sums.iter_mut().zip(&counts) {
        *sum = if *count != 0 {
            *sum / *count as f64
        } else {
            0.0
        };
    }

    // First value of `sums` is junk because `groups` are 1-based R values:
    sums.remove(0);

    sums
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uamutate() {
        // Define the input parameters for the function
        let filename1 = "./test_resources/dat1.json";
        let filename2 = "./test_resources/dat2.json";
        let varname = "bike_index";
        // let varextra: Vec<String> = Vec::new();
        let varextra = vec!["natural".to_string(), "social_index".to_string()];
        let nentries = 10;

        let varsall: Vec<String> = vec![varname.to_string()];
        let varsall = [varsall, varextra].concat();
        // let (mut values1, groups1) = read_write_file::readfile(filename1, &varsall, nentries);
        // let (values2, _groups2) = read_write_file::readfile(filename2, &varsall, nentries);

        // let sums = uamutate(&mut values1, groups1, &values2);
        let file1 = File::open(filename1).unwrap();
        let reader1 = BufReader::new(file1);
        let file2 = File::open(filename2).unwrap();
        let reader2 = BufReader::new(file2);
        let sums = uamutate(reader1, reader2, &varsall, nentries);

        assert!(!sums.is_empty());
    }
}
