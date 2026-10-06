// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//  	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::runtime::CompactError;
use std::{collections::VecDeque, fmt};

/// Store this owned script inside the lab's private state so rollback/forks include it.
/// Implement each generated TryWitnesses trait with an ordinary typed adapter.
///
/// Script failures cross that adapter as `CompactError`, just like application
/// failures. A lab reports them as redacted execution errors. Inspect a known
/// test failure deliberately with `LabError::execution_error()`; assertion text
/// does not establish whether a script or the application produced the error.
#[derive(Clone)]
pub struct WitnessScript<A, R> {
    answers: VecDeque<(A, Result<R, CompactError>)>,
    journal: Vec<A>,
}
impl<A: PartialEq, R> WitnessScript<A, R> {
    pub fn new(answers: impl IntoIterator<Item = (A, Result<R, CompactError>)>) -> Self {
        Self {
            answers: answers.into_iter().collect(),
            journal: vec![],
        }
    }
    /// Mismatched arguments and exhaustion leave the queue and journal intact.
    /// Accepted invocations consume one answer and journal the arguments, even
    /// when the scripted answer is an error. A failed lab invocation rolls back
    /// its owned working script together with the other owned private state.
    pub fn answer(&mut self, arguments: A) -> Result<R, CompactError> {
        let Some((expected, _)) = self.answers.front() else {
            return Err(CompactError::AssertionFailed(
                "witness script exhausted".into(),
            ));
        };
        if expected != &arguments {
            return Err(CompactError::AssertionFailed(
                "witness arguments differ from script".into(),
            ));
        }
        let (_, answer) = self.answers.pop_front().expect("front checked");
        self.journal.push(arguments);
        answer
    }
    pub fn remaining(&self) -> usize {
        self.answers.len()
    }
    /// Potentially private arguments, accessible only by explicit request.
    pub fn journal(&self) -> &[A] {
        &self.journal
    }
}
impl<A, R> fmt::Debug for WitnessScript<A, R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WitnessScript")
            .field("remaining", &self.answers.len())
            .field("calls", &self.journal.len())
            .finish()
    }
}
