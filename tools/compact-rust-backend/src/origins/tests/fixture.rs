use crate::{ConfiguredFrontend, FrontendArtifacts, FrontendRequest};
use serde_json::{Value, json};
use std::{
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

static NEXT_SANDBOX: AtomicU64 = AtomicU64::new(0);

pub(super) struct Sandbox {
    pub root: PathBuf,
    pub source: PathBuf,
    pub executable: PathBuf,
}
impl Sandbox {
    pub(super) fn new(ir: &Value, metadata: Option<&[u8]>, info: Option<&[u8]>, exit: i32) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let mut created = None;
        for _ in 0..64 {
            let sequence = NEXT_SANDBOX.fetch_add(1, Ordering::Relaxed);
            let root = std::env::temp_dir().join(format!(
                "compact-origins-composition-{}-{nonce}-{sequence}",
                std::process::id()
            ));
            match fs::create_dir(&root) {
                Ok(()) => {
                    created = Some(root);
                    break;
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => panic!("cannot create test sandbox {}: {error}", root.display()),
            }
        }
        let root = created.expect("exclusive test sandbox allocation exhausted 64 attempts");
        let source = root.join("source with spaces.compact");
        fs::write(&source, "x").unwrap();
        fs::write(root.join("ir.json"), serde_json::to_vec(ir).unwrap()).unwrap();
        let mut script = format!(
            "#!/bin/sh\nset -eu\nfor arg do out=\"$arg\"; done\nprintf '%s\\n' \"$@\" > {}\nmkdir -p \"$out/contract\" \"$out/compiler\"\ncp {} \"$out/contract/compact-rust-ir.json\"\n",
            quote(&root.join("args.txt")),
            quote(&root.join("ir.json"))
        );
        if let Some(metadata) = metadata {
            fs::write(root.join("origins.json"), metadata).unwrap();
            script.push_str(&format!(
                "cp {} \"$out/contract/compact-rust-origins.raw.json\"\n",
                quote(&root.join("origins.json"))
            ));
        }
        if let Some(info) = info {
            fs::write(root.join("info.json"), info).unwrap();
            script.push_str(&format!(
                "cp {} \"$out/compiler/contract-info.json\"\n",
                quote(&root.join("info.json"))
            ));
        }
        script.push_str(&format!("exit {exit}\n"));
        let executable = root.join("configured frontend.sh");
        fs::write(&executable, script).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
        }
        Self {
            root,
            source,
            executable,
        }
    }
    pub(super) fn invoke(&self) -> FrontendArtifacts {
        ConfiguredFrontend::new(self.executable.as_os_str())
            .invoke(self.request())
            .unwrap()
    }
    pub(super) fn request(&self) -> FrontendRequest {
        FrontendRequest {
            source: self.source.clone(),
            arguments: vec![OsString::from("--skip-zk")],
            stage_parent: self.root.clone(),
        }
    }
}
impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
fn quote(path: &Path) -> String {
    format!("'{}'", path.to_str().unwrap().replace('\'', "'\\''"))
}

pub(super) fn empty_ir() -> Value {
    json!({"schema_version":20,"ledger_fields":[],"circuits":[],"stateful_circuits":[]})
}

pub(super) fn pure(body: Value, result: Value) -> Value {
    json!({"schema_version":20,"ledger_fields":[],"circuits":[{"name":"probe","parameters":[],"result":result,"body":body}],"stateful_circuits":[]})
}

pub(super) fn metadata(ir: &Value, path: &Path, context: bool) -> Vec<u8> {
    fn address(rows: &mut Vec<Value>, value: &Value, parent: Option<usize>, edge: Value) {
        let id = rows.len();
        rows.push(json!([
            parent,
            edge,
            value.get("kind").and_then(Value::as_str),
            0
        ]));
        match value {
            Value::Object(map) => {
                for (field, value) in map {
                    address(rows, value, Some(id), json!({"field":field}));
                }
            }
            Value::Array(array) => {
                for (index, value) in array.iter().enumerate() {
                    address(rows, value, Some(id), json!({"index":index}));
                }
            }
            _ => {}
        }
    }
    let mut rows = Vec::new();
    address(&mut rows, ir, None, json!({"root":true}));
    let extent = if context {
        json!({"kind":"compiler_context","reason":"program_virtual_eof","attachable":false})
    } else {
        json!({"kind":"physical_source"})
    };
    serde_json::to_vec(&json!({"schema_version":3,"coordinate_system":"compact-decoded-source","end_exclusive":true,
        "files":[{"id":0,"path":path.to_str().unwrap()}],"spans":[[0,1,1,0,1,2,1,extent]],
        "origins":[[0,"provided",if context { "program" } else { "expression" }]],"addresses":rows})).unwrap()
}
