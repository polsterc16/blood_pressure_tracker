use std::env;
use std::fs;
use std::path::Path;

fn main() {
    let args: Vec<String> = env::args().collect();

    let query = &args[1];
    let file_path = &args[2];

    println!("Searching for {query}");
    println!("In file {file_path}");

    // let contents = fs::read_to_string(file_path).expect("Should have been able to read the file");

    // println!("With text:\n{contents}");

    setup_execution();

    run();
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    // println!("Searching for {data_path_valid}");
    Ok(())
}

fn setup_execution() -> Result<(), Box<dyn std::error::Error>> {
    let _data_path_valid = check_path()?;

    Ok(())
}

fn check_path() -> std::io::Result<()> {
    if Path::new("./data").exists() {
        return Ok(());
    }
    log_warning("Directory ./data missing.");

    match fs::create_dir("./data") {
        Ok(_) => {
            log_message("Directory ./data created.");
            Ok(())
        }
        Err(e) => {
            log_error("Unable to create directory ./data.");
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
