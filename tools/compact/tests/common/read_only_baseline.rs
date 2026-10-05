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

//! Synthetic parsed release metadata for read-only, no-install baselines only.
//! This does not attest to public releases or downloaded compiler artifacts.

use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread::{self, JoinHandle};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub struct ReadOnlyBaseline {
    root: tempfile::TempDir,
    caches: Vec<PathBuf>,
    cache_bytes: Vec<u8>,
    cached_at: u64,
    proxy: String,
    requests: Arc<AtomicUsize>,
    stop: Arc<AtomicBool>,
    server: Option<JoinHandle<()>>,
}

impl ReadOnlyBaseline {
    pub fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let home = root.path().join("home");
        let caches = vec![
            root.path().join("cache/compactc/github_cache.json"),
            home.join("Library/Caches/compactc/github_cache.json"),
        ];
        let cached_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let cache_bytes = serde_json::to_vec_pretty(&serde_json::json!({
            "artifacts": { "compilers": fixture_compilers() },
            "cached_at": cached_at,
            "rate_limit_remaining": null,
            "rate_limit_reset": null,
        }))
        .unwrap();
        for directory in ["home/.compact", "cache", "config", "data", "receipt"] {
            fs::create_dir_all(root.path().join(directory)).unwrap();
        }
        // dirs::cache_dir uses HOME/Library/Caches on macOS and XDG_CACHE_HOME
        // on Linux. Seed both inside this fixture without changing parent env.
        for cache in &caches {
            fs::create_dir_all(cache.parent().unwrap()).unwrap();
            fs::write(cache, &cache_bytes).unwrap();
        }

        // Any unexpected reqwest/Octocrab use meets a refusing local proxy.
        // Count connections without reading or retaining request headers.
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let proxy = format!("http://{}", listener.local_addr().unwrap());
        let requests = Arc::new(AtomicUsize::new(0));
        let stop = Arc::new(AtomicBool::new(false));
        let server_requests = Arc::clone(&requests);
        let server_stop = Arc::clone(&stop);
        let server = thread::spawn(move || {
            while !server_stop.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        server_requests.fetch_add(1, Ordering::SeqCst);
                        let _ = stream.set_write_timeout(Some(Duration::from_secs(1)));
                        let _ = stream.write_all(b"HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(5));
                    }
                    Err(error) => panic!("baseline refusal proxy failed: {error}"),
                }
            }
        });
        Self {
            root,
            caches,
            cache_bytes,
            cached_at,
            proxy,
            requests,
            stop,
            server: Some(server),
        }
    }

    // Refresh immediately before each child; assert_unchanged compares against
    // these exact bytes afterward. The owning test keeps the TempDir alive.
    pub fn environment(&mut self) -> HashMap<String, String> {
        self.cached_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let mut cache: serde_json::Value = serde_json::from_slice(&self.cache_bytes).unwrap();
        cache["cached_at"] = self.cached_at.into();
        self.cache_bytes = serde_json::to_vec_pretty(&cache).unwrap();
        for path in &self.caches {
            fs::write(path, &self.cache_bytes).unwrap();
        }
        let mut env = HashMap::new();
        for (key, directory) in [
            ("HOME", "home"),
            ("XDG_CACHE_HOME", "cache"),
            ("XDG_CONFIG_HOME", "config"),
            ("XDG_DATA_HOME", "data"),
            ("COMPACT_DIRECTORY", "home/.compact"),
            ("RECEIPT_HOME", "home"),
            ("AXOUPDATER_CONFIG_PATH", "receipt"),
        ] {
            env.insert(
                key.to_string(),
                self.root
                    .path()
                    .join(directory)
                    .to_str()
                    .unwrap()
                    .to_string(),
            );
        }
        for key in [
            "HTTP_PROXY",
            "HTTPS_PROXY",
            "ALL_PROXY",
            "http_proxy",
            "https_proxy",
            "all_proxy",
        ] {
            env.insert(key.to_string(), self.proxy.clone());
        }
        for key in ["NO_PROXY", "no_proxy"] {
            env.insert(key.to_string(), String::new());
        }
        // Never forward an inherited real credential to the refusal proxy.
        env.insert("GITHUB_TOKEN".to_string(), "adr226-no-network".to_string());
        env
    }

    pub fn home(&self) -> PathBuf {
        self.root.path().join("home")
    }

    pub fn assert_unchanged(&self) {
        let age = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            - self.cached_at;
        assert!(
            age < 900,
            "fixture exceeded the production cache's 900-second freshness window"
        );
        assert_eq!(
            self.requests.load(Ordering::SeqCst),
            0,
            "read-only baseline attempted network access"
        );
        for cache in &self.caches {
            assert_eq!(
                fs::read(cache).unwrap(),
                self.cache_bytes,
                "read-only call changed its seeded cache"
            );
        }
        for directory in ["receipt", "config", "data"] {
            assert_eq!(
                fs::read_dir(self.root.path().join(directory))
                    .unwrap()
                    .count(),
                0,
                "read-only call changed private updater/configuration state"
            );
        }
        Self::assert_no_install_at(&self.home().join(".compact"));
    }

    // Also check explicit --directory arguments owned by the calling test.
    pub fn assert_no_install_at(directory: &Path) {
        // check and list --installed may initialize these empty directories,
        // but no selector, receipt, symlink or other installed-state entry.
        for entry in fs::read_dir(directory).unwrap() {
            let entry = entry.unwrap();
            assert!(
                ["bin", "versions"]
                    .iter()
                    .any(|name| entry.file_name() == *name),
                "baseline created unexpected artifact state: {:?}",
                entry.file_name()
            );
            assert!(
                entry.file_type().unwrap().is_dir(),
                "baseline created a file or symlink"
            );
            assert_eq!(
                fs::read_dir(entry.path()).unwrap().count(),
                0,
                "baseline installed compiler state"
            );
        }
    }
}

