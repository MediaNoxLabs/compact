// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Reachable pure declarations certified through the shared semantic fold.
//! This is a bounded compiler certificate, not a formal proof or infallibility
//! guarantee. Body checking uses unit output and constructs no Rust syntax;
//! native emission consumes the same checked semantic owners.
mod checker;

use super::{RecordingGap, invalid};
use crate::ir::{Expr, PureCircuit, StatefulCircuit};
use crate::resource_limits::PureBodyBudget;
use crate::source_diagnostics::{Owner, RefusalObserver, RefusalSite};
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

#[derive(Clone)]
struct Dependency<'a> {
    name: String,
    source: &'a Expr,
}

struct Frame<'a> {
    source: &'a PureCircuit,
    dependencies: Vec<Dependency<'a>>,
    /// The actual parent occurrence selected by the existing DFS collector.
    called_from: Option<&'a Expr>,
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

    fn enter(
        &mut self,
        name: &str,
        path: &str,
        observer: Option<&dyn RefusalObserver>,
    ) -> Result<Frame<'a>, RecordingGap> {
        self.enter_from(name, path, observer, None)
    }

    /// The existing DFS supplies its actual incoming occurrence, never a lookup by value.
    fn enter_from(
        &mut self,
        name: &str,
        path: &str,
        observer: Option<&dyn RefusalObserver>,
        incoming: Option<&Expr>,
    ) -> Result<Frame<'a>, RecordingGap> {
        let source = self.pure.get(name).copied().ok_or_else(|| {
            refusal(observer, dependency_site(incoming));
            invalid(path, format!("unknown pure callee {name:?}"))
        })?;
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
            refusal(observer, dependency_site(incoming));
            return Err(invalid(
                path,
                "ambiguous pure/stateful declaration or inconsistent source name",
            ));
        }
        let local_work = self
            .budget
            .check_observed(source, observer)
            .map_err(|e| invalid(path, e.to_string()))?;
        // The shared body checker can clone/coerce hidden local aggregate types.
        // Bound their fixed layout before it runs, including all unused branches
        // of this reachable body. Unreachable declarations are not charged.
        let annotations =
            crate::resource_limits::recording_pure_annotations(source).map_err(|e| {
                refusal(observer, RefusalSite::unavailable());
                invalid(path, e.to_string())
            })?;
        if observer.is_some() {
            super::type_environment::check_fixed_layouts_observed(
                &annotations.types,
                path,
                observer,
            )?;
        } else {
            super::type_environment::check_fixed_layouts(&annotations.types, path)?;
        }
        if observer.is_some() {
            checker::check_named_structs_observed(&annotations.types, path, observer)?;
            checker::check_defaults_observed(&annotations.defaults, path, observer)?;
        } else {
            checker::check_named_structs(&annotations.types, path)?;
            checker::check_defaults(&annotations.defaults, path)?;
        }
        let normalized = crate::semantic_identifier(name).map_err(|e| {
            if let Some(observer) = observer {
                observer.refused(RefusalSite::field(Owner::Pure(source), "name"));
            }
            invalid(path, e.to_string())
        })?;
        if self
            .normalized_names
            .get(&normalized)
            .is_some_and(|prior| prior != name)
        {
            refusal(observer, RefusalSite::field(Owner::Pure(source), "name"));
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
                dependencies.borrow_mut().push(Dependency {
                    name: name.clone(),
                    source: node,
                });
            }
            false
        });
        Ok(Frame {
            source,
            dependencies: dependencies.into_inner(),
            called_from: None,
            next: 0,
            local_work,
        })
    }

    /// Iterative DFS, memoized per immutable environment. Cached dependency
    /// summaries still contribute to every ancestor's depth/work bounds.
    pub(super) fn certify(&mut self, name: &str, path: &str) -> Result<(), RecordingGap> {
        self.certify_observed(name, path, None)
    }
    pub(super) fn certify_observed(
        &mut self,
        name: &str,
        path: &str,
        observer: Option<&dyn RefusalObserver>,
    ) -> Result<(), RecordingGap> {
        if self.certificates.get(name).is_some() {
            return Ok(());
        }
        let (depth_limit, work_limit) = crate::resource_limits::recording_call_limits();
        let mut stack = vec![self.enter(name, path, observer)?];
        let mut active = HashSet::from([name.to_owned()]);
        while let Some(frame) = stack.last_mut() {
            if let Some(dependency) = frame.dependencies.get(frame.next).cloned() {
                frame.next += 1;
                if active.contains(&dependency.name) {
                    if let Some(observer) = observer {
                        observer
                            .refused(RefusalSite::field(Owner::Expr(dependency.source), "name"));
                    }
                    related_edges(observer, &stack, None);
                    return Err(invalid(
                        path,
                        format!("recursive pure helper {:?}", dependency.name),
                    ));
                }
                if let Some(done) = self.certificates.get(&dependency.name) {
                    if stack.len().saturating_add(done.depth) > depth_limit {
                        if let Some(observer) = observer {
                            observer.refused(RefusalSite::field(
                                Owner::Expr(dependency.source),
                                "name",
                            ));
                        }
                        related_edges(observer, &stack, None);
                        return Err(invalid(path, "pure helper call depth limit exceeded"));
                    }
                    continue;
                }
                if stack.len() >= depth_limit {
                    if let Some(observer) = observer {
                        observer
                            .refused(RefusalSite::field(Owner::Expr(dependency.source), "name"));
                    }
                    related_edges(observer, &stack, None);
                    return Err(invalid(path, "pure helper call depth limit exceeded"));
                }
                let dependency_path = format!("{path}.callee[{}]", dependency.name);
                let mut child = self
                    .enter_from(
                        &dependency.name,
                        &dependency_path,
                        observer,
                        Some(dependency.source),
                    )
                    .inspect_err(|_| related_edges(observer, &stack, Some(dependency.source)))?;
                child.called_from = Some(dependency.source);
                active.insert(dependency.name);
                stack.push(child);
                continue;
            }
            let frame = stack.pop().expect("active pure frame");
            let mut depth = 1;
            let mut work = frame.local_work;
            for dependency in &frame.dependencies {
                let child = self
                    .certificates
                    .get(&dependency.name)
                    .expect("completed dependency");
                depth = depth.max(child.depth.saturating_add(1));
                work = work.saturating_add(child.expanded_work);
            }
            if depth > depth_limit || work > work_limit {
                refusal(observer, RefusalSite::unavailable());
                related_edges(observer, &stack, frame.called_from);
                return Err(invalid(
                    path,
                    "pure helper depth or expanded work limit exceeded",
                ));
            }
            // Only now may shared typing inspect callee signatures: all direct
            // dependencies have already passed their borrowed resource scans.
            let body_path = format!("{path}.callee[{}]", frame.source.name);
            (if observer.is_some() {
                checker::check_observed(frame.source, self.pure, observer)
            } else {
                checker::check(frame.source, self.pure)
            })
            .map_err(|e| {
                related_edges(observer, &stack, frame.called_from);
                invalid(
                    &body_path,
                    format!("pure helper {:?}: {e}", frame.source.name),
                )
            })?;
            active.remove(&frame.source.name);
            // First and only body clone, after borrowed resource checks and
            // successful shared typing of this declaration and all callees.
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

#[cfg(test)]
mod diagnostic_tests;

/// Append only active original edges, inner-to-outer, after the real failure.
/// The already-checked DFS depth bounds this loop; no new traversal or policy.
fn related_edges(
    observer: Option<&dyn RefusalObserver>,
    stack: &[Frame<'_>],
    current: Option<&Expr>,
) {
    let Some(observer) = observer else {
        return;
    };
    if let Some(source) = current {
        observer.related(RefusalSite::node(Owner::Expr(source)));
    }
    for frame in stack.iter().rev() {
        if let Some(source) = frame.called_from {
            observer.related(RefusalSite::node(Owner::Expr(source)));
        }
    }
}

#[cfg(test)]
mod call_diagnostic_tests;

fn dependency_site(incoming: Option<&Expr>) -> RefusalSite<'_> {
    incoming.map_or_else(RefusalSite::unavailable, |source| {
        RefusalSite::field(Owner::Expr(source), "name")
    })
}
fn refusal(observer: Option<&dyn RefusalObserver>, site: RefusalSite<'_>) {
    if let Some(observer) = observer {
        observer.refused(site);
    }
}

#[cfg(test)]
mod source_guard_tests;
