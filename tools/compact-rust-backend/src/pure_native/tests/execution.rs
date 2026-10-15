// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Compiled controls expose observable order with instrumented runtime callees.
//! This tests emitted control syntax; it is not a Midnight proof/network test.
use super::*;
use std::{
    fs,
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};

struct Scratch(std::path::PathBuf);
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn condition_call(name: &str, value: Expr) -> Expr {
    Expr::Call {
        name: name.into(),
        arguments: vec![value],
    }
}
fn function(
    name: &str,
    signature: &str,
    source: &Expr,
    parameters: &HashMap<&str, (&Type, syn::Ident)>,
    calls: &HashMap<&str, &PureCircuit>,
) -> String {
    let (fragment, actual) = crate::native_fragment_with_calls(source, parameters, calls).unwrap();
    assert_eq!(actual, Type::Unit);
    let (_, statements) = fragment.into_views();
    format!(
        "fn {name}({signature}) -> Result<(), runtime::CompactError> {{ {} Ok(()) }}",
        quote::quote!(#(#statements)*)
    )
}

#[test]
fn compiled_control_keeps_condition_once_lazy_branches_first_failure_and_lexical_names() {
    let flag = Type::Boolean;
    let params = HashMap::from([
        ("flag", (&flag, syn::parse_quote!(flag))),
        ("inner_flag", (&flag, syn::parse_quote!(inner_flag))),
        (
            "__compact_condition",
            (&flag, syn::parse_quote!(__compact_condition)),
        ),
    ]);
    let mut declarations: Vec<PureCircuit> = ["left", "right", "first_fail", "after"]
        .into_iter()
        .map(|name| pure(name, Type::Unit))
        .collect();
    for name in ["probe", "inner"] {
        let mut declaration = pure(name, Type::Boolean);
        declaration.parameters = vec![Parameter {
            name: "value".into(),
            ty: Type::Boolean,
        }];
        declaration.body = reference("value");
        declarations.push(declaration);
    }
    declarations.push(pure("failing_probe", Type::Boolean));
    let calls = declarations
        .iter()
        .map(|source| (source.name.as_str(), source))
        .collect();
    let probe = || condition_call("probe", reference("flag"));
    let single = branch(probe(), call("left"), call("right"));
    let equal = branch(probe(), call("left"), call("left"));
    let failed_equal = branch(call("failing_probe"), call("left"), call("left"));
    let nested = branch(
        probe(),
        assertion(condition_call("inner", reference("inner_flag"))),
        Expr::Unit,
    );
    let unused = Expr::Sequence {
        steps: vec![call("first_fail"), call("after")],
        value: Box::new(Expr::Unit),
    };
    let lifted = branch(
        local(
            "saved",
            Type::Boolean,
            condition_call("probe", reference("__compact_condition")),
            reference("saved"),
        ),
        call("left"),
        call("right"),
    );
    let shadowed = Expr::Let {
        bindings: vec![
            LocalBinding {
                name: "flag".into(),
                ty: Type::Boolean,
                value: probe(),
            },
            LocalBinding {
                name: "flag".into(),
                ty: Type::Boolean,
                value: boolean(false),
            },
        ],
        body: Box::new(branch(reference("flag"), call("left"), call("right"))),
    };
    let methods = [
        function("single", "flag: bool", &single, &params, &calls),
        function("equal", "flag: bool", &equal, &params, &calls),
        function("failed_equal", "", &failed_equal, &params, &calls),
        function(
            "nested",
            "flag: bool, inner_flag: bool",
            &nested,
            &params,
            &calls,
        ),
        function("unused", "", &unused, &params, &calls),
        function(
            "lifted",
            "__compact_condition: bool",
            &lifted,
            &params,
            &calls,
        ),
        function("shadowed", "flag: bool", &shadowed, &params, &calls),
    ]
    .join("\n");
    assert!(methods.contains("__compact_condition_1"));
    let source = format!(
        r#"
mod runtime {{
    #[derive(Debug, PartialEq)] pub enum CompactError {{ AssertionFailed(String), Injected(&'static str) }}
}}
thread_local! {{ static EVENTS: std::cell::RefCell<Vec<u64>> = const {{ std::cell::RefCell::new(Vec::new()) }}; }}
fn record(value: u64) {{ EVENTS.with(|events| events.borrow_mut().push(value)); }}
fn events(expected: &[u64]) {{ EVENTS.with(|events| {{ assert_eq!(*events.borrow(), expected); events.borrow_mut().clear(); }}); }}
mod pure_circuits {{
    use super::runtime::CompactError;
    pub fn probe(value: bool) -> Result<bool, CompactError> {{ super::record(10); Ok(value) }}
    pub fn inner(value: bool) -> Result<bool, CompactError> {{ super::record(20); Ok(value) }}
    pub fn failing_probe() -> Result<bool, CompactError> {{ super::record(99); Err(CompactError::Injected("condition")) }}
    pub fn left() -> Result<(), CompactError> {{ super::record(1); Ok(()) }}
    pub fn right() -> Result<(), CompactError> {{ super::record(2); Ok(()) }}
    pub fn first_fail() -> Result<(), CompactError> {{ super::record(3); Err(CompactError::Injected("first")) }}
    pub fn after() -> Result<(), CompactError> {{ super::record(4); Ok(()) }}
}}
{methods}
fn main() {{
    single(true).unwrap(); events(&[10,1]);
    single(false).unwrap(); events(&[10,2]);
    equal(false).unwrap(); events(&[10,1]);
    assert_eq!(failed_equal(), Err(runtime::CompactError::Injected("condition"))); events(&[99]);
    nested(false,false).unwrap(); events(&[10]);
    nested(true,true).unwrap(); events(&[10,20]);
    assert_eq!(nested(true,false), Err(runtime::CompactError::AssertionFailed("guard".into()))); events(&[10,20]);
    assert_eq!(unused(), Err(runtime::CompactError::Injected("first"))); events(&[3]);
    lifted(true).unwrap(); events(&[10,1]);
    shadowed(true).unwrap(); events(&[10,2]);
}}
"#
    );
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let dir = Scratch(std::env::temp_dir().join(format!(
        "compact-unit-fragments-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    )));
    fs::create_dir(&dir.0).unwrap();
    let input = dir.0.join("control.rs");
    let executable = dir.0.join("control");
    fs::write(&input, source).unwrap();
    let compilation = Command::new(std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into()))
        .args(["--edition=2024", "--crate-name=unit_fragment_control"])
        .arg(input)
        .arg("-o")
        .arg(&executable)
        .output()
        .unwrap();
    assert!(
        compilation.status.success(),
        "{}",
        String::from_utf8_lossy(&compilation.stderr)
    );
    let run = Command::new(executable).output().unwrap();
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
}
