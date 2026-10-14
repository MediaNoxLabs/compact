//! Optional post-invocation snapshots, not consumed-source authentication.
//! The byte budget is cumulative across files and never rejects native code.
use super::transport::{
    self,
    model::{Endpoint, Extent, Origin},
};
use sha2::{Digest, Sha256};
use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

pub(crate) struct Snapshot {
    path: PathBuf,
    raw_sha256: [u8; 32],
    decoded: String,
}
pub(crate) struct SourceSnapshots {
    files: Vec<Option<Snapshot>>,
}
// Independent optional snapshot budget, not a Compact source-admission limit
// and not coupled to any future transport-only budget decision.
const SOURCE_SNAPSHOT_BYTES: usize = 4 * 1024 * 1024;

impl SourceSnapshots {
    pub(crate) fn read(metadata: &transport::UntrustedTransport, cwd: &Path) -> Self {
        let mut remaining = SOURCE_SNAPSHOT_BYTES;
        let files = metadata
            .files()
            .iter()
            .map(|file| {
                let path = cwd.join(&file.path);
                if !std::fs::metadata(&path).ok()?.is_file() {
                    return None;
                }
                let mut raw = Vec::new();
                if File::open(&path)
                    .ok()?
                    .take(remaining as u64 + 1)
                    .read_to_end(&mut raw)
                    .is_err()
                {
                    return None;
                }
                if raw.len() > remaining {
                    remaining = 0;
                    return None;
                }
                remaining -= raw.len();
                // Conservative production boundary: the retained Chez samples do
                // not prove replacement equivalence for every malformed sequence.
                // The frontend range remains available, but no optional excerpt or
                // endpoint-match claim can be issued from encoding-unverified bytes.
                if std::str::from_utf8(&raw).is_err() {
                    return None;
                }
                Some(Snapshot {
                    path,
                    raw_sha256: Sha256::digest(&raw).into(),
                    decoded: decode_compact_source(&raw),
                })
            })
            .collect();
        Self { files }
    }
    pub(crate) fn loaded(&self) -> usize {
        self.files.iter().filter(|file| file.is_some()).count()
    }
    pub(crate) fn unavailable(&self) -> usize {
        self.files.len() - self.loaded()
    }

    /// Validate at most a single selected diagnostic range after failure unwind.
    /// Do not run this per-origin: that would multiply source-text scans.
    pub(crate) fn range_matches_snapshot(&self, origin: &Origin) -> bool {
        if !matches!(origin.extent, Extent::PhysicalSource {}) {
            return false;
        }
        let Some(Some(file)) = self.files.get(origin.file_id as usize) else {
            return false;
        };
        file.endpoint_matches(&origin.start) && file.endpoint_matches(&origin.end)
    }
    pub(crate) fn fingerprints(&self) -> Vec<super::status::SourceSnapshotFingerprint> {
        self.files
            .iter()
            .enumerate()
            .filter_map(|(file_id, snapshot)| {
                let snapshot = snapshot.as_ref()?;
                Some(super::status::SourceSnapshotFingerprint {
                    file_id,
                    resolved_path: snapshot.path.to_string_lossy().into_owned(),
                    raw_sha256: snapshot.raw_sha256,
                })
            })
            .collect()
    }
    pub(crate) fn fingerprint(
        &self,
        file_id: usize,
    ) -> Option<super::status::SourceSnapshotFingerprint> {
        let snapshot = self.files.get(file_id)?.as_ref()?;
        Some(super::status::SourceSnapshotFingerprint {
            file_id,
            resolved_path: snapshot.path.to_string_lossy().into_owned(),
            raw_sha256: snapshot.raw_sha256,
        })
    }
}

impl Snapshot {
    fn endpoint_matches(&self, endpoint: &Endpoint) -> bool {
        let (mut line, mut column) = (1u64, 1u64);
        for (offset, ch) in self.decoded.chars().enumerate() {
            if offset as u64 == endpoint.offset {
                return (line, column) == (endpoint.line, endpoint.column);
            }
            if ch == '\n' {
                line += 1;
                column = 1;
            } else {
                column += 1;
            }
        }
        self.decoded.chars().count() as u64 == endpoint.offset
            && (line, column) == (endpoint.line, endpoint.column)
    }
}

/// LF newline normalization and leading BOM rule for production-valid UTF8.
/// The helper's replacement behavior has only four narrow retained Chez controls;
/// it is not universal malformed-UTF8 equivalence. Production read gates encoding.
pub(crate) fn decode_compact_source(raw: &[u8]) -> String {
    let bytes = raw.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(raw);
    let replaced = String::from_utf8_lossy(bytes);
    let mut chars = replaced.chars().peekable();
    let mut decoded = String::new();
    while let Some(ch) = chars.next() {
        match ch {
            '\r' => {
                if matches!(chars.peek(), Some('\n' | '\u{85}')) {
                    chars.next();
                }
                decoded.push('\n');
            }
            '\u{85}' | '\u{2028}' => decoded.push('\n'),
            other => decoded.push(other),
        }
    }
    decoded
}
