//! Fresh invocation output; final lock/atomic publication belongs to the caller.
use super::FrontendError;
use std::{
    fs, io,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

pub(super) struct FreshStage {
    path: PathBuf,
}
impl FreshStage {
    pub(super) fn new(parent: &Path) -> Result<Self, FrontendError> {
        if !parent.is_dir() {
            return Err(FrontendError::Io(
                "stage parent is not an existing directory".into(),
            ));
        }
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| FrontendError::Io(error.to_string()))?
            .as_nanos();
        for attempt in 0..64 {
            let path = parent.join(format!(
                ".compact-origin-invocation-{}-{nonce}-{attempt}",
                std::process::id()
            ));
            match fs::create_dir(&path) {
                Ok(()) => return Ok(Self { path }),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(FrontendError::Io(error.to_string())),
            }
        }
        Err(FrontendError::Io(
            "could not create exclusive invocation stage".into(),
        ))
    }
    pub(super) fn path(&self) -> &Path {
        &self.path
    }

    pub(super) fn promote_into(self, parent: &Path) -> Result<(), FrontendError> {
        if parent != self.path.parent().unwrap_or(Path::new("")) {
            return Err(FrontendError::Publication(
                "promotion target must be the invocation's staging parent".into(),
            ));
        }
        let entries: Vec<_> = fs::read_dir(&self.path)
            .map_err(publication_error)?
            .collect::<Result<_, _>>()
            .map_err(publication_error)?;
        for entry in &entries {
            match fs::symlink_metadata(parent.join(entry.file_name())) {
                Ok(_) => {
                    return Err(FrontendError::Publication(
                        "destination entry already exists".into(),
                    ));
                }
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(publication_error(error)),
            }
        }
        // Not an adversarial concurrent-filesystem guarantee. The caller owns
        // the staging lock and abandons the whole parent if any move fails.
        for entry in entries {
            fs::rename(entry.path(), parent.join(entry.file_name())).map_err(publication_error)?;
        }
        fs::remove_dir(&self.path).map_err(publication_error)
    }
}
impl Drop for FreshStage {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
fn publication_error(error: io::Error) -> FrontendError {
    FrontendError::Publication(error.to_string())
}
