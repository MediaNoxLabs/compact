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

//! Execute actual checked native Map syntax with the actual FixedVector implementation.
//! Instrumented callees expose order/errors; full generated consumers test integration.
use super::*;
use std::{fs, process::Command};

fn vector(element: Type, length: usize) -> Type {
    Type::Vector {
        element: Box::new(element),
        length,
    }
}
fn mapping(source: Expr, input: Type, name: &str, body: Expr, result: Type, length: usize) -> Expr {
    Expr::VectorMap {
        source: Box::new(source),
        parameter: Parameter {
            name: name.into(),
            ty: input,
        },
        body: Box::new(body),
        result,
        length,
    }
}
fn invoke(name: &str, arguments: Vec<Expr>) -> Expr {
    Expr::Call {
        name: name.into(),
        arguments,
    }
}
fn emitted(
    name: &str,
    source: &Expr,
    result: &Type,
    calls: &HashMap<&str, &PureCircuit>,
) -> String {
    let flag = Type::Boolean;
    let parameters = HashMap::from([
        ("fail_source", (&flag, syn::parse_quote!(fail_source))),
        ("fail_body", (&flag, syn::parse_quote!(fail_body))),
    ]);
    let (fragment, actual) = crate::native_fragment_with_calls(source, &parameters, calls).unwrap();
    assert_eq!(&actual, result);
    let (expression, _) = fragment.into_views();
    let result = crate::rust_type(result).unwrap();
    format!(
        "fn {name}(fail_source: bool, fail_body: bool) -> Result<{}, runtime::CompactError> {{ Ok({}) }}",
        result.to_token_stream(),
        expression.to_token_stream()
    )
}

