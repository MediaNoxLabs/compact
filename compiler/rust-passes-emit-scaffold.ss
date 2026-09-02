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
;;; This file: the initial-state scaffold and circuit argument lists.

      ;; emit-scaffold-seed: emit one initial-state scaffold line for a
      ;; leaf public-binding at the given indentation. ADT-aware seeding
      ;; (R1 / K1.1): the Compact ADTs whose initial-value isn't a plain
      ;; Cell — Map, Set, MerkleTree, HistoricMerkleTree — have dedicated
      ;; builders in midnight-compact-runtime that produce the exact StateValue
      ;; shape declared in midnight-ledger.ss. Cell / Counter / anything
      ;; else with a read op keeps the K1 path — new_cell(<default>).
      ;; Special case: tvector defaults to [T; N] which doesn't impl
      ;; Into<AlignedValue> upstream — route through new_cell_array which
      ;; concatenates per-element AVs.
      ;; Special case: tunsigned with a byte-length that doesn't match the
      ;; Rust integer width's byte count (e.g. `Uint<0..70000>` is u32 in
      ;; Rust but uses 3 bytes on state) — route through
      ;; `new_cell_bounded_uint(0u128, N)` so the on-state
      ;; `AlignmentAtom::Bytes` width matches TS.
      (define (emit-scaffold-seed public-binding indent)
        (let ([t (binding-type public-binding)])
          (cond
            [(tadt-name=? t 'Set)
             (out (format "~anew_map(),\n" indent))]
            [(tadt-name=? t 'Map)
             (out (format "~anew_map(),\n" indent))]
            [(tadt-name=? t 'List)
             (out (format "~anew_list(),\n" indent))]
            [(tadt-name=? t 'MerkleTree)
             (out (format "~anew_merkle_tree(~a),\n"
                          indent (tadt-merkle-height t)))]
            [(tadt-name=? t 'HistoricMerkleTree)
             (out (format "~anew_historic_merkle_tree(~a),\n"
                          indent (tadt-merkle-height t)))]
            [else
             (let ([read-type (tadt-read-op-type t)])
               (cond
                 [(type-is-tvector? read-type)
                  (out (format "~anew_cell_array(~a),\n"
                               indent (default-value-rust read-type)))]
                 [(tunsigned-bounded? read-type)
                  (out (format "~anew_cell_bounded_uint(0u128, ~a),\n"
                               indent (tunsigned-byte-length read-type)))]
                 [else
                  (out (format "~anew_cell(~a),\n"
                               indent (default-value-rust read-type)))]))])))

      ;; emit-scaffold-elements: walk one level of a public-ledger-array,
      ;; emitting each element at `indent`. Leaves seed their default via
      ;; emit-scaffold-seed; nested pl-arrays — the >16-field chunking the
      ;; front end computes (StateValue::Array caps at 16), the SAME
      ;; structure `binding-path-indices` and every read/write emitter
      ;; derive their idx_at_index chains from — recurse as nested
      ;; `new_array(vec![...])` scaffolds (A29). Flat (<=16-field)
      ;; contracts have no nested pl-arrays, so their output is unchanged.
      ;; Mirrors typescript-passes.ss::ledger-initializers.
      (define (emit-scaffold-elements pl-array indent)
        (nanopass-case (Ltypescript Public-Ledger-Array) pl-array
          [(public-ledger-array ,pl-array-elt* ...)
           (for-each
             (lambda (pl-array-elt)
               (nanopass-case (Ltypescript Public-Ledger-Array-Element) pl-array-elt
                 [,pl-array
                  (out (format "~anew_array(vec![\n" indent))
                  (emit-scaffold-elements pl-array (string-append indent "    "))
                  (out (format "~a]),\n" indent))]
                 [,public-binding
                  (emit-scaffold-seed public-binding indent)]))
             pl-array-elt*)]))

      ;; emit-initial-state: emits the `initial_state` constructor method
      ;; inside the open Contract impl block. K1 seeds each ledger field
      ;; with its type's default value; J2 then walks the constructor body
      ;; and emits the witness / pure-circuit prelude + a single
      ;; OpProgramVerify chain that writes each field its source-declared
      ;; value. If the constructor body shape isn't one we recognise, we
      ;; fall back to the K1-only return.
      (define (emit-initial-state ledger-field* ctor-arg* all-pelt*)
        (out "    pub fn initial_state(\n")
        (out "        &self,\n")
        (out "        ctx: ConstructorContext<PS>")
        ;; Constructor parameters: one per (var-name type) pair, emitted
        ;; after ctx in the same multi-line shape used elsewhere. Names go
        ;; through camel->snake (handles `$` and CamelCase); types via
        ;; type-rust.
        (for-each
          (lambda (arg)
            (nanopass-case (Ltypescript Argument) arg
              [(,var-name ,type)
               (out (format ",\n        ~a: ~a"
                            (camel->snake (id-sym var-name))
                            (type-rust type)))]))
          ctor-arg*)
        (out ",\n    ) -> Result<ConstructorResult<PS>, CompactError> {\n")
        ;; K1: walk the pl-array structure (all fields, not just exported)
        ;; and emit one seed per leaf binding using the read-op's result
        ;; type as the source of truth for the default value. The walk
        ;; preserves the IR's nesting (A29): contracts with >16 ledger
        ;; fields get the front end's chunked pl-array shape, so the
        ;; scaffold's nested new_array structure matches the idx_at_index
        ;; paths every read/write emitter derives from the same IR.
        ;; J2 (constructor body emission) then overrides these defaults
        ;; with whatever the source constructor assigns.
        (out "        let sv = new_array(vec![\n")
        (let* ([_
                (begin
                  (for-each
                    (lambda (lf)
                      (nanopass-case (Ltypescript Program-Element) lf
                        [(public-ledger-declaration ,pl-array ,lconstructor)
                         (emit-scaffold-elements pl-array "            ")]
                        [else (void)]))
                    ledger-field*)
                  (out "        ]);\n")
                  (out "        let state = ChargedState::new(sv);\n")
                  ;; Bucket-1: fully-qualify ContractAddress so a user struct
                  ;; named `ContractAddress` (e.g. midnight-did) does not
                  ;; shadow the upstream coin-structure type required by
                  ;; QueryContext::new.
                  (out "        let qctx = QueryContext::new(state, midnight_compact_runtime::ContractAddress::default());\n"))]
               ;; J2: emit the constructor body if we have one and its shape
               ;; matches. Fall back to the K1-only return otherwise (counter has
               ;; no constructor body, so it lands here naturally).
               [stmt (and (pair? ledger-field*)
                          (ldecl-constructor-stmt (car ledger-field*)))]
               [native-id-ht (build-native-id-ht all-pelt*)]
               [witness-id-ht (build-witness-id-ht all-pelt*)]
               [circuit-id-ht (build-circuit-id-ht all-pelt*)]
               [emitted?
                (and stmt
                     ;; Seed current-formal-arg-types with the
                     ;; constructor's args so var-ref-known-copy? can
                     ;; suppress redundant `.clone()` on primitive ctor
                     ;; parameters (`v: Field` in tiny.compact, etc.).
                     ;; The body walker mutates the same hashtable as it
                     ;; classifies const-bindings, so witness/pure-circuit
                     ;; results get their declared types recorded too.
                     ;;
                     ;; Iter 7: current-ledger-field-types is seeded by
                     ;; print-rust at the pass top (so impure circuits
                     ;; that write Vector<N,T> fields also see the map),
                     ;; not here.
                     (parameterize
                       ([current-formal-arg-types
                          (build-formal-arg-type-ht ctor-arg*)])
                       (emit-ctor-body-or-fallback stmt
                                                   native-id-ht witness-id-ht circuit-id-ht)))])
          ;; `emitted?` is #f for two very different reasons, and conflating
          ;; them was a MISCOMPILER (MediaNoxLabs/compact#45):
          ;;
          ;;   stmt = #f          there is no constructor at all. The bare
          ;;                      K1-only return below is correct — the state
          ;;                      really is just the seeded scaffold.
          ;;
          ;;   stmt present,      a constructor EXISTS and we failed to lower
          ;;   emitted? = #f      it. Emitting the same bare return silently
          ;;                      discards every write the author made, at
          ;;                      exit 0, with no diagnostic and no marker.
          ;;                      The contract then initialises to something
          ;;                      other than what the source says.
          ;;
          ;; The second case is strictly worse than emitting non-compiling
          ;; Rust: there is no signal at all, and byte-parity cannot catch it
          ;; either — a fixture of that shape just pins the wrong bytes as
          ;; expected. Refuse instead.
          ;; `stmt` is NOT a reliable "the author wrote a constructor" signal:
          ;; the front end synthesises a Ledger-Constructor for every contract
          ;; with ledger fields, so a contract with no constructor still has a
          ;; non-#f stmt — a bare unit `(tuple src)`. Testing `stmt` alone
          ;; therefore rejects every constructor-less contract, which an
          ;; earlier version of this fix did.
          ;;
          ;; `stmt-flatten` drops that bare unit, so a genuinely empty body
          ;; flattens to '() while a real one does not. Reject only when there
          ;; was something to lower and we failed to lower it.
          (when (and stmt (not emitted?) (pair? (stmt-flatten stmt)))
            (rust-feature-error (stmt-src stmt) 'ctor-body-emission
              "no walker shape matched the constructor body; ~a"
              "emitting the default scaffold here would silently discard every constructor write"))
          (unless emitted?
            (out "        Ok(ConstructorResult {\n")
            (out "            current_contract_state: qctx.state,\n")
            (out "            current_private_state: ctx.initial_private_state,\n")
            (out "            current_zswap_local_state: ctx.empty_zswap_local_state,\n")
            (out "        })\n")))
        (out "    }\n\n"))

      ;; emit-circuit-args: emit each Argument as ",\n        name: type"
      ;; after the leading `&self` / `ctx` params on an impure circuit method,
      ;; matching the existing multi-line method signature shape.
      (define (emit-circuit-args arg*)
        (for-each
          (lambda (arg)
            (nanopass-case (Ltypescript Argument) arg
              [(,var-name ,type)
               (out (format ",\n        ~a: ~a"
                            (camel->snake (id-sym var-name))
                            (type-rust type)))]))
          arg*))
