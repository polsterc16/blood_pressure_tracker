#![allow(unused)]
use chrono::DateTime;
use chrono::Datelike;
use chrono::Local;
use chrono::NaiveDateTime;
use chrono::TimeDelta;
use chrono::Timelike;
use chrono::Utc;
use clap::{Args, Parser};
use serde::Deserialize;
// use std::env;
// use std::error::Error;
use std::fs;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

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

#[derive(Debug, Deserialize)]
struct Measurement {
    date: String,
    time: String,
    sys: f32,
    dia: f32,
    pul: f32,
}
impl Measurement {
    pub fn new(sys: f32, dia: f32, pul: f32) -> Measurement {
        return Measurement {
            date: get_date(),
            time: get_time(),
            sys,
            dia,
            pul,
        };
    }

    pub fn get_bp(&self) -> (f32, f32, f32) {
        (self.sys, self.dia, self.pul)
    }
    pub fn get_bp_sys(&self) -> f32 {
        self.sys
    }
    pub fn get_bp_dia(&self) -> f32 {
        self.dia
    }
    pub fn get_bp_pul(&self) -> f32 {
        self.pul
    }
    pub fn get_date_str(&self) -> &str {
        &self.date[..]
    }
    pub fn get_time_str(&self) -> &str {
        &self.time[..]
    }
    pub fn get_csv_entry_string(&self) -> String {
        format!(
            "{},{},{:.1},{:.1},{:.1}",
            self.date, self.time, self.sys, self.dia, self.pul
        )
    }
    pub fn get_datetime(&self) -> DateTime<Utc> {
        let dt_string = format!("{} {}", self.date, self.time);
        let dt = NaiveDateTime::parse_from_str(&dt_string, "%Y-%m-%d %H:%M:%S")
            .expect(&format!("Unable to read time '{dt_string}'!"))
            .and_utc();
        dt
    }
}

#[derive(Debug, PartialEq)]
enum CsvOpenMode {
    Read,
    WriteReset,
    WriteAppend,
}

// ################################################################

const CSV_HEADER: &str = "date,time,sys,dia,pul";

// ################################################################
// ################################################################
fn main() {
    let cli = Cli::parse();
    println!("CLI: {:?}\n", &cli);

    worker_init_csv();

    match cli.add {
        Some(bp) => worker_bp_add(&bp),
        _ => (),
    }

    if cli.rebuild || cli.status || cli.output {
        let csv_entries = read_csv_content().expect("Unable to perform 'Read of CSV File'.");

        if cli.rebuild {
            worker_csv_rebuild(&csv_entries);
        }
        if cli.output {
            worker_output(&csv_entries);
        }
        if cli.status {
            worker_csv_status(&csv_entries);
        }
    }

    return;
}

/// Will read the CSV file, sort measurements and overwrite the file
fn worker_output(csv_entries: &Vec<Measurement>) {
    let interval: f32 = 12.0;

    let e_first = &csv_entries[0];
    let e_last = &csv_entries[csv_entries.len() - 1];
}

/// Will read the CSV file, sort measurements and overwrite the file
fn worker_csv_rebuild(csv_entries: &Vec<Measurement>) {
    let path_string = get_file_path_string();
    // Open CSV File and reset content
    let fh_csv = open_csv_file(&path_string, CsvOpenMode::WriteReset);

    // println!("Sorted entries:");
    // for entry in &csv_entries {
    for (index, entry) in (csv_entries).iter().enumerate() {
        let csv_line = entry.get_csv_entry_string();
        // println!("[{}] {:?}", index, csv_line);

        writeln!(&fh_csv, "{}", csv_line).expect(&format!(
            "Could not write to File '{path_string}': Entry [{index}] '{csv_line}'."
        ));
    }

    // Save changes to disk
    fh_csv
        .sync_all()
        .expect(&format!("Unable to save File '{path_string}'."));
}

