// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Rust function names shared by native validation and recorded helpers.

use std::collections::HashMap;

use crate::ir::{SourceLocation, StatefulCircuit};
use crate::{RenderError, located};

pub(crate) fn ident(name: &str) -> Result<syn::Ident, RenderError> {
    let rust_name = name.replace('$', "_");
    syn::parse_str::<syn::Ident>(&rust_name)
        .or_else(|_| syn::parse_str::<syn::Ident>(&format!("r#{rust_name}")))
        .map_err(|_| RenderError::InvalidIdentifier(name.to_owned()))
}

/// The name by which Rust resolves an identifier, without its optional raw prefix.
pub(crate) fn semantic_identifier(name: &str) -> Result<String, RenderError> {
    let rendered = ident(name)?.to_string();
    Ok(rendered.strip_prefix("r#").unwrap_or(&rendered).to_owned())
}

pub(crate) fn validate_circuit_function_namespace<'a>(
    declarations: impl IntoIterator<Item = (&'a str, Option<&'a SourceLocation>)>,
) -> Result<(), RenderError> {
    let mut emitted = HashMap::<String, &'a str>::new();
    for (name, source) in declarations {
        located(source, || {
            // `foo` and `r#foo` bind the same Rust name. Compare after Compact
            // `$` normalization and any Rust keyword escaping.
            let rust_identifier = semantic_identifier(name)?;
            if let Some(first) = emitted.get(&rust_identifier) {
                return Err(RenderError::ConflictingCircuitIdentifier {
                    first: (*first).to_owned(),
                    second: name.to_owned(),
                    rust_identifier,
                });
            }
            emitted.insert(rust_identifier, name);
            Ok(())
        })?;
    }
    Ok(())
}

fn recorded_body_candidate(name: &str) -> Result<syn::Ident, RenderError> {
    let semantic = semantic_identifier(name)?;
    ident(&format!("__compact_recorded_body_{semantic}"))
}

pub(crate) fn recorded_helper_ident(
    name: &str,
    circuits: &HashMap<&str, &StatefulCircuit>,
) -> Result<syn::Ident, RenderError> {
    let base = recorded_body_candidate(name)?;
    let base_name = semantic_identifier(&base.to_string())?;
    let mut duplicate_base = false;
    for other in circuits.keys() {
        if semantic_identifier(other)? == base_name {
            duplicate_base = true;
            break;
        }
        if *other != name
            && semantic_identifier(&recorded_body_candidate(other)?.to_string())? == base_name
        {
            duplicate_base = true;
            break;
        }
    }
    if !duplicate_base {
        return Ok(base);
    }

    // A declaration may itself use our preferred helper name. Keep the old
    // deterministic index and raw-name bytes for distinct `$`/`_` spellings.
    let mut ordered: Vec<_> = circuits.keys().copied().collect();
    ordered.sort_unstable();
    let index = ordered
        .iter()
        .position(|candidate| *candidate == name)
        .ok_or_else(|| RenderError::UnknownCircuit(name.to_owned()))?;
    let mut fallback = format!("__compact_recorded_body_{index}_x");
    for byte in name.bytes() {
        fallback.push_str(&format!("{byte:02x}"));
    }
    loop {
        let candidate = ident(&fallback)?;
        let candidate_name = semantic_identifier(&candidate.to_string())?;
        let occupied = circuits.keys().any(|other| {
            semantic_identifier(other).is_ok_and(|name| name == candidate_name)
                || recorded_body_candidate(other)
                    .and_then(|id| semantic_identifier(&id.to_string()))
                    .is_ok_and(|name| name == candidate_name)
        });
        if !occupied {
            return Ok(candidate);
        }
        fallback.push('_');
    }
}

/// Allocate parameter identifiers within one Rust binding namespace.
/// Callers select reserved local names; source binding keys remain unchanged.
pub(crate) fn parameter_idents(
    parameters: &[crate::ir::Parameter],
    reserved: &[&str],
) -> Vec<syn::Ident> {
    let mut used = std::collections::HashSet::<String>::new();
    used.extend(reserved.iter().map(|name| (*name).to_owned()));
    parameters
        .iter()
        .enumerate()
        .map(|(index, parameter)| {
            let candidate = ident(&parameter.name).ok().filter(|name| {
                let spelling = name.to_string();
                spelling != "_"
                    && !used.contains(&spelling)
                    && !used.contains(spelling.strip_prefix("r#").unwrap_or(&spelling))
            });
            let name = candidate.unwrap_or_else(|| {
                let base = format!("__compact_param_{index}");
                let mut fallback = base.clone();
                let mut suffix = 1;
                while used.contains(&fallback) {
                    fallback = format!("{base}_{suffix}");
                    suffix += 1;
                }
                syn::Ident::new(&fallback, proc_macro2::Span::call_site())
            });
            let spelling = name.to_string();
            used.insert(spelling.strip_prefix("r#").unwrap_or(&spelling).to_owned());
            name
        })
        .collect()
}
