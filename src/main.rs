#![allow(unused)]
use chrono::Local;
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
    #[command(flatten)]
    bp: BlPrArgs,

    /// Generate output
    #[arg(short, long)]
    output: bool,

    /// Display status
    #[arg(short, long)]
    status: bool,

    /// Rebuild CSV file
    #[arg(short, long)]
    rebuild: bool,
}
impl Cli {
    /// Returns true, if at least one option is set that requires reading the CSV file content.
    fn qry_read_csv(&self) -> bool {
        return self.output || self.status || self.rebuild;
    }
}

#[derive(Debug, Args)]
#[group(multiple = true)]
struct BlPrArgs {
    /// Systolic blood pressure, mmHg
    #[arg(requires_all=["dia","pul"])]
    sys: Option<f32>,

    /// Diastolic blood pressure, mmHg
    #[arg(requires_all=["sys","pul"])]
    dia: Option<f32>,

    /// Heart rate, #pulses/min
    #[arg(requires_all=["sys","dia"])]
    pul: Option<f32>,
}
impl BlPrArgs {
    fn check_state(self: &Self) -> BlPrArgState {
        if self.sys.is_none() && self.dia.is_none() && self.pul.is_none() {
            return BlPrArgState::Empty;
        } else if self.sys.is_some() && self.dia.is_some() && self.pul.is_some() {
            return BlPrArgState::Valid;
        } else {
            return BlPrArgState::Invalid;
        };
    }
    fn to_measurement(&self) -> Measurement {
        match self.check_state() {
            BlPrArgState::Empty => {
                panic!("Attempting to create Measurement from empty Blood Pressure args!")
            }
            BlPrArgState::Invalid => {
                panic!("Attempting to create Measurement from invalid Blood Pressure args!")
            }
            BlPrArgState::Valid => (),
        }
        let sys = self.sys.unwrap();
        let dia = self.dia.unwrap();
        let pul = self.pul.unwrap();
        return Measurement::new(sys, dia, pul);
    }
}

#[derive(Debug, PartialEq)]
enum BlPrArgState {
    Empty,
    Valid,
    Invalid,
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
    pub fn get_date(&self) -> &str {
        &self.date[..]
    }
    pub fn get_time(&self) -> &str {
        &self.time[..]
    }
    pub fn get_csv_entry(&self) -> String {
        format!(
            "{},{},{:.1},{:.1},{:.1}",
            self.date, self.time, self.sys, self.dia, self.pul
        )
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
    println!("CLI: {:?}\n", cli);

    worker_init_csv();

    worker_bp_add(&cli);

    if cli.qry_read_csv() {
        let mut csv_entries = read_csv_content().expect("Unable to perform 'Read of CSV File'.");

        if cli.rebuild {
            // worker_csv_rebuild();

            csv_entries.sort_by(|a, b| a.date.cmp(&b.date).then(a.time.cmp(&b.time)));

            let path_string = get_file_path_string();
            let fh_csv = open_csv_file(&path_string, CsvOpenMode::WriteReset);

            println!("Sorted entries:");
            // for entry in &csv_entries {
            for (index, entry) in (&csv_entries).iter().enumerate() {
                let csv_line = entry.get_csv_entry();
                println!("[{}] {:?}", index, csv_line);

                writeln!(&fh_csv, "{}", csv_line).expect(&format!(
                    "Could not write to File '{path_string}': Entry [{index}] '{csv_line}'."
                ));
            }

            return;
        }
        if cli.output {
            // worker_pdf_output();
        }
        if cli.status {
            worker_csv_status(&csv_entries);
        }
    }

    return;
}

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

fn worker_csv_status(csv_entries: &Vec<Measurement>) {
    let date_ym = get_date_ym();
    // let entries = read_csv_content().expect("Unable to perform 'Read of CSV File'.");

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

fn worker_bp_add(cli: &Cli) {
    let bp = &cli.bp;

    match bp.check_state() {
        BlPrArgState::Valid => (),
        BlPrArgState::Empty => return,
        BlPrArgState::Invalid => panic!("Invalid Blood Pressure args!"),
    }
    let measurement = bp.to_measurement();

    let path_string = get_file_path_string();
    let path_file = Path::new(&path_string);

    // Open CSV file in 'append' mode
    let fh_csv = OpenOptions::new()
        .write(true)
        .append(true)
        .open(path_file)
        .expect(&format!("Unable to open File '{}'.", path_string));

    // Append entry to CSV file
    if let Err(e) = writeln!(&fh_csv, "{}", measurement.get_csv_entry()) {
        eprintln!("Couldn't write to file: {}", e);
    }
}

fn worker_init_csv() {
    check_path().expect("Unable to perform 'Check of /data Directory'.");
    check_file().expect("Unable to perform 'Check of work File'.");
}

fn read_csv_content() -> Result<Vec<Measurement>, Box<dyn std::error::Error>> {
    let path_string = get_file_path_string();
    let path_file = Path::new(&path_string);

    let fh = File::open(path_file)?;
    let mut rdr = csv::ReaderBuilder::new().delimiter(b',').from_reader(fh);

    // let records_iter = rdr.deserialize();

    let records: Vec<Result<Measurement, csv::Error>> = rdr.deserialize().collect();
    let mut ret: Vec<Measurement> = Vec::with_capacity(records.len());

    for result in records {
        let entry: Measurement = result?;
        // println!("{:?}", entry);
        ret.push(entry);
    }

    Ok(ret)
}

fn check_file() -> Result<(), std::io::Error> {
    let path_string = get_file_path_string();
    let path_file = Path::new(&path_string);
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
                    path_string
                );
            }
        }
        log_message(&format!("Empty File '{}' missing csv header.", path_string));

        fh = File::create(path_file)?;
    } else {
        log_warning(&format!("File '{}' missing.", path_file.display()));

        fh = File::create_new(path_file)?;
        log_message(&format!("Empty File '{}' created.", path_string));
    }

    if let Err(e) = writeln!(&fh, "{}", CSV_HEADER) {
        eprintln!("Could not write to File: {}", e);
    }

    fh.sync_all()?;
    log_message(&format!("Csv header added to File '{}'.", path_string));

    Ok(())
}

fn check_path() -> std::io::Result<()> {
    let path_string = get_dir_path_string();
    let path_dir = Path::new(&path_string);

    if path_dir.exists() {
        return Ok(());
    }
    log_warning(&format!("Directory '{}' missing.", path_string));

    match fs::create_dir(path_dir) {
        Ok(_) => {
            log_message(&format!("Directory '{}' created.", path_string));
            Ok(())
        }
        Err(e) => {
            log_error(&format!("Unable to create Directory '{}'.", path_string));
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
