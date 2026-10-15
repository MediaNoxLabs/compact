//! Bounded exact-byte artifact reads. IR policy remains the existing four MiB.
use super::FrontendError;
use crate::origins::binding::IR_INPUT_BYTES;
use crate::origins::transport::MAX_BYTES;
use std::{
    fs::File,
    io::{self, Read},
    path::Path,
};

pub(super) enum Companion {
    Missing,
    Bytes(Vec<u8>),
    Oversized,
    Failed(String),
}
pub(super) fn optional_metadata(path: &Path) -> Companion {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Companion::Missing,
        Err(error) => return Companion::Failed(error.to_string()),
    };
    match bounded(file, MAX_BYTES) {
        Ok(Some(bytes)) => Companion::Bytes(bytes),
        Ok(None) => Companion::Oversized,
        Err(error) => Companion::Failed(error.to_string()),
    }
}

pub(super) fn required_ir(path: &Path) -> Result<Vec<u8>, FrontendError> {
    let file = File::open(path).map_err(|error| FrontendError::IrIngress(error.to_string()))?;
    bounded(file, IR_INPUT_BYTES)
        .map_err(|error| FrontendError::IrIngress(error.to_string()))?
        .ok_or_else(|| {
            FrontendError::IrIngress(format!(
                "compiler resource input_bytes exceeds {IR_INPUT_BYTES} (observed at least {})",
                IR_INPUT_BYTES + 1,
            ))
        })
}

pub(super) fn optional_info(path: &Path) -> Result<Option<Vec<u8>>, String> {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.to_string()),
    };
    bounded(file, IR_INPUT_BYTES)
        .map_err(|error| error.to_string())?
        .map(Some)
        .ok_or_else(|| format!("contract-info input_bytes exceeds {IR_INPUT_BYTES}"))
}

fn bounded(reader: impl Read, limit: usize) -> io::Result<Option<Vec<u8>>> {
    let mut bytes = Vec::new();
    reader.take(limit as u64 + 1).read_to_end(&mut bytes)?;
    Ok((bytes.len() <= limit).then_some(bytes))
}
