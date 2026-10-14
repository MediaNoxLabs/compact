// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Actual private lowerer guards and complete checked-plan controls.
//! Snapshot failures are intentionally unavailable, never matched back to source.
use super::super::super::type_environment::TypeEnvironment;
use super::*;
use crate::source_diagnostics::Slot;
use std::cell::RefCell;

mod guards;
mod helpers;
mod roots;
mod snapshots;
mod success;
use helpers::*;

type Event = (Option<(&'static str, usize)>, Slot);
#[derive(Default)]
struct Trace {
    primary: RefCell<Option<Event>>,
    events: RefCell<Vec<Event>>,
    related: RefCell<Vec<Event>>,
}
fn event(site: RefusalSite<'_>) -> Event {
    let owner = site.owner.map(|owner| match owner {
        Owner::Expr(source) => ("expr", std::ptr::from_ref(source) as usize),
        Owner::Action(source) => ("action", std::ptr::from_ref(source) as usize),
        Owner::Stateful(source) => ("stateful", std::ptr::from_ref(source) as usize),
        Owner::Parameter(source) => ("parameter", std::ptr::from_ref(source) as usize),
        Owner::Type(source) => ("type", std::ptr::from_ref(source) as usize),
        _ => panic!("guard invented another source owner"),
    });
    (owner, site.slot)
}
impl RefusalObserver for Trace {
    fn refused(&self, site: RefusalSite<'_>) {
        let event = event(site);
        if self.primary.borrow().is_none() {
            *self.primary.borrow_mut() = Some(event);
        }
        self.events.borrow_mut().push(event);
    }
    fn related(&self, site: RefusalSite<'_>) {
        self.related.borrow_mut().push(event(site));
    }
}
fn primary(trace: &Trace, site: RefusalSite<'_>) {
    assert_eq!(*trace.primary.borrow(), Some(event(site)));
}
