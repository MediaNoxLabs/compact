// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Reachable pure declarations certified through the native emitter's checks.
//! This is a bounded compiler certificate, not a formal proof or infallibility
//! guarantee. A temporary native-syntax bridge avoids a second typechecker;
//! extracting shared semantic typing remains architecture debt. No syntax is
//! retained in this owner or the recording plan.
mod native;

use super::{RecordingGap, invalid};
use crate::ir::{Expr, PureCircuit, StatefulCircuit};
use crate::resource_limits::PureBodyBudget;
use std::collections::{BTreeMap, HashMap, HashSet};

/// Only this module can mint or change the exact source snapshot.
#[derive(Debug)]
pub(super) struct CertifiedPure {
    source: PureCircuit,
    depth: usize,
    expanded_work: usize,
}
impl CertifiedPure {
    pub(super) fn source(&self) -> &PureCircuit {
        &self.source
    }
    pub(super) fn call_depth(&self) -> usize {
        self.depth
    }
    pub(super) fn expanded_work(&self) -> usize {
        self.expanded_work
    }
}

#[derive(Debug, Default)]
pub(super) struct Certificates {
    entries: BTreeMap<String, CertifiedPure>,
}
impl Certificates {
    pub(super) fn get(&self, name: &str) -> Option<&CertifiedPure> {
        self.entries.get(name)
    }
    pub(super) fn iter(&self) -> impl Iterator<Item = (&str, &CertifiedPure)> {
        self.entries
            .iter()
            .map(|(name, entry)| (name.as_str(), entry))
    }
    /// Attach only to the actual source environment that was certified. Scan
    /// current raw bodies before recursive Eq, including transitive callees.
    pub(super) fn verify_bindings(
        &self,
        pure: &HashMap<&str, &PureCircuit>,
    ) -> Result<(), RecordingGap> {
        let mut budget = PureBodyBudget::default();
        for (name, certificate) in self.iter() {
            let path = format!("pure[{name}]");
            if pure
                .values()
                .filter(|source| source.name == name)
                .take(2)
                .count()
                != 1
            {
                return Err(invalid(&path, "ambiguous certified pure source name"));
            }
            let current = pure
                .get(name)
                .copied()
                .ok_or_else(|| invalid(&path, "certified pure source is missing"))?;
            budget
                .check(current)
                .map_err(|e| invalid(&path, e.to_string()))?;
            if current != certificate.source() {
                return Err(invalid(
                    &path,
                    "certified pure source differs from current declaration",
                ));
            }
        }
        Ok(())
    }
}

struct Frame<'a> {
    source: &'a PureCircuit,
    dependencies: Vec<String>,
    next: usize,
    local_work: usize,
}

pub(super) struct Certifier<'a> {
    pure: &'a HashMap<&'a str, &'a PureCircuit>,
    stateful: &'a HashMap<&'a str, &'a StatefulCircuit>,
    certificates: Certificates,
    budget: PureBodyBudget,
    normalized_names: HashMap<String, String>,
}
impl<'a> Certifier<'a> {
    pub(super) fn new(
        pure: &'a HashMap<&'a str, &'a PureCircuit>,
        stateful: &'a HashMap<&'a str, &'a StatefulCircuit>,
    ) -> Self {
        Self {
            pure,
            stateful,
            certificates: Certificates::default(),
            budget: PureBodyBudget::default(),
            normalized_names: HashMap::new(),
        }
    }
    pub(super) fn get(&self, name: &str) -> Option<&CertifiedPure> {
        self.certificates.get(name)
    }
    pub(super) fn into_certificates(self) -> Certificates {
        self.certificates
    }

