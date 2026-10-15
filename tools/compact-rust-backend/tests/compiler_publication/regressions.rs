//! Output order/plain Serde regression controls through real fixture processes.
//! Provided origins belong to the configured fixture, not a genuine frontend claim.
use super::*;

#[test]
fn rustc_malformed_ir_keeps_the_exact_plain_serde_error() {
    let malformed = b"{\"schema_version\":20,\"circuits\":";
    let sandbox = Sandbox::new(malformed, br#"{"circuits":[]}"#);
    let output = sandbox.prior_output("small");
    let expected = serde_json::from_slice::<Contract>(malformed)
        .unwrap_err()
        .to_string();
    let result = sandbox.command(false, &output).output().unwrap();
    assert!(!result.status.success());
    assert_eq!(stderr(&result), format!("compact-rustc: {expected}\n"));
    sandbox.assert_old_output_and_clean_stages(&output);
}

fn provided_type_metadata(ir: &Value, source: &Path) -> Vec<u8> {
    fn address(rows: &mut Vec<Value>, value: &Value, parent: Option<usize>, edge: Value) {
        let id = rows.len();
        rows.push(json!([
            parent,
            edge,
            value.get("kind").and_then(Value::as_str),
            0
        ]));
        match value {
            Value::Object(fields) => {
                for (field, value) in fields {
                    address(rows, value, Some(id), json!({"field":field}));
                }
            }
            Value::Array(elements) => {
                for (index, value) in elements.iter().enumerate() {
                    address(rows, value, Some(id), json!({"index":index}));
                }
            }
            _ => {}
        }
    }
    let mut addresses = Vec::new();
    address(&mut addresses, ir, None, json!({"root":true}));
    // Exact raw3 fixture shape used by the maintained library origin tests.
    // The source file actually contains this one-character physical extent.
    serde_json::to_vec(&json!({
        "schema_version":3,"coordinate_system":"compact-decoded-source","end_exclusive":true,
        "files":[{"id":0,"path":source}],
        "spans":[[0,1,1,0,1,2,1,{"kind":"physical_source"}]],
        "origins":[[0,"provided","type"]],"addresses":addresses
    }))
    .unwrap()
}

#[test]
fn recording_primary_error_precedes_issued_provided_range_notes() {
    let ir = json!({
        "schema_version":20,
        "ledger_fields":[{"id":"round","index":0,"path":[0],"declaration":{"kind":"counter"}}],
        "circuits":[],
        "stateful_circuits":[{
            "name":"probe",
            "parameters":[{"name":"oversized","ty":{"kind":"vector","element":{"kind":"field"},"length":32769}}],
            "actions":[],"result":{"kind":"unsigned","max":"18446744073709551615"},
            "return_value":{"kind":"counter_read","field":"round","index":0}
        }]
    });
    let sandbox = Sandbox::new(
        &serde_json::to_vec(&ir).unwrap(),
        br#"{"circuits":[{"name":"probe","proof":true}]}"#,
    );
    fs::write(
        &sandbox.origins,
        provided_type_metadata(&ir, &sandbox.source),
    )
    .unwrap();
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
    assert!(
        error.starts_with("compactc: --rust-require-recording failed:\n"),
        "{error}"
    );
    let primary = error
        .find("unknown source: exported circuit \"probe\" has no complete recorded API:")
        .expect("original primary refusal text");
    let notes = error
        .find("compactc: recording gap for \"probe\": source range")
        .expect("actual issued recording primary range");
    assert!(primary < notes, "{error}");
    assert!(error[notes..].contains("provided/type"), "{error}");
    assert!(
        error[notes..].contains("line 1 char 1 to line 1 char 2"),
        "{error}"
    );
    assert!(error[notes..].contains("end exclusive"), "{error}");
    assert!(
        !error.contains("origin metadata protocol problem"),
        "{error}"
    );
    sandbox.assert_old_output_and_clean_stages(&output);
}
