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

//! The bounded, token-free identifier policy shared by semantic checks and output.
//!
//! Rules adapted from syn 2.0.117 `src/ident.rs` and proc-macro2 1.0.106
//! `src/parse.rs`, `src/fallback.rs` (David Tolnay and contributors), under
//! their Apache-2.0 license option. This specialized scanner preserves the
//! existing parser contract; it does not parse arbitrary Rust token streams.
//!
//! Callers retain the compiler's borrowed source-resource gates. Scanning is
//! linear in source bytes, comments use constant stack space, and allocation
//! is limited to normalized source/accepted spelling or the existing error.

use std::borrow::Cow;

use crate::RenderError;

/// A validated spelling. Only this module can create or alter its facts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CheckedName {
    text: String,
    raw: bool,
}

impl CheckedName {
    pub(crate) fn text(&self) -> &str {
        &self.text
    }

    pub(crate) fn is_raw(&self) -> bool {
        self.raw
    }

    pub(crate) fn into_text(self) -> String {
        self.text
    }
}

/// Preserve Compact `$` normalization, whole-text parsing, then the old
/// **untrimmed** raw retry. Whitespace-prefixed keywords must still refuse.
pub(crate) fn check_name(name: &str) -> Result<CheckedName, RenderError> {
    let normalized = if name.contains('$') {
        Cow::Owned(name.replace('$', "_"))
    } else {
        Cow::Borrowed(name)
    };
    parse_identifier(&normalized)
        .or_else(|| parse_raw_retry(&normalized))
        .map(|(text, raw)| CheckedName {
            text: text.to_owned(),
            raw,
        })
        .ok_or_else(|| RenderError::InvalidIdentifier(name.to_owned()))
}

fn parse_identifier(input: &str) -> Option<(&str, bool)> {
    // The pinned token-stream lexer strips exactly one initial byte-order mark.
    let input = input.strip_prefix('\u{feff}').unwrap_or(input);
    let input = trivia(input)?;
    let (input, raw) = match input.strip_prefix("r#") {
        Some(rest) => (rest, true),
        None => (input, false),
    };
    let (text, rest) = word(input)?;
    if !trivia(rest)?.is_empty() || !accepted(text, raw) {
        return None;
    }
    Some((text, raw))
}

fn parse_raw_retry(input: &str) -> Option<(&str, bool)> {
    // Equivalent to parsing r# + input: there is no trivia or BOM between the
    // prefix and the first name character, and an existing r# is not nested.
    let (text, rest) = word(input)?;
    if !trivia(rest)?.is_empty() || !accepted(text, true) {
        return None;
    }
    Some((text, true))
}

fn word(input: &str) -> Option<(&str, &str)> {
    let mut chars = input.char_indices();
    let (_, first) = chars.next()?;
    if first != '_' && !unicode_ident::is_xid_start(first) {
        return None;
    }
    let end = chars
        .find(|(_, ch)| !unicode_ident::is_xid_continue(*ch))
        .map_or(input.len(), |(index, _)| index);
    Some(input.split_at(end))
}

fn accepted(text: &str, raw: bool) -> bool {
    if raw {
        !matches!(text, "_" | "super" | "self" | "Self" | "crate")
    } else {
        // Exact syn 2.0.117 keyword list, based on the Rust 1.65 reference.
        // Contextual/reserved words absent from that list remain accepted.
        !matches!(
            text,
            "_" | "abstract"
                | "as"
                | "async"
                | "await"
                | "become"
                | "box"
                | "break"
                | "const"
                | "continue"
                | "crate"
                | "do"
                | "dyn"
                | "else"
                | "enum"
                | "extern"
                | "false"
                | "final"
                | "fn"
                | "for"
                | "if"
                | "impl"
                | "in"
                | "let"
                | "loop"
                | "macro"
                | "match"
                | "mod"
                | "move"
                | "mut"
                | "override"
                | "priv"
                | "pub"
                | "ref"
                | "return"
                | "Self"
                | "self"
                | "static"
                | "struct"
                | "super"
                | "trait"
                | "true"
                | "try"
                | "type"
                | "typeof"
                | "unsafe"
                | "unsized"
                | "use"
                | "virtual"
                | "where"
                | "while"
                | "yield"
        )
    }
}

/// Skip only ordinary comments and the pinned lexer's whitespace. A doc
/// comment is an attribute token, so it cannot surround an Ident-only parse.
fn trivia(mut input: &str) -> Option<&str> {
    loop {
        if input.starts_with("//")
            && (!input.starts_with("///") || input.starts_with("////"))
            && !input.starts_with("//!")
        {
            input = input.find('\n').map_or("", |index| &input[index..]);
        } else if let Some(rest) = input.strip_prefix("/**/") {
            input = rest;
        } else if input.starts_with("/*")
            && (!input.starts_with("/**") || input.starts_with("/***"))
            && !input.starts_with("/*!")
        {
            input = block_comment(input)?;
        } else if let Some(ch) = input.chars().next()
            && (ch.is_whitespace() || ch == '\u{200e}' || ch == '\u{200f}')
        {
            input = &input[ch.len_utf8()..];
        } else {
            return Some(input);
        }
    }
}

fn block_comment(input: &str) -> Option<&str> {
    let bytes = input.as_bytes();
    let mut depth = 0usize;
    let mut index = 0usize;
    while index + 1 < bytes.len() {
        if bytes[index] == b'/' && bytes[index + 1] == b'*' {
            depth += 1;
            index += 1;
        } else if bytes[index] == b'*' && bytes[index + 1] == b'/' {
            depth -= 1;
            if depth == 0 {
                return Some(&input[index + 2..]);
            }
            index += 1;
        }
        index += 1;
    }
    None
}

#[cfg(test)]
mod tests;
