use anyhow::anyhow;

use std::{
    fs,
    path::{Path, PathBuf},
    str::FromStr,
    sync::mpsc,
    thread::{self, JoinHandle},
    time::Duration,
};

use crate::processor::{dicom_extractor, ScanResult};

pub fn run(root: String, concurrency: usize) -> anyhow::Result<(ScanResult, usize)> {
    let (dirs_tx, dirs_rx) = mpsc::channel::<PathBuf>();
    let mut task_handles = Vec::<JoinHandle<ScanResult>>::new();
    let mut partial_results = Vec::<ScanResult>::new();
    let mut total_dir_count = 0usize;

    let root = {
        let path = PathBuf::from_str(&root)?;
        if !path.is_dir() {
            return Err(anyhow!("starting path is not a dir"));
        }
        path
    };
    // start execution with firt dir
    dirs_tx.send(root).expect("could not send path");

    loop {
        // spawn tasks within concurrency limit
        if let Ok(dir) = dirs_rx.try_recv() {
            let txc = dirs_tx.clone();
            if task_handles.len() < concurrency {
                total_dir_count += 1;
                task_handles.push(std::thread::spawn(move || process_dir(&dir, txc)));
            } else {
                dirs_tx.send(dir).expect("could not send path");
            }
        }

        // collect results from finished tasks
        if let Some(finished) = task_handles
            .iter()
            .position(|p| p.is_finished())
            .map(|i| task_handles.swap_remove(i).join())
        {
            if let Ok(result) =
                finished.inspect_err(|e| log::warn!("failed to join threads. detail: {e:?}"))
            {
                partial_results.push(result);
            }
        }
        // prevent busy waiting
        thread::sleep(Duration::from_millis(10));
        if task_handles.is_empty() {
            println!("not taksk left!");
            break;
        }
    }

    let results = partial_results.into_iter().reduce(|mut acc, mut i| {
        acc.dicom_file_count += i.dicom_entries.len();
        acc.total_file_count += i.total_file_count;
        acc.dicom_entries.append(&mut i.dicom_entries);
        acc
    });

    Ok((results.unwrap_or_default(), total_dir_count))
}

fn process_dir(dir: &Path, tx: mpsc::Sender<PathBuf>) -> ScanResult {
    let mut results = ScanResult::default();
    if let Ok(mut dir_iter) =
        fs::read_dir(dir).inspect_err(|e| log::warn!("Failed to read dir: {e}"))
    {
        while let Some(Ok(entry)) = dir_iter.next() {
            if entry.path().is_dir() {
                _ = tx.send(entry.path());
            } else {
                results.inc_total_file_count();
                if let Some(mut dentry) = dicom_extractor::handle_file(entry.path()) {
                    dentry.filepath = Some(entry.path());
                    results.add_entry(dentry);
                }
            }
        }
    }
    results
}
