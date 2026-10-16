// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Test-only observation of one native traversal. Never compiler authority.
use crate::ir::Expr;
use std::{cell::RefCell, collections::HashMap};

thread_local! {
    static VISITS: RefCell<Option<HashMap<usize, usize>>> = const { RefCell::new(None) };
}

pub(crate) fn record(expr: &Expr) {
    VISITS.with(|visits| {
        if let Some(visits) = visits.borrow_mut().as_mut() {
            *visits.entry(std::ptr::from_ref(expr) as usize).or_default() += 1;
        }
    });
}

pub(crate) fn observe<R>(run: impl FnOnce() -> R) -> (R, HashMap<usize, usize>) {
    struct Reset;
    impl Drop for Reset {
        fn drop(&mut self) {
            VISITS.with(|visits| {
                *visits.borrow_mut() = None;
            });
        }
    }
    VISITS.with(|visits| {
        assert!(
            visits.borrow().is_none(),
            "visit observation must be one traversal"
        );
        *visits.borrow_mut() = Some(HashMap::new());
    });
    let reset = Reset;
    let result = run();
    let visits = VISITS.with(|visits| visits.borrow_mut().take().expect("active observer"));
    drop(reset);
    (result, visits)
}
