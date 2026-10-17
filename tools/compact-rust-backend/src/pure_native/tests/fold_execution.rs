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

//! Run checked Fold syntax beside its frozen predecessor with non-Copy drop probes.
use super::*;
use std::{fs, process::Command};

#[test]
fn compiled_fold_matches_predecessor_order_refusals_empty_and_drop_cleanup() {
    let vector = |length| Type::Vector {
        element: Box::new(Type::Field),
        length,
    };
    let mut declarations = Vec::new();
    for (name, result, parameters) in [
        ("source", vector(3), vec![]),
        ("empty", vector(0), vec![]),
        ("initial", Type::Field, vec![]),
        ("unit_initial", Type::Unit, vec![]),
        ("unit_step", Type::Unit, vec![Type::Unit, Type::Field]),
        ("step", Type::Field, vec![Type::Field, Type::Field]),
    ] {
        let mut item = pure(name, result);
        item.parameters = parameters
            .into_iter()
            .enumerate()
            .map(|(index, ty)| Parameter {
                name: format!("arg{index}"),
                ty,
            })
            .collect();
        declarations.push(item);
    }
    let calls = declarations
        .iter()
        .map(|item| (item.name.as_str(), item))
        .collect();
    let mut functions = Vec::new();
    for (name, length, unit) in [
        ("ordinary", 3, false),
        ("zero", 0, false),
        ("unit", 3, true),
        ("unit_zero", 0, true),
    ] {
        let source = Expr::VectorFoldCall {
            name: if unit { "unit_step" } else { "step" }.into(),
            initial: Box::new(call(if unit { "unit_initial" } else { "initial" })),
            source: Box::new(call(if length == 0 { "empty" } else { "source" })),
            accumulator: if unit { Type::Unit } else { Type::Field },
            element: Type::Field,
            length,
        };
        let (fragment, ty) =
            crate::native_fragment_with_calls(&source, &HashMap::new(), &calls).unwrap();
        assert_eq!(ty, if unit { Type::Unit } else { Type::Field });
        let (expression, _) = fragment.into_views();
        let (old, _, _) = old_oracle::observe(&source, &HashMap::new(), &calls).unwrap();
        let body = |expression: String| {
            if unit {
                format!("({expression}); Ok(runtime::Field(0))")
            } else {
                format!("Ok({expression})")
            }
        };
        functions.push(format!(
            "fn {name}() -> Result<runtime::Field,runtime::CompactError> {{ {} }}",
            body(expression.to_token_stream().to_string())
        ));
        functions.push(format!(
            "fn old_{name}() -> Result<runtime::Field,runtime::CompactError> {{ {} }}",
            body(old)
        ));
    }
    let functions = functions.join("\n");
    let primitives =
        syn::parse_file(include_str!("../../../../../runtime-rs/src/primitives.rs")).unwrap();
    let selected: Vec<_> = primitives.items.into_iter().filter(|item| match item {
        syn::Item::Struct(item) => item.ident == "FixedVector",
        syn::Item::Impl(item) if item.trait_.is_none() => matches!(item.self_ty.as_ref(), syn::Type::Path(path) if path.path.segments.last().is_some_and(|part| part.ident == "FixedVector")),
        _ => false,
    }).collect();
    assert_eq!(selected.len(), 2);
    let primitive = quote::quote!(#(#selected)*);
    let program = format!(
        r#"
thread_local! {{
    static EVENTS: std::cell::RefCell<Vec<String>> = const {{ std::cell::RefCell::new(Vec::new()) }};
    static MODE: std::cell::Cell<u8> = const {{ std::cell::Cell::new(0) }};
}}
fn event(value: impl Into<String>) {{ EVENTS.with(|events| events.borrow_mut().push(value.into())); }}
fn mode() -> u8 {{ MODE.with(|mode| mode.get()) }}
mod runtime {{
    #[derive(Debug,PartialEq)] pub enum CompactError {{ Injected(&'static str) }}
    #[derive(Debug,Clone,PartialEq,Eq)] pub struct Field(pub u64);
    impl Drop for Field {{ fn drop(&mut self) {{ super::event(format!("drop:{{}}",self.0)); }} }}
    {primitive}
}}
mod pure_circuits {{
    use super::runtime::{{Field,FixedVector,CompactError}};
    pub fn source() -> Result<FixedVector<Field,3>,CompactError> {{
        super::event("source");
        if super::mode() == 1 {{ return Err(CompactError::Injected("source")); }}
        Ok(FixedVector::new([Field(1),Field(2),Field(3)]))
    }}
    pub fn empty() -> Result<FixedVector<Field,0>,CompactError> {{ super::event("empty"); Ok(FixedVector::new([])) }}
    pub fn initial() -> Result<Field,CompactError> {{
        super::event("initial");
        if super::mode() == 2 {{ Err(CompactError::Injected("initial")) }} else {{ Ok(Field(0)) }}
    }}
    pub fn unit_initial() -> Result<(),CompactError> {{
        super::event("unit_initial");
        if super::mode() == 2 {{ Err(CompactError::Injected("initial")) }} else {{ Ok(()) }}
    }}
    pub fn unit_step(_:(),item:Field) -> Result<(),CompactError> {{
        super::event(format!("unit_step:{{}}",item.0));
        if (super::mode() == 3 && item.0 == 2) || (super::mode() == 4 && item.0 == 1) {{ return Err(CompactError::Injected("step")); }}
        Ok(())
    }}
    pub fn step(acc:Field,item:Field) -> Result<Field,CompactError> {{
        super::event(format!("step:{{}}:{{}}",acc.0,item.0));
        if (super::mode() == 3 && item.0 == 2) || (super::mode() == 4 && item.0 == 1) {{ return Err(CompactError::Injected("step")); }}
        Ok(Field(acc.0*10+item.0))
    }}
}}
{functions}
fn capture(run:fn()->Result<runtime::Field,runtime::CompactError>,mode:u8) -> (Result<u64,&'static str>,Vec<String>) {{
    MODE.with(|value|value.set(mode));
    let result=match run() {{ Ok(value)=>Ok(value.0),Err(runtime::CompactError::Injected(error))=>Err(error) }};
    let events=EVENTS.with(|events|std::mem::take(&mut *events.borrow_mut()));
    (result,events)
}}
fn main() {{
    for mode in 0..5 {{
        let current=capture(ordinary,mode);
        let old=capture(old_ordinary,mode);
        assert_eq!(current,old,"mode {{mode}}");
        let expected=match mode {{0=>Ok(123),1=>Err("source"),2=>Err("initial"),_=>Err("step")}};
        assert_eq!(current.0,expected);
        match mode {{
            0=>{{ assert_eq!(&current.1[..2],["source","initial"]); assert!(current.1.iter().any(|event|event=="step:12:3")); }},
            1=>assert_eq!(current.1,["source"]),
            2=>assert_eq!(current.1,["source","initial","drop:1","drop:2","drop:3"]),
            _=>{{ assert!(!current.1.iter().any(|event|event.starts_with("step:")&&event.ends_with(":3"))); assert!(current.1.contains(&"drop:3".into())); }},
        }}
        let unit=capture(unit,mode);
        assert_eq!(unit,capture(old_unit,mode));
        assert_eq!(unit.0,match mode {{0=>Ok(0),1=>Err("source"),2=>Err("initial"),_=>Err("step")}});
        let unit_zero=capture(unit_zero,mode);
        assert_eq!(unit_zero,capture(old_unit_zero,mode));
        assert_eq!(unit_zero.0,if mode==2 {{Err("initial")}} else {{Ok(0)}});
        assert!(unit_zero.1.iter().all(|event|!event.starts_with("unit_step:")));
        let zero=capture(zero,mode);
        assert_eq!(zero,capture(old_zero,mode));
        assert_eq!(zero.0,if mode==2 {{Err("initial")}} else {{Ok(0)}});
        assert!(zero.1.iter().all(|event|!event.starts_with("step:")));
    }}
}}
"#
    );
    struct Scratch(std::path::PathBuf);
    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    let scratch = Scratch(
        std::env::temp_dir().join(format!("compact-fold-execution-{}", std::process::id())),
    );
    fs::create_dir(&scratch.0).unwrap();
    let input = scratch.0.join("fold.rs");
    let executable = scratch.0.join("fold");
    fs::write(&input, program).unwrap();
    let compilation = Command::new(std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into()))
        .args(["--edition=2024", "--crate-name=fold_execution"])
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
    assert!(!path.exists());
}
