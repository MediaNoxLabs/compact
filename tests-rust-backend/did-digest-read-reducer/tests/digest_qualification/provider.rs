// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;
#[derive(Clone)]
pub(super) struct Provider {
    admit: bool,
    fail: bool,
    pub calls: Rc<RefCell<Vec<&'static str>>>,
    pub private: Rc<RefCell<Vec<u64>>>,
}
impl Provider {
    pub fn new(row: &Value, fail: bool) -> Self {
        Self {
            admit: row["admit"].as_bool().unwrap(),
            fail,
            calls: Default::default(),
            private: Default::default(),
        }
    }
    pub fn answer(&self, private: u64) -> Result<(u64, bool), r::CompactError> {
        self.calls.borrow_mut().push("admitted");
        self.private.borrow_mut().push(private);
        if self.fail {
            return Err(r::CompactError::AssertionFailed(
                "admitted provider refused".into(),
            ));
        }
        Ok((private + 1, self.admit))
    }
}
impl c::TryWitnesses<u64> for Provider {
    fn admitted(
        &self,
        ctx: WitnessContext<'_, u64, c::LedgerView<'_>>,
    ) -> Result<(u64, bool), r::CompactError> {
        self.answer(*ctx.private_state)
    }
}
