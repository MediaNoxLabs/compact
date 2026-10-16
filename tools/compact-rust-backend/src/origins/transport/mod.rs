//! Untrusted raw3 origin transport, deliberately separate from semantic typing.
//!
//! A validated transport is not a frontend certificate or an attachable source
//! location. The prototype has no typed-node binder or diagnostic attachment.
mod budget;
pub(super) mod model;
mod validate;
mod wire;
mod wire_shape;

pub use model::Resource;
use sha2::{Digest, Sha256};
use std::io::Read;

pub use budget::{MAX_BYTES, MAX_ENTRIES, MAX_PATH_CHARACTERS, MAX_PENDING};

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    Io(String),
    BytesExceeded,
    Json(String),
    EntriesExceeded,
    Header,
    MissingTables,
    MixedProducerRefusal,
    ProducerNumericWidthRefusal,
    ProducerRefusal {
        resource: Resource,
        observed: u64,
        limit: u64,
    },
    Structure {
        table: &'static str,
        index: usize,
        reason: &'static str,
    },
    PathCharactersExceeded,
    PendingExceeded,
}

/// Bounded, structurally checked metadata from an unauthenticated companion.
/// The private fields cannot be used to mint a diagnostic or frontend authority.
#[derive(Debug)]
pub struct UntrustedTransport {
    data: model::Tables,
    raw_sha256: [u8; 32],
}

impl UntrustedTransport {
    pub(crate) fn table_counts(&self) -> (usize, usize, usize) {
        (
            self.data.files.len(),
            self.data.origins.len(),
            self.data.addresses.len(),
        )
    }

    /// Raw-byte identity alone is not a claim about its producer or its IR.
    pub(crate) fn raw_sha256(&self) -> [u8; 32] {
        self.raw_sha256
    }
}

pub(crate) fn sha256(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

/// Read at most four MiB plus one sentinel byte, then parse a closed raw3 model.
/// Serde may allocate strings and bounded tables within the accepted payload;
/// the ingress bound is not a peak-memory bound or a semantic work guarantee.
pub(crate) fn read_transport(reader: impl Read) -> Result<UntrustedTransport, Error> {
    let mut bytes = Vec::new();
    reader
        .take((MAX_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|error| Error::Io(error.to_string()))?;
    if bytes.len() > MAX_BYTES {
        return Err(Error::BytesExceeded);
    }
    let mut decoder = serde_json::Deserializer::from_slice(&bytes);
    let document: wire::Document = wire_shape::map_only(&mut decoder).map_err(decode_error)?;
    decoder.end().map_err(decode_error)?;
    let tables = validate::validate(wire::expand(document)?)?;
    Ok(UntrustedTransport {
        data: tables,
        raw_sha256: sha256(&bytes),
    })
}

// Isolated compatibility adapter for serde's fixed bounded-row visitor message.
// Do not classify arbitrary substrings or producer text as a resource refusal.
const ENTRY_LIMIT_MESSAGE: &str = "origin table entry limit exceeded";
fn decode_error(error: serde_json::Error) -> Error {
    let text = error.to_string();
    let positioned = format!(
        "{ENTRY_LIMIT_MESSAGE} at line {} column {}",
        error.line(),
        error.column()
    );
    if error.is_data() && (text == ENTRY_LIMIT_MESSAGE || text == positioned) {
        Error::EntriesExceeded
    } else {
        Error::Json(text)
    }
}
impl UntrustedTransport {
    pub(crate) fn rows(&self) -> &[model::Address] {
        &self.data.addresses.0
    }
    pub(crate) fn files(&self) -> &[model::File] {
        &self.data.files.0
    }
    pub(crate) fn origins(&self) -> &[model::Origin] {
        &self.data.origins.0
    }
}
