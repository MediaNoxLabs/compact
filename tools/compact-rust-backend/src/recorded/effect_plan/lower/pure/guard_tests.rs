// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Actual original call/argument guards; snapshots and budget summaries stay unavailable.
use super::super::calls::CallSite;
use super::*;
use crate::ir::{Contract, Parameter, PureCircuit, StateReturn, StatefulCircuit, StructField};
use crate::source_diagnostics::Slot;
use std::cell::RefCell;
mod guards;
mod helpers;
mod integration;
mod resources;
mod workers;
use helpers::*;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Site(&'static str, usize, Slot);
fn site(source: RefusalSite<'_>) -> Option<Site> {
    source.owner.map(|owner| {
        let (kind, pointer) = match owner {
            Owner::Expr(value) => ("expr", std::ptr::from_ref(value) as usize),
            Owner::Action(value) => ("action", std::ptr::from_ref(value) as usize),
            Owner::Pure(value) => ("pure", std::ptr::from_ref(value) as usize),
            Owner::Parameter(value) => ("parameter", std::ptr::from_ref(value) as usize),
            Owner::Type(value) => ("type", std::ptr::from_ref(value) as usize),
            _ => panic!("unexpected original pure-call source"),
        };
        Site(kind, pointer, source.slot)
    })
}
#[derive(Default)]
struct Trace {
    primary: RefCell<Option<Option<Site>>>,
    events: RefCell<Vec<Option<Site>>>,
    related: RefCell<Vec<Option<Site>>>,
    available: Option<Vec<Site>>,
}
impl RefusalObserver for Trace {
    fn refused(&self, source: RefusalSite<'_>) {
        let event =
            site(source).filter(|s| self.available.as_ref().is_none_or(|ids| ids.contains(s)));
        if self.primary.borrow().is_none() {
            *self.primary.borrow_mut() = Some(event);
        }
        self.events.borrow_mut().push(event);
    }
    fn related(&self, source: RefusalSite<'_>) {
        self.related.borrow_mut().push(site(source));
    }
}
fn primary(trace: &Trace, expected: RefusalSite<'_>) {
    assert_eq!(*trace.primary.borrow(), Some(site(expected)));
}
