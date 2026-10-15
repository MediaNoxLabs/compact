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

//! Typed names for compiler-declared ledger fields.
//!
//! Generated contracts expose one constant per supported ledger declaration. The runtime
//! owns VM execution and read-only state decoding; these descriptors tie a
//! value type to its physical ledger path. Their constructors are public to
//! support generated crates and are not an access-control boundary.

mod cell;
mod counter;
mod list;
mod map;
mod merkle;
mod set;

pub use cell::CellSlot;
pub use counter::CounterSlot;
pub use list::ListSlot;
pub use map::{MapNode, MapSlot};
pub use merkle::{HistoricMerkleStateView, MerkleSlot, PlainMerkleStateView};
pub use set::SetSlot;
