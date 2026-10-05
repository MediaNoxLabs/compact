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

//! Closed, hash-pinned transport for genuine cargo-dist self-update tests.

use serde_json::{Value, json};
use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use tempfile::TempDir;

const MANIFEST: &str = include_str!("../fixtures/self_update/manifest.json");
const OLD: &str = "0.5.0";
const NEW: &str = "0.5.1";
const ARCHIVE_CONTENT: &str = "compact";

fn target() -> &'static str {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("macos", "aarch64") => "aarch64-apple-darwin",
        ("macos", "x86_64") => "x86_64-apple-darwin",
        ("linux", "x86_64") => "x86_64-unknown-linux-musl",
        other => panic!("no pinned Compact self-update assets for {other:?}"),
    }
}

fn sha256(path: &Path) -> String {
    let mut command = if cfg!(target_os = "macos") {
        let mut cmd = Command::new("/usr/bin/shasum");
        cmd.args(["-a", "256"]);
        cmd
    } else {
        Command::new("/usr/bin/sha256sum")
    };
    let output = command
        .arg(path)
        .output()
        .expect("execute system SHA256 tool");
    assert!(output.status.success(), "hash {path:?}: {output:?}");
    String::from_utf8(output.stdout)
        .expect("SHA256 output is UTF-8")
        .split_whitespace()
        .next()
        .expect("SHA256 digest")
        .to_owned()
}

fn verify_asset(path: &Path, spec: &Value) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("missing pinned asset {path:?}: {error}"))?;
    if !metadata.file_type().is_file() {
        return Err(format!("asset must be a regular file: {path:?}"));
    }
    if metadata.len() != spec["bytes"].as_u64().expect("pinned asset size") {
        return Err(format!("asset byte count differs from pin: {path:?}"));
    }
    if sha256(path) != spec["sha256"].as_str().expect("pinned asset SHA256") {
        return Err(format!("asset SHA256 differs from pin: {path:?}"));
    }
    Ok(())
}

struct Asset {
    path: PathBuf,
    bytes: Vec<u8>,
}

fn asset(root: &Path, version: &str, spec: &Value) -> Asset {
    let name = spec["name"].as_str().expect("pinned asset name");
    let path = root.join(format!("compact-v{version}")).join(name);
    verify_asset(&path, spec).unwrap_or_else(|error| panic!("{error}"));
    Asset {
        bytes: fs::read(&path).expect("read pinned asset"),
        path,
    }
}

fn archived_executable(archive: &Path) -> Vec<u8> {
    let member = format!("compact-{}/{ARCHIVE_CONTENT}", target());
    let output = Command::new("/usr/bin/tar")
        .args([
            "-xOf",
            archive.to_str().expect("UTF-8 archive path"),
            &member,
        ])
        .output()
        .expect("extract pinned archive executable");
    assert!(output.status.success(), "extract {member}: {output:?}");
    output.stdout
}

struct LocalServer {
    base: String,
    requests: Arc<Mutex<Vec<String>>>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl LocalServer {
    fn new(mut routes: HashMap<String, Vec<u8>>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind private release server");
        listener
            .set_nonblocking(true)
            .expect("nonblocking release server");
        let base = format!("http://{}", listener.local_addr().expect("server address"));
        let latest = json!({
            "tag_name": "compact-v0.5.1",
            "name": "compact 0.5.1",
            "url": format!("{base}/api/v3/repos/midnightntwrk/compact/releases/tags/compact-v0.5.1"),
            "prerelease": false,
            "assets": [{
                "name": "compact-installer.sh",
                "url": format!("{base}/assets/compact-v0.5.1/compact-installer.sh"),
                "browser_download_url": format!("{base}/compact-v0.5.1/compact-installer.sh")
            }]
        });
        routes.insert(
            "/api/v3/repos/midnightntwrk/compact/releases/latest".to_owned(),
            serde_json::to_vec(&latest).expect("local release metadata JSON"),
        );
        let requests = Arc::new(Mutex::new(Vec::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let thread_requests = Arc::clone(&requests);
        let thread_stop = Arc::clone(&stop);
        let thread = thread::spawn(move || {
            while !thread_stop.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((stream, _)) => serve(stream, &routes, &thread_requests),
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(10));
                    }
                    Err(error) => panic!("release server accept: {error}"),
                }
            }
        });
        Self {
            base,
            requests,
            stop,
            thread: Some(thread),
        }
    }

    fn routes(&self) -> Vec<String> {
        self.requests.lock().expect("request ledger").clone()
    }
}

