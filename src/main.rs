use chrono::Local;
use serde::Deserialize;
use std::env;
use std::error::Error;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::Path;

#[derive(Debug, Deserialize)]
struct Measurement {
    date: String,
    time: String,
    sys: String,
    dia: String,
    pul: String,
}
impl Measurement {
    pub fn new_full(
        &self,
        date: String,
        time: String,
        sys: String,
        dia: String,
        pul: String,
    ) -> Measurement {
        self.check_parameter(&date, &time, &sys, &dia, &pul);
        Measurement {
            date,
            time,
            sys,
            dia,
            pul,
        }
    }
    pub fn new(&self, sys: String, dia: String, pul: String) -> Measurement {
        self.new_full(get_date(), get_time(), sys, dia, pul)
    }
    fn check_parameter(&self, date: &str, time: &str, sys: &str, dia: &str, pul: &str) {
        if !date.contains("-") {
            panic!("Date '{}' not in correct format (YYYY-mm-dd)!", date);
        }
        if !time.contains(":") {
            panic!("Date '{}' not in correct format (HH:MM:SS)!", time);
        }
    }
    pub fn get_measurement(&self) -> (&str, &str, &str) {
        (&self.sys, &self.dia, &self.pul)
    }
}

fn main() {
    let _args: Vec<String> = env::args().collect();

    // let query = &args[1];
    // let file_path = &args[2];

    // println!("Searching for {query}");
    // println!("In file {file_path}");

    // let contents = fs::read_to_string(file_path).expect("Should have been able to read the file");

    // println!("With text:\n{contents}");

    setup_execution();

    run().unwrap();
}

fn run() -> Result<(), Box<dyn Error>> {
    // println!("Searching for {data_path_valid}");
    read_csv_content().expect("Unable to perform 'Read of CSV File'.");

    Ok(())
}

fn setup_execution() {
    check_path().expect("Unable to perform 'Check of /data Directory'.");
    check_file().expect("Unable to perform 'Check of work File'.");
}

fn read_csv_content() -> Result<Vec<Measurement>, Box<dyn std::error::Error>> {
    let path_string = format!("./data/{}.csv", get_date_ym());
    let file_path = Path::new(&path_string);

    let fh = File::open(file_path)?;
    let mut rdr = csv::ReaderBuilder::new().delimiter(b';').from_reader(fh);

    let records: Vec<_> = rdr.records().collect();
    let records_len = records.len();
    println!("{} contains {} entries:", path_string, records_len);

    let ret: Vec<Measurement> = Vec::with_capacity(records_len);

    for result in records {
        let entry = result?;
        println!("{:?}", entry);
    }

    Ok(ret)
}

fn check_file() -> Result<(), std::io::Error> {
    let path_string = format!("./data/{}.csv", get_date_ym());
    let fpath = Path::new(&path_string);

    if fpath.exists() {
        return Ok(());
    }
    log_warning(&format!("File '{}' missing.", fpath.display()));

    let mut fh = match File::create_new(fpath) {
        Ok(f) => {
            log_message(&format!("File '{}' created.", fpath.display()));
            f
        }
        Err(e) => {
            log_error(&format!("Unable to create File '{}'.", fpath.display()));
            return Err(e);
        }
    };

    writeln!(fh, "Date;Time;SYS;DIA;PUL")?;

    Ok(())
}

fn check_path() -> std::io::Result<()> {
    let path_string = "./data";
    let dir_path = Path::new(path_string);

    if dir_path.exists() {
        return Ok(());
    }
    log_warning(&format!("Directory '{}' missing.", path_string));

    match fs::create_dir(dir_path) {
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
