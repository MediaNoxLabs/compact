// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//  	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Argument boundaries and routing for the existing asset proof scenarios.
//!
//! Scenario modules own proving, ledger application, and their acceptance policy.
//! This module only selects a scenario and requires its single output-directory argument.

use std::error::Error;
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug)]
pub(super) enum AssetProof {
    Freshness,
    Writable,
    CustodyGrant,
    RecordWrite,
    WatchWrite,
    Removal,
    GrantEffective,
    StoredRecordFresh,
}

impl AssetProof {
    pub(super) fn from_selector(selector: Option<&OsStr>) -> Option<Self> {
        match selector?.to_str()? {
            "--asset-freshness" => Some(Self::Freshness),
            "--asset-writable" => Some(Self::Writable),
            "--asset-custody-grant" => Some(Self::CustodyGrant),
            "--asset-record-write" => Some(Self::RecordWrite),
            "--asset-watch-write" => Some(Self::WatchWrite),
            "--asset-removal" => Some(Self::Removal),
            "--asset-grant-effective" => Some(Self::GrantEffective),
            "--asset-stored-record-fresh" => Some(Self::StoredRecordFresh),
            _ => None,
        }
    }

    fn selector(self) -> &'static str {
        match self {
            Self::Freshness => "--asset-freshness",
            Self::Writable => "--asset-writable",
            Self::CustodyGrant => "--asset-custody-grant",
            Self::RecordWrite => "--asset-record-write",
            Self::WatchWrite => "--asset-watch-write",
            Self::Removal => "--asset-removal",
            Self::GrantEffective => "--asset-grant-effective",
            Self::StoredRecordFresh => "--asset-stored-record-fresh",
        }
    }

    fn usage(self) -> String {
        format!(
            "usage: compact-rust-proof-smoke {} <proof-output>",
            self.selector()
        )
    }

    pub(super) fn parse_root(
        self,
        arguments: &mut impl Iterator<Item = OsString>,
    ) -> Result<PathBuf, Box<dyn Error>> {
        let root = arguments.next().ok_or_else(|| self.usage())?;
        if arguments.next().is_some() {
            return Err(self.usage().into());
        }
        Ok(PathBuf::from(root))
    }

    pub(super) fn run(self, root: &Path) -> Result<(), Box<dyn Error>> {
        match self {
            Self::Freshness => super::asset_freshness::run(root),
            Self::Writable => super::asset_writable::run(root),
            Self::CustodyGrant => super::asset_custody_grant::run(root),
            Self::RecordWrite => super::asset_record_write::run(root),
            Self::WatchWrite => super::asset_watch_write::run(root),
            Self::Removal => super::asset_removal::run(root),
            Self::GrantEffective => super::asset_grant_effective::run(root),
            Self::StoredRecordFresh => super::asset_stored_record_fresh::run(root),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::AssetProof;
    use std::ffi::{OsStr, OsString};

    #[test]
    fn argument_errors_keep_each_usage_and_stop_after_the_first_extra() {
        for selector in [
            "--asset-freshness",
            "--asset-writable",
            "--asset-custody-grant",
            "--asset-record-write",
            "--asset-watch-write",
            "--asset-removal",
            "--asset-grant-effective",
            "--asset-stored-record-fresh",
        ] {
            let proof = AssetProof::from_selector(Some(OsStr::new(selector))).unwrap();
            let expected = format!("usage: compact-rust-proof-smoke {selector} <proof-output>");
            let missing = proof.parse_root(&mut std::iter::empty()).unwrap_err();
            assert_eq!(missing.to_string(), expected);

            let mut extra = ["output", "unexpected", "unconsumed"]
                .map(OsString::from)
                .into_iter();
            assert_eq!(
                proof.parse_root(&mut extra).unwrap_err().to_string(),
                expected
            );
            assert_eq!(extra.next(), Some(OsString::from("unconsumed")));
        }
    }

    #[test]
    fn output_path_is_not_reinterpreted_as_options_or_normalized() {
        let proof = AssetProof::from_selector(Some(OsStr::new("--asset-freshness"))).unwrap();
        for value in ["", "--output", "a path/../output"] {
            let root = OsString::from(value);
            let mut arguments = std::iter::once(root.clone());
            assert_eq!(
                proof.parse_root(&mut arguments).unwrap().as_os_str(),
                root.as_os_str()
            );
            assert!(arguments.next().is_none());
        }
    }

    #[test]
    fn does_not_claim_other_scenarios_positional_paths_or_partial_selectors() {
        for selector in [
            None,
            Some(OsStr::new("--schnorr-attestation")),
            Some(OsStr::new("counter-output")),
            Some(OsStr::new("--asset-freshness-extra")),
        ] {
            assert!(AssetProof::from_selector(selector).is_none());
        }
    }

    #[cfg(unix)]
    #[test]
    fn non_utf8_output_paths_are_preserved_and_selectors_are_not_lossily_matched() {
        use std::os::unix::ffi::{OsStrExt, OsStringExt};

        let root = OsString::from_vec(b"output-\xff".to_vec());
        let proof = AssetProof::from_selector(Some(OsStr::new("--asset-writable"))).unwrap();
        let parsed = proof
            .parse_root(&mut std::iter::once(root.clone()))
            .unwrap();
        assert_eq!(parsed.as_os_str().as_bytes(), root.as_os_str().as_bytes());
        assert!(
            AssetProof::from_selector(Some(OsStr::from_bytes(b"--asset-writable\xff"))).is_none()
        );
    }
}
