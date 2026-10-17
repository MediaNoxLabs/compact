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

//! Finite generated-source measurements. Counts identify review opportunities,
//! not redundant operations, semantic coverage or an automatic quality score.
use quote::ToTokens;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, error::Error, fs, path::Path};
use syn::visit_mut::{self, VisitMut};

#[derive(Default, Deserialize, Serialize)]
struct Metrics {
    lines: usize,
    internal_identifier_occurrences: usize,
    distinct_internal_identifiers: usize,
    longest_internal_identifier: usize,
    maximum_block_depth: usize,
    single_expression_blocks: usize,
    literal_unit_bindings: usize,
    clone_calls: usize,
    collect_calls: usize,
}

#[derive(Default)]
struct Survey {
    metrics: Metrics,
    identifiers: BTreeSet<String>,
    signatures: Vec<String>,
    block_depth: usize,
}

impl Survey {
    fn measure(source: &str) -> Result<Self, syn::Error> {
        let mut survey = Self::default();
        survey.visit_file_mut(&mut syn::parse_file(source)?);
        survey.metrics.lines = source.lines().count();
        survey.metrics.distinct_internal_identifiers = survey.identifiers.len();
        Ok(survey)
    }
}

impl VisitMut for Survey {
    fn visit_ident_mut(&mut self, ident: &mut syn::Ident) {
        let name = ident.to_string();
        if name.starts_with("__compact_") {
            self.metrics.internal_identifier_occurrences += 1;
            self.metrics.longest_internal_identifier =
                self.metrics.longest_internal_identifier.max(name.len());
            self.identifiers.insert(name);
        }
    }

    fn visit_block_mut(&mut self, block: &mut syn::Block) {
        self.block_depth += 1;
        self.metrics.maximum_block_depth = self.metrics.maximum_block_depth.max(self.block_depth);
        if matches!(block.stmts.as_slice(), [syn::Stmt::Expr(_, None)]) {
            self.metrics.single_expression_blocks += 1;
        }
        visit_mut::visit_block_mut(self, block);
        self.block_depth -= 1;
    }

    fn visit_local_mut(&mut self, local: &mut syn::Local) {
        if let (syn::Pat::Type(pat), Some(init)) = (&local.pat, &local.init)
            && matches!(pat.ty.as_ref(), syn::Type::Tuple(t) if t.elems.is_empty())
            && matches!(init.expr.as_ref(), syn::Expr::Tuple(t) if t.elems.is_empty())
        {
            self.metrics.literal_unit_bindings += 1;
        }
        visit_mut::visit_local_mut(self, local);
    }

    fn visit_expr_method_call_mut(&mut self, call: &mut syn::ExprMethodCall) {
        match call.method.to_string().as_str() {
            "clone" => self.metrics.clone_calls += 1,
            "collect" => self.metrics.collect_calls += 1,
            _ => {}
        }
        visit_mut::visit_expr_method_call_mut(self, call);
    }

