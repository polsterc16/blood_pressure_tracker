#![allow(unused)]
#![allow(unused_labels)]
use anyhow::{Context, bail};
use chrono::Local;
use clap::Parser;
use pretty_simple_display::DebugPretty;
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

mod bp_container;
mod file_warden;
mod time_tools;

use bp_container::*;
use file_warden::*;
use time_tools::*;

// ################################################################

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Add blood pressure measurement
    #[arg(short, long, num_args = 3, value_names=["sys","dia","pul"])]
    add: Option<Vec<f32>>,

    /// Generate Output
    #[arg(short, long)]
    output: bool,

    /// Display status of CSV file
    #[arg(short, long)]
    status: bool,

    /// Rebuild CSV file
    #[arg(short, long)]
    rebuild: bool,
}

// ################################################################
// ################################################################
fn main() {
    let cli = Cli::parse();
    // println!("CLI: {:?}\n", &cli);

    let dir_data_str = "data";
    let dir_output_str = "output";

    let dym_now = DateYearMonth::from_now();
    let file_name_now = dym_now.to_string();

    let fw_data = FileWarden::new_option(Some(dir_data_str), Some(&file_name_now), None);
    let fw_output = FileWarden::new_option(Some(dir_output_str), Some(&file_name_now), None);
    fw_data.check_create_directory();
    fw_output.check_create_directory();

    let mut csv_worker = FileWardenCsv::from_file_warden(&fw_data);
    csv_worker.check_file().unwrap();

    // let json_worker = FileWardenJson::new_option(Some("output"), None);
    // let x = json_worker.get_dir_content();
    // println!("Content: {:?}", x);

    if let Some(bp) = cli.add {
        worker_bp_add(&mut csv_worker, &bp);
    }

    if cli.rebuild || cli.status || cli.output {
        // let csv_collection = read_csv_content().expect("Unable to perform 'Read of CSV File'.");
        let csv_collection = csv_worker.get_csv_content().unwrap();

        if cli.rebuild {
            worker_csv_rebuild(&mut csv_worker, &csv_collection);
        }
        if cli.output {
            worker_output(&csv_worker, &csv_collection);
        }
        if cli.status {
            worker_csv_status(&csv_worker, &csv_collection);
        }
    }

    return;
}

/// Will read the CSV file, sort measurements and overwrite the file
fn worker_output(csv_worker: &FileWardenCsv, csv_collection: &CollectionCsv) {
    let coll_m2 = csv_collection.to_coll_m2(2);

    let coll_month = CollectionMonth::from_coll_m2_consume(coll_m2);

    let mut out_month = OutputMonth::new(&coll_month);
    out_month.set_name(&csv_worker.get_file_name());
    // println!("{out_month:?}");

    // println!("{coll_month:?}");
    // if true {
    //     return;
    // }
    // println!("Attempt json export");
    // let pretty = serde_json::to_string_pretty(&coll_month).unwrap(); // pretty-printed
    // println!("{pretty}");
}

/// Will read the CSV file, sort measurements and overwrite the file
fn worker_csv_rebuild(csv_worker: &mut FileWardenCsv, csv_collection: &CollectionCsv) {
    // Open CSV File and reset content
    let fh_csv = csv_worker.file_open(&FileOpenMode::Write).unwrap();

    for entry in csv_collection.get_ref() {
        writeln!(&fh_csv, "{}", entry)
            .context("Could not write to file")
            .unwrap();
    }

    // Save changes to disk
    fh_csv.sync_all().context("Unable to save File .").unwrap();
}

fn worker_csv_status(csv_worker: &FileWardenCsv, csv_collection: &CollectionCsv) {
    let csv_entries = csv_collection.get_ref();
    let date_ym = csv_worker.get_file_name();

    let csv_len = csv_entries.len();
    println!("File for '{date_ym}' contains {csv_len} entries.");

    if csv_len > 0 {
        let csv_tail_range = if csv_len <= 5 {
            0..csv_len
        } else {
            (csv_len - 5)..csv_len
        };

        println!("Latest entries:");
        for i in csv_tail_range {
            println!("[{}] {:?}", i + 1, csv_entries[i]);
        }
    }
}

fn worker_bp_add(csv_worker: &mut FileWardenCsv, bp: &Vec<f32>) {
    let sys = bp[0];
    let dia = bp[1];
    let pul = bp[2];

    let measurement = MeasCsv::new(sys, dia, pul);

    // Open CSV file in 'append' mode
    let fh_csv = csv_worker.file_open(&FileOpenMode::Append).unwrap();

    // Append entry to CSV file
    writeln!(&fh_csv, "{}", measurement)
        .context("Could not write to file")
        .unwrap();

    // Save changes to disk
    fh_csv.sync_all().context("Unable to save File .").unwrap();
}

// fn worker_init_csv() {
//     let path_dir_string = get_dir_path_string();
//     let path_file_string = get_file_path_string();
//
//     check_path(&path_dir_string).expect(&format!(
//         "Unable to perform 'Check of Directory {path_dir_string}'."
//     ));
//     check_file(&path_file_string).expect(&format!(
//         "Unable to perform 'Check of work File {path_file_string}'."
//     ));
// }

// fn log_message(msg: &str) {
//     println!("[MESSAGE]\t{msg}");
// }
// fn log_warning(wrn: &str) {
//     println!("[WARNING]\t{wrn}");
// }
// fn log_error(err: &str) {
//     println!("[ERROR]\t{err}");
// }

// ################################################################
