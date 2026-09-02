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

;;; Part of the former single-file `rust-passes-walker.ss` (3,792 lines),
;;; split by capability so each piece is reviewable on its own. The split
;;; is textual: every one of these files is `include`d into the same
;;; `(definitions ...)` block in rust-passes.ss, where internal defines are
;;; mutually recursive, so grouping carries no ordering constraint and no
;;; behavioural change. Byte parity over the fixture corpus is what proves
;;; that.
;;;
;;; This file: terminal emitters: writes, mutations, loops, if/else.

      ;; emit-ctor-prelude: emit the accumulated witness/pure-circuit/let
      ;; lines from the constructor body walk. They're already complete Rust
      ;; lines (with indentation + trailing newline), so just splat them out.
      (define (emit-ctor-prelude lines)
        (for-each out lines))

      ;; emit-body-writes: emit the OpProgramVerify chain for the collected
      ;; ledger field writes, then the query_for_verify call and the final
      ;; return value. `writes` is a list of (path-idx . expr). `mode` is
      ;; 'ctor (constructor) or 'circuit (impure circuit body); they differ
      ;; in the QueryContext source (`&qctx` vs `&ctx.current_query_context`)
      ;; and the return shape (`ConstructorResult` vs `CircuitResults`).
      ;; `witness-emitted?` controls whether we use `current_private_state`
      ;; (threaded through witness calls) or `ctx.initial_private_state`
      ;; (ctor mode) / falls back to the inline `..ctx` spread (circuit mode).
      ;; A28: build the `.push/.push/.ins` builder lines for ONE cell-write
      ;; (path-idx . value-expr). Extracted from emit-body-writes so the
      ;; constructor read-your-writes flush (ctor-write-flush-lines) can reuse
      ;; the exact same value-builder selection (new_cell / new_cell_array /
      ;; new_cell_bounded_uint). Returns a list of Rust line strings.
      (define (cell-write-op-lines w local-binds
                                   native-id-ht witness-id-ht circuit-id-ht)
        (let* ([idx (car w)]
               [val-expr (cdr w)]
               [rust-val (arg-rust-clone-if-var val-expr local-binds
                                                native-id-ht witness-id-ht circuit-id-ht)]
               [dest-type
                (let ([ht (current-ledger-field-types)])
                  (and ht (hashtable-ref ht idx #f)))]
               [dest-read-type
                (and dest-type
                     (guard (c [#t #f]) (tadt-read-op-type dest-type)))]
               [use-cell-array?
                (and dest-read-type (type-is-tvector? dest-read-type))]
               [use-bounded-uint?
                (and dest-read-type
                     (not use-cell-array?)
                     (guard (c [#t #f]) (tunsigned-bounded? dest-read-type)))]
               [bounded-uint-bytes
                (and use-bounded-uint?
                     (guard (c [#t #f]) (tunsigned-byte-length dest-read-type)))]
               [cell-builder
                (cond
                  [use-cell-array? "new_cell_array"]
                  [use-bounded-uint? "new_cell_bounded_uint"]
                  [else "new_cell"])])
          (list
            (format "            .push(false, new_cell(~au8))\n" idx)
            (if use-bounded-uint?
                (format "            .push(true, ~a(~a as u128, ~a))\n"
                        cell-builder rust-val bounded-uint-bytes)
                (format "            .push(true, ~a(~a))\n" cell-builder rust-val))
            "            .ins(false, 1)\n")))

      ;; A28: emit a mid-constructor flush of the pending cell-writes. Applies
      ;; them to the local `qctx` via query_for_verify and rebinds `qctx` to the
      ;; result, giving read-your-writes: a subsequent impure-circuit call (and
      ;; its ledger-read args) then sees the just-written fields instead of the
      ;; unmodified initial ledger. Returns FORWARD-order line strings; the
      ;; caller prepends them before the impure call. `n` disambiguates the
      ;; temp binding across multiple flushes.
      (define (ctor-write-flush-lines writes local-binds
                                      native-id-ht witness-id-ht circuit-id-ht n)
        ;; `writes` are TAGGED mutations (as accumulated by emit-body-or-fallback
        ;; and consumed by emit-body-mutations): ('cell-write idx . expr) and
        ;; ('pl-call src adt-op path-elt* expr*). Dispatch per tag, reusing the
        ;; same builder helpers so the flushed ops match the final chain.
        (append
          (list "        let ops = OpProgramVerify::<DefaultDB>::new()\n")
          (apply append
            (map (lambda (m)
                   (case (car m)
                     [(cell-write)
                      (cell-write-op-lines (cdr m) local-binds
                                           native-id-ht witness-id-ht circuit-id-ht)]
                     [(pl-call)
                      (or (pl-call-builder-lines
                            (cadr m) (caddr m) (cadddr m) (car (cddddr m))
                            local-binds native-id-ht witness-id-ht circuit-id-ht)
                          '())]
                     [else '()]))
                 writes))
          (list
            "            .build();\n"
            (format "        let _ctor_flush_~a = query_for_verify(&qctx, &ops, ctx.gas_limit.clone(), &ctx.cost_model)?;\n" n)
            (format "        let qctx = _ctor_flush_~a.context;\n" n))))

      (define (emit-body-writes writes mode local-binds
                                native-id-ht witness-id-ht circuit-id-ht
                                witness-emitted?)
        (out "        let ops = OpProgramVerify::<DefaultDB>::new()\n")
        (for-each
          (lambda (w)
            (for-each out (cell-write-op-lines w local-binds
                                               native-id-ht witness-id-ht circuit-id-ht)))
          writes)
        (out "            .build();\n")
        (out "\n")
        (cond
          [(eq? mode 'ctor)
           (out "        let results = query_for_verify(&qctx, &ops, ctx.gas_limit.clone(), &ctx.cost_model)?;\n")
           (out "\n")
           (out "        Ok(ConstructorResult {\n")
           (out "            current_contract_state: results.context.state,\n")
           (out (if witness-emitted?
                    "            current_private_state,\n"
                    "            current_private_state: ctx.initial_private_state,\n"))
           (out (ctor-zswap-result-field))
           (out "        })\n")]
          [else
           ;; 'circuit mode: results live on the inbound ctx and we wrap
           ;; everything in a CircuitResults with unit result.
           (out "        let results = query_for_verify(\n")
           (out "            &ctx.current_query_context,\n")
           (out "            &ops,\n")
           (out "            ctx.gas_limit.clone(),\n")
           (out "            &ctx.cost_model,\n")
           (out "        )?;\n")
           (out "\n")
           (out "        Ok(CircuitResults {\n")
           (out "            result: (),\n")
           (out "            context: CircuitContext {\n")
           (out "                current_query_context: results.context,\n")
           (when witness-emitted?
             (out "                current_private_state,\n"))
           (out "                ..ctx\n")
           (out "            },\n")
           (out (circuit-gas-result-field))
           (out "        })\n")]))

      ;; emit-ctor-writes: backwards-compatible alias used by the ctor path.
      (define (emit-ctor-writes writes local-binds
                                native-id-ht witness-id-ht circuit-id-ht
                                witness-emitted?)
        (emit-body-writes writes 'ctor local-binds
                          native-id-ht witness-id-ht circuit-id-ht
                          witness-emitted?))

      ;; emit-body-mutations: A4 generalisation of emit-body-writes that
      ;; handles a mixed source-ordered list of mutations on the ledger
      ;; state. Each entry is either:
      ;;   ('cell-write idx . val-expr)         — same shape emit-body-writes
      ;;                                          consumes; emitted as the
      ;;                                          legacy push/push/ins triad.
      ;;   ('pl-call src adt-op path-elt* exprs) — non-write public-ledger
      ;;                                          ADT update call (e.g.
      ;;                                          Counter.increment); lowered
      ;;                                          via expand-vm-code +
      ;;                                          vminstr->builder-call into
      ;;                                          a sequence of builder lines.
      ;;
      ;; A single OpProgramVerify chain is built that splices the per-entry
      ;; builder lines in source order, followed by one `.build()` and the
      ;; same query_for_verify + return shape emit-body-writes produces.
      ;;
      ;; Returns #t on success. Falls back to #f only if any pl-call
      ;; lowering produces #f lines — same failure semantics as the
      ;; terminal emit-non-write-public-ledger-terminal helper.
      (define (emit-body-mutations mutations mode local-binds
                                   native-id-ht witness-id-ht circuit-id-ht
                                   witness-emitted?)
        (let ([all-lines
               (let loop ([ms mutations] [acc '()])
                 (cond
                   [(null? ms) (apply append (reverse acc))]
                   [else
                    (let ([m (car ms)])
                      (case (car m)
                        [(cell-write)
                         ;; m = ('cell-write idx . val-expr)
                         (let* ([idx (cadr m)]
                                [val-expr (cddr m)]
                                [rust-val (arg-rust-clone-if-var
                                            val-expr local-binds
                                            native-id-ht witness-id-ht circuit-id-ht)]
                                ;; Iter 7: Vector<N,T> ledger fields require
                                ;; `new_cell_array([T; N])` (orphan-rule
                                ;; workaround). Look up the destination
                                ;; field's binding-type via the path-idx
                                ;; → Type map populated by emit-initial-state.
                                [dest-type
                                 (let ([ht (current-ledger-field-types)])
                                   (and ht (hashtable-ref ht idx #f)))]
                                [dest-read-type
                                 (and dest-type
                                      (guard (c [#t #f])
                                        (tadt-read-op-type dest-type)))]
                                [use-cell-array?
                                 (and dest-read-type
                                      (type-is-tvector? dest-read-type))]
                                ;; Bug-11 (2026-06-29): see the parallel
                                ;; comment in emit-body-writes — Uint<L..U>
                                ;; destinations with non-power-of-2 byte
                                ;; lengths route through
                                ;; `new_cell_bounded_uint(value as u128,
                                ;; byte_len)`.
                                [use-bounded-uint?
                                 (and dest-read-type
                                      (not use-cell-array?)
                                      (guard (c [#t #f])
                                        (tunsigned-bounded? dest-read-type)))]
                                [bounded-uint-bytes
                                 (and use-bounded-uint?
                                      (guard (c [#t #f])
                                        (tunsigned-byte-length dest-read-type)))]
                                [cell-builder
                                 (cond
                                   [use-cell-array? "new_cell_array"]
                                   [use-bounded-uint? "new_cell_bounded_uint"]
                                   [else "new_cell"])]
                                [value-line
                                 (cond
                                   [use-bounded-uint?
                                    (format "            .push(true, ~a(~a as u128, ~a))\n"
                                            cell-builder rust-val bounded-uint-bytes)]
                                   [else
                                    (format "            .push(true, ~a(~a))\n"
                                            cell-builder rust-val)])]
                                [lines
                                 (list
                                   (format "            .push(false, new_cell(~au8))\n" idx)
                                   value-line
                                   "            .ins(false, 1)\n")])
                           (loop (cdr ms) (cons lines acc)))]
                        [(pl-call)
                         ;; m = ('pl-call src adt-op path-elt* expr*)
                         (let* ([src (cadr m)]
                                [adt-op (caddr m)]
                                [path-elt* (cadddr m)]
                                [expr* (car (cddddr m))]
                                [lines
                                 (pl-call-builder-lines
                                   src adt-op path-elt* expr*
                                   local-binds
                                   native-id-ht witness-id-ht circuit-id-ht)])
                           (and lines (loop (cdr ms) (cons lines acc))))]))]))])
          (cond
            [(not all-lines) #f]
            [else
             (out "        let ops = OpProgramVerify::<DefaultDB>::new()\n")
             (for-each out all-lines)
             (out "            .build();\n")
             (out "\n")
             (cond
               [(eq? mode 'ctor)
                (out "        let results = query_for_verify(&qctx, &ops, ctx.gas_limit.clone(), &ctx.cost_model)?;\n")
                (out "\n")
                (out "        Ok(ConstructorResult {\n")
                (out "            current_contract_state: results.context.state,\n")
                (out (if witness-emitted?
                         "            current_private_state,\n"
                         "            current_private_state: ctx.initial_private_state,\n"))
                (out (ctor-zswap-result-field))
                (out "        })\n")]
               [else
                (out "        let results = query_for_verify(\n")
                (out "            &ctx.current_query_context,\n")
                (out "            &ops,\n")
                (out "            ctx.gas_limit.clone(),\n")
                (out "            &ctx.cost_model,\n")
                (out "        )?;\n")
                (out "\n")
                (out "        Ok(CircuitResults {\n")
                (out "            result: (),\n")
                (out "            context: CircuitContext {\n")
                (out "                current_query_context: results.context,\n")
                (when witness-emitted?
                  (out "                current_private_state,\n"))
                (out "                ..ctx\n")
                (out "            },\n")
                (out (circuit-gas-result-field))
                (out "        })\n")])
             #t])))

      ;; pl-call-builder-lines: shared lowering of a non-write public-ledger
      ;; ADT update call into builder-call lines. Mirrors the inner pipeline
      ;; of emit-non-write-public-ledger-terminal — lift each arg expression
      ;; to a vm-rust-expr carrier, expand the vm-code, render each vminstr
      ;; as a builder-call line. Returns the list of rendered Rust line
      ;; strings on success, #f if any step fails.
      ;;
      ;; Used by emit-body-mutations to interleave non-write PL calls with
      ;; cell-writes on a single OpProgramVerify chain (the A4 walker
      ;; extension).
      (define (pl-call-builder-lines
                src adt-op path-elt* expr* local-binds
                native-id-ht witness-id-ht circuit-id-ht)
        (nanopass-case (Ltypescript ADT-Op) adt-op
          [(,ledger-op ,op-class (,adt-name (,adt-formal* ,adt-arg*) ...) ((,var-name* ,type*) ...) ,type ,vm-code)
           (cond
             [(not (fx= (length expr*) (length var-name*))) #f]
             [else
              (let ([path-vals (map path-elt->vm-value path-elt*)]
                    [expr-vals
                     (map (lambda (e)
                            (let ([rendered
                                   (guard (c [#t #f])
                                     (arg-rust-clone-if-var
                                       e local-binds
                                       native-id-ht witness-id-ht circuit-id-ht))])
                              (and rendered (make-vm-rust-expr rendered))))
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
                             (expand-vm-code src path-vals #f arg-alist
                               (vm-code-code vm-code)))]
                          [lines (and vminstr* (map vminstr->builder-call vminstr*))])
                     (cond
                       [(or (not lines) (memv #f lines)) #f]
                       [else lines]))]))])]))

      ;; emit-non-write-public-ledger-terminal: emit a terminal
      ;; `(public-ledger field (idx) <op> expr*)` whose op-class is NOT
      ;; `write`. Used for ADT update ops like HistoricMerkleTree.insert
      ;; that drive the OpProgramVerify chain through expand-vm-code +
      ;; vminstr->builder-call (the I3a infrastructure from E4.1) rather
      ;; than the hardcoded Cell.write pattern in emit-body-writes.
      ;;
      ;; The walker's accumulated `pre-lines` (witness / pure-circuit /
      ;; let bindings) are emitted first; then each arg expression is
      ;; resolved through `local-binds` and rendered to a Rust string,
      ;; lifted into a vm-rust-expr carrier that survives vm-code
      ;; expansion intact.
      ;;
      ;; Returns #t on success, #f if any step fails so the caller can
      ;; fall back to `unimplemented!()`.
      (define (emit-non-write-public-ledger-terminal
                src adt-op path-elt* expr* local-binds mode witness-emitted?
                pre-lines native-id-ht witness-id-ht circuit-id-ht)
        (nanopass-case (Ltypescript ADT-Op) adt-op
          [(,ledger-op ,op-class (,adt-name (,adt-formal* ,adt-arg*) ...) ((,var-name* ,type*) ...) ,type ,vm-code)
           (cond
             [(not (fx= (length expr*) (length var-name*))) #f]
             [else
              (let ([path-vals (map path-elt->vm-value path-elt*)]
                    [expr-vals
                     (map (lambda (e)
                            ;; Lift each arg to a vm-rust-expr carrier so
                            ;; expand-vm-code transports the rendered Rust
                            ;; intact down to vminstr->builder-call's push
                            ;; value rendering. Resolution via local-binds
                            ;; chases var-refs through the const-bindings
                            ;; the walker accumulated above.
                            (let ([rendered
                                   (guard (c [#t #f])
                                     (arg-rust-clone-if-var e local-binds
                                                            native-id-ht
                                                            witness-id-ht
                                                            circuit-id-ht))])
                              (and rendered (make-vm-rust-expr rendered))))
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
                             (expand-vm-code src path-vals #f arg-alist
                               (vm-code-code vm-code)))]
                          [lines (and vminstr* (map vminstr->builder-call vminstr*))])
                     (cond
                       [(or (not lines) (memv #f lines)) #f]
                       [else
                        ;; Emit the accumulated prelude (witness / pure /
                        ;; bare-call lines), then the OpProgramVerify
                        ;; chain, then query_for_verify + the final return.
                        (emit-ctor-prelude pre-lines)
                        (out "        let ops = OpProgramVerify::<DefaultDB>::new()\n")
                        (for-each out lines)
                        (out "            .build();\n")
                        (out "\n")
                        (cond
                          [(eq? mode 'ctor)
                           (out "        let results = query_for_verify(&qctx, &ops, ctx.gas_limit.clone(), &ctx.cost_model)?;\n")
                           (out "\n")
                           (out "        Ok(ConstructorResult {\n")
                           (out "            current_contract_state: results.context.state,\n")
                           (out (if witness-emitted?
                                    "            current_private_state,\n"
                                    "            current_private_state: ctx.initial_private_state,\n"))
                           (out (ctor-zswap-result-field))
                           (out "        })\n")]
                          [else
                           (out "        let results = query_for_verify(\n")
                           (out "            &ctx.current_query_context,\n")
                           (out "            &ops,\n")
                           (out "            ctx.gas_limit.clone(),\n")
                           (out "            &ctx.cost_model,\n")
                           (out "        )?;\n")
                           (out "\n")
                           (out "        Ok(CircuitResults {\n")
                           (out "            result: (),\n")
                           (out "            context: CircuitContext {\n")
                           (out "                current_query_context: results.context,\n")
                           (when witness-emitted?
                             (out "                current_private_state,\n"))
                           (out "                ..ctx\n")
                           (out "            },\n")
                           (out (circuit-gas-result-field))
                           (out "        })\n")])
                        #t]))]))])]))

      ;; emit-if-then-else-terminal: E6.2's impure if-mid-body emission.
      ;; Emits the prelude (witness / pure-circuit / assert lines the
      ;; walker accumulated), then a Rust `if cond { ... } else { ... }`
      ;; where each branch is an OpProgramVerify chain + query_for_verify
      ;; producing a QueryResults; the if-expression yields that
      ;; QueryResults, which we unpack into the final CircuitResults
      ;; return.
      ;;
      ;; Narrow shape:
      ;;   - terminal `(if cond then-stmt else-stmt)` (last statement
      ;;     of the body's flat sequence; no post-if statements yet)
      ;;   - each branch is a single non-write public-ledger ADT-update
      ;;     call (e.g. `tally_yes.increment(1);`)
      ;;
      ;; Returns #t on success, #f if any sub-step fails (caller falls
      ;; back to `unimplemented!()`).
      (define (emit-if-then-else-terminal
                cond-expr then-parts else-parts
                local-binds mode witness-emitted? pre-lines
                native-id-ht witness-id-ht circuit-id-ht)
        (let* ([cond-str
                (guard (c [#t #f])
                  (cond-rust cond-expr local-binds
                             native-id-ht witness-id-ht circuit-id-ht))]
               [then-lines
                (and then-parts
                     (compute-pl-builder-lines
                       (car then-parts) (cadr then-parts) (caddr then-parts)
                       (cadddr then-parts) local-binds
                       native-id-ht witness-id-ht circuit-id-ht))]
               [else-lines
                (and else-parts
                     (compute-pl-builder-lines
                       (car else-parts) (cadr else-parts) (caddr else-parts)
                       (cadddr else-parts) local-binds
                       native-id-ht witness-id-ht circuit-id-ht))])
          (cond
            [(or (not cond-str) (not then-lines) (not else-lines)) #f]
            [(rendered-has-todo? cond-str) #f]
            [else
             (emit-ctor-prelude pre-lines)
             (let ([qctx-ref (if (eq? mode 'ctor)
                                 "&qctx"
                                 "&ctx.current_query_context")])
               (out (format "        let _if_results = if ~a {\n" cond-str))
               (out "            let ops = OpProgramVerify::<DefaultDB>::new()\n")
               (for-each (lambda (l) (out (format "    ~a" l))) then-lines)
               (out "                .build();\n")
               (out (format "            query_for_verify(~a, &ops, ctx.gas_limit.clone(), &ctx.cost_model)?\n"
                            qctx-ref))
               (out "        } else {\n")
               (out "            let ops = OpProgramVerify::<DefaultDB>::new()\n")
               (for-each (lambda (l) (out (format "    ~a" l))) else-lines)
               (out "                .build();\n")
               (out (format "            query_for_verify(~a, &ops, ctx.gas_limit.clone(), &ctx.cost_model)?\n"
                            qctx-ref))
               (out "        };\n")
               (out "\n"))
             (cond
               [(eq? mode 'ctor)
                (out "        Ok(ConstructorResult {\n")
                (out "            current_contract_state: _if_results.context.state,\n")
                (out (if witness-emitted?
                         "            current_private_state,\n"
                         "            current_private_state: ctx.initial_private_state,\n"))
                (out (ctor-zswap-result-field))
                (out "        })\n")]
               [else
                (out "        Ok(CircuitResults {\n")
                (out "            result: (),\n")
                (out "            context: CircuitContext {\n")
                (out "                current_query_context: _if_results.context,\n")
                (when witness-emitted?
                  (out "                current_private_state,\n"))
                (out "                ..ctx\n")
                (out "            },\n")
                (out "            gas_cost: _if_results.gas_cost,\n")
                (out "        })\n")])
             #t])))

      ;; emit-for-range-terminal: Iter 4 — emit a terminal
      ;; `(for var-name lo..hi body)` range loop whose body is a single
      ;; non-write public-ledger ADT update call. Compile-time unrolls
      ;; the body's builder lines (hi - lo) times into a single
      ;; OpProgramVerify chain, then emits one `query_for_verify` plus
      ;; the standard ConstructorResult / CircuitResults return. The
      ;; loop var is not substituted into the body — bodies that
      ;; reference `i` (e.g. `mp.insert(i)`) currently fall back to the
      ;; emitter's unimplemented path. Returns #t on success, #f
      ;; otherwise so the caller falls back.
      ;; emit-for-iter-terminal: Iter 5/6 — emit a terminal
      ;; `(statement-expression (fold ...))` desugared from a Compact
      ;; `for (const x of <static-len iterable>) { body }`. Unrolls the
      ;; body's builder lines `len` times, substituting `elt-name` with
      ;; the i-th literal expression from `literals` before computing
      ;; the per-iteration builder lines. Returns #t on success, #f
      ;; otherwise.
      ;;
      ;; When the body doesn't reference `elt-name`, the substitution
      ;; is a no-op and each iteration produces identical builder
      ;; lines — recovering Iter 5's behaviour. When the body uses the
      ;; element directly as a call argument (e.g. `c.increment(x)`),
      ;; the per-iteration substitution materialises the literal
      ;; integer at compute-pl-builder-lines time so addi's immediate
      ;; resolves to a plain integer.
      (define (emit-for-iter-terminal
                len elt-name literals
                src adt-op path-elt* expr* local-binds mode witness-emitted?
                pre-lines native-id-ht witness-id-ht circuit-id-ht)
        (let ([per-iter-lines
               (let loop ([lits literals] [acc '()])
                 (cond
                   [(null? lits) (and (not (memv #f acc)) (reverse acc))]
                   [else
                    (let* ([lit (car lits)]
                           [subst-expr*
                            (map (lambda (e)
                                   (expr-subst-var-ref e elt-name lit))
                                 expr*)]
                           [lines
                            (compute-pl-builder-lines
                              src adt-op path-elt* subst-expr* local-binds
                              native-id-ht witness-id-ht circuit-id-ht)])
                      (loop (cdr lits) (cons lines acc)))]))])
          (cond
            [(or (not per-iter-lines) (not (fx= (length per-iter-lines) len))) #f]
            [else
             (emit-ctor-prelude pre-lines)
             (out "        let ops = OpProgramVerify::<DefaultDB>::new()\n")
             (for-each (lambda (group) (for-each out group)) per-iter-lines)
             (out "            .build();\n")
             (out "\n")
             (cond
               [(eq? mode 'ctor)
                (out "        let results = query_for_verify(&qctx, &ops, ctx.gas_limit.clone(), &ctx.cost_model)?;\n")
                (out "\n")
                (out "        Ok(ConstructorResult {\n")
                (out "            current_contract_state: results.context.state,\n")
                (out (if witness-emitted?
                         "            current_private_state,\n"
                         "            current_private_state: ctx.initial_private_state,\n"))
                (out (ctor-zswap-result-field))
                (out "        })\n")]
               [else
                (out "        let results = query_for_verify(\n")
                (out "            &ctx.current_query_context,\n")
                (out "            &ops,\n")
                (out "            ctx.gas_limit.clone(),\n")
                (out "            &ctx.cost_model,\n")
                (out "        )?;\n")
                (out "\n")
                (out "        Ok(CircuitResults {\n")
                (out "            result: (),\n")
                (out "            context: CircuitContext {\n")
                (out "                current_query_context: results.context,\n")
                (when witness-emitted?
                  (out "                current_private_state,\n"))
                (out "                ..ctx\n")
                (out "            },\n")
                (out (circuit-gas-result-field))
                (out "        })\n")])
             #t])))

      (define (emit-for-range-terminal
                lo hi
                src adt-op path-elt* expr* local-binds mode witness-emitted?
                pre-lines native-id-ht witness-id-ht circuit-id-ht)
        (let ([body-lines
               (compute-pl-builder-lines
                 src adt-op path-elt* expr* local-binds
                 native-id-ht witness-id-ht circuit-id-ht)]
              [iter-count (- hi lo)])
          (cond
            [(or (not body-lines) (< iter-count 0)) #f]
            [else
             (emit-ctor-prelude pre-lines)
             (out "        let ops = OpProgramVerify::<DefaultDB>::new()\n")
             ;; Compile-time unroll: emit the body's builder lines N
             ;; times. Since the loop body doesn't read `i`, the N
             ;; emitted line groups are identical — the VM state
             ;; mutates N times because each .ins() commits an in-place
             ;; update independently.
             (let loop ([k 0])
               (cond
                 [(fx= k iter-count) #f]
                 [else
                  (for-each out body-lines)
                  (loop (+ k 1))]))
             (out "            .build();\n")
             (out "\n")
             (cond
               [(eq? mode 'ctor)
                (out "        let results = query_for_verify(&qctx, &ops, ctx.gas_limit.clone(), &ctx.cost_model)?;\n")
                (out "\n")
                (out "        Ok(ConstructorResult {\n")
                (out "            current_contract_state: results.context.state,\n")
                (out (if witness-emitted?
                         "            current_private_state,\n"
                         "            current_private_state: ctx.initial_private_state,\n"))
                (out (ctor-zswap-result-field))
                (out "        })\n")]
               [else
                (out "        let results = query_for_verify(\n")
                (out "            &ctx.current_query_context,\n")
                (out "            &ops,\n")
                (out "            ctx.gas_limit.clone(),\n")
                (out "            &ctx.cost_model,\n")
                (out "        )?;\n")
                (out "\n")
                (out "        Ok(CircuitResults {\n")
                (out "            result: (),\n")
                (out "            context: CircuitContext {\n")
                (out "                current_query_context: results.context,\n")
                (when witness-emitted?
                  (out "                current_private_state,\n"))
                (out "                ..ctx\n")
                (out "            },\n")
                (out (circuit-gas-result-field))
                (out "        })\n")])
             #t])))
