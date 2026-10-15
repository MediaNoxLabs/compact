//! Only this live configured-process boundary constructs an invocation receipt.
//! The selected executable is trusted by configuration, not attested by a hash.
use super::{
    status::{MetadataProblem, MetadataState, MetadataUnavailable},
    transport,
};
use sha2::{Digest, Sha256};
use std::{
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

mod input;
pub(crate) mod render;
mod stage;
use stage::FreshStage;

#[derive(Debug)]
pub enum FrontendError {
    Io(String),
    ProcessExit { code: Option<i32> },
    IrIngress(String),
    Publication(String),
}
impl std::fmt::Display for FrontendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "configured frontend: {self:?}")
    }
}
impl std::error::Error for FrontendError {}

pub struct FrontendRequest {
    pub source: PathBuf,
    pub arguments: Vec<OsString>,
    /// Existing directory used for an exclusive fresh child. Final publication
    /// is the caller's separate locked/atomic staging transaction.
    /// Relative generated assets remain anchored to that child until the caller
    /// rebases them for the final output path; promotion alone does not do this.
    pub stage_parent: PathBuf,
}

pub struct ConfiguredFrontend {
    executable: OsString,
}
impl ConfiguredFrontend {
    pub fn new(executable: impl Into<OsString>) -> Self {
        Self {
            executable: executable.into(),
        }
    }

    pub fn invoke(&self, request: FrontendRequest) -> Result<FrontendArtifacts, FrontendError> {
        let cwd = std::env::current_dir().map_err(io_error)?;
        let stage = FreshStage::new(&request.stage_parent)?;
        let executable_fingerprint = executable_hash(&self.executable);
        let status = Command::new(&self.executable)
            .args(&request.arguments)
            .arg("--emit-rust-ir")
            .arg(&request.source)
            .arg(stage.path())
            .status()
            .map_err(io_error)?;
        if !status.success() {
            return Err(FrontendError::ProcessExit {
                code: status.code(),
            });
        }

        let ir_path = stage.path().join("contract/compact-rust-ir.json");
        let exact_ir = input::required_ir(&ir_path)?;
        let companion =
            input::optional_metadata(&stage.path().join("contract/compact-rust-origins.raw.json"));
        let (metadata, companion_sha256) = match companion {
            input::Companion::Missing => (
                MetadataState::Unavailable(MetadataUnavailable::Missing),
                None,
            ),
            input::Companion::Bytes(bytes) => {
                let hash = digest(&bytes);
                (MetadataState::read(bytes.as_slice()), Some(hash))
            }
            input::Companion::Oversized => (
                MetadataState::Unavailable(MetadataUnavailable::Budget {
                    resource: "origin_bytes",
                    limit: transport::MAX_BYTES as u64,
                    observed_at_least: transport::MAX_BYTES as u64 + 1,
                }),
                None,
            ),
            input::Companion::Failed(error) => {
                (MetadataState::Invalid(MetadataProblem::Io(error)), None)
            }
        };
        let contract_info = input::optional_info(&stage.path().join("compiler/contract-info.json"));
        let receipt = InvocationReceipt {
            ir_sha256: digest(&exact_ir),
            companion_sha256,
            contract_info_sha256: contract_info
                .as_ref()
                .ok()
                .and_then(|info| info.as_ref().map(|bytes| digest(bytes))),
            executable: self.executable.clone(),
            executable_fingerprint,
            source: request.source,
            cwd,
        };
        Ok(FrontendArtifacts {
            stage,
            exact_ir,
            metadata,
            contract_info,
            receipt,
        })
    }
}

// Private constructor and fields. No replay bytes/sidecar directory API can
// fabricate live issuance. These hashes are integrity identities, not signatures.
struct InvocationReceipt {
    ir_sha256: [u8; 32],
    companion_sha256: Option<[u8; 32]>,
    contract_info_sha256: Option<[u8; 32]>,
    executable: OsString,
    executable_fingerprint: Option<[u8; 32]>,
    source: PathBuf,
    cwd: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InvocationFingerprint {
    pub exact_ir_sha256: [u8; 32],
    pub companion_sha256: Option<[u8; 32]>,
    pub contract_info_sha256: Option<[u8; 32]>,
    pub configured_executable: OsString,
    /// Available for an explicit readable file; not resolved from PATH and not
    /// a guarantee against replacement between hashing and process execution.
    pub configured_executable_sha256: Option<[u8; 32]>,
    pub source_operand: PathBuf,
    pub invocation_cwd: PathBuf,
    pub fresh_output_dir: PathBuf,
}

pub struct FrontendArtifacts {
    stage: FreshStage,
    exact_ir: Vec<u8>,
    metadata: MetadataState,
    contract_info: Result<Option<Vec<u8>>, String>,
    receipt: InvocationReceipt,
}
impl FrontendArtifacts {
    /// Original invocation directory. Retain this path before consuming the
    /// artifacts if publishing relative generated assets such as JS source maps.
    pub fn output_dir(&self) -> &Path {
        self.stage.path()
    }
    pub fn fingerprint(&self) -> InvocationFingerprint {
        InvocationFingerprint {
            exact_ir_sha256: self.receipt.ir_sha256,
            companion_sha256: self.receipt.companion_sha256,
            contract_info_sha256: self.receipt.contract_info_sha256,
            configured_executable: self.receipt.executable.clone(),
            configured_executable_sha256: self.receipt.executable_fingerprint,
            source_operand: self.receipt.source.clone(),
            invocation_cwd: self.receipt.cwd.clone(),
            fresh_output_dir: self.stage.path().to_path_buf(),
        }
    }

