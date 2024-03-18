use std::env;
use std::fs::File;
use std::io::{BufWriter, Write};

use time::OffsetDateTime;

use crate::processor::ScanResult;

pub fn save_results(results: ScanResult) {
    save_to_file(
        results.dicom_entries.iter(),
        format!("entries_{}.txt", OffsetDateTime::now_utc().unix_timestamp()),
    );
}

fn save_to_file<R, I>(items: R, filename: String)
where
    R: Iterator<Item = I>,
    I: ToString,
{
    let mut full_path = env::current_dir().expect("Failed to get current dir");
    full_path.push(filename);
    log::info!("Saving file '{}'", full_path.to_string_lossy());

    let mut file = BufWriter::new(File::create(&full_path).expect("Could not create file"));
    for item in items {
        writeln!(file, "{}", item.to_string()).expect("Failed to save result");
    }
    file.flush().expect("Failed to flush buffer");
}
