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
use std::fmt;
// use std::env;
// use std::error::Error;
use std::fs;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

// ################################################################

const CSV_HEADER: &str = "date,time,sys,dia,pul";
const SECS_IN_DAYS: f32 = 86400_f32;

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

#[derive(Debug, Deserialize, Clone)]
struct Measurement {
    date: String,
    time: String,
    sys: f32,
    dia: f32,
    pul: f32,
}
impl fmt::Display for Measurement {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{},{},{:.1},{:.1},{:.1}",
            self.date, self.time, self.sys, self.dia, self.pul
        )
    }
}
impl Measurement {
    /// Generates new 'Measurement' object for the given BP values at the current time
    pub fn new(sys: f32, dia: f32, pul: f32) -> Measurement {
        return Measurement {
            date: get_date(),
            time: get_time(),
            sys,
            dia,
            pul,
        };
    }
    /// Returns the BP as tuple (`sys`, `dia`, `pul`)
    pub fn get_bp(&self) -> (f32, f32, f32) {
        (self.sys, self.dia, self.pul)
    }
    /// Returns the BP field `sys`
    pub fn get_bp_sys(&self) -> f32 {
        self.sys
    }
    /// Returns the BP field `dia`
    pub fn get_bp_dia(&self) -> f32 {
        self.dia
    }
    /// Returns the BP field `pul`
    pub fn get_bp_pul(&self) -> f32 {
        self.pul
    }
    /// Returns the `date` field
    pub fn get_date_str(&self) -> &str {
        &self.date[..]
    }
    /// Returns the `time` field
    pub fn get_time_str(&self) -> &str {
        &self.time[..]
    }
    /// Returns the `String` representation of the 'Measurement' object
    // pub fn get_csv_entry_string(&self) -> String {
    //     format!(
    //         "{},{},{:.1},{:.1},{:.1}",
    //         self.date, self.time, self.sys, self.dia, self.pul
    //     )
    // }
    pub fn get_datetime(&self) -> DateTime<Utc> {
        let dt_string = format!("{} {}", self.date, self.time);
        let dt = NaiveDateTime::parse_from_str(&dt_string, "%Y-%m-%d %H:%M:%S")
            .expect(&format!("Unable to read time '{dt_string}'!"))
            .and_utc();
        dt
    }
    /// Create 'Meas2' object from this
    pub fn to_m2(&'_ self) -> Meas2<'_> {
        Meas2::new(&self)
    }
}

/// Measurement-2 exists only as long as the ref Measurement-1
#[derive(Debug)]
struct Meas2<'a> {
    meas1: &'a Measurement,
    datetime: DateTime<Utc>,
    day_fine: Option<f32>,
    day_coarse: Option<f32>,
}
impl<'a> Meas2<'a> {
    pub fn new(meas1: &'a Measurement) -> Meas2<'a> {
        Meas2 {
            meas1,
            datetime: meas1.get_datetime(),
            day_fine: None,
            day_coarse: None,
        }
    }
    /// Set `day_float` with arg as either
    /// - `f32`
    /// - `Option<f32>`
    pub fn set_day_fine<T: ArgF32OrOptionTrait>(&mut self, day_fine: T) {
        match day_fine.to_f32_or_option() {
            ArgF32OrOption::F32(day_fine) => {
                self.day_fine = Some(day_fine);
            }
            ArgF32OrOption::Option(day_fine_option) => {
                self.day_fine = day_fine_option;
            }
        }
    }
    /// Get `day_fine`
    pub fn get_day_fine(&self) -> Option<f32> {
        self.day_fine
    }
    /// Try to get `day_fine` as `f32`.
    /// # Panic
    /// Panics if `day_fine` is `None`.
    pub fn get_day_fine_try(&self) -> f32 {
        match self.day_fine {
            Some(d_float) => return d_float,
            None => panic!("Cannot return `day_fine` as `f32` (field `day_fine` is `None`)!"),
        }
    }
    /// Set `day_coarse` with arg as either
    /// - `f32`
    /// - `Option<f32>`
    pub fn set_day_coarse<T: ArgF32OrOptionTrait>(&mut self, day_coarse: T) {
        match day_coarse.to_f32_or_option() {
            ArgF32OrOption::F32(day_coarse) => {
                self.day_coarse = Some(day_coarse);
            }
            ArgF32OrOption::Option(day_coarse_option) => {
                self.day_coarse = day_coarse_option;
            }
        }
    }
    /// Get `day_coarse`
    pub fn get_day_coarse(&self) -> Option<f32> {
        self.day_coarse
    }
    /// Try to get `day_coarse` as `f32`.
    /// # Panic
    /// Panics if `day_coarse` is `None`.
    pub fn get_day_coarse_try(&self) -> f32 {
        match self.day_coarse {
            Some(d_coarse) => return d_coarse,
            None => panic!("Cannot return `day_coarse` as `f32` (field `day_coarse` is `None`)!"),
        }
    }
    /// Try to calculate `day_coarse` from `day_fine` for given arg `interval` (must be greater than 0).
    /// # Panic
    /// Panics if
    /// - `interval` is `0`
    /// - `day_fine` is `None`
    pub fn calc_day_coarse_try(&mut self, interval: u8) {
        match self.day_fine {
            Some(d_fine) => {
                if interval == 0 {
                    panic!("Argument `interval` must be greater than 0!")
                }
                let d_coarse = (d_fine * interval as f32).floor() / interval as f32;
                self.set_day_coarse(d_coarse);
            }
            None => panic!("Cannot calculate `day_coarse` (field `day_fine` is `None`)!"),
        }
    }
    // /// Calculate TimeDelta and set `day_float`
    // pub fn calc_td(&mut self, datetime: &DateTime<Utc>) {
    //     let td: TimeDelta = self.datetime - datetime;
    //     self.set_day_float(td.as_seconds_f32() / (86400.0f32)); // seconds to days
    // }
    /// Returns the field `datetime`
    pub fn get_datetime(&self) -> &DateTime<Utc> {
        &self.datetime
    }
    /// Returns the BP as tuple (`sys`, `dia`, `pul`)
    pub fn get_bp(&self) -> (f32, f32, f32) {
        self.meas1.get_bp()
    }
    /// Returns the BP field `sys`
    pub fn get_bp_sys(&self) -> f32 {
        self.meas1.get_bp_sys()
    }
    /// Returns the BP field `dia`
    pub fn get_bp_dia(&self) -> f32 {
        self.meas1.get_bp_dia()
    }
    /// Returns the BP field `pul`
    pub fn get_bp_pul(&self) -> f32 {
        self.meas1.get_bp_pul()
    }
}

#[derive(Debug)]
struct CollectionMeas1 {
    vec_meas1: Vec<Measurement>,
}
impl CollectionMeas1 {
    /// Add (and consume) a `Measurement` object to vector field.
    pub fn add_meas_consume(&self, m: Measurement) {}

