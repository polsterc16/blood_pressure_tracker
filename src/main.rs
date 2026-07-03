#![allow(unused)]
use chrono::DateTime;
use chrono::Local;
use chrono::NaiveDateTime;
use chrono::TimeDelta;
use chrono::Utc;
use clap::Parser;
use serde::Deserialize;
use std::fmt;
use std::fs;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

// ################################################################

const CSV_HEADER: &str = "date,time,sys,dia,pul";
const SECS_IN_DAYS_F32: f32 = 86400_f32;

// ################################################################

type BpType = (f32, f32, f32);

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

/// Struct for importing entries from CSV file
#[derive(Debug, Deserialize, Clone)]
struct MeasCsv {
    date: String,
    time: String,
    sys: f32,
    dia: f32,
    pul: f32,
}
impl fmt::Display for MeasCsv {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{},{},{:.1},{:.1},{:.1}",
            self.date, self.time, self.sys, self.dia, self.pul
        )
    }
}
impl MeasCsv {
    /// Generates new 'MeasCsv' object for the given BP values at the current time
    pub fn new(sys: f32, dia: f32, pul: f32) -> MeasCsv {
        return MeasCsv {
            date: get_date(),
            time: get_time(),
            sys,
            dia,
            pul,
        };
    }
    /// Returns the BP as tuple (`sys`, `dia`, `pul`)
    pub fn get_bp(&self) -> BpType {
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
    /// Get `DateTime<Utc>` from `String` fields `date`, `time`
    pub fn get_datetime(&self) -> DateTime<Utc> {
        let dt_string = format!("{} {}", self.date, self.time);
        let dt = NaiveDateTime::parse_from_str(&dt_string, "%Y-%m-%d %H:%M:%S")
            .expect(&format!("Unable to read time '{dt_string}'!"))
            .and_utc();
        dt
    }
    /// Get `get_day_zero`, the day before the first day of the month
    pub fn get_day_zero(&self) -> DateTime<Utc> {
        // Create String for first day of current month
        let day_1_s = format!("{}-01 00:00:00", &self.date[..7]);

        // Get `DateTime` for first day of current month
        let day_1 = NaiveDateTime::parse_from_str(&day_1_s, "%F %T")
            .expect(&format!("Unable to read date '{}'!", day_1_s))
            .and_utc();

        // return `day_zero`
        day_1 - TimeDelta::days(1)
    }
    /// Create 'Meas2' object from this
    pub fn to_m2(&'_ self, day_zero: DateTime<Utc>, interval: u8) -> Meas2<'_> {
        Meas2::new(&self, day_zero, interval)
    }
}

/// Measurement-2 exists only as long as the ref MeasCsv
#[derive(Debug)]
struct Meas2<'a> {
    meas1: &'a MeasCsv,
    datetime: DateTime<Utc>,
    day_fine: f32,
    day_coarse: f32,
}
impl<'a> Meas2<'a> {
    pub fn new(meas1: &'a MeasCsv, day_zero: DateTime<Utc>, interval: u8) -> Meas2<'a> {
        let mut m2 = Meas2 {
            meas1,
            datetime: meas1.get_datetime(),
            day_fine: 0_f32,
            day_coarse: 0_f32,
        };
        m2.calc_day_fine(day_zero);
        m2.calc_day_coarse(interval);

        m2
    }
    /// Set field `day_fine`
    pub fn set_day_fine(&mut self, day_fine: f32) {
        self.day_fine = day_fine;
    }
    /// Calc (and set) field `day_fine`
    pub fn calc_day_fine(&mut self, day_zero: DateTime<Utc>) {
        let td: TimeDelta = *self.get_datetime() - day_zero;
        self.set_day_fine(td.num_seconds() as f32 / SECS_IN_DAYS_F32);
    }
    /// Get field `day_fine`
    pub fn get_day_fine(&self) -> f32 {
        self.day_fine
    }
    /// Set field `day_coarse`
    pub fn set_day_coarse(&mut self, day_coarse: f32) {
        self.day_coarse = day_coarse;
    }
    /// Calc (and set) field `day_coarse`
    pub fn calc_day_coarse(&mut self, interval: u8) {
        let mut day_coarse = (self.get_day_fine() * interval as f32).floor() / interval as f32;
        day_coarse += 24_f32 / (2 * interval) as f32;

        self.set_day_coarse(day_coarse);
    }
    /// Get field `day_coarse`
    pub fn get_day_coarse(&self) -> f32 {
        self.day_coarse
    }
    /// Returns the field `datetime`
    pub fn get_datetime(&self) -> &DateTime<Utc> {
        &self.datetime
    }
    /// Returns the BP as tuple (`sys`, `dia`, `pul`)
    pub fn get_bp(&self) -> BpType {
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
    vec_meas1: Vec<MeasCsv>,
}
impl CollectionMeas1 {
    pub fn new() -> CollectionMeas1 {
        CollectionMeas1 {
            vec_meas1: Vec::new(),
        }
    }
    pub fn new_with_capacity(capacity: usize) -> CollectionMeas1 {
        CollectionMeas1 {
            vec_meas1: Vec::with_capacity(capacity),
        }
    }
    /// Add (and consume) a `MeasCsv` object to vector field.
    pub fn add_meas1_consume(&mut self, meas_csv: MeasCsv) {
        self.vec_meas1.push(meas_csv);
    }
    /// Clear vector field.
    pub fn clear(&mut self) {
        self.vec_meas1.clear();
    }
    /// Get ref to vector field.
    pub fn get_ref(&self) -> &Vec<MeasCsv> {
        &self.vec_meas1
    }
    /// Sort collection vector by fields `date`, `time`
    pub fn sort(&mut self) {
        self.vec_meas1
            .sort_by(|a, b| a.date.cmp(&b.date).then(a.time.cmp(&b.time)));
    }
    /// Create 'Meas2 collection' object from this
    pub fn to_coll_m2(&'_ self, interval: u8) -> CollectionMeas2<'_> {
        CollectionMeas2::new(&self, interval)
    }
}

#[derive(Debug)]
struct CollectionMeas2<'a> {
    coll_meas1: &'a CollectionMeas1,
    vec_meas2: Vec<Meas2<'a>>,
    day_zero: DateTime<Utc>,
}
impl<'a> CollectionMeas2<'a> {
    pub fn new(coll_meas1: &'a CollectionMeas1, interval: u8) -> CollectionMeas2<'a> {
        let v_ref = coll_meas1.get_ref();
        let v_size = v_ref.len();
        if v_size == 0 {
            panic!("'coll_meas1' is empty!")
        }

        // Determine `day_zero` from first entry in 'CollectionMeas1'
        let day_zero = (&v_ref[0]).get_day_zero();

        // Create 'CollectionMeas2' object
        let mut coll: CollectionMeas2<'a> = CollectionMeas2 {
            coll_meas1,
            vec_meas2: Vec::with_capacity(v_size),
            day_zero,
        };

        // Populate 'vec_meas2' of 'CollectionMeas2' object
        for m_csv in v_ref {
            let m2 = m_csv.to_m2(day_zero, interval);
            coll.add_meas2_consume(m2);
        }

        return coll;
    }
    /// Add (and consume) a `Meas2` object to vector field.
    fn add_meas2_consume(&mut self, meas_2: Meas2<'a>) {
        self.vec_meas2.push(meas_2);
    }
    /// Get ref to vector field.
    pub fn get_ref(&'a self) -> &'a Vec<Meas2<'a>> {
        &self.vec_meas2
    }
    /// Sort collection vector by field `datetime`
    pub fn sort(&mut self) {
        self.vec_meas2.sort_by_key(|k| k.datetime);
    }
}

// ################################################################

#[derive(Debug, PartialEq)]
enum CsvOpenMode {
    Read,
    WriteReset,
    WriteAppend,
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
        let csv_collection = read_csv_content().expect("Unable to perform 'Read of CSV File'.");

        if cli.rebuild {
            worker_csv_rebuild(&csv_collection);
        }
        if cli.output {
            worker_output(&csv_collection);
        }
        if cli.status {
            worker_csv_status(&csv_collection);
        }
    }

    return;
}

/// Will read the CSV file, sort measurements and overwrite the file
fn worker_output(csv_collection: &CollectionMeas1) {
    let coll_m2 = csv_collection.to_coll_m2(2);

    let v_m2 = &coll_m2.vec_meas2;
    for idx in 0..20 {
        println!("[{idx}]\t{:?}", v_m2[idx]);
    }
}

/// Will read the CSV file, sort measurements and overwrite the file
fn worker_csv_rebuild(csv_collection: &CollectionMeas1) {
    let path_string = get_file_path_string();
    // Open CSV File and reset content
    let fh_csv = open_csv_file(&path_string, CsvOpenMode::WriteReset);

    // println!("Sorted entries:");
    // for entry in &csv_entries {
    for (index, entry) in csv_collection.get_ref().iter().enumerate() {
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

fn worker_csv_status(csv_collection: &CollectionMeas1) {
    let csv_entries = csv_collection.get_ref();
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

    let measurement = MeasCsv::new(sys, dia, pul);

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

fn read_csv_content() -> Result<CollectionMeas1, Box<dyn std::error::Error>> {
    let path_string = get_file_path_string();

    let fh_csv = open_csv_file(&path_string, CsvOpenMode::Read);
    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(b',')
        .from_reader(fh_csv);

    // let records_iter = rdr.deserialize();

    let records: Vec<Result<MeasCsv, csv::Error>> = rdr.deserialize().collect();
    // let mut ret: Vec<MeasCsv> = Vec::with_capacity(records.len());
    let mut coll = CollectionMeas1::new_with_capacity(records.len());

    for result in records {
        let entry: MeasCsv = result?;
        // println!("{:?}", entry);
        // ret.push(entry);
        coll.add_meas1_consume(entry);
    }

    // // Sort vector of Measurement by date, time
    // ret.sort_by(|a, b| a.date.cmp(&b.date).then(a.time.cmp(&b.time)));
    coll.sort();

    Ok(coll)
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
