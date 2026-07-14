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
use bp_container::*;
use file_warden::*;

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

    let dir_str = "data";
    let file_name = format!("{}", get_date_ym());
    // let file_name = String::from("2026-06");
    // let file_ext = "csv";
    let csv_worker = FileHandlerCsv::new(&dir_str, &file_name);

    csv_worker.check_file().unwrap();
    // worker_init_csv();

    match cli.add {
        Some(bp) => worker_bp_add(&csv_worker, &bp),
        _ => (),
    }

    if cli.rebuild || cli.status || cli.output {
        // let csv_collection = read_csv_content().expect("Unable to perform 'Read of CSV File'.");
        let csv_collection = csv_worker.get_csv_content().unwrap();

        if cli.rebuild {
            worker_csv_rebuild(&csv_worker, &csv_collection);
        }
        if cli.output {
            worker_output(&csv_worker, &csv_collection);
        }
        if cli.status {
            worker_csv_status(&csv_collection);
        }
    }

    return;
}

/// Will read the CSV file, sort measurements and overwrite the file
fn worker_output(csv_worker: &FileHandlerCsv, csv_collection: &CollectionCsv) {
    let coll_m2 = csv_collection.to_coll_m2(2);

    let coll_month = CollectionMonth::from_coll_m2_consume(coll_m2);

    let mut out_month = OutputMonth::from_coll_month(&coll_month);
    out_month.set_name(&csv_worker.get_file_name());
    println!("{out_month:?}");

    // println!("{coll_month:?}");
    // if true {
    //     return;
    // }
    // println!("Attempt json export");
    // let pretty = serde_json::to_string_pretty(&coll_month).unwrap(); // pretty-printed
    // println!("{pretty}");
}

