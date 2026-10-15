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

//! Render regression tests grouped by compiler behavior in a single test binary.
//! Test bodies and assertions stay with their domain; shared fixture setup is in `support`.

#[path = "render/asset_recording.rs"]
mod asset_recording;
#[path = "render/constructors.rs"]
mod constructors;
#[path = "render/diagnostics.rs"]
mod diagnostics;
#[path = "render/historic_merkle_recording.rs"]
mod historic_merkle_recording;
#[path = "render/ledger_views.rs"]
mod ledger_views;
#[path = "render/lifecycle_recording.rs"]
mod lifecycle_recording;
#[path = "render/list_recording.rs"]
mod list_recording;
#[path = "render/map_recording.rs"]
mod map_recording;
#[path = "render/membership_recording.rs"]
mod membership_recording;
#[path = "render/merkle_recording.rs"]
mod merkle_recording;
#[path = "render/numeric_types.rs"]
mod numeric_types;
#[path = "render/private_effects.rs"]
mod private_effects;
#[path = "render/pure_rendering.rs"]
mod pure_rendering;
#[path = "render/pure_validation.rs"]
mod pure_validation;
#[path = "render/recorded_calls.rs"]
mod recorded_calls;
#[path = "render/recorded_conditionals.rs"]
mod recorded_conditionals;
#[path = "render/recorded_guards.rs"]
mod recorded_guards;
#[path = "render/recorded_hashes.rs"]
mod recorded_hashes;
#[path = "render/recorded_returns.rs"]
mod recorded_returns;
#[path = "render/recorded_structs.rs"]
mod recorded_structs;
#[path = "render/recorded_witnesses.rs"]
mod recorded_witnesses;
#[path = "render/recording_capabilities.rs"]
mod recording_capabilities;
#[path = "render/set_recording.rs"]
mod set_recording;
#[path = "render/shielded_recording.rs"]
mod shielded_recording;
#[path = "render/state_validation.rs"]
mod state_validation;
#[path = "render/support.rs"]
mod support;
#[path = "render/value_types.rs"]
mod value_types;
