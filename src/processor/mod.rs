mod dicom_extractor;
mod file_scanner;

use std::{fmt::Display, io, path::PathBuf};

use tokio::task::JoinSet;
use tokio_stream::wrappers::ReadDirStream;

#[derive(Debug)]
pub struct DicomEntry {
    pub filepath: Option<PathBuf>,
    patient_name: Option<String>,
    patient_id: Option<String>,
}

#[derive(Debug, Default)]
pub struct ScanResult {
    pub dicom_entries: Vec<DicomEntry>,
    pub dicom_file_count: usize,
    pub total_file_count: usize,
}

pub async fn run(root: String, concurrency: usize) -> io::Result<(ScanResult, usize)> {
    let mut results = Vec::<ScanResult>::new();
    let mut work_queue = Vec::<PathBuf>::new();
    let mut task_set = JoinSet::new();
    let mut total_dir_count = 1usize;

    let root_dir = tokio::fs::read_dir(root).await?;

    // spawn initial task for root dir
    task_set.spawn(async move {
        let entries = ReadDirStream::new(root_dir);
        file_scanner::process_dir(entries).await
    });

    // extract results from root and spawn worker tasks
    while let Some(join_result) = task_set.join_next().await {
        match join_result {
            Ok((r, mut d)) => {
                total_dir_count += d.len();
                work_queue.append(&mut d);
                results.push(r);

                while task_set.len() <= concurrency {
                    if let Some(path) = work_queue.pop() {
                        task_set.spawn(async move {
                            match tokio::fs::read_dir(&path).await {
                                Ok(rd) => {
                                    let rd_stream = ReadDirStream::new(rd);
                                    file_scanner::process_dir(rd_stream).await
                                }
                                Err(e) => {
                                    log::warn!(
                                        "Error reading entries in: {}. Details: {}",
                                        path.to_string_lossy(),
                                        e
                                    );
                                    (ScanResult::default(), Vec::default())
                                }
                            }
                        });
                    } else {
                        // no new directories to visit
                        break;
                    }
                }
            }
            Err(e) => log::warn!("Error joining task handle: {}", e),
        }
    }

    let results = results.into_iter().reduce(|mut acc, mut i| {
        acc.dicom_file_count += i.dicom_entries.len();
        acc.total_file_count += i.total_file_count;
        acc.dicom_entries.append(&mut i.dicom_entries);
        acc
    });
    Ok((results.unwrap_or_default(), total_dir_count))
}

impl Display for DicomEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{};{};{};",
            self.filepath
                .as_deref()
                .map(|v| v.to_string_lossy().into_owned())
                .unwrap_or_default(),
            self.patient_name.as_deref().unwrap_or_default(),
            self.patient_id.as_deref().unwrap_or_default()
        )
    }
}

impl ScanResult {
    pub fn add_entry(&mut self, e: DicomEntry) {
        self.dicom_entries.push(e);
    }
    pub fn inc_total_file_count(&mut self) {
        self.total_file_count += 1;
    }
}
