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

//! Writable installer fixture backed by verified genuine archive bytes.
//! Catalogue isolation is separate from ADR226's read-only no-install invariant.

use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread::{self, JoinHandle};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub struct ArchiveFixture {
    root: tempfile::TempDir,
    catalogue: serde_json::Value,
    cache_bytes: Vec<u8>,
    caches: Vec<PathBuf>,
    server: ArchiveServer,
}

impl ArchiveFixture {
    pub fn new() -> Self {
        let cache = std::env::var_os("COMPACT_TEST_ARCHIVE_CACHE").expect(
            "COMPACT_TEST_ARCHIVE_CACHE must name explicitly acquired real compiler archives",
        );
        Self::from_cache(Path::new(&cache))
            .expect("genuine compiler archive fixture validation failed")
    }

    pub fn from_cache(cache: &Path) -> Result<Self, String> {
        let platform = match (std::env::consts::OS, std::env::consts::ARCH) {
            ("macos", "aarch64") => "aarch64-darwin",
            ("macos", "x86_64") => "x86_64-apple-darwin",
            ("linux", "x86_64") => "x86_64-unknown-linux-musl",
            ("linux", "aarch64") => "aarch64-unknown-linux-musl",
            _ => return Err("unsupported archive fixture platform".into()),
        };
        let helper =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/acquire_compilers.py");
        let checked = Command::new("python3")
            .args([helper.as_os_str(), "--cache".as_ref(), cache.as_os_str()])
            .args(["--platform", platform, "--verify-only"])
            .env("PYTHONDONTWRITEBYTECODE", "1")
            .output()
            .map_err(|e| e.to_string())?;
        if !checked.status.success() {
            return Err(String::from_utf8_lossy(&checked.stderr).into_owned());
        }
        let receipt: serde_json::Value =
            serde_json::from_slice(&checked.stdout).map_err(|e| e.to_string())?;
        let mut archives = BTreeMap::new();
        for row in receipt["archives"]
            .as_array()
            .ok_or("missing verified archives")?
        {
            let path = row["path"].as_str().ok_or("missing archive path")?;
            let id = row["asset_id"].as_u64().ok_or("missing asset id")?;
            archives.insert(
                format!("/assets/{id}"),
                fs::read(path).map_err(|e| e.to_string())?,
            );
        }
        let server = ArchiveServer::new(archives);
        let manifest: serde_json::Value =
            serde_json::from_str(include_str!("../fixtures/compiler-archives.json"))
                .map_err(|e| e.to_string())?;
        let mut compilers = serde_json::Map::new();
        for release in manifest["catalogue"]
            .as_array()
            .ok_or("missing catalogue")?
        {
            let version = release["version"].as_str().ok_or("missing version")?;
            let mut compiler = serde_json::Map::new();
            compiler.insert("version".into(), version.into());
            for (field, platform) in [
                ("aarch64_macos", "aarch64-darwin"),
                ("x86_macos", "x86_64-apple-darwin"),
                ("x86_linux", "x86_64-unknown-linux-musl"),
                ("aarch64_linux", "aarch64-unknown-linux-musl"),
            ] {
                let mut asset = release["platforms"][platform].clone();
                if !asset.is_null() {
                    asset["browser_download_url"] =
                        format!("{}/assets/{}", server.origin, asset["id"]).into();
                }
                compiler.insert(field.into(), asset);
            }
            compilers.insert(version.into(), compiler.into());
        }
        let root = tempfile::tempdir().map_err(|e| e.to_string())?;
        for directory in ["home", "cache", "config", "data", "compact", "tmp"] {
            fs::create_dir_all(root.path().join(directory)).map_err(|e| e.to_string())?;
        }
        let caches = vec![
            root.path().join("cache/compactc/github_cache.json"),
            root.path()
                .join("home/Library/Caches/compactc/github_cache.json"),
        ];
        Ok(Self {
            root,
            catalogue: compilers.into(),
            cache_bytes: vec![],
            caches,
            server,
        })
    }

    pub fn directory(&self) -> PathBuf {
        self.root.path().join("compact")
    }

    pub fn environment(&mut self) -> HashMap<String, String> {
        self.cache_bytes = serde_json::to_vec_pretty(&serde_json::json!({
            "artifacts": { "compilers": self.catalogue },
            "cached_at": SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            "rate_limit_remaining": null,
            "rate_limit_reset": null,
        }))
        .unwrap();
        for path in &self.caches {
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, &self.cache_bytes).unwrap();
        }
        let mut env = HashMap::new();
        for (key, directory) in [
            ("HOME", "home"),
            ("XDG_CACHE_HOME", "cache"),
            ("XDG_CONFIG_HOME", "config"),
            ("XDG_DATA_HOME", "data"),
            ("COMPACT_DIRECTORY", "compact"),
            ("AXOUPDATER_CONFIG_PATH", "config"),
            ("TMPDIR", "tmp"),
        ] {
            env.insert(
                key.into(),
                self.root.path().join(directory).to_str().unwrap().into(),
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
            env.insert(key.into(), self.server.origin.clone());
        }
        for key in ["NO_PROXY", "no_proxy"] {
            env.insert(key.into(), String::new());
        }
        env.insert("GITHUB_TOKEN".into(), "adr228-no-external-network".into());
        env
    }