impl Drop for LocalServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            thread.join().expect("release server thread");
        }
    }
}

fn serve(mut stream: TcpStream, routes: &HashMap<String, Vec<u8>>, requests: &Mutex<Vec<String>>) {
    stream
        .set_nonblocking(false)
        .expect("accepted release connection is blocking");
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .expect("server read timeout");
    stream
        .set_write_timeout(Some(Duration::from_secs(10)))
        .expect("server write timeout");
    let mut request = Vec::new();
    let mut buffer = [0u8; 1024];
    while request.len() < 16 * 1024 && !request.ends_with(b"\r\n\r\n") {
        match stream.read(&mut buffer) {
            Ok(0) => break,
            Ok(length) => request.extend_from_slice(&buffer[..length]),
            Err(_) => break,
        }
    }
    let line = String::from_utf8_lossy(&request)
        .lines()
        .next()
        .unwrap_or_default()
        .to_owned();
    let mut words = line.split_whitespace();
    let method = words.next().unwrap_or("");
    let path = words.next().unwrap_or("");
    requests
        .lock()
        .expect("request ledger")
        .push(format!("{method} {path}"));
    let (status, body) = match (method, routes.get(path)) {
        ("GET", Some(body)) => ("200 OK", body.as_slice()),
        _ => (
            "404 Not Found",
            b"closed historical release route".as_slice(),
        ),
    };
    let header = format!(
        "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream
        .write_all(header.as_bytes())
        .expect("release response header");
    stream
        .write_all(body)
        .expect("complete pinned release response");
}

fn bounded(mut command: Command, what: &str) -> Output {
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|error| panic!("start {what}: {error}"));
    let start = Instant::now();
    loop {
        if child.try_wait().expect("poll child").is_some() {
            return child.wait_with_output().expect("read child output");
        }
        if start.elapsed() > Duration::from_secs(120) {
            child.kill().expect("kill timed-out child");
            child.wait().expect("reap timed-out child");
            panic!("{what} exceeded 120 seconds");
        }
        thread::sleep(Duration::from_millis(50));
    }
}

pub struct SelfUpdateFixture {
    private: TempDir,
    binary: PathBuf,
    receipt: PathBuf,
    old_receipt: Value,
    old_executable: Vec<u8>,
    new_executable: Vec<u8>,
    server: LocalServer,
    host_files: Vec<(PathBuf, Option<String>)>,
}

