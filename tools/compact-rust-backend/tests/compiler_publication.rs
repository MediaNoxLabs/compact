//! Live configured fixture processes; no arbitrary-byte invocation authority.
//! These exercise CLI publication/priority, not genuine Compact range coverage.
#![cfg(unix)]
#[path = "compiler_publication/regressions.rs"]
mod regressions;
use compact_rust_backend::{ConfiguredFrontend, FrontendRequest, RenderOptions, ir::Contract};
use serde_json::{Value, json};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicUsize, Ordering},
};

struct Sandbox {
    root: PathBuf,
    source: PathBuf,
    compiler: PathBuf,
    ir: PathBuf,
    info: PathBuf,
    origins: PathBuf,
}
impl Sandbox {
    fn new(ir: &[u8], info: &[u8]) -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let root = std::env::temp_dir().join(format!(
            "compact-source-publication-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        let source = root.join("source.compact");
        fs::write(&source, "// configured fixture input\n").unwrap();
        let ir_path = root.join("ir.json");
        fs::write(&ir_path, ir).unwrap();
        let info_path = root.join("info.json");
        fs::write(&info_path, info).unwrap();
        let origins = root.join("origins.json");
        fs::write(&origins, b"[]").unwrap();
        let compiler = root.join("frontend");
        fs::write(
            &compiler,
            r#"#!/bin/sh
set -eu
for arg in "$@"; do output="$arg"; done
if [ "${PUBLICATION_EXIT:-0}" != 0 ]; then exit "$PUBLICATION_EXIT"; fi
mkdir -p "$output/contract" "$output/compiler"
if [ "${PUBLICATION_NO_IR:-0}" != 1 ]; then
  cp "$PUBLICATION_IR" "$output/contract/compact-rust-ir.json"
fi
cp "$PUBLICATION_INFO" "$output/compiler/contract-info.json"
printf '{}' > "$output/compiler/contract-manifest.json"
if [ "${PUBLICATION_BAD_ORIGINS:-0}" = 1 ]; then
  cp "$PUBLICATION_ORIGINS" "$output/contract/compact-rust-origins.raw.json"
fi
"#,
        )
        .unwrap();
        fs::set_permissions(&compiler, fs::Permissions::from_mode(0o700)).unwrap();
        Self {
            root,
            source,
            compiler,
            ir: ir_path,
            info: info_path,
            origins,
        }
    }
    fn command(&self, compactc: bool, output: &Path) -> Command {
        let mut command = Command::new(if compactc {
            env!("CARGO_BIN_EXE_compactc")
        } else {
            env!("CARGO_BIN_EXE_compact-rustc")
        });
        if compactc {
            command
                .args(["--target", "rust", "--rust-runtime-root"])
                .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
                .env("COMPACTC_SCHEME", &self.compiler);
        } else {
            command.env("COMPACTC", &self.compiler);
        }
        command
            .arg(&self.source)
            .arg(output)
            .env("PUBLICATION_IR", &self.ir)
            .env("PUBLICATION_INFO", &self.info)
            .env("PUBLICATION_ORIGINS", &self.origins);
        command
    }
    fn prior_output(&self, name: &str) -> PathBuf {
        let output = self.root.join(name);
        fs::create_dir_all(output.join("contract")).unwrap();
        fs::write(output.join("contract/lib.rs"), b"previous complete output").unwrap();
        output
    }
    fn assert_old_output_and_clean_stages(&self, output: &Path) {
        assert_eq!(
            fs::read(output.join("contract/lib.rs")).unwrap(),
            b"previous complete output"
        );
        assert!(fs::read_dir(&self.root).unwrap().all(|entry| {
            !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .contains("compactc-stage-")
        }));
    }
}
impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
fn empty() -> Vec<u8> {
    serde_json::to_vec(
        &json!({"schema_version":20,"ledger_fields":[],"circuits":[],"stateful_circuits":[]}),
    )
    .unwrap()
}
fn unknown_parameter() -> Vec<u8> {
    serde_json::to_vec(&json!({"schema_version":20,"ledger_fields":[],"stateful_circuits":[],"circuits":[{"name":"probe","parameters":[],"result":{"kind":"field"},"body":{"kind":"parameter","name":"missing_original"}}]})).unwrap()
}
fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).unwrap()
}

