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

//! Compiled emitter controls use an instrumented cast to expose execution order.
//! Actual Compact/runtime behavior is covered by the vector-widen consumer crate.

use super::*;
use std::fs;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

#[test]
fn fallibility_follows_nested_conversion_types_without_changing_infallible_maps() {
    for (source, target) in [
        (vector(uint("255"), 2), vector(uint("65535"), 2)),
        (
            vector(vector(uint("255"), 2), 3),
            vector(vector(uint("65535"), 2), 3),
        ),
        (
            vector(tuple(vec![uint("255"), Type::Boolean]), 2),
            vector(tuple(vec![uint("65535"), Type::Boolean]), 2),
        ),
        (
            vector(tuple(vec![uint("255")]), 2),
            vector(vector(uint("65535"), 1), 2),
        ),
        (
            vector(vector(uint("255"), 1), 2),
            vector(tuple(vec![uint("65535")]), 2),
        ),
    ] {
        let lowered = coercion(syn::parse_quote!(input), &source, &target, 0).unwrap();
        assert!(lowered.may_fail);
        assert!(
            lowered
                .expression
                .to_token_stream()
                .to_string()
                .contains("Vec :: with_capacity")
        );
    }
    let source = vector(tuple(vec![uint("255"), Type::Boolean]), 2);
    let target = vector(tuple(vec![Type::Field, Type::Boolean]), 2);
    let lowered = coercion(syn::parse_quote!(source()?), &source, &target, 0).unwrap();
    assert!(
        !lowered.may_fail,
        "source evaluation failure is outside element mapping"
    );
    let text = lowered.expression.to_token_stream().to_string();
    assert!(text.contains(". map"));
    assert!(!text.contains("Vec ::"));
    let identity = coercion(syn::parse_quote!(input), &source, &source, 0).unwrap();
    assert!(!identity.may_fail);
    assert_eq!(identity.expression.to_token_stream().to_string(), "input");
}

#[test]
fn nested_vector_emission_size_does_not_multiply_by_dimensions() {
    let emitted = |length| {
        let source = vector(vector(uint("255"), length), length);
        let target = vector(vector(uint("65535"), length), length);
        coerce_expression(syn::parse_quote!(input), &source, &target, 0)
            .unwrap()
            .to_token_stream()
            .to_string()
    };
    let small = emitted(2);
    let large = emitted(1_000_000);
    assert_eq!(large.matches("for ").count(), 2);
    assert_eq!(large.matches("cast_unsigned").count(), 1);
    assert!(large.len() < small.len() + 32);
}

struct Scratch(std::path::PathBuf);
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn compiled_coercion_evaluates_source_once_and_stops_at_first_error_in_order() {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let dir = Scratch(std::env::temp_dir().join(format!(
        "compact-vector-coercion-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    )));
    fs::create_dir(&dir.0).unwrap();
    let source = vector(vector(uint("255"), 2), 2);
    let target = vector(vector(uint("65535"), 2), 2);
    let lowered =
        coerce_expression(syn::parse_quote!(source(fail_source)?), &source, &target, 0).unwrap();
    let unpinned = coerce_expression(
        syn::parse_quote!(runtime::FixedVector::new([1_u128, 2])),
        &vector(uint("255"), 2),
        &vector(uint("65535"), 2),
        0,
    )
    .unwrap();
    let harness = format!(
        r#"
mod runtime {{
    use std::cell::RefCell;
    thread_local! {{ pub static EVENTS: RefCell<Vec<u128>> = const {{ RefCell::new(Vec::new()) }}; }}
    #[derive(Debug, PartialEq)] pub struct FixedVector<T, const N: usize>(pub [T; N]);
    impl<T, const N: usize> FixedVector<T,N> {{
        pub fn new(value: [T;N]) -> Self {{ Self(value) }}
        pub fn into_array(self) -> [T;N] {{ self.0 }}
    }}
    pub fn cast_unsigned<const SOURCE: u128, const TARGET: u128>(v: u128) -> Result<u128,&'static str> {{
        assert_eq!((SOURCE,TARGET), (255,65535));
        EVENTS.with(|events| events.borrow_mut().push(v));
        if v == 3 {{ Err("injected first cast failure") }} else {{ Ok(v) }}
    }}
}}
fn source(fail: bool) -> Result<runtime::FixedVector<runtime::FixedVector<u128,2>,2>, &'static str> {{
    runtime::EVENTS.with(|events| events.borrow_mut().push(99));
    if fail {{ return Err("source failure"); }}
    Ok(runtime::FixedVector::new([runtime::FixedVector::new([1,2]),runtime::FixedVector::new([3,4])]))
}}
fn convert(fail_source: bool) -> Result<runtime::FixedVector<runtime::FixedVector<u128,2>,2>, &'static str> {{
    Ok({lowered})
}}
fn inferred_length() -> Result<usize, &'static str> {{
    // No return type or annotated binding constrains the converted array length.
    let converted = {unpinned};
    Ok(converted.into_array().len())
}}
fn main() {{
    assert_eq!(inferred_length().unwrap(), 2);
    runtime::EVENTS.with(|events| {{ assert_eq!(*events.borrow(), [1,2]); events.borrow_mut().clear(); }});
    assert_eq!(convert(false), Err("injected first cast failure"));
    runtime::EVENTS.with(|events| {{ assert_eq!(*events.borrow(), [99,1,2,3]); events.borrow_mut().clear(); }});
    assert_eq!(convert(true), Err("source failure"));
    runtime::EVENTS.with(|events| assert_eq!(*events.borrow(), [99]));
}}
"#,
        lowered = lowered.to_token_stream(),
        unpinned = unpinned.to_token_stream()
    );
    let input = dir.0.join("control.rs");
    let executable = dir.0.join("control");
    fs::write(&input, harness).unwrap();
    let compilation = Command::new(std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into()))
        .args(["--edition=2024", "--crate-name=vector_coercion_control"])
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
    let run = Command::new(executable).output().unwrap();
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
}
