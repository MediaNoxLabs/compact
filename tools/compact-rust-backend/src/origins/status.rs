//! Metadata has a separate typed outcome; it never becomes native admission.
use super::{binding, transport};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MetadataUnavailable {
    Missing,
    Budget {
        resource: &'static str,
        limit: u64,
        observed_at_least: u64,
    },
    NumericWidth,
    TypedInputUnavailable,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MetadataProblem {
    Io(String),
    Codec(String),
    Binding(String),
    PairChanged { artifact: &'static str },
}

impl std::fmt::Display for MetadataProblem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "origin metadata protocol problem: {self:?}")
    }
}
impl std::error::Error for MetadataProblem {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceSnapshotFingerprint {
    pub file_id: usize,
    pub resolved_path: String,
    pub raw_sha256: [u8; 32],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BindingSummary {
    pub files: usize,
    pub origins: usize,
    pub addresses: usize,
    pub typed_slots: usize,
    pub transport_only: usize,
    pub post_invocation_source_snapshots: usize,
    pub source_snapshots_unavailable: usize,
    pub physical_ranges_matching_snapshot: usize,
    pub physical_ranges_unverified: usize,
    pub compiler_context_ranges: usize,
    pub post_invocation_snapshot_fingerprints: Vec<SourceSnapshotFingerprint>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MetadataStatus {
    Available(BindingSummary),
    Unavailable(MetadataUnavailable),
    Invalid(MetadataProblem),
}

pub(crate) enum MetadataState {
    Valid(transport::UntrustedTransport),
    Unavailable(MetadataUnavailable),
    Invalid(MetadataProblem),
}

impl MetadataState {
    pub(crate) fn read(reader: impl std::io::Read) -> Self {
        match transport::read_transport(reader) {
            Ok(validated) => Self::Valid(validated),
            Err(error) => Self::from_error(error),
        }
    }

    fn from_error(error: transport::Error) -> Self {
        use transport::Error;
        let unavailable = match error {
            Error::BytesExceeded => MetadataUnavailable::Budget {
                resource: "origin_bytes",
                limit: transport::MAX_BYTES as u64,
                observed_at_least: transport::MAX_BYTES as u64 + 1,
            },
            Error::EntriesExceeded => MetadataUnavailable::Budget {
                resource: "origin_table_entries",
                limit: transport::MAX_ENTRIES as u64,
                observed_at_least: transport::MAX_ENTRIES as u64 + 1,
            },
            Error::PathCharactersExceeded => MetadataUnavailable::Budget {
                resource: "origin_path_characters",
                limit: transport::MAX_PATH_CHARACTERS as u64,
                observed_at_least: transport::MAX_PATH_CHARACTERS as u64 + 1,
            },
            Error::PendingExceeded => MetadataUnavailable::Budget {
                resource: "origin_pending",
                limit: transport::MAX_PENDING as u64,
                observed_at_least: transport::MAX_PENDING as u64 + 1,
            },
            Error::ProducerRefusal {
                resource,
                observed,
                limit,
            } => MetadataUnavailable::Budget {
                resource: resource_name(resource),
                observed_at_least: observed,
                limit,
            },
            Error::ProducerNumericWidthRefusal => MetadataUnavailable::NumericWidth,
            other => return Self::Invalid(MetadataProblem::Codec(format!("{other:?}"))),
        };
        Self::Unavailable(unavailable)
    }

    pub(crate) fn prior_status(&self) -> MetadataStatus {
        match self {
            Self::Valid(_) => {
                MetadataStatus::Unavailable(MetadataUnavailable::TypedInputUnavailable)
            }
            Self::Unavailable(reason) => MetadataStatus::Unavailable(reason.clone()),
            Self::Invalid(problem) => MetadataStatus::Invalid(problem.clone()),
        }
    }
}

pub(crate) fn binding_problem(error: binding::Error) -> MetadataStatus {
    MetadataStatus::Invalid(MetadataProblem::Binding(format!("{error:?}")))
}

fn resource_name(resource: transport::Resource) -> &'static str {
    use transport::Resource;
    match resource {
        Resource::Spans => "origin_spans",
        Resource::Bytes => "origin_bytes",
        Resource::Addresses => "origin_addresses",
        Resource::Files => "origin_files",
        Resource::Records => "origin_records",
        Resource::Pending => "origin_pending",
        Resource::PathCharacters => "origin_path_characters",
    }
}
