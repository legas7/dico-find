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
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let mut task_set = JoinSet::new();
    let mut total_dir_count = 1usize;

    let root_dir = tokio::fs::read_dir(root).await?;
    let txc = tx.clone();

    // spawn initial task for root dir
    task_set.spawn(async move {
        let entries = ReadDirStream::new(root_dir);
        file_scanner::process_dir(entries, txc).await
    });

    while let Some(join_result) = task_set.join_next().await {
        if let Ok(sr) = join_result.inspect_err(|e| log::warn!("Error joining task handle: {}", e))
        {
            results.push(sr);
        }

        while let Ok(dir) = rx.try_recv() {
            let txc = tx.clone();
            if task_set.len() <= concurrency {
                total_dir_count += 1;
                task_set.spawn(async move {
                    if let Ok(rd) = tokio::fs::read_dir(&dir).await {
                        let rd_stream = ReadDirStream::new(rd);
                        file_scanner::process_dir(rd_stream, txc).await
                    } else {
                        ScanResult::default()
                    }
                });
            } else {
                _ = tx.send(dir);
                break;
            }
        }
    }

    let results = results.into_iter().reduce(|mut acc, mut i| {
        acc.dicom_file_count += i.dicom_file_count;
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