    /// Render from the retained exact bytes, never from a later edited file.
    pub fn render(
        &self,
        options: render::RenderOptions,
    ) -> Result<render::FrontendRender, Box<render::NativeCompileFailure>> {
        render::render(self, options)
    }

    /// Move completed entries into an existing empty locked staging parent.
    /// Reject preexisting destination entries; this is not final publication.
    /// Pending metadata status and retained IR/origin/proof-info bytes are
    /// checked before any move. This does not authenticate every generated asset
    /// or attest which source bytes the configured process consumed.
    ///
    /// # Caller publication duties
    ///
    /// Retain [`Self::output_dir`], render and enforce selected capability gates,
    /// then promote. Before final publication, rebase relative source-map paths
    /// from the original invocation map directory to the **final requested map
    /// directory**, complete packaging and refresh any affected output manifest.
    /// The caller owns the lock, rollback and atomic final directory replacement.
    /// This method neither rebases assets nor refreshes manifests. Use `compactc
    /// --target rust` for the complete compiler publication transaction; both
    /// source compiler CLIs perform their source-map relocation explicitly.
    pub fn promote_into(
        self,
        rendered: &render::FrontendRender,
        parent: &Path,
    ) -> Result<(), FrontendError> {
        if self.fingerprint() != *rendered.fingerprint() {
            return Err(FrontendError::Publication(
                "render outcome belongs to another invocation".into(),
            ));
        }
        rendered
            .finish_metadata()
            .map_err(|error| FrontendError::Publication(error.to_string()))?;
        self.verify_staged_pair()
            .map_err(|error| FrontendError::Publication(error.to_string()))?;
        self.stage.promote_into(parent)
    }

    fn verify_staged_pair(&self) -> Result<(), MetadataProblem> {
        let ir = input::required_ir(&self.stage.path().join("contract/compact-rust-ir.json"))
            .map_err(|error| MetadataProblem::Io(error.to_string()))?;
        if digest(&ir) != self.receipt.ir_sha256 {
            return Err(MetadataProblem::PairChanged {
                artifact: "exact IR",
            });
        }
        let companion = input::optional_metadata(
            &self
                .stage
                .path()
                .join("contract/compact-rust-origins.raw.json"),
        );
        let same = match (&companion, self.receipt.companion_sha256, &self.metadata) {
            (input::Companion::Bytes(bytes), Some(hash), _) => digest(bytes) == hash,
            (
                input::Companion::Missing,
                None,
                MetadataState::Unavailable(MetadataUnavailable::Missing),
            ) => true,
            (
                input::Companion::Oversized,
                None,
                MetadataState::Unavailable(MetadataUnavailable::Budget {
                    resource: "origin_bytes",
                    ..
                }),
            ) => true,
            // IO failures have no authentic byte preimage and cannot be promoted
            // as an origin-aware artifact pair.
            _ => false,
        };
        if !same {
            return Err(MetadataProblem::PairChanged {
                artifact: "raw origins",
            });
        }
        let info = input::optional_info(&self.stage.path().join("compiler/contract-info.json"))
            .map_err(MetadataProblem::Io)?;
        if info.as_ref().map(|bytes| digest(bytes)) != self.receipt.contract_info_sha256 {
            return Err(MetadataProblem::PairChanged {
                artifact: "contract info",
            });
        }
        Ok(())
    }
}

pub(crate) fn digest(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}
fn io_error(error: std::io::Error) -> FrontendError {
    FrontendError::Io(error.to_string())
}
fn executable_hash(executable: &OsString) -> Option<[u8; 32]> {
    // Avoid reading an unbounded executable into memory. A bare PATH command
    // intentionally has no guessed filesystem identity.
    if Path::new(executable).components().count() < 2 {
        return None;
    }
    let metadata = fs::metadata(executable).ok()?;
    if !metadata.is_file() {
        return None;
    }
    let mut file = fs::File::open(executable).ok()?;
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 8192];
    let mut remaining = metadata.len();
    while remaining != 0 {
        let count = remaining.min(buffer.len() as u64) as usize;
        let read = std::io::Read::read(&mut file, &mut buffer[..count]).ok()?;
        if read == 0 {
            return None;
        }
        hash.update(&buffer[..read]);
        remaining -= read as u64;
    }
    Some(hash.finalize().into())
}
