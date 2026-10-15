// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Original root layout error participants, not inferred physical source ranges.
use super::*;
use crate::ir::SourceLocation;

type Event = (&'static str, usize, Slot);
#[derive(Default)]
struct LayoutTrace(RefCell<Vec<Event>>);
impl RefusalObserver for LayoutTrace {
    fn refused(&self, site: RefusalSite<'_>) {
        let (kind, pointer) = match site.owner.expect("original layout participant") {
            Owner::Ledger(value) => ("ledger", std::ptr::from_ref(value) as usize),
            Owner::LedgerKind(value) => ("kind", std::ptr::from_ref(value) as usize),
            _ => panic!("unexpected layout owner"),
        };
        self.0.borrow_mut().push((kind, pointer, site.slot));
    }
}
fn location(line: usize) -> SourceLocation {
    SourceLocation {
        file: "layout.compact".into(),
        line,
        column: 1,
    }
}
fn contract(fields: Vec<LedgerField>) -> Contract {
    Contract {
        schema_version: SCHEMA_VERSION,
        type_aliases: vec![],
        ledger_fields: fields,
        constructor: None,
        witnesses: vec![],
        circuits: vec![],
        stateful_circuits: vec![],
    }
}
fn fields(count: usize) -> Vec<LedgerField> {
    expected_ledger_paths(count)
        .into_iter()
        .enumerate()
        .map(|(position, path)| LedgerField {
            source: Some(location(position + 1)),
            id: format!("stored_{position}"),
            index: path[0],
            path,
            declaration: LedgerFieldKind::Counter,
        })
        .collect()
}
fn error(source: &Contract, trace: &LayoutTrace, expected: RenderError) {
    let bare = crate::render_with_native_refusal(source, None, None)
        .err()
        .expect("layout refusal");
    let observed = crate::render_with_native_refusal(source, None, Some(trace))
        .err()
        .expect("observed layout refusal");
    assert_eq!(observed, bare, "original typed and Located cause unchanged");
    assert_eq!(observed, expected);
}
fn ledger_site(trace: &LayoutTrace, field: &LedgerField, slot: Slot) {
    assert_eq!(
        trace.0.borrow().as_slice(),
        &[("ledger", std::ptr::from_ref(field) as usize, slot)]
    );
}
fn parity(source: &Contract) {
    let trace = LayoutTrace::default();
    let bare = crate::render_with_native_refusal(source, None, None).unwrap();
    let observed = crate::render_with_native_refusal(source, None, Some(&trace)).unwrap();
    assert_eq!(observed.source, bare.source);
    assert_eq!(
        serde_json::to_value(observed.capabilities).unwrap(),
        serde_json::to_value(bare.capabilities).unwrap()
    );
    assert!(trace.0.borrow().is_empty());
}

#[test]
fn layout_index_and_combined_path_guards_keep_original_field_and_located_cause() {
    for case in 0..3 {
        let mut source = contract(fields(1));
        let target = &mut source.ledger_fields[0];
        let (path, slot) = match case {
            0 => {
                target.path.clear();
                target.index = 1;
                (vec![1], Slot::Field("index"))
            }
            1 => {
                target.path = vec![1];
                (vec![1], Slot::Node)
            }
            _ => {
                target.index = 1;
                (vec![0], Slot::Node)
            }
        };
        let target = &source.ledger_fields[0];
        let expected = if case == 0 {
            RenderError::InvalidLedgerIndex(1)
        } else {
            RenderError::InvalidLedgerPath(path)
        }
        .at(target.source.as_ref());
        let trace = LayoutTrace::default();
        error(&source, &trace, expected);
        ledger_site(&trace, target, slot);
    }
}

#[test]
fn merkle_depth_is_original_kind_and_prior_layout_refusal_still_wins() {
    for historic in [false, true] {
        for depth in [1, 33] {
            let mut source = contract(fields(1));
            source.ledger_fields[0].declaration = if historic {
                LedgerFieldKind::HistoricMerkleTree {
                    depth,
                    ty: Type::Field,
                }
            } else {
                LedgerFieldKind::MerkleTree {
                    depth,
                    ty: Type::Field,
                }
            };
            let target = &source.ledger_fields[0];
            let trace = LayoutTrace::default();
            error(
                &source,
                &trace,
                RenderError::InvalidMerkleTreeDepth(depth).at(target.source.as_ref()),
            );
            assert_eq!(
                trace.0.borrow().as_slice(),
                &[(
                    "kind",
                    std::ptr::from_ref(&target.declaration) as usize,
                    Slot::Field("depth")
                )]
            );
            source.ledger_fields[0].index = 1;
            let target = &source.ledger_fields[0];
            let trace = LayoutTrace::default();
            error(
                &source,
                &trace,
                RenderError::InvalidLedgerPath(vec![0]).at(target.source.as_ref()),
            );
            ledger_site(&trace, target, Slot::Node);
        }
    }
}

#[test]
fn duplicate_site_is_later_sorted_original_field_not_last_input_or_equal_name() {
    let mut originals = fields(2);
    for field in &mut originals {
        field.id = "duplicate".into();
    }
    originals.reverse();
    let source = contract(originals);
    assert_eq!(source.ledger_fields[0].path, [1]);
    let trace = LayoutTrace::default();
    error(
        &source,
        &trace,
        RenderError::DuplicateLedgerField("duplicate".into())
            .at(source.ledger_fields[0].source.as_ref()),
    );
    ledger_site(&trace, &source.ledger_fields[0], Slot::Field("id"));
    let mut source = source;
    source.ledger_fields[1].index = 1;
    let trace = LayoutTrace::default();
    error(
        &source,
        &trace,
        RenderError::InvalidLedgerPath(vec![0]).at(source.ledger_fields[1].source.as_ref()),
    );
    ledger_site(&trace, &source.ledger_fields[1], Slot::Node);
}

#[test]
fn unsupported_three_segment_path_reaches_existing_guard_after_expected_path_match() {
    // 226 fields require 16 outer groups, whose batching itself has two segments.
    // Each leaf adds one segment; this uses the actual production expected paths.
    let expected = expected_ledger_paths(226);
    assert_eq!(expected[0], [0, 0, 0]);
    assert!(expected.iter().all(|path| path.len() == 3));
    let source = contract(fields(226));
    for (field, expected) in source.ledger_fields.iter().zip(&expected) {
        assert_eq!(field.physical_path(), *expected);
        assert_eq!(field.physical_path().first(), Some(&field.index));
    }
    let target = &source.ledger_fields[0];
    let trace = LayoutTrace::default();
    error(
        &source,
        &trace,
        RenderError::UnsupportedLedgerPath(expected[0].clone()).at(target.source.as_ref()),
    );
    ledger_site(&trace, target, Slot::Field("path"));
}

#[test]
fn accepted_batching_and_merkle_depth_boundaries_keep_complete_output_without_events() {
    for count in [0, 1, 15, 16, 225] {
        parity(&contract(fields(count)));
    }
    for historic in [false, true] {
        for depth in [2, 32] {
            let mut source = contract(fields(1));
            source.ledger_fields[0].declaration = if historic {
                LedgerFieldKind::HistoricMerkleTree {
                    depth,
                    ty: Type::Field,
                }
            } else {
                LedgerFieldKind::MerkleTree {
                    depth,
                    ty: Type::Field,
                }
            };
            parity(&source);
        }
    }
    let original = contract(fields(1));
    let mut sources = [original.clone(), original];
    let mut identities = Vec::new();
    for source in &mut sources {
        source.ledger_fields[0].path.clear();
        source.ledger_fields[0].index = 1;
        let trace = LayoutTrace::default();
        let target = &source.ledger_fields[0];
        error(
            source,
            &trace,
            RenderError::InvalidLedgerIndex(1).at(target.source.as_ref()),
        );
        ledger_site(&trace, target, Slot::Field("index"));
        identities.push(std::ptr::from_ref(target) as usize);
    }
    assert_ne!(identities[0], identities[1]);
}
