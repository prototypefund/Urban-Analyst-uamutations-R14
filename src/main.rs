//! This is a stand-alone crate which implements the mutation algorithm for [Urban
//! Analyst](https://urbananalyst.city). The algorithm mutates selected properties for one city to
//! become more like those of another selected city.

extern crate uamutations;

const NENTRIES: usize = 1000;

const FNAME1: &str = "./test_resources/dat1.json";
const FNAME2: &str = "./test_resources/dat2.json";
const VARNAME: &str = "bike_index";
const OUTFILENAME: &str = "output.txt";

/// Entry point for the Urban Analyst mutation algorithm.
///
/// This exists only to locally call and run the library.
fn main() {
    uamutations::uamutate(FNAME1, FNAME2, VARNAME, NENTRIES, OUTFILENAME);
}
