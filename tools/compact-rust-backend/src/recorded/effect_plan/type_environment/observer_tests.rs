// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Private source-identity controls; root owns execution and physical binding.
use super::*;
use crate::ir::{Expr, LocalBinding, NativeWitnessBuiltin, Parameter, StateAction, StateReturn};
use crate::source_diagnostics::Slot;
use std::cell::RefCell;

mod helpers;
mod old_reference;
use helpers::*;
mod pipeline;
mod sources;
mod validation;
