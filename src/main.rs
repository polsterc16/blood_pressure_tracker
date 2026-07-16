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

    let mut fw_bp = BpFileWarden::new(&file_name_now, dir_data_str, dir_output_str);
    fw_bp.csv_data.check_create_directory();
    fw_bp.json_output.check_create_directory();

    // let csv_data_worker = &mut fw_bp.csv_data;
    // let json_output_worker = &mut fw_bp.json_output;

    fw_bp.csv_data.check_file().unwrap();

    // let json_worker = FileWardenJson::new_option(Some("output"), None);
    // let x = json_worker.get_dir_content();
    // println!("Content: {:?}", x);

    if let Some(bp) = cli.add {
        worker_bp_add(&mut fw_bp, &bp);
    }

    if cli.rebuild || cli.status || cli.output {
        // let csv_collection = read_csv_content().expect("Unable to perform 'Read of CSV File'.");
        let csv_collection = fw_bp.csv_data.get_csv_content().unwrap();

        if cli.rebuild {
            worker_csv_rebuild(&mut fw_bp, &csv_collection);
        }
        if cli.output {
            worker_output(&mut fw_bp, &csv_collection);
        }
        if cli.status {
            worker_csv_status(&mut fw_bp, &csv_collection);
        }
    }

    return;
}

/// Will read the CSV file, sort measurements and overwrite the file
fn worker_output(fw_bp: &mut BpFileWarden, csv_collection: &CollectionCsv) {
    let coll_m2 = csv_collection.to_coll_m2(2);

    let coll_month = CollectionMonth::from_coll_m2_consume(coll_m2);

    let mut out_month = OutputMonth::new(&coll_month, &fw_bp.json_output);
    let json_out = serde_json::to_string_pretty(&out_month).unwrap();

    let fw_json_output = &mut fw_bp.json_output;
    let fh_json = fw_json_output.file_open(&FileOpenMode::Write).unwrap();
    writeln!(&fh_json, "{}", json_out)
        .context("Could not write to file")
        .unwrap();
}

/// Will read the CSV file, sort measurements and overwrite the file
fn worker_csv_rebuild(fw_bp: &mut BpFileWarden, csv_collection: &CollectionCsv) {
    let fw_csv_data = &mut fw_bp.csv_data;
    // Open CSV File and reset content
    let fh_csv = fw_csv_data.file_open(&FileOpenMode::Write).unwrap();

    for entry in csv_collection.get_ref() {
        writeln!(&fh_csv, "{}", entry)
            .context("Could not write to file")
            .unwrap();
    }

    // Save changes to disk
    fh_csv.sync_all().context("Unable to save File .").unwrap();
}

fn worker_csv_status(fw_bp: &mut BpFileWarden, csv_collection: &CollectionCsv) {
    let fw_csv_data = &mut fw_bp.csv_data;
    let csv_entries = csv_collection.get_ref();
    let date_ym = fw_csv_data.get_file_name();

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

fn worker_bp_add(fw_bp: &mut BpFileWarden, bp: &Vec<f32>) {
    let fw_csv_data = &mut fw_bp.csv_data;
    let sys = bp[0];
    let dia = bp[1];
    let pul = bp[2];

    let measurement = MeasCsv::new(sys, dia, pul);

    // Open CSV file in 'append' mode
    let fh_csv = fw_csv_data.file_open(&FileOpenMode::Append).unwrap();

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