    fn enter(&mut self, name: &str, path: &str) -> Result<Frame<'a>, RecordingGap> {
        let source = self
            .pure
            .get(name)
            .copied()
            .ok_or_else(|| invalid(path, format!("unknown pure callee {name:?}")))?;
        if self.stateful.contains_key(name)
            || self.stateful.values().any(|source| source.name == name)
            || self
                .pure
                .values()
                .filter(|source| source.name == name)
                .take(2)
                .count()
                != 1
            || source.name != name
        {
            return Err(invalid(
                path,
                "ambiguous pure/stateful declaration or inconsistent source name",
            ));
        }
        let local_work = self
            .budget
            .check(source)
            .map_err(|e| invalid(path, e.to_string()))?;
        // The native body checker can clone/coerce hidden local aggregate types.
        // Bound their fixed layout before it runs, including all unused branches
        // of this reachable body. Unreachable declarations are not charged.
        let annotations = crate::resource_limits::recording_pure_annotations(source)
            .map_err(|e| invalid(path, e.to_string()))?;
        super::type_environment::check_fixed_layouts(&annotations.types, path)?;
        native::check_named_structs(&annotations.types, path)?;
        native::check_defaults(&annotations.defaults, path)?;
        let normalized =
            crate::semantic_identifier(name).map_err(|e| invalid(path, e.to_string()))?;
        if self
            .normalized_names
            .get(&normalized)
            .is_some_and(|prior| prior != name)
        {
            return Err(invalid(
                path,
                "pure helper names collide after Rust normalization",
            ));
        }
        self.normalized_names.insert(normalized, name.into());
        // Resource scan above makes this existing bounded recursive visitor
        // safe to use. Returning false traverses every child, even unused/dead.
        let dependencies = std::cell::RefCell::new(Vec::new());
        crate::circuit_analysis::expression_contains(&source.body, &|node| {
            if let Expr::Call { name, .. } | Expr::VectorFoldCall { name, .. } = node {
                dependencies.borrow_mut().push(name.clone());
            }
            false
        });
        Ok(Frame {
            source,
            dependencies: dependencies.into_inner(),
            next: 0,
            local_work,
        })
    }

    /// Iterative DFS, memoized per immutable environment. Cached dependency
    /// summaries still contribute to every ancestor's depth/work bounds.
    pub(super) fn certify(&mut self, name: &str, path: &str) -> Result<(), RecordingGap> {
        if self.certificates.get(name).is_some() {
            return Ok(());
        }
        let (depth_limit, work_limit) = crate::resource_limits::recording_call_limits();
        let mut stack = vec![self.enter(name, path)?];
        let mut active = HashSet::from([name.to_owned()]);
        while let Some(frame) = stack.last_mut() {
            if let Some(dependency) = frame.dependencies.get(frame.next).cloned() {
                frame.next += 1;
                if active.contains(&dependency) {
                    return Err(invalid(
                        path,
                        format!("recursive pure helper {dependency:?}"),
                    ));
                }
                if let Some(done) = self.certificates.get(&dependency) {
                    if stack.len().saturating_add(done.depth) > depth_limit {
                        return Err(invalid(path, "pure helper call depth limit exceeded"));
                    }
                    continue;
                }
                if stack.len() >= depth_limit {
                    return Err(invalid(path, "pure helper call depth limit exceeded"));
                }
                let dependency_path = format!("{path}.callee[{dependency}]");
                let child = self.enter(&dependency, &dependency_path)?;
                active.insert(dependency);
                stack.push(child);
                continue;
            }
            let frame = stack.pop().expect("active pure frame");
            let mut depth = 1;
            let mut work = frame.local_work;
            for dependency in &frame.dependencies {
                let child = self
                    .certificates
                    .get(dependency)
                    .expect("completed dependency");
                depth = depth.max(child.depth.saturating_add(1));
                work = work.saturating_add(child.expanded_work);
            }
            if depth > depth_limit || work > work_limit {
                return Err(invalid(
                    path,
                    "pure helper depth or expanded work limit exceeded",
                ));
            }
            // Only now may native typing inspect callee signatures: all direct
            // dependencies have already passed their borrowed resource scans.
            let body_path = format!("{path}.callee[{}]", frame.source.name);
            native::check(frame.source, self.pure).map_err(|e| {
                invalid(
                    &body_path,
                    format!("pure helper {:?}: {e}", frame.source.name),
                )
            })?;
            active.remove(&frame.source.name);
            // First and only body clone, after borrowed resource checks and
            // successful native typing of this declaration and all callees.
            self.certificates.entries.insert(
                frame.source.name.clone(),
                CertifiedPure {
                    source: frame.source.clone(),
                    depth,
                    expanded_work: work,
                },
            );
        }
        Ok(())
    }
}

#[cfg(all(test, feature = "experimental-recording-plan"))]
mod resource_tests;
#[cfg(test)]
mod tests;
