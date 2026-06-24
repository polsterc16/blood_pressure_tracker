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

#[derive(PartialEq)]
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

    // pub fn get_measurement(&self) -> (f32, f32, f32) {
    //     (self.sys, self.dia, self.pul)
    // }
}

// ################################################################
// ################################################################
fn main() {
    let cli = Cli::parse();
    println!("CLI: {:?}\n", cli);

    // let bp = &cli.bp;
    // let show_status = &cli.status;
    // let generate_output = &cli.output;

    worker_init_csv();

    worker_bp_add(&cli);

    if cli.output {
        // worker_pdf_output();
    } else if cli.status {
        worker_csv_status();
    };

    return;

    // run().unwrap();
}

fn worker_csv_status() {
    let date_ym = get_date_ym();
    let entries = read_csv_content().expect("Unable to perform 'Read of CSV File'.");
    let l = entries.len();
    let r = if l <= 5 { 0..l } else { (l - 5)..l };

    println!("File for '{date_ym}' contains {l} entries.");

    if l > 0 {
        println!("Latest entries:");
        for i in r {
            println!("[{}] {:?}", i + 1, entries[i]);
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
    let m = bp.to_measurement();

    let path_string = get_file_path_string();
    let path_file = Path::new(&path_string);

    let fh = OpenOptions::new()
        .write(true)
        .append(true)
        .open(path_file)
        .expect(&format!("Unable to open File '{}'.", path_file.display()));

    if let Err(e) = writeln!(
        &fh,
        "{},{},{:.1},{:.1},{:.1}",
        m.date, m.time, m.sys, m.dia, m.pul
    ) {
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
    const CSV_HEADER: &str = "date,time,sys,dia,pul";

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