impl SelfUpdateFixture {
    pub fn new() -> Self {
        let root = PathBuf::from(std::env::var("COMPACT_SELF_ASSETS_DIR").expect(
            "COMPACT_SELF_ASSETS_DIR is required; run tests/fixtures/self_update/acquire.py first",
        ));
        let manifest: Value =
            serde_json::from_str(MANIFEST).expect("historical asset manifest JSON");
        assert_eq!(manifest["format"], "compact-self-update-assets/v1");
        assert_eq!(manifest["repository"], "midnightntwrk/compact");
        let old = &manifest["releases"][OLD];
        let new = &manifest["releases"][NEW];
        let old_script = asset(&root, OLD, &old["installer"]);
        let old_archive = asset(&root, OLD, &old["archives"][target()]);
        let new_script = asset(&root, NEW, &new["installer"]);
        let new_archive = asset(&root, NEW, &new["archives"][target()]);
        let old_executable = archived_executable(&old_archive.path);
        let new_executable = archived_executable(&new_archive.path);
        assert_ne!(
            old_executable, new_executable,
            "historical releases must differ"
        );

        let private = tempfile::tempdir().expect("private self-update home");
        let home = private.path();
        for folder in [
            ".local/bin",
            ".config/compact",
            ".cache",
            ".local/share",
            "tmp",
        ] {
            fs::create_dir_all(home.join(folder)).expect("private fixture directory");
        }
        let binary = home.join(".local/bin/compact");
        let receipt = home.join(".config/compact/compact-receipt.json");
        let host_files = host_files();
        let server = release_server(old_archive, new_script, new_archive);
        let fixture = Self {
            private,
            binary,
            receipt,
            old_receipt: Value::Null,
            old_executable,
            new_executable,
            server,
            host_files,
        };
        let mut installer = fixture.command("/bin/sh", OLD);
        installer.arg(&old_script.path);
        let output = bounded(installer, "pinned 0.5.0 installer");
        assert!(output.status.success(), "old installer: {output:?}");
        assert_eq!(
            fs::read(&fixture.binary).expect("old installed binary"),
            fixture.old_executable
        );
        let old_receipt: Value =
            serde_json::from_slice(&fs::read(&fixture.receipt).expect("old receipt"))
                .expect("old cargo-dist receipt JSON");
        fixture.assert_receipt(&old_receipt, OLD);
        let version = fixture.invoke(OLD, &["--version"]);
        assert_eq!(
            String::from_utf8_lossy(&version.stdout).trim(),
            "compact 0.5.0"
        );
        assert!(version.status.success(), "old version: {version:?}");
        let mut fixture = fixture;
        fixture.old_receipt = old_receipt;
        fixture
    }

    pub fn home(&self) -> &Path {
        self.private.path()
    }

    pub fn check(&self) -> Output {
        let output = self.invoke(
            NEW,
            &[
                "--directory",
                self.home().to_str().expect("UTF-8 home"),
                "self",
                "check",
            ],
        );
        assert!(output.status.success(), "historical self check: {output:?}");
        assert!(String::from_utf8_lossy(&output.stdout).contains("Update available -- 0.5.1"));
        self.assert_routes(false);
        self.assert_host_unchanged();
        output
    }

    pub fn update(&self) -> Output {
        let output = self.invoke(
            NEW,
            &[
                "--directory",
                self.home().to_str().expect("UTF-8 home"),
                "self",
                "update",
            ],
        );
        assert!(
            output.status.success(),
            "historical self update: {output:?}"
        );
        assert_eq!(
            fs::read(&self.binary).expect("updated binary"),
            self.new_executable
        );
        let new_receipt: Value =
            serde_json::from_slice(&fs::read(&self.receipt).expect("updated receipt"))
                .expect("updated cargo-dist receipt JSON");
        self.assert_receipt(&new_receipt, NEW);
        assert_eq!(new_receipt["source"], self.old_receipt["source"]);
        assert_eq!(new_receipt["provider"], self.old_receipt["provider"]);
        assert_eq!(
            new_receipt["install_prefix"],
            self.old_receipt["install_prefix"]
        );
        let version = self.invoke(NEW, &["--version"]);
        assert!(version.status.success(), "new version: {version:?}");
        assert_eq!(
            String::from_utf8_lossy(&version.stdout).trim(),
            "compact 0.5.1"
        );
        self.assert_routes(true);
        self.assert_host_unchanged();
        output
    }

    pub fn assert_up_to_date(&self) {
        let output = self.invoke(
            NEW,
            &[
                "--directory",
                self.home().to_str().expect("UTF-8 home"),
                "self",
                "check",
            ],
        );
        assert!(output.status.success(), "updated self check: {output:?}");
        assert!(
            String::from_utf8_lossy(&output.stdout).contains("0.5.1 -- Up to date"),
            "updated self check: {output:?}"
        );
        assert!(
            output.stderr.is_empty(),
            "updated self check stderr: {output:?}"
        );
        self.assert_routes(true);
        self.assert_host_unchanged();
        let mut files = vec![self.binary.clone(), self.receipt.clone()];
        let mut actual = Vec::new();
        visit_files(self.home(), &mut actual);
        actual.sort();
        files.sort();
        assert_eq!(
            actual, files,
            "unexpected writes in private self-update home"
        );
    }