    fn visit_signature_mut(&mut self, signature: &mut syn::Signature) {
        // Includes private functions and parameter names. This catches signature
        // drift; it is not a complete public API or macro-expansion fingerprint.
        self.signatures
            .push(signature.to_token_stream().to_string());
        visit_mut::visit_signature_mut(self, signature);
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Corpus {
    schema: u32,
    baseline_revision: String,
    scope: String,
    files: Vec<Input>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    name: String,
    path: String,
}

#[derive(Deserialize, Serialize)]
struct Row {
    name: String,
    path: String,
    source_sha256: String,
    declared_function_signatures_sha256: String,
    metrics: Metrics,
}

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn report(root: &Path, corpus_path: &Path) -> Result<serde_json::Value, Box<dyn Error>> {
    let bytes = fs::read(corpus_path)?;
    let corpus: Corpus = serde_json::from_slice(&bytes)?;
    if corpus.schema != 1 || corpus.files.is_empty() {
        return Err("expected nonempty readability corpus schema 1".into());
    }
    let mut paths = BTreeSet::new();
    let mut names = BTreeSet::new();
    let mut rows = Vec::new();
    for input in corpus.files {
        if !paths.insert(input.path.clone()) || !names.insert(input.name.clone()) {
            return Err("duplicate corpus path or name".into());
        }
        let source = fs::read_to_string(root.join(&input.path))?;
        let survey = Survey::measure(&source)?;
        rows.push(Row {
            name: input.name,
            path: input.path,
            source_sha256: hash(source.as_bytes()),
            declared_function_signatures_sha256: hash(&serde_json::to_vec(&survey.signatures)?),
            metrics: survey.metrics,
        });
    }
    Ok(serde_json::json!({
        "schema": 1,
        "corpus_sha256": hash(&bytes),
        "baseline_revision": corpus.baseline_revision,
        "scope": corpus.scope,
        "syntax_scope": "syn AST without macro expansion; clone/collect/block counts are not redundancy proofs",
        "files": rows,
    }))
}

/// Compare only the finite survey contract. This is not a semantic/API verifier.
fn compare(
    current: &serde_json::Value,
    baseline: &[u8],
) -> Result<serde_json::Value, Box<dyn Error>> {
    #[derive(Deserialize)]
    struct Snapshot {
        schema: u32,
        corpus_sha256: String,
        files: Vec<Row>,
    }
    let before: Snapshot = serde_json::from_slice(baseline)?;
    let after: Snapshot = serde_json::from_value(current.clone())?;
    if before.schema != 1 || after.schema != 1 || before.corpus_sha256 != after.corpus_sha256 {
        return Err("comparison requires schema 1 reports of the identical corpus".into());
    }
    let mut old = std::collections::BTreeMap::new();
    for row in before.files {
        if old
            .insert((row.name.clone(), row.path.clone()), row)
            .is_some()
        {
            return Err("duplicate baseline case".into());
        }
    }
    if old.is_empty() || old.len() != after.files.len() {
        return Err("comparison case accounting differs".into());
    }
    let mut regressions = Vec::new();
    for row in after.files {
        let previous = old
            .remove(&(row.name.clone(), row.path.clone()))
            .ok_or("comparison case identity differs")?;
        if previous.declared_function_signatures_sha256 != row.declared_function_signatures_sha256 {
            regressions.push(serde_json::json!({"case": row.name, "metric": "declared_function_signatures_sha256"}));
        }
        let before = serde_json::to_value(previous.metrics)?;
        let after = serde_json::to_value(row.metrics)?;
        // Both values originate from typed Metrics, so every field is a count.
        for (metric, value) in after.as_object().ok_or("invalid metric object")? {
            let previous = before
                .get(metric)
                .and_then(|v| v.as_u64())
                .ok_or("missing baseline metric")?;
            let value = value.as_u64().ok_or("invalid metric count")?;
            if value > previous {
                regressions.push(serde_json::json!({"case": row.name, "metric": metric, "before": previous, "after": value}));
            }
        }
    }
    Ok(serde_json::json!({
        "scope": "same-corpus per-file counts and declared function signatures only; not full public API or behavior",
        "passed": regressions.is_empty(),
        "regressions": regressions,
    }))
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = std::env::args_os().skip(1).collect::<Vec<_>>();
    if !(2..=3).contains(&args.len()) {
        return Err(
            "usage: readability ROOT CORPUS_JSON [BASELINE_REPORT_JSON] (prints JSON to stdout)"
                .into(),
        );
    }
    let mut output = report(Path::new(&args[0]), Path::new(&args[1]))?;
    let mut passed = true;
    if let Some(path) = args.get(2) {
        let comparison = compare(&output, &fs::read(path)?)?;
        passed = comparison["passed"] == true;
        output["comparison"] = comparison;
    }
    println!("{}", serde_json::to_string_pretty(&output)?);
    if !passed {
        return Err(
            "readability regression: review reported tradeoffs before updating evidence".into(),
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_syntax_without_treating_comments_or_strings_as_code() {
        let source = r#"fn example(__compact_arg: ()) {
            // let noise: () = (); fake.clone().collect();
            let __compact_effect_0: () = ();
            let text = "__compact_fake .clone()";
            { __compact_arg.clone().collect::<Vec<_>>() }
        }"#;
        let s = Survey::measure(source).unwrap();
        assert_eq!(s.metrics.literal_unit_bindings, 1);
        assert_eq!(s.metrics.internal_identifier_occurrences, 3);
        assert_eq!(s.metrics.distinct_internal_identifiers, 2);
        assert_eq!(s.metrics.clone_calls, 1);
        assert_eq!(s.metrics.collect_calls, 1);
        assert_eq!(s.metrics.maximum_block_depth, 2);
    }

    #[test]
    fn body_changes_preserve_signature_measurement_but_signature_changes_do_not() {
        let before =
            Survey::measure("pub fn a(value: ()) -> () { let x: () = (); value }").unwrap();
        let after = Survey::measure("pub fn a(value: ()) -> () { value }").unwrap();
        let renamed = Survey::measure("pub fn b(value: ()) -> () { value }").unwrap();
        assert_eq!(before.signatures, after.signatures);
        assert_ne!(before.signatures, renamed.signatures);
        assert_eq!(before.metrics.literal_unit_bindings, 1);
        assert_eq!(after.metrics.literal_unit_bindings, 0);
    }

    #[test]
    fn malformed_rust_refuses_instead_of_reporting_zero_metrics() {
        assert!(Survey::measure("pub fn missing(").is_err());
    }

    fn snapshot(source: &str) -> serde_json::Value {
        let survey = Survey::measure(source).unwrap();
        serde_json::json!({
            "schema": 1, "corpus_sha256": "same-corpus",
            "files": [{"name": "case", "path": "case.rs", "source_sha256": hash(source.as_bytes()),
                "declared_function_signatures_sha256": hash(&serde_json::to_vec(&survey.signatures).unwrap()),
                "metrics": survey.metrics}]
        })
    }

    #[test]
    fn comparison_accepts_simplification_but_reports_nesting_and_signature_regressions() {
        let before = snapshot("pub fn a() { let __compact_effect_0: () = (); }");
        let bytes = serde_json::to_vec(&before).unwrap();
        assert_eq!(
            compare(&snapshot("pub fn a() {}"), &bytes).unwrap()["passed"],
            true
        );
        for source in ["pub fn b() {}", "pub fn a() { { () } }"] {
            let result = compare(&snapshot(source), &bytes).unwrap();
            assert_eq!(result["passed"], false);
            assert!(!result["regressions"].as_array().unwrap().is_empty());
        }
    }

    #[test]
    fn comparison_refuses_changed_corpus_missing_cases_and_duplicate_cases() {
        let current = snapshot("pub fn a() {}");
        for field in ["corpus_sha256", "schema", "files"] {
            let mut changed = current.clone();
            changed[field] = serde_json::json!(null);
            assert!(compare(&current, &serde_json::to_vec(&changed).unwrap()).is_err());
        }
        let mut changed = current.clone();
        changed["corpus_sha256"] = "different".into();
        assert!(compare(&current, &serde_json::to_vec(&changed).unwrap()).is_err());
        changed = current.clone();
        changed["files"] = serde_json::json!([]);
        assert!(compare(&current, &serde_json::to_vec(&changed).unwrap()).is_err());
        changed = current.clone();
        let row = changed["files"][0].clone();
        changed["files"].as_array_mut().unwrap().push(row);
        assert!(compare(&current, &serde_json::to_vec(&changed).unwrap()).is_err());
    }
}