    pub fn run(&mut self, args: &[&str]) -> Output {
        let output = Command::new(env!("CARGO_BIN_EXE_compact"))
            .env_clear()
            .env("PATH", std::env::var_os("PATH").unwrap_or_default())
            .env("RUST_BACKTRACE", "0")
            .envs(self.environment())
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .args(args)
            .output()
            .unwrap();
        self.assert_no_external_requests();
        output
    }

    pub fn assert_no_external_requests(&self) {
        assert_eq!(
            self.server.refused.load(Ordering::SeqCst),
            0,
            "installer attempted an unconfigured or external HTTP request"
        );
        for path in &self.caches {
            assert_eq!(
                fs::read(path).unwrap(),
                self.cache_bytes,
                "installer changed pinned release cache"
            );
        }
    }

    pub fn downloads(&self) -> usize {
        self.server.accepted.load(Ordering::SeqCst)
    }
}

struct ArchiveServer {
    origin: String,
    refused: Arc<AtomicUsize>,
    accepted: Arc<AtomicUsize>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl ArchiveServer {
    fn new(archives: BTreeMap<String, Vec<u8>>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let refused = Arc::new(AtomicUsize::new(0));
        let accepted = Arc::new(AtomicUsize::new(0));
        let stop = Arc::new(AtomicBool::new(false));
        let (r, a, s, allowed) = (
            Arc::clone(&refused),
            Arc::clone(&accepted),
            Arc::clone(&stop),
            origin.clone(),
        );
        let thread = thread::spawn(move || {
            while !s.load(Ordering::SeqCst) {
                let mut stream = match listener.accept() {
                    Ok((stream, _)) => stream,
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(5));
                        continue;
                    }
                    Err(e) => panic!("archive server accept: {e}"),
                };
                stream.set_nonblocking(false).unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                stream
                    .set_write_timeout(Some(Duration::from_secs(30)))
                    .unwrap();
                let mut header = Vec::new();
                let mut byte = [0];
                while header.len() < 16384 && !header.ends_with(b"\r\n\r\n") {
                    if stream.read_exact(&mut byte).is_err() {
                        break;
                    }
                    header.push(byte[0]);
                }
                let request = String::from_utf8_lossy(&header);
                let first: Vec<_> = request
                    .lines()
                    .next()
                    .unwrap_or("")
                    .split_whitespace()
                    .collect();
                // All child HTTP uses this proxy. Only its exact own origin and
                // pinned asset route can pass; CONNECT/external/unknown fail.
                let path = first.get(1).and_then(|url| url.strip_prefix(&allowed));
                let body = path
                    .filter(|p| p.starts_with('/'))
                    .and_then(|p| archives.get(p));
                if first.first() != Some(&"GET") || body.is_none() {
                    r.fetch_add(1, Ordering::SeqCst);
                    let _ = stream.write_all(b"HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
                    continue;
                }
                let body = body.unwrap();
                // A new production download may send Range: bytes=0-. This
                // fixture accepts only that full archive request, never a
                // misleading partial response presented as a complete ZIP.
                if request.lines().any(|line| {
                    line.to_ascii_lowercase().starts_with("range:")
                        && !line.eq_ignore_ascii_case("range: bytes=0-")
                }) {
                    r.fetch_add(1, Ordering::SeqCst);
                    let _ = stream.write_all(b"HTTP/1.1 416 Range Not Satisfiable\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
                    continue;
                }
                a.fetch_add(1, Ordering::SeqCst);
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: application/zip\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                let _ = stream
                    .write_all(response.as_bytes())
                    .and_then(|()| stream.write_all(body));
            }
        });
        Self {
            origin,
            refused,
            accepted,
            stop,
            thread: Some(thread),
        }
    }
}

impl Drop for ArchiveServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(thread) = self.thread.take() {
            let result = thread.join();
            if !thread::panicking() {
                result.unwrap();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpStream;

    #[test]
    fn transport_refuses_external_unknown_and_partial_requests() {
        let server = ArchiveServer::new(BTreeMap::from([(
            "/assets/1".into(),
            b"transport test bytes".to_vec(),
        )]));
        let request = |target: &str, range: &str| {
            let mut stream =
                TcpStream::connect(server.origin.trim_start_matches("http://")).unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            write!(
                stream,
                "GET {target} HTTP/1.1\r\nHost: irrelevant\r\n{range}Connection: close\r\n\r\n"
            )
            .unwrap();
            let mut response = String::new();
            stream.read_to_string(&mut response).unwrap();
            response
        };
        assert!(
            request(&format!("{}/assets/1", server.origin), "").ends_with("transport test bytes")
        );
        assert!(request("http://example.invalid/assets/1", "").starts_with("HTTP/1.1 503"));
        assert!(request(&format!("{}/assets/2", server.origin), "").starts_with("HTTP/1.1 503"));
        assert!(
            request(
                &format!("{}/assets/1", server.origin),
                "Range: bytes=5-\r\n"
            )
            .starts_with("HTTP/1.1 416")
        );
        assert_eq!(server.accepted.load(Ordering::SeqCst), 1);
        assert_eq!(server.refused.load(Ordering::SeqCst), 3);
    }
}
