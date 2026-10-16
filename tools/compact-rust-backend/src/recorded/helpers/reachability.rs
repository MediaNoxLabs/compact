// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Emission reachability after complete helper and public-body validation.
//!
//! The registry comes from allocated helper items, never a naming prefix or IR
//! call shape. Unknown syntax conservatively keeps candidates. This pass does
//! not change the shared-name set used for admission or skip any validation.
use std::collections::{HashMap, HashSet};
use syn::{Item, ext::IdentExt, visit_mut::VisitMut};

#[cfg(test)]
mod tests;

/// Keep reachable private helpers in their original order. Public syntax is
/// borrowed mutably only because our existing syn feature is `visit-mut`:
/// the visitor never changes nodes, avoiding a clone of recursive generated ASTs.
pub(crate) fn retain_reachable_helpers(
    mut helpers: Vec<Item>,
    public_items: &mut [Item],
    public_methods: &mut [syn::ImplItemFn],
    borrowed_methods: &mut [syn::ImplItemFn],
) -> Vec<Item> {
    let Some(registry) = registry(&helpers) else {
        // New producer item kinds or ambiguous allocation must never cause a
        // false deletion. Qualification can teach this pass their semantics.
        return helpers;
    };
    if registry.is_empty() {
        return helpers;
    }
    let mut roots = references(public_items, &registry);
    for methods in [public_methods, borrowed_methods] {
        let mut visitor = RecordedHelperReferences {
            registry: &registry,
            found: References::default(),
        };
        for method in methods {
            visitor.visit_impl_item_fn_mut(method);
        }
        roots.targets.extend(visitor.found.targets);
        roots.opaque |= visitor.found.opaque;
    }
    if roots.opaque {
        return helpers;
    }
    let edges: Vec<_> = helpers
        .iter_mut()
        .map(|item| references(std::slice::from_mut(item), &registry))
        .collect();
    // Opaque helper syntax may expand declarations/references at module scope
    // even when the ordinary function body appears unreachable.
    if edges.iter().any(|references| references.opaque) {
        return helpers;
    }
    let mut pending: Vec<_> = roots.targets.into_iter().collect();
    let mut reachable = HashSet::new();
    while let Some(index) = pending.pop() {
        if !reachable.insert(index) {
            continue;
        }
        let references = &edges[index];
        pending.extend(references.targets.iter().copied());
    }
    helpers
        .into_iter()
        .enumerate()
        .filter_map(|(index, item)| reachable.contains(&index).then_some(item))
        .collect()
}

fn registry(helpers: &[Item]) -> Option<HashMap<String, usize>> {
    let mut registry = HashMap::new();
    for (index, item) in helpers.iter().enumerate() {
        let Item::Fn(function) = item else {
            return None;
        };
        if !matches!(function.vis, syn::Visibility::Inherited) {
            return None;
        }
        // Already allocated Rust identifiers have undergone Compact `$`
        // normalization. Raw spelling does not change Rust name identity.
        let name = function.sig.ident.unraw().to_string();
        if registry.insert(name, index).is_some() {
            return None;
        }
    }
    Some(registry)
}

#[derive(Default)]
struct References {
    targets: HashSet<usize>,
    opaque: bool,
}
struct RecordedHelperReferences<'a> {
    registry: &'a HashMap<String, usize>,
    found: References,
}
fn references(items: &mut [Item], registry: &HashMap<String, usize>) -> References {
    let mut visitor = RecordedHelperReferences {
        registry,
        found: References::default(),
    };
    for item in items {
        visitor.visit_item_mut(item);
    }
    visitor.found
}

impl VisitMut for RecordedHelperReferences<'_> {
    fn visit_path_mut(&mut self, path: &mut syn::Path) {
        // Include qualified paths and function values, not only direct calls.
        // A same-spelled local/type may over-retain, but cannot delete a live
        // helper. All nested generic expressions are visited normally.
        for segment in &path.segments {
            if let Some(index) = self.registry.get(&segment.ident.unraw().to_string()) {
                self.found.targets.insert(*index);
            }
        }
        syn::visit_mut::visit_path_mut(self, path);
    }
    fn visit_macro_mut(&mut self, _: &mut syn::Macro) {
        // Expansion can synthesize a reference without a literal helper name.
        self.found.opaque = true;
    }
    fn visit_attribute_mut(&mut self, attribute: &mut syn::Attribute) {
        // These built-in attributes cannot introduce a helper reference. An
        // unknown/custom attribute (including cfg_attr) may expand arbitrary
        // syntax, so retain candidates rather than interpret opaque tokens.
        if ![
            "doc", "allow", "warn", "deny", "forbid", "expect", "cfg", "inline",
        ]
        .iter()
        .any(|name| attribute.path().is_ident(name))
        {
            self.found.opaque = true;
        }
    }
    fn visit_expr_mut(&mut self, node: &mut syn::Expr) {
        self.found.opaque |= matches!(node, syn::Expr::Verbatim(_));
        syn::visit_mut::visit_expr_mut(self, node);
    }
    fn visit_type_mut(&mut self, node: &mut syn::Type) {
        self.found.opaque |= matches!(node, syn::Type::Verbatim(_));
        syn::visit_mut::visit_type_mut(self, node);
    }
    fn visit_pat_mut(&mut self, node: &mut syn::Pat) {
        self.found.opaque |= matches!(node, syn::Pat::Verbatim(_));
        syn::visit_mut::visit_pat_mut(self, node);
    }
    fn visit_item_mut(&mut self, node: &mut Item) {
        // UseTree aliases/globs can hide a helper behind a different local name.
        self.found.opaque |= matches!(node, Item::Verbatim(_) | Item::Use(_));
        syn::visit_mut::visit_item_mut(self, node);
    }
    fn visit_impl_item_mut(&mut self, node: &mut syn::ImplItem) {
        self.found.opaque |= matches!(node, syn::ImplItem::Verbatim(_));
        syn::visit_mut::visit_impl_item_mut(self, node);
    }
    fn visit_trait_item_mut(&mut self, node: &mut syn::TraitItem) {
        self.found.opaque |= matches!(node, syn::TraitItem::Verbatim(_));
        syn::visit_mut::visit_trait_item_mut(self, node);
    }
    fn visit_foreign_item_mut(&mut self, node: &mut syn::ForeignItem) {
        self.found.opaque |= matches!(node, syn::ForeignItem::Verbatim(_));
        syn::visit_mut::visit_foreign_item_mut(self, node);
    }
}