    fn assert_receipt(&self, receipt: &Value, version: &str) {
        assert_eq!(receipt["version"], version);
        assert_eq!(receipt["source"]["owner"], "midnightntwrk");
        assert_eq!(receipt["source"]["name"], "compact");
        assert_eq!(receipt["source"]["app_name"], "compact");
        assert_eq!(receipt["source"]["release_type"], "github");
        assert_eq!(receipt["provider"]["source"], "cargo-dist");
        assert_eq!(
            receipt["install_prefix"],
            self.binary
                .parent()
                .expect("install directory")
                .to_str()
                .expect("UTF-8 install directory")
        );
        assert_eq!(receipt["modify_path"], false);
        assert_eq!(receipt["binaries"], json!(["compact"]));
    }

    fn assert_host_unchanged(&self) {
        for (path, original) in &self.host_files {
            assert_eq!(
                &optional_sha256(path),
                original,
                "host installation changed: {path:?}"
            );
        }
    }

    fn invoke(&self, download_version: &str, args: &[&str]) -> Output {
        let mut command = self.command(&self.binary, download_version);
        command.args(args);
        bounded(command, "private historical compact")
    }

    fn command(&self, program: impl AsRef<std::ffi::OsStr>, download_version: &str) -> Command {
        let home = self.home();
        let base = &self.server.base;
        let mut command = Command::new(program);
        command.env_clear();
        command.current_dir(home);
        command.env("HOME", home);
        command.env(
            "USER",
            std::env::var("USER").unwrap_or_else(|_| "compact-test".to_owned()),
        );
        // The existing Intel macOS snapshot covers cargo-dist's honest
        // missing-sha256sum branch. Keep that platform's child PATH to real
        // system user tools; /sbin/sha256sum on newer macOS would otherwise
        // silently switch the observed branch. The asset cache was already
        // verified independently of the installer on every platform.
        let system_path = if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
            "/usr/bin:/bin"
        } else {
            "/usr/bin:/bin:/usr/sbin:/sbin"
        };
        command.env("PATH", system_path);
        command.env("TMPDIR", home.join("tmp"));
        command.env("XDG_CONFIG_HOME", home.join(".config"));
        command.env("XDG_CACHE_HOME", home.join(".cache"));
        command.env("XDG_DATA_HOME", home.join(".local/share"));
        command.env("XDG_BIN_HOME", home.join(".local/bin"));
        command.env("CARGO_HOME", home.join(".cargo"));
        command.env("COMPACT_INSTALL_DIR", home.join(".local/bin"));
        command.env("COMPACT_NO_MODIFY_PATH", "1");
        command.env("COMPACT_INSTALLER_GHE_BASE_URL", format!("{base}/"));
        command.env(
            "COMPACT_DOWNLOAD_URL",
            format!("{base}/compact-v{download_version}"),
        );
        command.env("AXOUPDATER_CONFIG_PATH", home.join(".config/compact"));
        for key in [
            "HTTP_PROXY",
            "HTTPS_PROXY",
            "ALL_PROXY",
            "http_proxy",
            "https_proxy",
            "all_proxy",
        ] {
            command.env(key, base);
        }
        command.env("NO_PROXY", "127.0.0.1,localhost");
        command.env("no_proxy", "127.0.0.1,localhost");
        command.env("LANG", "C");
        command.env("LC_ALL", "C");
        command.env("TERM", "dumb");
        command.env("RUST_BACKTRACE", "0");
        command
    }

    fn assert_routes(&self, updated: bool) {
        let requests = self.server.routes();
        let archive_name = format!("compact-{}.tar.xz", target());
        let old_archive = format!("GET /compact-v{OLD}/{archive_name}");
        let api = "GET /api/v3/repos/midnightntwrk/compact/releases/latest";
        let new_installer = format!("GET /compact-v{NEW}/compact-installer.sh");
        let new_archive = format!("GET /compact-v{NEW}/{archive_name}");
        assert_eq!(
            requests.first(),
            Some(&old_archive),
            "old installer route: {requests:?}"
        );
        assert!(
            requests.iter().any(|request| request == api),
            "metadata route: {requests:?}"
        );
        let allowed = [
            old_archive.as_str(),
            api,
            new_installer.as_str(),
            new_archive.as_str(),
        ];
        assert!(
            requests
                .iter()
                .all(|request| allowed.contains(&request.as_str())),
            "unapproved release/proxy route: {requests:?}"
        );
        if updated {
            let installer_position = requests
                .iter()
                .position(|request| request == &new_installer)
                .expect("new installer route");
            let archive_position = requests
                .iter()
                .position(|request| request == &new_archive)
                .expect("new archive route");
            assert!(
                installer_position > 1 && archive_position > installer_position,
                "update order: {requests:?}"
            );
            assert_eq!(
                requests
                    .iter()
                    .filter(|request| *request == &new_installer)
                    .count(),
                1
            );
            assert_eq!(
                requests
                    .iter()
                    .filter(|request| *request == &new_archive)
                    .count(),
                1
            );
        } else {
            assert!(!requests.contains(&new_installer));
            assert!(!requests.contains(&new_archive));
        }
    }
}

