use std::path::PathBuf;

use dicom_core::DataElement;
use dicom_dictionary_std::tags;
use dicom_object::{file::OpenFileOptions, mem::InMemDicomObject};

use super::DicomEntry;

pub fn handle_file(file: PathBuf) -> Option<DicomEntry> {
    if let Ok(file) = OpenFileOptions::new()
        .read_until(tags::PATIENT_ID)
        .open_file(file)
    {
        let patient_name = file.element(tags::PATIENT_NAME).ok();
        let patient_id = file.element(tags::PATIENT_ID).ok();
        return Some(DicomEntry {
            filepath: None,
            patient_name: extract_string(patient_name),
            patient_id: extract_string(patient_id),
        });
    }
    None
}

fn extract_string(el: Option<&DataElement<InMemDicomObject>>) -> Option<String> {
    el.and_then(|e| e.to_str().ok().map(|v| v.into_owned()))
}
