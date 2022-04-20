use mars_raw_utils::prelude::*;
use sciimg::prelude::*;

#[macro_use]
extern crate clap;

use clap::{Arg, App};

use std::process;

fn main() {
    
    let matches = App::new(crate_name!())
                    .version(crate_version!())
                    .author(crate_authors!())
                    .arg(Arg::with_name(constants::param::PARAM_INPUTS)
                        .short(constants::param::PARAM_INPUTS_SHORT)
                        .long(constants::param::PARAM_INPUTS)
                        .value_name("INPUT")
                        .help("Input")
                        .required(true)
                        .multiple(true)
                        .takes_value(true))
                    .arg(Arg::with_name("output")
                        .short("o")
                        .long("output")
                        .value_name("OUTPUT")
                        .help("Output")
                        .required(true)
                        .takes_value(true))                        
                    .arg(Arg::with_name(constants::param::PARAM_VERBOSE)
                        .short(constants::param::PARAM_VERBOSE)
                        .help("Show verbose output"))
                    .get_matches_from(wild::args());

    if matches.is_present(constants::param::PARAM_VERBOSE) {
        print::set_verbose(true);
    }

    let output = matches.value_of("output").unwrap();

    let input_files: Vec<&str> = matches.values_of(constants::param::PARAM_INPUTS).unwrap().collect();

    let mut mean : RgbImage = RgbImage::new_empty().unwrap();
    let mut count : ImageBuffer = ImageBuffer::new_empty().unwrap();
    let mut ones : ImageBuffer = ImageBuffer::new_empty().unwrap();

    for in_file in input_files.iter() {
        if path::file_exists(in_file) {
            vprintln!("Processing File: {}", in_file);
            
            let raw = RgbImage::open(&String::from(*in_file)).unwrap();

            if mean.is_empty() {
                mean = raw;
                count = ImageBuffer::new(mean.width, mean.height).unwrap();
                ones = ImageBuffer::new_with_fill(mean.width, mean.height, 1.0).unwrap();
            } else {

                if raw.width != mean.width || raw.height != mean.height {
                    eprintln!("Input image has differing dimensions, cannot continue");
                    process::exit(1);
                }

                mean.add(&raw);
            }

            count = count.add(&ones).unwrap();
        } else {
            eprintln!("File not found: {}", in_file);
        }
    }

    if !mean.is_empty() {
        mean.divide_from_each(&count);

        if path::parent_exists_and_writable(output) {
            vprintln!("Writing image to {}", output);
            mean.save(output);
        } else {
            eprintln!("Unable to write output image, parent doesn't exist or is not writable");
        }

    } else {
        println!("No images processed, cannot create output");
    }

}