fn optional_sha256(path: &Path) -> Option<String> {
    path.is_file().then(|| sha256(path))
}

fn host_files() -> Vec<(PathBuf, Option<String>)> {
    let mut paths = Vec::new();
    if let Some(home) = std::env::var_os("HOME") {
        let home = PathBuf::from(home);
        paths.push(home.join(".local/bin/compact"));
        paths.push(home.join(".config/compact/compact-receipt.json"));
    }
    if let Some(config) = std::env::var_os("XDG_CONFIG_HOME") {
        paths.push(PathBuf::from(config).join("compact/compact-receipt.json"));
    }
    if let Some(path) = std::env::var_os("PATH")
        && let Some(binary) = std::env::split_paths(&path)
            .map(|directory| directory.join("compact"))
            .find(|binary| binary.is_file())
    {
        paths.push(binary);
    }
    paths.sort();
    paths.dedup();
    paths
        .into_iter()
        .map(|path| {
            let digest = optional_sha256(&path);
            (path, digest)
        })
        .collect()
}

fn visit_files(directory: &Path, actual: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(directory).expect("list private self-update home") {
        let entry = entry.expect("private directory entry");
        let file_type = entry.file_type().expect("private file type");
        assert!(
            !file_type.is_symlink(),
            "unexpected symlink in private self-update home"
        );
        if file_type.is_dir() {
            visit_files(&entry.path(), actual);
        } else {
            assert!(file_type.is_file(), "unexpected private file type");
            actual.push(entry.path());
        }
    }
}

pub fn assert_asset_validation_refusals() {
    let manifest: Value = serde_json::from_str(MANIFEST).expect("historical asset manifest JSON");
    let spec = &manifest["releases"][OLD]["installer"];
    let private = tempfile::tempdir().expect("private corrupt asset dir");
    let path = private.path().join("compact-installer.sh");
    assert!(
        verify_asset(&path, spec).is_err(),
        "missing asset must refuse"
    );
    let original = PathBuf::from(std::env::var("COMPACT_SELF_ASSETS_DIR").expect("asset cache"))
        .join(format!("compact-v{OLD}/compact-installer.sh"));
    verify_asset(&original, spec).expect("original pinned installer");
    let mut changed = fs::read(original).expect("read pinned installer");
    changed[0] ^= 1;
    fs::write(&path, changed).expect("write same-size corrupted asset");
    assert!(
        verify_asset(&path, spec).is_err(),
        "same-size changed SHA256 must refuse"
    );
}

fn release_server(old_archive: Asset, new_script: Asset, new_archive: Asset) -> LocalServer {
    let mut routes = HashMap::new();
    let archive_name = format!("compact-{}.tar.xz", target());
    routes.insert(format!("/compact-v{OLD}/{archive_name}"), old_archive.bytes);
    routes.insert(
        format!("/compact-v{NEW}/compact-installer.sh"),
        new_script.bytes,
    );
    routes.insert(format!("/compact-v{NEW}/{archive_name}"), new_archive.bytes);
    // The owner/repository identity remains that of the original release.
    // Only the transport host is loopback.
    LocalServer::new(routes)
}
