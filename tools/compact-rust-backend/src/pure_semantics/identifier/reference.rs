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

//! Exact test-only identifier functions from signed 807bed34.
//! Original naming.rs SHA-256:
//! 195add310b477edb75df58abb91ac7cc5ede949d399429cce04815bbd02683c0
//! A compatibility oracle only: production never delegates to this parser.

use crate::RenderError;

pub(crate) fn ident(name: &str) -> Result<syn::Ident, RenderError> {
    let rust_name = name.replace('$', "_");
    syn::parse_str::<syn::Ident>(&rust_name)
        .or_else(|_| syn::parse_str::<syn::Ident>(&format!("r#{rust_name}")))
        .map_err(|_| RenderError::InvalidIdentifier(name.to_owned()))
}

/// The name by which Rust resolves an identifier, without its optional raw prefix.
pub(crate) fn semantic_identifier(name: &str) -> Result<String, RenderError> {
    let rendered = ident(name)?.to_string();
    Ok(rendered.strip_prefix("r#").unwrap_or(&rendered).to_owned())
}
