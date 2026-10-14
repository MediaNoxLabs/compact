// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Shared locked atomic publication transaction for both source compilers.
use fs2::FileExt;
use std::{
    error::Error,
    ffi::OsStr,
    fs, io,
    path::{Component, Path, PathBuf},
    process,
    time::{SystemTime, UNIX_EPOCH},
};

/// Keep failed Rust-target runs out of the requested output directory.
pub(crate) struct StagedOutput {
    path: PathBuf,
    published: bool,
    _lock: fs::File,
}

fn interrupted_backup(name: &str, candidate: &OsStr) -> bool {
    let prefix = format!(".{name}.compactc-stage-");
    let Some(parts) = candidate
        .to_str()
        .and_then(|candidate| candidate.strip_prefix(&prefix))
        .and_then(|candidate| candidate.strip_suffix("-previous"))
    else {
        return false;
    };
    let numbers = parts.split('-').collect::<Vec<_>>();
    numbers.len() == 3
        && numbers
            .iter()
            .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
}

fn recover_interrupted_output(
    output: &Path,
    parent: &Path,
    name: &str,
) -> Result<(), Box<dyn Error>> {
    let mut backups = Vec::new();
    for entry in fs::read_dir(parent)? {
        let entry = entry?;
        if !interrupted_backup(name, &entry.file_name()) {
            continue;
        }
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path)?;
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            return Err(format!(
                "interrupted Rust output backup is not a directory: {}",
                path.display()
            )
            .into());
        }
        backups.push(path);
    }
    if backups.len() > 1 {
        return Err(format!(
            "multiple interrupted Rust output backups for {}: {}",
            output.display(),
            backups
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )
        .into());
    }
    match fs::symlink_metadata(output) {
        Ok(metadata) if !metadata.is_dir() || metadata.file_type().is_symlink() => {
            return Err(format!(
                "Rust output must be a directory, not a file or symlink: {}",
                output.display()
            )
            .into());
        }
        Ok(_) => {
            if let Some(backup) = backups.first() {
                eprintln!(
                    "compactc: warning: previous Rust output remains at {} after an interrupted publication",
                    backup.display()
                );
            }
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            if let Some(backup) = backups.first() {
                fs::rename(backup, output)?;
            }
        }
        Err(error) => return Err(error.into()),
    }
    Ok(())
}

impl StagedOutput {
    pub(crate) fn new(output: &Path) -> Result<Self, Box<dyn Error>> {
        let Some(Component::Normal(name)) = output.components().next_back() else {
            return Err(
                "Rust output must name a directory, not current or parent directory".into(),
            );
        };
        let parent = output
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        fs::create_dir_all(parent)?;
        let name = name.to_string_lossy();
        let lock_path = parent.join(format!(".{name}.compactc.lock"));
        match fs::symlink_metadata(&lock_path) {
            Ok(metadata) if !metadata.is_file() || metadata.file_type().is_symlink() => {
                return Err(format!(
                    "Rust output lock must be a regular file, not a directory or symlink: {}",
                    lock_path.display()
                )
                .into());
            }
            Err(error) if error.kind() != io::ErrorKind::NotFound => return Err(error.into()),
            _ => {}
        }
        let lock = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&lock_path)?;
        FileExt::lock_exclusive(&lock)?;
        recover_interrupted_output(output, parent, &name)?;
        let nonce = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        for attempt in 0..100 {
            let path = parent.join(format!(
                ".{}.compactc-stage-{}-{nonce}-{attempt}",
                name,
                process::id()
            ));
            match fs::create_dir(&path) {
                Ok(()) => {
                    return Ok(Self {
                        path,
                        published: false,
                        _lock: lock,
                    });
                }
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error.into()),
            }
        }
        Err("could not create a unique Rust output staging directory".into())
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn publish(mut self, output: &Path) -> Result<(), Box<dyn Error>> {
        match fs::symlink_metadata(output) {
            Ok(metadata) => {
                if !metadata.is_dir() || metadata.file_type().is_symlink() {
                    return Err(format!(
                        "Rust output changed to a file or symlink before publication: {}",
                        output.display()
                    )
                    .into());
                }
                let backup = self.path.with_file_name(format!(
                    "{}-previous",
                    self.path
                        .file_name()
                        .expect("staged output has a filename")
                        .to_string_lossy()
                ));
                match fs::symlink_metadata(&backup) {
                    Ok(_) => {
                        return Err(format!(
                            "Rust output backup path already exists: {}",
                            backup.display()
                        )
                        .into());
                    }
                    Err(error) if error.kind() != io::ErrorKind::NotFound => {
                        return Err(error.into());
                    }
                    Err(_) => {}
                }
                fs::rename(output, &backup)?;
                if let Err(error) = fs::rename(&self.path, output) {
                    if let Err(restore) = fs::rename(&backup, output) {
                        return Err(format!(
                            "could not publish Rust output: {error}; previous output remains at {} (restore failed: {restore})",
                            backup.display()
                        )
                        .into());
                    }
                    return Err(error.into());
                }
                self.published = true;
                if let Err(error) = fs::remove_dir_all(&backup) {
                    eprintln!(
                        "compactc: warning: generated output is complete, but could not remove previous output at {}: {error}",
                        backup.display()
                    );
                }
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                fs::rename(&self.path, output)?;
                self.published = true;
            }
            Err(error) => return Err(error.into()),
        }
        Ok(())
    }
}

impl Drop for StagedOutput {
    fn drop(&mut self) {
        if !self.published {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}
