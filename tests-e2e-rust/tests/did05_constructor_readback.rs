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

//
// A28 constructor read-your-writes regression guard for the did.compact 0.5.0
// fixture.
//
// The 0.5.0 constructor writes `controllerPublicKey` / `recoveryAuthorityPublicKey`
// from witnesses, then calls
// `assertControllerPublicKeyDistinctFromRecoveryAuthority(controllerPublicKey)`
// — a circuit that reads BOTH keys from the ledger. Before A28 the codegen
// batched every write into one OpProgramVerify applied at the end, so the
// assert (its argument read AND the callee's internal recoveryAuthority read)
// ran against the *unmodified* initial ledger and compared `JubjubPoint::default()`
// values — silently defeating the distinctness invariant. A28 flushes the
// pending writes to `qctx` before the impure call so the reads see the
// witnessed values.
//
// codegen_regression already byte-locks this output, but that only catches
// *drift* from the committed fixture — it would not catch a future
// regeneration that reintroduces the read-before-write and is committed as the
// new baseline. This test asserts the structural invariant directly: within
// the generated constructor, the write-flush must precede the distinctness
// assert call (and its argument read).

const DID05_LIB: &str = include_str!("../contracts/did-05/lib.rs");

/// Extract the body of `fn initial_state(...) { ... }` from the generated
/// contract. Returns the substring from the signature to the balanced closing
/// brace (a simple brace counter is enough — the generated code is regular).
fn initial_state_body(src: &str) -> &str {
    let start = src
        .find("fn initial_state")
        .expect("generated contract has an initial_state constructor");
    let open = start
        + src[start..]
            .find('{')
            .expect("initial_state has an opening brace");
    let mut depth = 0usize;
    for (i, ch) in src[open..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return &src[start..=open + i];
                }
            }
            _ => {}
        }
    }
    panic!("unbalanced braces in initial_state body");
}

#[test]
fn constructor_flushes_writes_before_distinctness_assert() {
    let body = initial_state_body(DID05_LIB);

    let assert_at = body
        .find("assert_controller_public_key_distinct_from_recovery_authority")
        .expect("constructor calls the distinctness assert helper");

    // A write-flush (`let qctx = _ctor_flush_N.context;`) must appear before the
    // assert call — otherwise the assert runs against the pre-write ledger.
    let flush_at = body.find("let qctx = _ctor_flush_").unwrap_or_else(|| {
        panic!(
            "constructor must flush pending writes (`let qctx = _ctor_flush_*`) \
             before the distinctness assert — read-before-write regression (A28)"
        )
    });

    assert!(
        flush_at < assert_at,
        "write-flush (byte {flush_at}) must precede the distinctness assert (byte {assert_at}); \
         the assert would otherwise read the unmodified initial ledger (A28 regression)"
    );

    // The assert's controller-key argument is read from the ledger; that read
    // must also sit after the flush so it sees the witnessed value.
    if let Some(carg_at) = body.find("_carg_") {
        assert!(
            flush_at < carg_at,
            "the assert argument's ledger read (byte {carg_at}) must follow the \
             write-flush (byte {flush_at}) so it reads the written controller key"
        );
    }
}