#[test]
fn compiled_map_keeps_source_once_first_failure_nested_order_and_unit_assertions() {
    let bools = vector(Type::Boolean, 4);
    let empty = vector(Type::Boolean, 0);
    let matrix = vector(vector(Type::Boolean, 2), 2);
    let mut declarations = Vec::new();
    for (name, result, names) in [
        ("source", bools.clone(), vec!["fail_source", "fail_body"]),
        ("empty", empty.clone(), vec![]),
        ("matrix", matrix.clone(), vec![]),
        ("probe", Type::Boolean, vec!["value"]),
        ("observe", Type::Boolean, vec!["value"]),
    ] {
        let mut declaration = pure(name, result);
        declaration.parameters = names
            .into_iter()
            .map(|name| Parameter {
                name: name.into(),
                ty: Type::Boolean,
            })
            .collect();
        declarations.push(declaration);
    }
    let calls = declarations
        .iter()
        .map(|declaration| (declaration.name.as_str(), declaration))
        .collect();
    let source = || {
        invoke(
            "source",
            vec![reference("fail_source"), reference("fail_body")],
        )
    };
    let ordinary = mapping(
        source(),
        Type::Boolean,
        "item",
        invoke("probe", vec![reference("item")]),
        Type::Boolean,
        4,
    );
    let unit = mapping(
        source(),
        Type::Boolean,
        "item",
        assertion(invoke("observe", vec![reference("item")])),
        Type::Unit,
        4,
    );
    let zero = mapping(
        call("empty"),
        Type::Boolean,
        "item",
        assertion(boolean(false)),
        Type::Unit,
        0,
    );
    let inner = mapping(
        reference("row"),
        Type::Boolean,
        "item",
        invoke("probe", vec![reference("item")]),
        Type::Boolean,
        2,
    );
    let nested = mapping(
        call("matrix"),
        vector(Type::Boolean, 2),
        "row",
        inner,
        vector(Type::Boolean, 2),
        2,
    );
    let functions = [
        emitted("ordinary", &ordinary, &bools, &calls),
        emitted("unit", &unit, &vector(Type::Unit, 4), &calls),
        emitted("zero", &zero, &vector(Type::Unit, 0), &calls),
        emitted("nested", &nested, &matrix, &calls),
    ]
    .join("\n");
    assert_eq!(functions.matches(". try_map").count(), 5);
    let primitives =
        syn::parse_file(include_str!("../../../../../runtime-rs/src/primitives.rs")).unwrap();
    let selected: Vec<_> = primitives.items.into_iter().filter(|item| match item {
        syn::Item::Struct(item) => item.ident == "FixedVector",
        syn::Item::Impl(item) if item.trait_.is_none() => matches!(item.self_ty.as_ref(), syn::Type::Path(path) if path.path.segments.last().is_some_and(|segment| segment.ident == "FixedVector")),
        _ => false,
    }).collect();
    assert_eq!(
        selected.len(),
        2,
        "exact FixedVector declaration and inherent impl"
    );
    let primitive = quote::quote!(#(#selected)*);
    let program = format!(
        r#"
mod runtime {{
    #[derive(Debug, PartialEq)] pub enum CompactError {{ AssertionFailed(String), Injected(&'static str) }}
    {primitive}
}}
thread_local! {{ static EVENTS: std::cell::RefCell<Vec<u64>> = const {{ std::cell::RefCell::new(Vec::new()) }}; }}
fn record(value: u64) {{ EVENTS.with(|events| events.borrow_mut().push(value)); }}
fn events(expected: &[u64]) {{ EVENTS.with(|events| {{ assert_eq!(*events.borrow(), expected); events.borrow_mut().clear(); }}); }}
mod pure_circuits {{
    use super::runtime::{{CompactError, FixedVector}};
    pub fn source(fail_source: bool, fail_body: bool) -> Result<FixedVector<bool,4>,CompactError> {{
        super::record(100);
        if fail_source {{ return Err(CompactError::Injected("source")); }}
        Ok(FixedVector::new([true,!fail_body,true,true]))
    }}
    pub fn empty() -> Result<FixedVector<bool,0>,CompactError> {{ super::record(101); Ok(FixedVector::new([])) }}
    pub fn matrix() -> Result<FixedVector<FixedVector<bool,2>,2>,CompactError> {{
        super::record(102); Ok(FixedVector::new([FixedVector::new([true,true]),FixedVector::new([false,true])]))
    }}
    pub fn probe(value: bool) -> Result<bool,CompactError> {{
        super::record(if value {{ 11 }} else {{ 12 }});
        if value {{ Ok(value) }} else {{ Err(CompactError::Injected("body")) }}
    }}
    pub fn observe(value: bool) -> Result<bool,CompactError> {{ super::record(if value {{ 21 }} else {{ 22 }}); Ok(value) }}
}}
{functions}
fn main() {{
    assert_eq!(ordinary(false,false),Ok(runtime::FixedVector::new([true;4]))); events(&[100,11,11,11,11]);
    assert_eq!(ordinary(true,true),Err(runtime::CompactError::Injected("source"))); events(&[100]);
    assert_eq!(ordinary(false,true),Err(runtime::CompactError::Injected("body"))); events(&[100,11,12]);
    assert_eq!(unit(false,false),Ok(runtime::FixedVector::new([();4]))); events(&[100,21,21,21,21]);
    assert_eq!(unit(false,true),Err(runtime::CompactError::AssertionFailed("guard".into()))); events(&[100,21,22]);
    assert_eq!(unit(true,false),Err(runtime::CompactError::Injected("source"))); events(&[100]);
    assert_eq!(zero(false,false),Ok(runtime::FixedVector::new([]))); events(&[101]);
    assert_eq!(nested(false,false),Err(runtime::CompactError::Injected("body"))); events(&[102,11,11,12]);
}}
"#
    );
    struct Scratch(std::path::PathBuf);
    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    let scratch =
        Scratch(std::env::temp_dir().join(format!("compact-map-execution-{}", std::process::id())));
    fs::create_dir(&scratch.0).unwrap();
    let input = scratch.0.join("map.rs");
    let executable = scratch.0.join("map");
    fs::write(&input, program).unwrap();
    let compilation = Command::new(std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into()))
        .args(["--edition=2024", "--crate-name=map_execution"])
        .arg(&input)
        .arg("-o")
        .arg(&executable)
        .output()
        .unwrap();
    assert!(
        compilation.status.success(),
        "{}",
        String::from_utf8_lossy(&compilation.stderr)
    );
    let run = Command::new(&executable).output().unwrap();
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    let path = scratch.0.clone();
    drop(scratch);
    assert!(!path.exists(), "executable fixture removed");
}
