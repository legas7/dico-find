use std::path::PathBuf;

use tokio_stream::{wrappers::ReadDirStream, StreamExt};

use crate::processor::dicom_extractor;

use super::ScanResult;

pub async fn process_dir(mut entries: ReadDirStream) -> (ScanResult, Vec<PathBuf>) {
    let mut dir_entries = Vec::<PathBuf>::new();
    let mut result = ScanResult::default();

    while let Some(entry) = entries.next().await {
        match entry {
            Ok(entry) => match entry.metadata().await {
                Ok(meta) => {
                    if meta.is_dir() {
                        log::debug!("Visiting new dir: {:?}", entry.path());
                        dir_entries.push(entry.path());
                    } else if meta.is_file() {
                        log::debug!("Processing new file: {:?}", entry.path());
                        result.inc_total_file_count();
                        if let Some(mut dentry) = dicom_extractor::handle_file(entry.path()) {
                            dentry.filepath = Some(entry.path());
                            log::debug!("Extracted attributes from dicom file: {:?}", dentry);
                            result.add_entry(dentry);
                        }
                    }
                }
                Err(e) => log::warn!(
                    "Error reading metadata for: {}. Details: {}",
                    entry.path().to_string_lossy(),
                    e
                ),
            },
            Err(e) => log::warn!("Error reading from ReadDirStream: {}", e),
        }
    }
    (result, dir_entries)
}
