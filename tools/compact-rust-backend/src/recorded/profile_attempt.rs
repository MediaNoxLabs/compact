// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! A closed recording profile may decline a circuit or reject its audited body.

use super::RecordingGap;

pub(super) enum ProfileAttempt<T> {
    NotApplicable,
    Rejected(RecordingGap),
    Admitted(T),
}

impl<T> ProfileAttempt<T> {
    pub(super) fn into_option(self) -> Option<T> {
        match self {
            Self::Admitted(value) => Some(value),
            Self::NotApplicable | Self::Rejected(_) => None,
        }
    }
}
