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
;;; This file: public-ledger call bodies, if-bodies, impure circuit emission.

      ;; emit-public-ledger-call-body: emit the I3a body — an
      ;; OpProgramVerify builder chain matching the adt-op's vm-code,
      ;; followed by the query_for_verify wrapper + Ok return. Returns #t
      ;; on success, #f if any step (path translation, vm-code expansion,
      ;; vminstr rendering) couldn't be handled — the caller falls back
      ;; to `unimplemented!()` in that case.
      (define (emit-public-ledger-call-body src adt-op path-elt* expr*)
        (nanopass-case (Ltypescript ADT-Op) adt-op
          [(,ledger-op ,op-class (,adt-name (,adt-formal* ,adt-arg*) ...) ((,var-name* ,type*) ...) ,type ,vm-code)
           (cond
             [(not (fx= (length expr*) (length var-name*))) #f]
             [else
              ;; Lift each path-elt + expr to a VM value (#f on anything
              ;; we don't yet know how to translate) before invoking
              ;; expand-vm-code. We bail out as soon as we hit something
              ;; unsupported so the placeholder is preserved.
              (let ([path-vals (map path-elt->vm-value path-elt*)]
                    [expr-vals (map expr->vm-value expr*)])
                (cond
                  [(memv #f path-vals) #f]
                  [(memv #f expr-vals) #f]
                  [else
                   (let* ([arg-alist
                           (append (map cons adt-formal* adt-arg*)
                                   (map (lambda (var-name v)
                                          (cons (id-sym var-name) v))
                                        var-name*
                                        expr-vals))]
                          [vminstr*
                           (expand-vm-code src path-vals #f arg-alist
                             (vm-code-code vm-code))]
                          [lines (map vminstr->builder-call vminstr*)])
                     (cond
                       [(memv #f lines) #f]
                       [else
                        (out "        let ops = OpProgramVerify::<DefaultDB>::new()\n")
                        (for-each out lines)
                        (out "            .build();\n")
                        (out "\n")
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
                        (out "                ..ctx\n")
                        (out "            },\n")
                        (out "            gas_cost: results.gas_cost,\n")
                        (out "        })\n")
                        #t]))]))])]))

      ;; emit-if-expression-body: emit the I3b/4 body shape — a single
      ;; if-expression in statement position producing a non-unit value.
      ;; The cond / then / else are rendered via ctor-expr-rust so existing
      ;; logic for inlining `in_state`, ledger reads in expression position,
      ;; and `some` / `none` runtime mapping all apply uniformly.
      ;;
      ;; Returns #t on success, #f if any rendered sub-expression contains
      ;; an `unimplemented!()` marker (caller falls back to `unimplemented!()`).
      ;;
      ;; The wrap uses `RunningCost::default()` since there are no ledger
      ;; writes — pure read-only circuits don't run query_for_verify.
      (define (emit-if-expression-body return-type cond-expr then-expr else-expr
                                       native-id-ht witness-id-ht circuit-id-ht)
        (let* ([cond-str (cond-rust cond-expr '()
                                    native-id-ht witness-id-ht circuit-id-ht)]
               [then-str (ctor-expr-rust then-expr '()
                                         native-id-ht witness-id-ht circuit-id-ht)]
               [else-str (ctor-expr-rust else-expr '()
                                         native-id-ht witness-id-ht circuit-id-ht)])
          (cond
            [(or (rendered-has-todo? cond-str)
                 (rendered-has-todo? then-str)
                 (rendered-has-todo? else-str))
             #f]
            [else
             (out (format "        let result = if ~a {\n" cond-str))
             (out (format "            ~a\n" then-str))
             (out "        } else {\n")
             (out (format "            ~a\n" else-str))
             (out "        };\n")
             (out "        Ok(CircuitResults {\n")
             (out "            result,\n")
             (out "            context: ctx,\n")
             (out "            gas_cost: compact_runtime::RunningCost::default(),\n")
             (out "        })\n")
             #t])))

      ;; emit-if-chain-body: A19 generalisation of emit-if-expression-body.
      ;; Renders a non-unit-returning impure circuit body composed of zero
      ;; or more if/else-if arms followed by a final else expression, then
      ;; wraps the value in CircuitResults. arms is a list of
      ;; (cond-expr then-expr) pairs (possibly empty for a single-return
      ;; body); else-expr is the final value.
      (define (emit-if-chain-body return-type arms else-expr
                                  native-id-ht witness-id-ht circuit-id-ht)
        (let* ([arm-strs
                (map (lambda (a)
                       (let ([c-str (cond-rust (car a) '()
                                               native-id-ht witness-id-ht
                                               circuit-id-ht)]
                             [t-str (ctor-expr-rust (cadr a) '()
                                                    native-id-ht witness-id-ht
                                                    circuit-id-ht)])
                         (list c-str t-str)))
                     arms)]
               [else-str (ctor-expr-rust else-expr '()
                                         native-id-ht witness-id-ht
                                         circuit-id-ht)]
               [any-todo?
                (or (rendered-has-todo? else-str)
                    (let loop ([xs arm-strs])
                      (cond
                        [(null? xs) #f]
                        [(or (rendered-has-todo? (caar xs))
                             (rendered-has-todo? (cadar xs))) #t]
                        [else (loop (cdr xs))])))])
          (cond
            [any-todo? #f]
            [(null? arms)
             ;; No arms: body is just `return <expr>;`. Emit directly.
             (out (format "        let result = ~a;\n" else-str))
             (out "        Ok(CircuitResults {\n")
             (out "            result,\n")
             (out "            context: ctx,\n")
             (out "            gas_cost: compact_runtime::RunningCost::default(),\n")
             (out "        })\n")
             #t]
            [else
             (let loop ([xs arm-strs] [first? #t])
               (cond
                 [(null? xs)
                  (out " else {\n")
                  (out (format "            ~a\n" else-str))
                  (out "        };\n")]
                 [first?
                  (out (format "        let result = if ~a {\n" (caar xs)))
                  (out (format "            ~a\n" (cadar xs)))
                  (out "        }")
                  (loop (cdr xs) #f)]
                 [else
                  (out (format " else if ~a {\n" (caar xs)))
                  (out (format "            ~a\n" (cadar xs)))
                  (out "        }")
                  (loop (cdr xs) #f)]))
             (out "        Ok(CircuitResults {\n")
             (out "            result,\n")
             (out "            context: ctx,\n")
             (out "            gas_cost: compact_runtime::RunningCost::default(),\n")
             (out "        })\n")
             #t])))

      ;; rendered-has-todo?: returns #t if the rendered Rust string
      ;; contains a TODO marker (`/* TODO`). Used by body emitters to
      ;; bail out (fall through to the method-level rust-feature-error)
      ;; when a sub-render produced a partially-supported placeholder
      ;; like `/* TODO ... */ true` (still valid Rust, but not
      ;; production-ready). Sub-renders that hit truly-unsupported
      ;; paths now raise via rust-feature-error rather than emit an
      ;; `unimplemented!()` string, so the predicate only needs to
      ;; scan for the `/* TODO` form.
      (define (rendered-has-todo? s)
        (and (string? s)
             (substring? s "/* TODO")))

      ;; substring?: simple substring search. Returns #t if `needle` appears
      ;; anywhere in `haystack`.
      (define (substring? haystack needle)
        (let ([hl (string-length haystack)]
              [nl (string-length needle)])
          (and (fx>= hl nl)
               (let loop ([i 0])
                 (cond
                   [(fx> (fx+ i nl) hl) #f]
                   [(string=? (substring haystack i (fx+ i nl)) needle) #t]
                   [else (loop (fx+ i 1))])))))

      ;; hoist-ctx-args: for an impure-circuit call `self.<m>(ctx, <args>)`,
      ;; any argument whose rendered text references `ctx` — a ledger read
      ;; emits `&ctx.current_query_context` — must be bound to a temp BEFORE
      ;; the call moves `ctx`, or Rust rejects the borrow-after-move
      ;; (did.compact 0.5.0's assertController ->
      ;; Schnorr_schnorrVerifyDigest(digest, sig, controllerPublicKey), where
      ;; controllerPublicKey is a JubjubPoint ledger read). Returns
      ;; (values hoist-lines arg-tail): `hoist-lines` are the `let _carg… = …;`
      ;; strings (the caller emits or prepends them), and `arg-tail` is the
      ;; ", "-prefixed argument list with hoisted args replaced by their temp
      ;; names. Non-ctx args stay inline, so contracts without ctx-reading
      ;; call arguments are byte-unchanged.
      (define (hoist-ctx-args arg-strs step)
        (let hloop ([xs arg-strs] [i 0] [lines '()] [names '()])
          (cond
            [(null? xs)
             (values (reverse lines)
                     (let join ([ys (reverse names)] [s ""])
                       (cond
                         [(null? ys) s]
                         [else (join (cdr ys)
                                     (string-append s ", " (car ys)))])))]
            [(substring? (car xs) "ctx")
             (let ([n (format "_carg_~a_~a" step i)])
               (hloop (cdr xs) (fx+ i 1)
                      (cons (format "        let ~a = ~a;\n" n (car xs)) lines)
                      (cons n names)))]
            [else
             (hloop (cdr xs) (fx+ i 1) lines (cons (car xs) names))])))

      ;; A22: emit an impure-circuit call plus context threading.
      ;;
      ;; In 'circuit mode the inbound `ctx` is already a CircuitContext, so
      ;; we hand it straight to the callee and rebind `ctx` from the result:
      ;;     let _cr_N = <target>(ctx, args)?;
      ;;     let ctx = _cr_N.context;
      ;;
      ;; In 'ctor mode the inbound `ctx` is a ConstructorContext (fields
      ;; initial_private_state / empty_zswap_local_state / cost_model /
      ;; gas_limit) and the working query context lives in the local `qctx`.
      ;; A circuit callee needs a CircuitContext, so we build one from qctx +
      ;; the current private state, call, then re-extract qctx and the private
      ;; state. Crucially we do NOT shadow `ctx`: the constructor's final
      ;; ConstructorResult still reads ctx.empty_zswap_local_state /
      ;; ctx.initial_private_state, so those fields must survive. cost_model /
      ;; gas_limit / empty_zswap are cloned because ctx is used again later.
      ;; Returns lines in FORWARD (source) order.
      (define (impure-call-thread-lines cr-name target arg-tail step mode
                                        witness-emitted?)
        (if (eq? mode 'ctor)
            (let* ([cctx (format "_cctx_~a" step)]
                   [priv (if witness-emitted?
                             "current_private_state"
                             "ctx.initial_private_state")]
                   ;; A25: the first ctor impure call seeds the callee's
                   ;; zswap-local state from `ctx.empty_zswap_local_state`;
                   ;; subsequent calls chain off the `_zswap` the previous
                   ;; call extracted. Either way we re-extract the callee's
                   ;; returned zswap-local state so a zswap-affecting
                   ;; constructor circuit's changes survive into the
                   ;; ConstructorResult (see ctor-zswap-result-field).
                   [first? (not (ctor-zswap-threaded?))]
                   [zswap-in (if first?
                                 "ctx.empty_zswap_local_state.clone()"
                                 "_zswap")])
              (ctor-zswap-threaded? #t)
              (list
                (format "        let ~a = CircuitContext {\n" cctx)
                (format "            current_private_state: ~a,\n" priv)
                "            current_query_context: qctx,\n"
                (format "            current_zswap_local_state: ~a,\n" zswap-in)
                "            cost_model: ctx.cost_model.clone(),\n"
                "            gas_limit: ctx.gas_limit.clone(),\n"
                "        };\n"
                (format "        let ~a = ~a(~a~a)?;\n" cr-name target cctx arg-tail)
                (format "        let qctx = ~a.context.current_query_context;\n" cr-name)
                (format "        let current_private_state = ~a.context.current_private_state;\n"
                        cr-name)
                (format "        let _zswap = ~a.context.current_zswap_local_state;\n"
                        cr-name)))
            ;; 'circuit mode: ctx is a CircuitContext; hand it straight to the
            ;; callee and rebind. A27: when the body accumulates gas, add the
            ;; callee's cost so a pre-terminal helper (assert / recordUpdate)
            ;; does not drop its gas from the final CircuitResults.
            (if (circuit-gas-acc?)
                (list
                  (format "        let ~a = ~a(ctx~a)?;\n" cr-name target arg-tail)
                  (format "        let ctx = ~a.context;\n" cr-name)
                  (format "        __gas_acc += ~a.gas_cost.clone();\n" cr-name))
                (list
                  (format "        let ~a = ~a(ctx~a)?;\n" cr-name target arg-tail)
                  (format "        let ctx = ~a.context;\n" cr-name)))))

      ;; cond-rust: render a boolean condition expression. Like
      ;; ctor-expr-rust but for `(call ...)` of an impure circuit
      ;; (e.g. tiny.compact's `in_state`) we try inline-circuit-call
      ;; first, since impure circuits can't be a direct Rust call target.
      (define (cond-rust expr local-binds
                         native-id-ht witness-id-ht circuit-id-ht)
        (let ([e (expr-strip-cast expr)])
          (nanopass-case (Ltypescript Expression) e
            [(call ,src ,function-name ,expr* ...)
             (let ([ne (eq-hashtable-ref native-id-ht function-name #f)]
                   [w (eq-hashtable-ref witness-id-ht function-name #f)]
                   [c (eq-hashtable-ref circuit-id-ht function-name #f)])
               (cond
                 [(or ne w (and c (id-pure? function-name)))
                  (ctor-expr-rust e local-binds
                                  native-id-ht witness-id-ht circuit-id-ht)]
                 [c
                  (or (inline-circuit-call c expr* local-binds
                                           native-id-ht witness-id-ht circuit-id-ht)
                      (format "/* TODO M3-I3b/4: inline ~a in if-cond */ true"
                              (id-sym function-name)))]
                 [else
                  (format "/* TODO M3-I3b/4: inline ~a in if-cond */ true"
                          (id-sym function-name))]))]
            [else
             (ctor-expr-rust e local-binds
                             native-id-ht witness-id-ht circuit-id-ht)])))

      ;; emit-impure-circuit: emit an impure circuit as a method on
      ;; `impl<PS, W> Contract<PS, W>`. Takes `&self, ctx: CircuitContext<PS>`
      ;; plus the source-level args typed via type-rust, and returns
      ;; `Result<CircuitResults<PS, T>, CompactError>` for the declared T.
      ;;
      ;; I3a recognises the narrow shape — a single `(public-ledger ...)`
      ;; statement returning `()` (e.g. counter.compact's
      ;; `round.increment(1);`) — and emits the corresponding Op program
      ;; via `expand-vm-code`. Anything richer keeps the `unimplemented!()`
      ;; placeholder so I3b+ can take it on without losing the build.
      (define (emit-impure-circuit cdefn native-id-ht witness-id-ht circuit-id-ht)
        (nanopass-case (Ltypescript Program-Element) cdefn
          [(circuit ,src ,function-name (,arg* ...) ,type ,stmt)
           ;; Exported impure circuits land on the public Contract API.
           ;; Non-exported ones are private helpers callable from bare-call
           ;; statements in other circuit bodies — same emission shape, but
           ;; `pub(crate)` so downstream crates don't see them. Mirrors the
           ;; pure-circuit visibility convention in emit-pure-circuit.
           (out (format "    ~a fn ~a(\n"
                        (if (id-exported? function-name) "pub" "pub(crate)")
                        (id->rust-name function-name)))
           (out "        &self,\n")
           (out "        ctx: CircuitContext<PS>")
           (emit-circuit-args arg*)
           (out (format ",\n    ) -> Result<CircuitResults<PS, ~a>, CompactError> {\n"
                        (type-rust type)))
           (parameterize ([current-formal-arg-types (build-formal-arg-type-ht arg*)])
           (let ([emitted?
                  (or
                    ;; I3b/4: single if-expression body returning non-unit.
                    ;; tiny.compact's `get()` lowers to this shape. We dispatch
                    ;; before the unit-only paths so a non-unit if-body
                    ;; doesn't fall through to `unimplemented!()`.
                    (let ([parts (stmt->if-expression-body stmt)])
                      (and parts
                           (not (unit-type? type))
                           (emit-if-expression-body
                             type (car parts) (cadr parts) (caddr parts)
                             native-id-ht witness-id-ht circuit-id-ht)))
                    ;; A19: multi-arm if/else-if returning non-unit, or a
                    ;; single trailing return-expression body. Closes
                    ;; did.compact's verificationMethodExists (single
                    ;; return `member(x) || member(y)`) and
                    ;; verificationMethodRelationMember (5-arm if/else-if
                    ;; with trailing `return false`).
                    (let ([chain (stmt->if-chain-body stmt)])
                      (and chain
                           (not (unit-type? type))
                           (emit-if-chain-body
                             type (car chain) (cadr chain)
                             native-id-ht witness-id-ht circuit-id-ht)))
                    (and (unit-type? type)
                         (or
                           ;; I3a: counter-style single public-ledger call.
                           (let ([call (stmt->single-public-ledger-call stmt)])
                             (and call
                                  (emit-public-ledger-call-body
                                    src
                                    (cadr call)        ; adt-op
                                    (car call)         ; path-elt*
                                    (caddr call))))    ; expr*
                           ;; I3b/2: tiny.compact `set`-style body — leading
                           ;; asserts + const bindings + ledger writes. We
                           ;; pre-validate via body-walkable? so partial /
                           ;; broken emissions (e.g. tiny's `clear`, which
                           ;; needs `==` and `default<T>`) fall back to
                           ;; `unimplemented!()` rather than producing
                           ;; uncompilable Rust.
                           (and (body-walkable? stmt
                                                native-id-ht witness-id-ht circuit-id-ht)
                                (emit-body-or-fallback stmt 'circuit
                                                       native-id-ht witness-id-ht circuit-id-ht))
                           ;; Streaming walker for richer multi-stage bodies
                           ;; (zerocash.spend, election.vote$commit /
                           ;; vote$reveal). Tried when emit-body-or-fallback
                           ;; bails — A18 dropped the body-needs-streaming?
                           ;; preference gate so terminal multi-arm if-chains
                           ;; (did.compact's insertVerificationMethodRelation,
                           ;; removeVerificationMethodRelationFromLedger) also
                           ;; route through streaming.
                           (and (body-streaming-walkable?
                                  stmt native-id-ht witness-id-ht circuit-id-ht)
                                (emit-streaming-body
                                  stmt native-id-ht witness-id-ht circuit-id-ht)))))])
             (unless emitted?
               (rust-feature-error src 'circuit-body-emission
                 "no walker shape matched circuit body for ~a"
                 (id-sym function-name))))
           (out "    }\n\n"))]))