fn worker_csv_status(csv_entries: &Vec<Measurement>) {
    let date_ym = get_date_ym();

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

fn worker_bp_add(bp: &Vec<f32>) {
    let sys = bp[0];
    let dia = bp[1];
    let pul = bp[2];

    let measurement = Measurement::new(sys, dia, pul);

    let path_string = get_file_path_string();

    // Open CSV file in 'append' mode
    let fh_csv = open_csv_file(&path_string, CsvOpenMode::WriteAppend);

    // Append entry to CSV file
    if let Err(e) = writeln!(&fh_csv, "{}", measurement.get_csv_entry_string()) {
        eprintln!("Couldn't write to file: {}", e);
    }

    // Save changes to disk
    fh_csv
        .sync_all()
        .expect(&format!("Unable to save File '{path_string}'."));
}

fn worker_init_csv() {
    let path_dir_string = get_dir_path_string();
    let path_file_string = get_file_path_string();

    check_path(&path_dir_string).expect(&format!(
        "Unable to perform 'Check of Directory {path_dir_string}'."
    ));
    check_file(&path_file_string).expect(&format!(
        "Unable to perform 'Check of work File {path_file_string}'."
    ));
}

// ################################################################

fn open_csv_file(path_str: &str, mode: CsvOpenMode) -> File {
    let path_file = Path::new(path_str);

    let fh_csv: File;
    match mode {
        CsvOpenMode::Read => {
            fh_csv = OpenOptions::new()
                .read(true)
                .open(path_file)
                .expect(&format!(
                    "Unable to open File '{}' in {:?}.",
                    path_str, mode
                ));
        }
        CsvOpenMode::WriteReset => {
            fh_csv = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(path_file)
                .expect(&format!(
                    "Unable to open File '{}' in {:?}.",
                    path_str, mode
                ));

            // Write CSV header line
            writeln!(&fh_csv, "{}", CSV_HEADER).expect(&format!(
                "Could not write to File '{}' in {:?}.",
                path_str, mode
            ));
        }
        CsvOpenMode::WriteAppend => {
            fh_csv = OpenOptions::new()
                .write(true)
                .append(true)
                .open(path_file)
                .expect(&format!(
                    "Unable to open File '{}' in {:?}.",
                    path_str, mode
                ));
        }
    }
    fh_csv
}

fn read_csv_content() -> Result<Vec<Measurement>, Box<dyn std::error::Error>> {
    let path_string = get_file_path_string();

    let fh_csv = open_csv_file(&path_string, CsvOpenMode::Read);
    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(b',')
        .from_reader(fh_csv);

    // let records_iter = rdr.deserialize();

    let records: Vec<Result<Measurement, csv::Error>> = rdr.deserialize().collect();
    let mut ret: Vec<Measurement> = Vec::with_capacity(records.len());

    for result in records {
        let entry: Measurement = result?;
        // println!("{:?}", entry);
        ret.push(entry);
    }

    // Sort vector of Measurement by date, time
    ret.sort_by(|a, b| a.date.cmp(&b.date).then(a.time.cmp(&b.time)));

    Ok(ret)
}

fn check_file(path_file_str: &str) -> Result<(), std::io::Error> {
    // let path_string = get_file_path_string();
    let path_file = Path::new(path_file_str);
    let fh: File;

    if path_file.exists() {
        let file_meta = fs::metadata(path_file).expect("unable to read metadata");
        // println!("metadata len: {:?}", file_meta.len());

        if file_meta.len() > 0 {
            let f_read = File::open(path_file)?;
            let reader = BufReader::new(f_read);

            let mut lines = reader.lines();
            let line = lines
                .next()
                .expect("Unable to read first Line of File")
                .expect("Unable to read first Line of File");

            if CSV_HEADER == &line[..] {
                return Ok(());
            } else {
                panic!(
                    "File '{}' has content, but is missing csv header!",
                    path_file_str
                );
            }
        }
        log_message(&format!(
            "Empty File '{}' missing csv header.",
            path_file_str
        ));

        fh = File::create(path_file)?;
    } else {
        log_warning(&format!("File '{}' missing.", path_file_str));

        fh = File::create_new(path_file)?;
        log_message(&format!("Empty File '{}' created.", path_file_str));
    }

    if let Err(e) = writeln!(&fh, "{}", CSV_HEADER) {
        eprintln!("Could not write to File: {}", e);
    }

    fh.sync_all()?;
    log_message(&format!("Csv header added to File '{}'.", path_file_str));

    Ok(())
}

fn check_path(path_dir_str: &str) -> std::io::Result<()> {
    let path_dir = Path::new(path_dir_str);

    if path_dir.exists() {
        return Ok(());
    }
    log_warning(&format!("Directory '{}' missing.", path_dir_str));

    match fs::create_dir(path_dir) {
        Ok(_) => {
            log_message(&format!("Directory '{}' created.", path_dir_str));
            Ok(())
        }
        Err(e) => {
            log_error(&format!("Unable to create Directory '{}'.", path_dir_str));
            Err(e)
        }
    }
}

fn log_message(msg: &str) {
    println!("[MESSAGE]\t{msg}");
}
fn log_warning(wrn: &str) {
    println!("[WARNING]\t{wrn}");
}
fn log_error(err: &str) {
    println!("[ERROR]\t{err}");
}

fn get_date() -> String {
    Local::now().format("%Y-%m-%d").to_string()
}
fn get_date_ym() -> String {
    Local::now().format("%Y-%m").to_string()
}
fn get_time() -> String {
    Local::now().format("%H:%M:%S").to_string()
}

fn get_file_path_string() -> String {
    format!("./data/{}.csv", get_date_ym())
}
fn get_dir_path_string() -> String {
    format!("./data")
}