#[test]
fn both_source_clis_replace_complete_output_after_success_and_cleanup_refusals() {
    let good = empty();
    let sandbox = Sandbox::new(&good, br#"{"circuits":[]}"#);
    let expected = compact_rust_backend::render(&serde_json::from_slice(&good).unwrap()).unwrap();
    for compactc in [false, true] {
        let output = sandbox.prior_output(if compactc { "public" } else { "small" });
        let result = sandbox.command(compactc, &output).output().unwrap();
        assert!(result.status.success(), "{}", stderr(&result));
        assert_eq!(
            fs::read_to_string(output.join("contract/lib.rs")).unwrap(),
            expected
        );
        if compactc {
            let capabilities: Value = serde_json::from_slice(
                &fs::read(output.join("contract/rust-capabilities.json")).unwrap(),
            )
            .unwrap();
            assert_eq!(capabilities["schema_version"], 3);
            assert!(output.join("contract/Cargo.toml").is_file());
        }
    }
    fs::write(&sandbox.ir, unknown_parameter()).unwrap();
    for compactc in [false, true] {
        let output = sandbox.prior_output(if compactc { "public-bad" } else { "small-bad" });
        let result = sandbox.command(compactc, &output).output().unwrap();
        assert!(!result.status.success());
        assert!(stderr(&result).contains("missing_original"));
        sandbox.assert_old_output_and_clean_stages(&output);
    }
}

#[test]
fn malformed_origins_never_mask_semantics_but_block_successful_publication() {
    let sandbox = Sandbox::new(&unknown_parameter(), br#"{"circuits":[]}"#);
    for compactc in [false, true] {
        let output = sandbox.prior_output(if compactc { "public" } else { "small" });
        let result = sandbox
            .command(compactc, &output)
            .env("PUBLICATION_BAD_ORIGINS", "1")
            .output()
            .unwrap();
        assert!(!result.status.success());
        assert!(stderr(&result).contains("missing_original"));
        sandbox.assert_old_output_and_clean_stages(&output);
        fs::write(&sandbox.ir, empty()).unwrap();
        let result = sandbox
            .command(compactc, &output)
            .env("PUBLICATION_BAD_ORIGINS", "1")
            .output()
            .unwrap();
        assert!(!result.status.success());
        assert!(stderr(&result).contains("origin metadata protocol problem"));
        sandbox.assert_old_output_and_clean_stages(&output);
        fs::write(&sandbox.ir, unknown_parameter()).unwrap();
    }
}

#[test]
fn frontend_exit_codes_and_missing_fresh_ir_do_not_reuse_previous_output() {
    let sandbox = Sandbox::new(&empty(), br#"{"circuits":[]}"#);
    for compactc in [false, true] {
        let output = sandbox.prior_output(if compactc { "public" } else { "small" });
        let result = sandbox
            .command(compactc, &output)
            .env("PUBLICATION_EXIT", "7")
            .output()
            .unwrap();
        assert_eq!(result.status.code(), Some(if compactc { 7 } else { 1 }));
        sandbox.assert_old_output_and_clean_stages(&output);
        let result = sandbox
            .command(compactc, &output)
            .env("PUBLICATION_NO_IR", "1")
            .output()
            .unwrap();
        assert!(!result.status.success());
        sandbox.assert_old_output_and_clean_stages(&output);
    }
}

#[test]
fn compactc_retains_ir_then_info_then_semantic_failure_priority() {
    let bad_ir = b"{\"schema_version\":20,\"circuits\":";
    let bad_info = b"broken proof information";
    let sandbox = Sandbox::new(bad_ir, bad_info);
    let output = sandbox.prior_output("public");
    let result = sandbox.command(true, &output).output().unwrap();
    let expected = serde_json::from_slice::<Contract>(bad_ir)
        .unwrap_err()
        .to_string();
    assert!(stderr(&result).contains(&expected), "{}", stderr(&result));
    sandbox.assert_old_output_and_clean_stages(&output);
    fs::write(&sandbox.ir, unknown_parameter()).unwrap();
    let result = sandbox.command(true, &output).output().unwrap();
    let expected = serde_json::from_slice::<Value>(bad_info)
        .unwrap_err()
        .to_string();
    assert!(stderr(&result).contains(&expected), "{}", stderr(&result));
    assert!(!stderr(&result).contains("missing_original"));
    sandbox.assert_old_output_and_clean_stages(&output);
}

#[test]
fn compactc_keeps_exact_recording_flag_failure_before_origin_protocol_failure() {
    let ir = serde_json::to_vec(&json!({"schema_version":20,"ledger_fields":[{"id":"round","index":0,"path":[0],"declaration":{"kind":"counter"}}],"circuits":[],"stateful_circuits":[{"name":"probe","parameters":[{"name":"oversized","ty":{"kind":"vector","element":{"kind":"field"},"length":32769}}],"actions":[],"result":{"kind":"unsigned","max":"18446744073709551615"},"return_value":{"kind":"counter_read","field":"round","index":0}}]})).unwrap();
    let sandbox = Sandbox::new(&ir, br#"{"circuits":[{"name":"probe","proof":true}]}"#);
    let output = sandbox.prior_output("public");
    let mut command = Command::new(env!("CARGO_BIN_EXE_compactc"));
    command
        .args([
            "--target",
            "rust",
            "--rust-require-recording",
            "--rust-runtime-root",
        ])
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
        .arg(&sandbox.source)
        .arg(&output)
        .env("COMPACTC_SCHEME", &sandbox.compiler)
        .env("PUBLICATION_IR", &sandbox.ir)
        .env("PUBLICATION_INFO", &sandbox.info)
        .env("PUBLICATION_ORIGINS", &sandbox.origins)
        .env("PUBLICATION_BAD_ORIGINS", "1");
    let result = command.output().unwrap();
    assert!(!result.status.success());
    let error = stderr(&result);
    assert!(error.contains("--rust-require-recording failed:\nunknown source: exported circuit \"probe\" has no complete recorded API:"),"{error}");
    assert!(!error.contains("origin metadata protocol problem"));
    sandbox.assert_old_output_and_clean_stages(&output);
}

#[test]
fn issued_pair_mutation_refuses_promotion_without_publishing_partial_entries() {
    let sandbox = Sandbox::new(&empty(), br#"{"circuits":[]}"#);
    // A live fixture process owns its outputs; no read-existing-artifacts API.
    let compiler = sandbox.root.join("fixed-frontend");
    let script = "#!/bin/sh\nset -eu\nir=\"$2\"\ninfo=\"$4\"\nfor arg in \"$@\"; do output=\"$arg\"; done\nmkdir -p \"$output/contract\" \"$output/compiler\"\ncp \"$ir\" \"$output/contract/compact-rust-ir.json\"\ncp \"$info\" \"$output/compiler/contract-info.json\"\n";
    fs::write(&compiler, script).unwrap();
    fs::set_permissions(&compiler, fs::Permissions::from_mode(0o700)).unwrap();
    for artifact in [
        "contract/compact-rust-ir.json",
        "compiler/contract-info.json",
    ] {
        let parent = sandbox.root.join(if artifact.starts_with("contract") {
            "ir-stage"
        } else {
            "info-stage"
        });
        fs::create_dir(&parent).unwrap();
        let issued = ConfiguredFrontend::new(compiler.as_os_str())
            .invoke(FrontendRequest {
                source: sandbox.source.clone(),
                arguments: vec![
                    "--fixture-ir".into(),
                    sandbox.ir.as_os_str().to_owned(),
                    "--fixture-info".into(),
                    sandbox.info.as_os_str().to_owned(),
                ],
                stage_parent: parent.clone(),
            })
            .unwrap();
        let rendered = issued.render(RenderOptions::default()).unwrap();
        assert_eq!(rendered.fingerprint().source_operand, sandbox.source);
        fs::write(
            issued.output_dir().join(artifact),
            b"changed after issuance",
        )
        .unwrap();
        assert!(issued.promote_into(&rendered, &parent).is_err());
        assert!(
            fs::read_dir(&parent).unwrap().next().is_none(),
            "failed invocation child cleaned; no contract/compiler promoted"
        );
    }
}
