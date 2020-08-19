extern crate plotly;
use plotly::common::Mode;
use plotly::{Plot, Scatter};

extern crate clap;
extern crate bdf_parser;

use bdf_parser::bdf_reader::*;

use std::collections::HashMap;

use clap::{App, Arg};

fn main() -> std::io::Result<()> {
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

    let first_channel = bdf_data.body.keys().next().unwrap();
    let first_channel_data = bdf_data.body.values().next().unwrap();
    let time = (0..first_channel_data.len()).collect::<Vec<usize>>();

    //downsample
    let decimated_fcd = first_channel_data.into_iter()
        .enumerate()
        .filter_map(|x| {
            if x.0 % 1000 == 0 {
                Some(x.1.clone())
            } else {
                None
            }
        }).collect::<Vec<i32>>();


    let trace = Scatter::new(time, decimated_fcd)
        .name(&first_channel)
        .mode(Mode::Lines);

    let mut plot = Plot::new();
    plot.add_trace(trace);
    plot.show();

    Ok(())
}
