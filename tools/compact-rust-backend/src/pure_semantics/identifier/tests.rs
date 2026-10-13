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

//! Differential qualification of name policy, native spelling and collisions.

use super::check_name;
use crate::{RenderError, naming};

#[path = "reference.rs"]
mod reference;

fn compare(name: &str) {
    let old = reference::ident(name).map(|identifier| identifier.to_string());
    let new = check_name(name).map(|name| naming::checked_ident(&name).to_string());
    assert_eq!(new, old, "rendered identifier for {name:?}");
    assert_eq!(
        naming::semantic_identifier(name),
        reference::semantic_identifier(name),
        "semantic identifier for {name:?}"
    );
}

#[test]
fn every_pinned_keyword_and_raw_exclusion_matches_old_parser() {
    let keywords = [
        "_",
        "abstract",
        "as",
        "async",
        "await",
        "become",
        "box",
        "break",
        "const",
        "continue",
        "crate",
        "do",
        "dyn",
        "else",
        "enum",
        "extern",
        "false",
        "final",
        "fn",
        "for",
        "if",
        "impl",
        "in",
        "let",
        "loop",
        "macro",
        "match",
        "mod",
        "move",
        "mut",
        "override",
        "priv",
        "pub",
        "ref",
        "return",
        "Self",
        "self",
        "static",
        "struct",
        "super",
        "trait",
        "true",
        "try",
        "type",
        "typeof",
        "unsafe",
        "unsized",
        "use",
        "virtual",
        "where",
        "while",
        "yield",
        // Contextual/new keywords intentionally do not replace the pinned list.
        "union",
        "auto",
        "default",
        "macro_rules",
        "gen",
        "safe",
        "raw",
    ];
    for keyword in keywords {
        for before in ["", " ", "/* ordinary */", "\u{200e}", "\u{feff}"] {
            for after in ["", " ", "/* ordinary */", "// ordinary", "\u{200f}"] {
                compare(&format!("{before}{keyword}{after}"));
                compare(&format!("{before}r#{keyword}{after}"));
            }
        }
    }
    assert_eq!(naming::ident("type").unwrap().to_string(), "r#type");
    assert_eq!(
        naming::ident(" type"),
        Err(RenderError::InvalidIdentifier(" type".into()))
    );
    assert_eq!(
        naming::ident("self"),
        Err(RenderError::InvalidIdentifier("self".into()))
    );
}

#[test]
fn comments_docs_bom_and_whitespace_match_old_parser() {
    let trivia = [
        "",
        " ",
        "\t\r\n\x0b\x0c",
        "\u{0085}",
        "\u{00a0}",
        "\u{1680}",
        "\u{2000}\u{2001}\u{2002}\u{2003}\u{2004}\u{2005}\u{2006}",
        "\u{2007}\u{2008}\u{2009}\u{200a}\u{2028}\u{2029}\u{202f}\u{205f}\u{3000}",
        "\u{200e}\u{200f}",
        "\u{feff}",
        "\u{feff}\u{feff}",
        "/**/",
        "/***/",
        "/****/",
        "/* outer /* inner */ end */",
        "/* unterminated",
        "/* /* */",
        "*/",
        "/*/",
        "/** doc */",
        "/*! doc */",
        "// normal\n",
        "//// normal\n",
        "/// doc\n",
        "//! doc\n",
        "// normal\r\n",
        "// normal\r",
        "// normal",
        "//\0\n",
        "/* \0 */",
        "/* outer /** inner doc */ */",
    ];
    for before in trivia {
        for after in trivia {
            for name in ["foo", "r#foo", "type", "r#type"] {
                compare(&format!("{before}{name}{after}"));
            }
        }
    }
}

#[test]
fn unicode_xid_and_non_identifier_tokens_match_old_parser() {
    let names = [
        "",
        "_",
        "__",
        "x_y",
        "$",
        "$foo",
        "foo$bar",
        "r#$foo",
        "r#_",
        "r#self",
        "r#Self",
        "r#crate",
        "r#super",
        "r#r#foo",
        "r#",
        "r##foo",
        "é",
        "e\u{301}",
        "\u{301}e",
        "变量",
        "Σ",
        "א",
        "क",
        "ᚠ",
        "℘",
        "＿",
        "a９",
        "９a",
        "a\u{200c}",
        "a\u{200d}",
        "a\u{200b}",
        "🦀",
        "a🦀",
        "'a",
        "'static",
        "1",
        "1foo",
        "foo bar",
        "foo::bar",
        "(foo)",
        "{foo}",
        "[foo]",
        "foo;",
        "foo!",
        "\"foo\"",
        "r\"foo\"",
        "r#\"foo\"#",
        "b'c'",
        "br#\"foo\"#",
        "cr#\"foo\"#",
        "(/*ERROR*/)",
    ];
    for name in names {
        compare(name);
        compare(&format!("r#{name}"));
        compare(&format!("/* a */{name}// b"));
    }
    // Sample across the Unicode scalar space in start/continuation positions.
    // Unicode policy itself is supplied by the exact locked unicode-ident.
    for scalar in (0..=0x10ffff).step_by(257) {
        if let Some(ch) = char::from_u32(scalar) {
            compare(&format!("{ch}x"));
            compare(&format!("x{ch}"));
            compare(&format!("r#{ch}x"));
        }
    }
}

#[test]
fn systematic_short_mutations_match_old_parser() {
    let alphabet = [
        'a', 'r', '_', '$', '#', '/', '*', '!', ' ', '\n', '\r', '\0', '\'', '"', '0', 'é',
        '\u{301}', '\u{200e}', '\u{feff}',
    ];
    compare("");
    for a in alphabet {
        compare(&a.to_string());
        for b in alphabet {
            compare(&format!("{a}{b}"));
            for c in alphabet {
                compare(&format!("{a}{b}{c}"));
            }
        }
    }
}

#[test]
fn deeply_nested_comments_and_long_names_use_no_recursive_scanner() {
    let nesting = 8192;
    let comment = format!("{}{}", "/*".repeat(nesting), "*/".repeat(nesting));
    compare(&format!("{comment}field{comment}"));
    compare(&format!("field{}", "/*".repeat(nesting)));
    compare(&"x".repeat(262144));
}

#[test]
fn normalized_namespace_collisions_match_old_semantic_keys() {
    for (first, second) in [
        ("a$b", "a_b"),
        ("foo", "r#foo"),
        ("type", "r#type"),
        ("/* comment */foo", "foo"),
        ("\u{feff}foo", "foo"),
        ("é", "r#é"),
        ("$foo", "_foo"),
    ] {
        let key = reference::semantic_identifier(first).unwrap();
        assert_eq!(reference::semantic_identifier(second).unwrap(), key);
        assert_eq!(
            naming::validate_circuit_function_namespace([(first, None), (second, None)]),
            Err(RenderError::ConflictingCircuitIdentifier {
                first: first.into(),
                second: second.into(),
                rust_identifier: key,
            })
        );
    }
    // Rust resolves these distinct Unicode spellings as the old parser did;
    // this refactor introduces no normalization beyond Compact's `$` rule.
    let composed = "é";
    let decomposed = "e\u{301}";
    assert_ne!(
        reference::semantic_identifier(composed),
        reference::semantic_identifier(decomposed)
    );
    assert!(
        naming::validate_circuit_function_namespace([(composed, None), (decomposed, None)]).is_ok()
    );
}
