use std::path::PathBuf;

use tokio_stream::{wrappers::ReadDirStream, StreamExt};

use crate::processor::dicom_extractor;

use super::ScanResult;

pub async fn process_dir(mut entries: ReadDirStream) -> (ScanResult, Vec<PathBuf>) {
    let mut dir_entries = Vec::<PathBuf>::new();
    let mut file_entries = Vec::<PathBuf>::new();
    let mut total_file_count = 0usize;

    while let Some(entry) = entries.next().await {
        match entry {
            Ok(entry) => match entry.metadata().await {
                Ok(meta) => {
                    if meta.is_dir() {
                        log::debug!("Visiting new dir: {:?}", entry.path());
                        dir_entries.push(entry.path());
                    } else if meta.is_file() {
                        log::debug!("Processing new file: {:?}", entry.path());
                        total_file_count += 1;
                        file_entries.push(entry.path());
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
    let results = tokio::task::spawn_blocking(|| {
        file_entries
            .into_iter()
            .filter_map(dicom_extractor::handle_file)
            .collect::<Vec<_>>()
    })
    .await
    .inspect_err(|e| log::warn!("Error joining task handle: {}", e))
    .unwrap_or_default();

    (
        ScanResult {
            dicom_file_count: results.len(),
            dicom_entries: results,
            total_file_count,
        },
        dir_entries,
    )
}