impl Drop for ReadOnlyBaseline {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(server) = self.server.take() {
            // Do not double-panic while unwinding an assertion failure.
            let result = server.join();
            if !thread::panicking() {
                result.unwrap();
            }
        }
    }
}

fn fixture_compilers() -> serde_json::Value {
    let mut compilers = serde_json::Map::new();
    // Fixed synthetic inventory matches the existing read-only output fixture.
    // Deliberately independent of LATEST_COMPACTC_VERSION: changing that test
    // expectation alone must not silently refresh this cached release set.
    for (version, platforms) in [
        ("0.31.0", [true, true, true, true]),
        ("0.30.0", [true, true, true, true]),
        ("0.29.0", [true, true, true, true]),
        ("0.28.0", [true, true, true, false]),
        ("0.26.0", [true, true, true, false]),
        ("0.25.0", [true, true, true, false]),
        ("0.24.0", [true, true, true, false]),
        ("0.23.0", [false, true, true, false]),
        ("0.22.0", [true, false, true, false]),
    ] {
        let asset = serde_json::json!({
            "url": "https://example.invalid/adr226/asset",
            "browser_download_url": "https://example.invalid/adr226/compiler.zip",
            "id": 226,
            "node_id": "ADR226-synthetic",
            "name": "synthetic-compiler.zip",
            "label": null,
            "state": "uploaded",
            "content_type": "application/zip",
            "size": 0,
            "download_count": 0,
            "created_at": "2026-10-06T00:00:00Z",
            "updated_at": "2026-10-06T00:00:00Z",
            "uploader": null,
        });
        let mut compiler = serde_json::Map::new();
        compiler.insert("version".to_string(), serde_json::json!(version));
        for (platform, present) in ["x86_macos", "aarch64_macos", "x86_linux", "aarch64_linux"]
            .into_iter()
            .zip(platforms)
        {
            compiler.insert(
                platform.to_string(),
                if present {
                    asset.clone()
                } else {
                    serde_json::Value::Null
                },
            );
        }
        compilers.insert(version.to_string(), compiler.into());
    }
    compilers.into()
}