/// Will read the CSV file, sort measurements and overwrite the file
fn worker_csv_rebuild(csv_worker: &FileHandlerCsv, csv_collection: &CollectionCsv) {
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

fn worker_csv_status(csv_collection: &CollectionCsv) {
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

fn worker_bp_add(csv_worker: &FileHandlerCsv, bp: &Vec<f32>) {
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

// ################################################################

mod file_warden {
    use anyhow::{Context, bail};
    use pretty_simple_display::DebugPretty;
    use serde::{Deserialize, Serialize};
    use std::fs::{self, File, OpenOptions};
    use std::io::{BufRead, BufReader, Write};
    use std::path::{Path, PathBuf};

    use crate::bp_container::*;

    #[derive(Serialize, Deserialize, DebugPretty)]
    pub struct FileHandler {
        path_dir: PathBuf,
        path_file: PathBuf,
        file_name: String,
        file_ext: String,
    }
    impl FileHandler {
        pub fn new(directory: &str, file_name: &str, file_ext: &str) -> Self {
            let p_dir = Path::new(&directory).to_owned();
            let mut p_file = Path::new(&directory).join(&file_name).to_owned();
            p_file.add_extension(file_ext);

            let ret_obj = Self {
                path_dir: p_dir,
                path_file: p_file,
                file_name: String::from(file_name),
                file_ext: String::from(file_ext),
            };
            ret_obj
        }
        pub fn get_path_dir(&self) -> &PathBuf {
            &self.path_dir
        }
        pub fn get_path_dir_str(&self) -> String {
            self.get_path_dir().display().to_string()
        }
        pub fn get_path_file(&self) -> &PathBuf {
            &self.path_file
        }
        pub fn get_path_file_str(&self) -> String {
            self.get_path_file().display().to_string()
        }

        pub fn get_file_name(&self) -> String {
            self.file_name.clone()
        }
        pub fn get_file_ext(&self) -> String {
            self.file_ext.clone()
        }
        /// Checks if directory exists and tries to create it, if not.
        ///
        /// # anyhow::Errors
        /// - Unable to create directory
        pub fn check_directory(&self) -> anyhow::Result<()> {
            let path_dir = self.get_path_dir();

            if path_dir.exists() {
                return Ok(());
            }
            // log_warning(&format!("Directory missing: `{:?}`", path_dir,));

            fs::create_dir(path_dir)
                .context(format!("Unable to create directory: `{:?}`", path_dir))?;

            // log_message(&format!("Directory created: `{:?}`", path_dir));
            return Ok(());
        }
        /// Checks if file exists.
        ///
        /// | Case                     | Returns                                 |
        /// | ------------------------ | --------------------------------------- |
        /// | File does not exist      | `Ok( FileState::Missing )`              |
        /// | File exists              | `Ok( FileState::Exists(filesize:u64) )` |
        /// | Missing file permissions | `anyhow::Error`                         |
        ///
        /// # anyhow::Errors
        /// - Unable to get `metadata` of file
        pub fn check_file_exists(&self) -> anyhow::Result<FileState> {
            let path_file = self.get_path_file();

            if !path_file.exists() {
                return Ok(FileState::Missing);
            }
            let metadata = fs::metadata(path_file).context(format!(
                "Unable to get `metadata` of file: `{:?}`",
                path_file
            ))?;
            return Ok(FileState::Exists(metadata.len()));
        }
        /// Will try to open the file.
        ///
        /// | `FileOpenMode` | Action    |
        /// | -------------- | --------- |
        /// | `Read`         | Open file in Read mode  |
        /// | `Write`        | Open (or create) file in Write mode: Overwrite and truncate previous content  |
        /// | `Append`       | Open (or create) file in Write mode: Append to previous content               |
        ///
        /// # anyhow::Errors
        /// - Unable to open file (mode)
        pub fn file_open(&self, mode: &FileOpenMode) -> anyhow::Result<File> {
            let path_file = self.get_path_file();

            let fh = match mode {
                FileOpenMode::Read => OpenOptions::new().read(true).open(path_file),
                FileOpenMode::Write => OpenOptions::new()
                    .write(true)
                    .create(true)
                    .truncate(true)
                    .open(path_file),
                FileOpenMode::Append => OpenOptions::new().write(true).append(true).open(path_file),
            }
            .context(format!(
                "Unable to open file (`{:?}`): `{:?}`",
                mode, path_file,
            ))?;

            return Ok(fh);
        }
    }

    #[derive(Serialize, Deserialize, DebugPretty)]
    pub struct FileHandlerCsv {
        fh_core: FileHandler,
    }
    impl FileHandlerCsv {
        const CSV_HEADER: &str = "date,time,sys,dia,pul";
        const FILE_EXTENSION: &str = "csv";

        pub fn new(directory: &str, file_name: &str) -> Self {
            let ret_obj = Self {
                fh_core: FileHandler::new(directory, file_name, Self::FILE_EXTENSION),
            };

            ret_obj
        }
        /// Check if file/directory exists
        /// - If missing: create with default content
        /// - If exists: check if header valid
        ///
        /// # anyhow::Errors
        /// - `self.fh_core.check_directory`
        ///   - Unable to create directory
        /// - `self.fh_core.check_file_exists`
        ///   - Unable to get `metadata` of file
        /// - `self.create_init_file`
        ///   - Unable to open file (mode)
        ///   - Unable to write to file
        /// - `self.check_file_header`
        ///   - Unable to open file (mode)
        ///   - IO error while reading first line
        ///   - Major error: File is empty!
        ///   - File has missing/wrong csv header
        pub fn check_file(&self) -> anyhow::Result<()> {
            self.fh_core.check_directory()?;

            let f_state = self.fh_core.check_file_exists()?;
            match f_state {
                FileState::Missing => {
                    self.create_init_file()?;
                    Ok(())
                }
                FileState::Exists(file_size) => {
                    if file_size == 0 {
                        self.create_init_file()?;
                        Ok(())
                    } else {
                        self.check_file_header()?;
                        Ok(())
                    }
                }
            }
        }

        /// Create/initialize the file with default content
        ///
        /// # anyhow::Errors
        /// - Unable to write to file
        /// - `self.file_open`
        ///   - Unable to open file (mode)
        fn create_init_file(&self) -> anyhow::Result<()> {
            let mode = FileOpenMode::Write;
            let path_str = self.fh_core.get_path_file_str();

            let fh = self.file_open(&mode)?;
            fh.sync_all()
                .context(format!("Unable to save file `{}`.", path_str))?;

            return Ok(());
        }

        /// Check the header (first line) of file
        ///
        /// # anyhow::Errors
        /// - IO error while reading first line
        /// - Major error: File is empty!
        /// - File has missing/wrong csv header
        /// - `self.file_open`
        ///   - Unable to open file (mode)
        fn check_file_header(&self) -> anyhow::Result<()> {
            let mode = FileOpenMode::Read;
            let path_str = self.fh_core.get_path_file_str();

            let f_read = self.file_open(&mode)?;

            let reader = BufReader::new(f_read);
            let mut lines = reader.lines();

            // Get first line
            let res = match lines.next() {
                Some(x) => x,
                None => bail!("Major error: File is empty!"),
            };
            let line = res.context("IO error while reading first line")?;

            if !(Self::CSV_HEADER == &line[..]) {
                bail!(
                    "File has missing/wrong csv header: `{}`\nT: [{}]\nF: [{}]",
                    path_str,
                    Self::CSV_HEADER,
                    &line[..]
                );
            }
            Ok(())
        }

        /// Will try to open the file.
        ///
        /// | `FileOpenMode` | Action    |
        /// | -------------- | --------- |
        /// | `Read`         | Open file in Read mode  |
        /// | `Write`        | Open (or create) file in Write mode: Overwrite and truncate previous content  |
        /// | `Append`       | Open (or create) file in Write mode: Append to previous content               |
        ///
        /// # anyhow::Errors
        /// - Unable to open file (mode)
        /// - Could not write `CSV_HEADER` to file
        pub fn file_open(&self, mode: &FileOpenMode) -> anyhow::Result<File> {
            let fh = self.fh_core.file_open(mode)?;
            match mode {
                FileOpenMode::Write => {
                    // Write CSV header line
                    writeln!(&fh, "{}", Self::CSV_HEADER)
                        .context("Could not write `CSV_HEADER` to file.")
                        .unwrap();
                }
                _ => (),
            };
            return Ok(fh);
        }

        pub fn get_csv_content(&self) -> anyhow::Result<CollectionCsv> {
            let fh_csv = self.file_open(&FileOpenMode::Read)?;

            // create CSV reader
            let mut rdr = csv::ReaderBuilder::new()
                .delimiter(b',')
                .from_reader(fh_csv);

            // deserialize reader into `MeasCsv` struct
            let records: Vec<Result<MeasCsv, csv::Error>> = rdr.deserialize().collect();

            let mut ret_coll = CollectionCsv::new_with_capacity(records.len());

            // try to insert `MeasCsv` objs into `CollectionCsv `
            for result in records {
                let entry: MeasCsv = result.context("Unable to parse entry of CSV file.")?;
                ret_coll.add_csv_consume(entry);
            }

            // Sort collection vector by fields date, time
            ret_coll.sort();

            return Ok(ret_coll);
        }
        pub fn get_file_name(&self) -> String {
            self.fh_core.get_file_name()
        }
        pub fn get_file_ext(&self) -> String {
            self.fh_core.get_file_ext()
        }
    }

    // ################################################################
    // ################################################################

    // #[derive(Debug, PartialEq)]
    // enum CsvOpenMode {
    //     Read,
    //     WriteReset,
    //     WriteAppend,
    // }

    /// | `FileOpenMode` | Meaning   |
    /// | -------------- | --------- |
    /// | `Read`         | Open file in Read mode  |
    /// | `Write`        | Open (or create) file in Write mode: Overwrite and truncate previous content  |
    /// | `Append`       | Open (or create) file in Write mode: Append to previous content               |
    #[derive(Debug, PartialEq)]
    pub enum FileOpenMode {
        /// Open file in Read mode
        Read,

        ///Open (or create) file in Write mode: Overwrite and truncate previous content
        Write,

        /// Open (or create) file in Write mode: Append to previous content
        Append,
    }
    impl FileOpenMode {
        /// Returns `true` if the file open mode is [`Read`].
        ///
        /// [`Read`]: FileOpenMode::Read
        pub fn is_read(&self) -> bool {
            matches!(self, Self::Read)
        }
        /// Returns `true` if the file open mode is [`Write`].
        ///
        /// [`Write`]: FileOpenMode::Write
        pub fn is_write(&self) -> bool {
            matches!(self, Self::Write)
        }
        /// Returns `true` if the file open mode is [`Append`].
        ///
        /// [`Append`]: FileOpenMode::Append
        pub fn is_append(&self) -> bool {
            matches!(self, Self::Append)
        }
    }

    /// | `FileState`   | Meaning   |
    /// | ------------- | --------- |
    /// | `Missing`     | File does not exist                   |
    /// | `Exists(u64)` | File exists and is `u64` bytes long   |
    #[derive(Debug, PartialEq)]
    pub enum FileState {
        /// File does not exist
        Missing,

        /// File exists and is `u64` bytes long
        Exists(u64),
    }
    impl FileState {
        /// Returns `true` if the file state is [`Missing`].
        ///
        /// [`Missing`]: FileState::Missing
        pub fn is_missing(&self) -> bool {
            matches!(self, Self::Missing)
        }
        /// Returns `true` if the file state is [`Exists`].
        ///
        /// [`Exists`]: FileState::Exists
        pub fn is_exists(&self) -> bool {
            matches!(self, Self::Exists(..))
        }
    }
}
