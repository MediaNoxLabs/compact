;;; This file is part of Compact.
;;; Copyright (C) 2026 Midnight Foundation
;;; SPDX-License-Identifier: Apache-2.0
;;; Licensed under the Apache License, Version 2.0 (the "License");
;;; you may not use this file except in compliance with the License.
;;; You may obtain a copy of the License at
;;;
;;;  	http://www.apache.org/licenses/LICENSE-2.0
;;;
;;; Unless required by applicable law or agreed to in writing, software
;;; distributed under the License is distributed on an "AS IS" BASIS,
;;; WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
;;; See the License for the specific language governing permissions and
;;; limitations under the License.

;;; Part of the former single-file `rust-passes-emit.ss` (3,397 lines),
;;; split by capability so each piece is reviewable on its own. The split
;;; is textual: every one of these files is `include`d into the same
;;; `(definitions ...)` block in rust-passes.ss, where internal defines are
;;; mutually recursive, so grouping carries no ordering constraint and no
;;; behavioural change. Byte parity over the fixture corpus is what proves
;;; that.
;;;
;;; This file: ledger reads and call-site emission.

      ;; emit-ledger-read-expr: render a `(public-ledger ... read)` IR
      ;; node as a Rust block expression that runs a gather query and
      ;; decodes the result. Used by expr-rust / ctor-expr-rust when the
      ;; read appears in expression position (clear()'s `apk == authority`,
      ;; in_state's inlined `state == s`, zerocash.spend's
      ;; `nullifiers.member(old)` and `commitments.checkRoot(...)`).
      ;;
      ;; The qctx source comes from the (current-qctx-ref) dynamic
      ;; parameter so circuit-body emissions read from
      ;; `&ctx.current_query_context` while constructor-body emissions
      ;; would read from `&qctx`.
      ;;
      ;; Optional `expr*` carries the ADT-read's runtime arguments (the
      ;; element for Set.member, the candidate root for
      ;; HistoricMerkleTree.checkRoot, the key for Map.member / lookup).
      ;; When present, we route through expand-vm-code + the gather
      ;; vminstr renderer so the resulting OpProgramGather chain mirrors
      ;; the adt-op's vm-code (including the additional push and
      ;; member / eq / root steps).
      ;;
      ;; A20: the no-arg branch (expr* empty) also routes through
      ;; expand-vm-code first so reads whose vm-code does more than
      ;; `(dup) (idx ...) (popeq)` — Set.size, Set.isEmpty, Map.size,
      ;; Map.isEmpty, List.isEmpty, List.length — render the correct
      ;; OpProgramGather chain rather than silently miscompiling to
      ;; the hardcoded template. Cell.read and Counter.read also flow
      ;; through expand-vm-code and produce the same shape as before.
      ;; The hardcoded template survives as a last-resort fallback for
      ;; any read whose vm-code instructions vminstr->gather-builder-call
      ;; doesn't yet recognise.
      ;; emit-struct-field-zero-read: F2.2 — emit a gather block for
      ;; reading a ledger cell whose value is a tstruct, decoding only
      ;; the leading (field-0) atom with the provided decoder. The
      ;; AlignedValue layout for a tstruct prepends field-0's atoms,
      ;; so `decoder(_av)` reads exactly the projected field's bytes.
      ;; Mirrors the no-arg branch of emit-ledger-read-expr but skips
      ;; the whole-struct decoder check.
      (define (emit-struct-field-zero-read path-elt* decoder)
        (let* ([path-idx*
                (map (lambda (pe)
                       (nanopass-case (Ltypescript Path-Element) pe
                         [,path-index path-index]
                         [else #f]))
                     path-elt*)]
               [idx-lines
                (let join ([xs path-idx*] [acc ""])
                  (cond
                    [(null? xs) acc]
                    [else
                     (join (cdr xs)
                           (string-append
                             acc
                             (format "                .idx_at_index(~au8, false)\n" (car xs))))]))])
          (string-append
            "{\n"
            "            let _gather_ops = OpProgramGather::<DefaultDB>::new()\n"
            "                .dup(0)\n"
            idx-lines
            "                .popeq(true)\n"
            "                .build();\n"
            "            let _gather_results = query_for_read(\n"
            "                " (current-qctx-ref) ",\n"
            "                &_gather_ops,\n"
            "                None,\n"
            "                &initial_cost_model(),\n"
            "            )\n"
            "            .map_err(|e| CompactError::AssertionFailed(format!(\"ledger query failed: {:?}\", e)))?;\n"
            "            let _av = match _gather_results.events.last() {\n"
            "                Some(compact_runtime::onchain_vm::result_mode::GatherEvent::Read(av)) => av,\n"
            "                _ => return Err(CompactError::AssertionFailed(\"ledger: expected Read event\".into())),\n"
            "            };\n"
            "            " decoder "(_av)?\n"
            "        }")))

      (define (emit-ledger-read-expr path-elt* adt-op . opt-args)
        (let* ([expr* (if (pair? opt-args) (car opt-args) '())]
               [native-ht (and (pair? opt-args) (pair? (cdr opt-args)) (cadr opt-args))])
          (nanopass-case (Ltypescript ADT-Op) adt-op
            [(,ledger-op ,op-class (,adt-name (,adt-formal* ,adt-arg*) ...) ((,var-name* ,type*) ...) ,type ,vm-code)
             (cond
               [(not (eq? op-class 'read))
                (rust-feature-error #f 'ledger-op-non-read
                  "non-read public-ledger op in expression position (op-class=~a)"
                  op-class)]
               [else
                (let* ([path-idx*
                        (map (lambda (pe)
                               (nanopass-case (Ltypescript Path-Element) pe
                                 [,path-index path-index]
                                 [else #f]))
                             path-elt*)]
                       [decoder (decoder-for-type type)])
                  (cond
                    [(memv #f path-idx*)
                     (rust-feature-error #f 'ledger-read-non-index-path
                       "ledger read with non-index path element")]
                    [(not decoder)
                     (rust-feature-error #f 'ledger-read-decoder-missing
                       "no decoder available for ledger read type")]
                    [(not (null? expr*))
                     ;; F1.2/2: ADT read-with-arg path. Run the adt-op's
                     ;; vm-code through expand-vm-code with the concrete
                     ;; path + lifted-arg substitutions, then render each
                     ;; vminstr as one line of the OpProgramGather chain.
                     ;; Falls back to `unimplemented!()` if any step is
                     ;; unsupported (e.g. an arg shape vm-value->rust
                     ;; doesn't yet handle).
                     (or
                       (emit-ledger-read-expr-with-args
                         path-elt* adt-op expr* native-ht decoder)
                       (rust-feature-error #f 'adt-read-with-arg-lowering
                         "ADT read-with-arg lowering failed for ledger op"))]
                    [else
                     ;; A20: route no-arg adt-op reads through the same
                     ;; expand-vm-code machinery as the with-arg path so
                     ;; the gather chain mirrors the actual vm-code
                     ;; (e.g. Set.size's `(size)`, Set.isEmpty's
                     ;; `(push align-0-8) (eq)`, List.isEmpty's `(type)`).
                     ;; The previous hardcoded `dup → idx → popeq`
                     ;; template happened to match Cell.read and
                     ;; Counter.read but silently miscompiled any read
                     ;; whose vm-code contained additional instructions.
                     ;; Fall back to that template only when expansion
                     ;; itself fails (so existing Cell/Counter coverage
                     ;; is preserved even for adt-ops whose vm-code
                     ;; touches instructions vminstr->gather-builder-call
                     ;; doesn't yet recognise).
                     (or
                       (emit-ledger-read-expr-with-args
                         path-elt* adt-op '() native-ht decoder)
                       (let ([idx-lines
                              (let join ([xs path-idx*] [acc ""])
                                (cond
                                  [(null? xs) acc]
                                  [else
                                   (join (cdr xs)
                                         (string-append
                                           acc
                                           (format "                .idx_at_index(~au8, false)\n" (car xs))))]))])
                         (string-append
                           "{\n"
                           "            let _gather_ops = OpProgramGather::<DefaultDB>::new()\n"
                           "                .dup(0)\n"
                           idx-lines
                           "                .popeq(true)\n"
                           "                .build();\n"
                           "            let _gather_results = query_for_read(\n"
                           "                " (current-qctx-ref) ",\n"
                           "                &_gather_ops,\n"
                           "                None,\n"
                           "                &initial_cost_model(),\n"
                           "            )\n"
                           "            .map_err(|e| CompactError::AssertionFailed(format!(\"ledger query failed: {:?}\", e)))?;\n"
                           "            let _av = match _gather_results.events.last() {\n"
                           "                Some(compact_runtime::onchain_vm::result_mode::GatherEvent::Read(av)) => av,\n"
                           "                _ => return Err(CompactError::AssertionFailed(\"ledger: expected Read event\".into())),\n"
                           "            };\n"
                           "            " decoder "(_av)?\n"
                           "        }")))]))])])))

      ;; emit-ledger-read-expr-with-args: F1.2/2 — render the ADT read
      ;; via expand-vm-code, producing a Rust block expression. Returns
      ;; #f if the path / args / vminstrs can't be lowered, so the
      ;; caller can fall back to the placeholder. `native-ht` may be #f
      ;; for cases where the caller didn't supply one — we then can't
      ;; render non-literal args and bail out.
      (define (emit-ledger-read-expr-with-args path-elt* adt-op expr* native-ht decoder)
        (nanopass-case (Ltypescript ADT-Op) adt-op
          [(,ledger-op ,op-class (,adt-name (,adt-formal* ,adt-arg*) ...) ((,var-name* ,type*) ...) ,type ,vm-code)
           (cond
             [(not (fx= (length expr*) (length var-name*))) #f]
             [else
              (let ([path-vals (map path-elt->vm-value path-elt*)]
                    [expr-vals
                     (map (lambda (e)
                            (if native-ht
                                (expr->vm-value e native-ht)
                                (expr->vm-value e)))
                          expr*)])
                (cond
                  [(memv #f path-vals) #f]
                  [(memv #f expr-vals) #f]
                  [else
                   (let* ([arg-alist
                           (append (map cons adt-formal* adt-arg*)
                                   (map (lambda (vn v)
                                          (cons (id-sym vn) v))
                                        var-name* expr-vals))]
                          [vminstr*
                           (guard (c [#t #f])
                             (expand-vm-code #f path-vals #f arg-alist
                               (vm-code-code vm-code)))]
                          [lines (and vminstr*
                                      (map vminstr->gather-builder-call vminstr*))])
                     (cond
                       [(or (not lines) (memv #f lines)) #f]
                       [else
                        (string-append
                          "{\n"
                          "            let _gather_ops = OpProgramGather::<DefaultDB>::new()\n"
                          (apply string-append lines)
                          "                .build();\n"
                          "            let _gather_results = query_for_read(\n"
                          "                " (current-qctx-ref) ",\n"
                          "                &_gather_ops,\n"
                          "                None,\n"
                          "                &initial_cost_model(),\n"
                          "            )\n"
                          "            .map_err(|e| CompactError::AssertionFailed(format!(\"ledger query failed: {:?}\", e)))?;\n"
                          "            let _av = match _gather_results.events.last() {\n"
                          "                Some(compact_runtime::onchain_vm::result_mode::GatherEvent::Read(av)) => av,\n"
                          "                _ => return Err(CompactError::AssertionFailed(\"ledger: expected Read event\".into())),\n"
                          "            };\n"
                          "            " decoder "(_av)?\n"
                          "        }")]))]))])]))

      ;; tuple-arg-rust: emit a Rust expression for a Tuple-Argument
      ;; (`single` or `spread`). I3b/1 only needs `single`; `spread` emits a
      ;; TODO placeholder.
      (define (tuple-arg-rust ta native-id-ht)
        (nanopass-case (Ltypescript Tuple-Argument) ta
          [(single ,src ,expr) (expr-rust expr native-id-ht)]
          [(spread ,src ,nat ,expr)
           (rust-feature-error src 'tuple-spread
             "tuple spread (`...expr`) not supported")]))

      ;; call-rust: emit a Rust call expression for `(call src function-name
      ;; expr* ...)`. Resolves the function-name id to a native binding via
      ;; native-id-ht. For native calls whose Rust signature doesn't line up
      ;; 1:1 with Compact's (e.g. persistent_hash takes `&[u8]` whereas
      ;; Compact's persistentHash takes a typed value), emit a specialised
      ;; form. For natives with a clean 1:1 mapping (none yet exercised by
      ;; tiny.compact), emit a vanilla `<rust-name>(<arg>, ...)`. For
      ;; non-native (user-defined) circuit calls, fall back to the
      ;; snake-cased local name — a follow-up wedge will resolve these
      ;; properly.
      ;; pure-call-arg-rust: render a call argument via expr-rust, then
      ;; suffix `.clone()` when the argument is a var-ref or a
      ;; var-rooted field access whose type is not Rust `Copy`. Pure
      ;; circuits are free functions taking args by value, so a non-Copy
      ;; struct passed as `foo(x)` would move `x` and break later reads
      ;; of `x` in the same body (midnight-verifiable-credentials:
      ;; `assertValidDigitalPassportCredential` passes `credential` to
      ;; several helpers in sequence). `.clone()` borrows, so the owner
      ;; stays usable. Mirrors arg-rust-clone-if-var's decision logic but
      ;; renders via expr-rust (the pure walker's renderer) and consults
      ;; current-formal-arg-types (seeded by emit-pure-circuit) for the
      ;; Copy check. Also used for by-value native args (jubjub_point_x
      ;; takes `JubjubPoint` by value and is invoked twice on the same
      ;; field in a && expression).
      (define (pure-call-arg-rust e native-id-ht)
        (let ([rendered (expr-rust e native-id-ht)]
              [stripped (expr-strip-cast e)])
          (nanopass-case (Ltypescript Expression) stripped
            [(var-ref ,src ,var-name)
             (if (var-ref-known-copy? var-name)
                 rendered
                 (string-append rendered ".clone()"))]
            [(elt-ref ,src ,expr ,elt-name ,nat)
             (cond
               [(not (elt-ref-rooted-in-var? stripped)) rendered]
               [(elt-ref-known-copy? stripped) rendered]
               [else (string-append rendered ".clone()")])]
            [else rendered])))

      (define (call-rust src function-name expr* native-id-ht)
        (let ([ne (eq-hashtable-ref native-id-ht function-name #f)]
              [sym (id-sym function-name)])
          (cond
            [(or (eq? sym 'some) (eq? sym 'none))
             ;; I3b/4: stdlib circuits with no native binding — go through
             ;; the runtime-side `std_lib` path. Without the circuit pelt
             ;; here we can't inject `::<T>`, but tiny.compact's get()
             ;; reaches this through ctor-call-rust which does have the
             ;; pelt and ascribes the generic — this branch is a safety
             ;; net for any future ascription-free use site.
             (let ([args
                    (map (lambda (e) (expr-rust e native-id-ht)) expr*)])
               (format "compact_runtime::std_lib::~a(~a)"
                       sym
                       (let join ([xs args] [acc ""])
                         (cond
                           [(null? xs) acc]
                           [(null? (cdr xs)) (string-append acc (car xs))]
                           [else (join (cdr xs)
                                       (string-append acc (car xs) ", "))]))))]
            [(and ne (equal? (native-entry-rust-function ne)
                             "compact_runtime::persistent_hash"))
             ;; R3: alignment-aware lowering of Compact's
             ;; `persistentHash<T>(value)`. The TS path constructs an
             ;; `AlignedValue` from `value` (via the runtime type
             ;; descriptor) and feeds the alignment-framed byte stream to
             ;; SHA-256; see `node_modules/@midnight-ntwrk/compact-runtime/
             ;; src/built-ins.ts::persistentHash` and the wasm-side
             ;; `onchain-runtime-wasm/src/primitives.rs::persistent_hash`.
             ;;
             ;; We mirror that by converting each constituent of the
             ;; argument to an `AlignedValue` and calling
             ;; `persistent_hash_aligned`, which delegates to
             ;; `ValueReprAlignedValue::binary_repr` + the upstream SHA-256
             ;; persistent hash. When the argument is a `Vector<N, T>` the
             ;; IR represents it as a `(tuple ...)` and we lift each
             ;; element separately so each gets its own alignment atom; for
             ;; any other shape we wrap the single argument in a one-element
             ;; slice.
             ;;
             ;; Previous emit (I3b/1):
             ;;   `persistent_hash(&[a, b, ...].concat()).0`
             ;; produces byte-identical output for uniform `Bytes<N>` inputs
             ;; (tiny.compact's `public_key`), but diverges for mixed-type,
             ;; `Field`-, or `Compress`-bearing inputs because raw byte
             ;; concat skips the per-atom framing (Bytes<n> zero-padding,
             ;; Fr little-endian normalisation, Compress hashing).
             (cond
               [(fx= (length expr*) 1)
                (let ([arg (car expr*)])
                  (let ([elt-strs
                         ;; If the single argument is a `tuple` IR node
                         ;; (Compact-level Vector), break it apart so each
                         ;; element becomes its own AlignedValue. Otherwise,
                         ;; emit a one-element slice.
                         (nanopass-case (Ltypescript Expression) arg
                           [(tuple ,src ,tuple-arg* ...)
                            (map (lambda (ta)
                                   (format "compact_runtime::AlignedValue::from(~a)"
                                           (tuple-arg-rust ta native-id-ht)))
                                 tuple-arg*)]
                           [else
                            (list (format "compact_runtime::AlignedValue::from(~a)"
                                          (expr-rust arg native-id-ht)))])])
                    (string-append
                      "compact_runtime::std_lib::persistent_hash_aligned(&["
                      (let join ([xs elt-strs] [acc ""])
                        (cond
                          [(null? xs) acc]
                          [(null? (cdr xs)) (string-append acc (car xs))]
                          [else (join (cdr xs)
                                      (string-append acc (car xs) ", "))]))
                      "])")))]
               [else
                (rust-feature-error src 'persistent-hash-arity
                  "persistentHash arity ~a not yet supported (expected 1)"
                  (length expr*))])]
            [(and ne (equal? (native-entry-rust-function ne)
                             "compact_runtime::transient_hash"))
             ;; R5: alignment-aware lowering of Compact's
             ;; `transientHash<A>(value): Field`. The upstream
             ;; `transient_hash(elems: &[Fr]) -> Fr` takes a field-element
             ;; slice, but Compact surfaces a single typed value. Mirror
             ;; persistent_hash's R3 path: convert each constituent to an
             ;; `AlignedValue` and call `transient_hash_aligned`, which
             ;; field-reprs the concatenated alignment into a `Vec<Fr>`
             ;; preimage (via `ValueReprAlignedValue: FieldRepr`) and
             ;; Poseidon-hashes it — matching the TS runtime's
             ;; `transientHash(alignment, toValue(value))`. A `Vector<N,T>`
             ;; arg lowers to a `(tuple ...)` and is broken apart so each
             ;; element gets its own alignment atom; any other shape wraps
             ;; the single argument in a one-element slice.
             (cond
               [(fx= (length expr*) 1)
                (let ([arg (car expr*)])
                  (let ([elt-strs
                         (nanopass-case (Ltypescript Expression) arg
                           [(tuple ,src ,tuple-arg* ...)
                            (map (lambda (ta)
                                   (format "compact_runtime::AlignedValue::from(~a)"
                                           (tuple-arg-rust ta native-id-ht)))
                                 tuple-arg*)]
                           [else
                            (list (format "compact_runtime::AlignedValue::from(~a)"
                                          (expr-rust arg native-id-ht)))])])
                    (string-append
                      "compact_runtime::std_lib::transient_hash_aligned(&["
                      (let join ([xs elt-strs] [acc ""])
                        (cond
                          [(null? xs) acc]
                          [(null? (cdr xs)) (string-append acc (car xs))]
                          [else (join (cdr xs) (string-append acc (car xs) ", "))]))
                      "])")))]
               [else
                (rust-feature-error src 'transient-hash-arity
                  "transientHash arity ~a not yet supported (expected 1)"
                  (length expr*))])]
            [(and ne (equal? (native-entry-rust-function ne)
                             "compact_runtime::persistent_commit"))
             ;; R4: lowering of Compact's
             ;; `persistentCommit<T>(value, opening)`. The upstream
             ;; `persistent_commit<T: BinaryHashRepr + ?Sized>(value: &T,
             ;; opening: HashOutput)` takes `value` by reference and
             ;; `opening` (`Bytes<32>` = `[u8;32]`, Copy) by value. We
             ;; render `persistent_commit(&<value>, <opening>)`. The
             ;; borrow is non-moving so `value` remains usable in the
             ;; surrounding body. `BinaryHashRepr` is impl'd for `[u8;N]`,
             ;; all unsigned/signed ints, bool, tuples, and user structs
             ;; (via the H6 `Aligned` + `From<S> for Value` blanket that
             ;; gives `AlignedValue: From<S>`, whose `BinaryHashRepr`
             ;; delegates through `ValueReprAlignedValue`) — so
             ;; `Bytes<N>`, `Uint<N>`, and struct-typed values all
             ;; type-check. Mirrors the TS runtime's `persistentCommit`
             ;; (alignment-framed SHA-256 commit).
             (cond
               [(fx= (length expr*) 2)
                ;; `persistent_commit<T: BinaryHashRepr>(&T, HashOutput) ->
                ;; HashOutput`. The Compact `persistentCommit<A>(v, o):
                ;; Bytes<32>` surfaces `[u8; 32]`, so wrap the opening in
                ;; `HashOutput(...)` and extract `.0` from the result.
                ;; `HashOutput` isn't in compact_runtime's curated prelude
                ;; but is reachable as
                ;; `compact_runtime::base_crypto::hash::HashOutput`
                ;; (midnight-base-crypto re-exports the crate as
                ;; `base_crypto` and `hash` is a pub module).
                (string-append
                  "compact_runtime::persistent_commit(&"
                  (expr-rust (car expr*) native-id-ht)
                  ", compact_runtime::base_crypto::hash::HashOutput("
                  (expr-rust (cadr expr*) native-id-ht)
                  ")).0")]
               [else
                (rust-feature-error src 'persistent-commit-arity
                  "persistentCommit arity ~a not yet supported (expected 2)"
                  (length expr*))])]
            [ne
             ;; A native with a 1:1 binding. Emit `<rust-name>(<arg>, ...)`.
             ;; Args render via pure-call-arg-rust so by-value natives
             ;; that take a non-Copy struct / JubjubPoint (jubjub_point_x,
             ;; jubjub_point_y) get a defensive `.clone()` when the same
             ;; value is read again later in the body — the
             ;; digital-passport's
             ;; `assertDigitalPassportIssuanceResultMatchesRequest` calls
             ;; jubjubPointX then jubjubPointY on the same holderPublicKey
             ;; inside a `&&` expression; without the clone the first
             ;; call moves the field and the second fails to borrow.
             (let ([rust-name (native-call-site-rust ne)]
                   [args
                    (map (lambda (e) (pure-call-arg-rust e native-id-ht)) expr*)])
               (string-append
                 rust-name
                 "("
                 (let join ([xs args] [acc ""])
                   (cond
                     [(null? xs) acc]
                     [(null? (cdr xs)) (string-append acc (car xs))]
                     [else (join (cdr xs) (string-append acc (car xs) ", "))]))
                 ")"))]
            [else
             ;; A user-defined circuit call. Resolve via
             ;; current-circuit-id-ht (threaded by emit-pure-circuit): if
             ;; the callee is a user pure circuit, route to
             ;; `pure_circuits::<snake>(...)` — both exported (`pub fn`)
             ;; and non-exported (`pub(crate) fn`) pure circuits land in
             ;; the `pure_circuits` module. Args use pure-call-arg-rust so
             ;; non-Copy struct args are cloned (the callee takes them by
             ;; value). Impure circuits and unknown callees keep the
             ;; existing non-native-call error — the impure walker
             ;; resolves pure-circuit calls earlier via ctor-call-rust, so
             ;; this else is only reached during pure-circuit emission
             ;; where current-circuit-id-ht is populated.
             (let ([c (eq-hashtable-ref (current-circuit-id-ht) function-name #f)])
               (cond
                 [(and c (id-pure? function-name))
                  (let ([rust-name (id->rust-name function-name)]
                        [args (map (lambda (e) (pure-call-arg-rust e native-id-ht)) expr*)])
                    ;; Append `?` so the `Result<T, CompactError>` a pure
                    ;; circuit returns is unwrapped at the call site. Every
                    ;; generated position that calls a pure circuit (pure-
                    ;; circuit bodies, impure circuit bodies, constructors,
                    ;; conditions) lives inside a function returning
                    ;; `Result<_, CompactError>`, so `?` is valid. The
                    ;; `Ok(...)` tail-wrap (Phase 5) wraps the already-`?`-ed
                    ;; call as `Ok(pure_circuits::foo(...)?)` — a single `?`.
                    (string-append
                      "pure_circuits::"
                      (symbol->string rust-name)
                      "("
                      (let join ([xs args] [acc ""])
                        (cond
                          [(null? xs) acc]
                          [(null? (cdr xs)) (string-append acc (car xs))]
                          [else (join (cdr xs) (string-append acc (car xs) ", "))]))
                      ")?"))]
                 [else
                  (rust-feature-error src 'non-native-call
                    "call to non-native circuit ~a not supported in this position"
                    (id-sym function-name))]))])))