    /// Sort collection vector by fields `date`, `time`
    pub fn sort(&mut self) {
        self.vec_meas1
            .sort_by(|a, b| a.date.cmp(&b.date).then(a.time.cmp(&b.time)));
    }
}

#[derive(Debug)]
struct CollectionMeas2<'a> {
    coll_meas1: &'a CollectionMeas1,
    vec_meas2: Vec<Meas2<'a>>,
    day_zero: Option<DateTime<Utc>>,
}
impl<'a> CollectionMeas2<'a> {
    pub fn add_meas_ref(m: Meas2) {}
}

// ################################################################

#[derive(Debug, PartialEq)]
enum CsvOpenMode {
    Read,
    WriteReset,
    WriteAppend,
}

/// Helper Enum for overloading a method for
/// - arg as `f32`
/// - arg as `Option<f32>`
#[derive(Debug)]
enum ArgF32OrOption {
    F32(f32),
    Option(Option<f32>),
}
trait ArgF32OrOptionTrait {
    fn to_f32_or_option(&self) -> ArgF32OrOption;
}
impl ArgF32OrOptionTrait for f32 {
    fn to_f32_or_option(&self) -> ArgF32OrOption {
        ArgF32OrOption::F32(*self)
    }
}
impl ArgF32OrOptionTrait for Option<f32> {
    fn to_f32_or_option(&self) -> ArgF32OrOption {
        ArgF32OrOption::Option(*self)
    }
}

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
        // let csv_line = entry.get_csv_entry_string();
        // println!("[{}] {:?}", index, csv_line);

        writeln!(&fh_csv, "{}", entry).expect(&format!(
            "Could not write to File '{path_string}': Entry [{index}] '{entry}'."
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
    if let Err(e) = writeln!(&fh_csv, "{}", measurement) {
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
