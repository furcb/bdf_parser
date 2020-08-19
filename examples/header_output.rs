extern crate clap;
extern crate bdf_parser;

use bdf_parser::bdf_reader::*;

use std::collections::HashMap;

use clap::{App, Arg};

fn main() {

    let matches = App::new("bdf_parser")
        .version("0.0")
        .author("furcb")
        .about("EEG BDF file parser")
        .arg(Arg::with_name("PATH")
            .help("path to bdf file")
            .required(true))
        .get_matches();

    // Get file and set seek head to first byte
    let bdf_file_path = matches.value_of("PATH").unwrap();

    let bdf_data = BDF::parse(bdf_file_path).unwrap();

    println!("{:#?}", bdf_data.header.file_metadata)
}
