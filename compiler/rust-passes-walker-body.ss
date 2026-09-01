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
;;; This file: body walkability and the body/constructor dispatchers.

      ;; body-walkable?: pre-validate that the flat statement sequence is
      ;; one our walker can emit without producing TODO/unimplemented
      ;; markers. Mirrors emit-body-or-fallback's case analysis but only
      ;; inspects (never emits).
      (define (body-walkable? stmt native-id-ht witness-id-ht circuit-id-ht)
        (let ([stmts (stmt-flatten stmt)])
          (let loop ([stmts stmts])
            (cond
              [(null? stmts) #t]
              [(stmt->assert (car stmts)) =>
               (lambda (a)
                 (and (assert-cond-supported?
                        (car a) native-id-ht witness-id-ht circuit-id-ht)
                      (loop (cdr stmts))))]
              [(const-binding (car stmts)) =>
               (lambda (b)
                 (let* ([rhs (cdr b)]
                        [classified
                         (classify-const-rhs rhs witness-id-ht circuit-id-ht)])
                   (and (or (eq? (car classified) 'witness)
                            ;; M3.5-E4.4: accept ALL user pure-circuit
                            ;; callees (both exported and non-exported).
                            ;; Non-exported ones now land in
                            ;; `pure_circuits` as `pub(crate) fn`
                            ;; (Blocker 1), so they're equally callable.
                            (eq? (car classified) 'pure-circuit)
                            ;; M3.5-E5: accept exported impure circuit
                            ;; callees — emitted as `self.<name>(ctx, ...)`
                            ;; with context threading via `_cr_N`.
                            (eq? (car classified) 'impure-exported)
                            ;; I3b/3: also accept plain const RHS shapes
                            ;; (e.g. `const tmp = default<Bytes<32>>;`)
                            ;; whose expression is something expr-rust
                            ;; can render. emit-body-or-fallback's else
                            ;; branch already handles these.
                            (expr-supported? rhs native-id-ht
                                             witness-id-ht circuit-id-ht))
                        (loop (cdr stmts)))))]
              ;; E4.2: witness / pure-circuit call as a statement (no
              ;; const binding). zerocash_mint's `private$add_coin(coin);`
              ;; takes this shape — the return value is discarded but the
              ;; call still has side effects (witness updates private state).
              ;; Pure-circuit callees must additionally be exported (only
              ;; exported pure circuits land in the `pure_circuits` mod).
              ;;
              ;; DID-walker-A: terminal bare-calls are also accepted (e.g.
              ;; rotateControllerKey ends with `recordUpdate();`). The
              ;; emit-body-or-fallback loop's bare-call clause already
              ;; handles terminal positions by accumulating the call's
              ;; ctx-rebind into pre-lines and then exiting at the
              ;; (null? stmts) tail. The accumulated writes from earlier
              ;; statements anchor the OpProgramVerify chain — bodies
              ;; with no writes at all still drop to #f at that tail
              ;; and fall through to the streaming walker / error path.
              [(stmt->bare-call (car stmts)) =>
               (lambda (c)
                 (let* ([fn-id (car c)]
                        [arg* (cdr c)]
                        [classified
                         (classify-call fn-id arg* witness-id-ht circuit-id-ht)])
                   (and (or (eq? (car classified) 'witness)
                            ;; M3.5-E4.4: accept ALL user pure-circuit
                            ;; bare-call callees, exported or not — both
                            ;; now land in pure_circuits.
                            (eq? (car classified) 'pure-circuit)
                            ;; M3.5-E5 / DID-walker-A: accept bare calls
                            ;; to any user impure circuit (exported as
                            ;; pub fn, non-exported as pub(crate) fn).
                            ;; Both emit as `self.<name>(ctx, ...)?`.
                            (eq? (car classified) 'impure-exported))
                        (for-all (lambda (e)
                                   (expr-supported?
                                     e native-id-ht witness-id-ht circuit-id-ht))
                                 arg*)
                        (loop (cdr stmts)))))]
              ;; A4: non-write `public-ledger` call (ADT update op like
              ;; Counter.increment) in NON-TERMINAL position. did.compact's
              ;; `recordUpdate` body is the canonical example:
              ;;
              ;;   operationCount.increment(1);   <- non-terminal PL call
              ;;   version.increment(1);           <- non-terminal PL call
              ;;   updated = disclose(timestamp);  <- Cell.write (terminal)
              ;;
              ;; emit-body-or-fallback accumulates these into the same
              ;; tagged `mutations` chain as Cell.writes; emit-body-mutations
              ;; lays them out in source order on a single OpProgramVerify.
              ;; Same constraints as the terminal-PL-call clause below
              ;; (single-index path; supported arg expressions); the only
              ;; difference is `(pair? (cdr stmts))` instead of
              ;; `(null? (cdr stmts))`.
              [(and (pair? (cdr stmts))
                    (let ([parts (stmt->public-ledger-call (car stmts))])
                      (and parts
                           ;; A10: exclude only the legacy single-index
                           ;; Cell.write shape (handled by the cell-write
                           ;; template path below); multi-index writes flow
                           ;; through this pl-call clause and use vm-code.
                           (not (stmt->public-ledger-write (car stmts)))
                           parts))) =>
               (lambda (parts)
                 (let ([path-elt* (caddr parts)]
                       [expr* (cadddr parts)])
                   ;; A10: admit multi-index paths. Each path-elt must still
                   ;; be a path-index (numeric); path-elt->vm-value rejects
                   ;; runtime-keyed paths with #f and the emitter falls back
                   ;; to `unimplemented!()`.
                   (and (for-all (lambda (pe)
                                   (nanopass-case (Ltypescript Path-Element) pe
                                     [,path-index #t]
                                     [else #f]))
                                 path-elt*)
                        (for-all (lambda (e)
                                   (expr-supported?
                                     e native-id-ht witness-id-ht circuit-id-ht))
                                 expr*)
                        (loop (cdr stmts)))))]
              ;; E4.3: terminal `public-ledger` call. The legacy
              ;; stmt->public-ledger-write handles Cell.write specifically;
              ;; for ADT update ops (e.g. HistoricMerkleTree.insert) we
              ;; admit any single-index path whose arg expressions are
              ;; expr-supported? — the emission path will lift each arg
              ;; into a vm-rust-expr carrier and let expand-vm-code +
              ;; vminstr->builder-call render the OpProgramVerify chain.
              [(and (null? (cdr stmts))
                    (stmt->public-ledger-call (car stmts))) =>
               (lambda (parts)
                 (let ([path-elt* (caddr parts)]
                       [expr* (cadddr parts)])
                   ;; A10: admit multi-index paths (same rationale as the
                   ;; non-terminal pl-call clause above).
                   (and (for-all (lambda (pe)
                                   (nanopass-case (Ltypescript Path-Element) pe
                                     [,path-index #t]
                                     [else #f]))
                                 path-elt*)
                        (for-all (lambda (e)
                                   (expr-supported?
                                     e native-id-ht witness-id-ht circuit-id-ht))
                                 expr*))))]
              ;; E6.2: terminal `(if cond then-stmt else-stmt)` where
              ;; each branch is a single non-write public-ledger ADT
              ;; update call (e.g. `tally_yes.increment(1);`). The
              ;; emission path emits an `if` whose branches each carry
              ;; their own OpProgramVerify + query_for_verify; ctx is
              ;; threaded out via the if-expression's QueryResults.
              ;;
              ;; Local-binds aren't available in body-walkable? (we only
              ;; have the flat-statement view), so the per-branch
              ;; emittability check here is a coarse one — match the
              ;; shape but defer expr support to expr-supported?. The
              ;; emitter does the precise check via
              ;; if-then-else-branch-pl-call? and falls back if either
              ;; branch's builder lines can't be computed.
              [(and (null? (cdr stmts))
                    (stmt->if-then-else (car stmts))) =>
               (lambda (parts)
                 (let* ([then-stmt (cadr parts)]
                        [else-stmt (caddr parts)]
                        [then-call (branch->single-pl-call then-stmt)]
                        [else-call (branch->single-pl-call else-stmt)])
                   (and then-call else-call
                        ;; Both branches must be single-index path
                        ;; public-ledger calls whose arg expressions are
                        ;; expr-supported?. Mirror the predicate above.
                        (let ([then-path (caddr then-call)]
                              [then-exprs (cadddr then-call)]
                              [else-path (caddr else-call)]
                              [else-exprs (cadddr else-call)])
                          (and (fx= (length then-path) 1)
                               (fx= (length else-path) 1)
                               (nanopass-case (Ltypescript Path-Element) (car then-path)
                                 [,path-index #t]
                                 [else #f])
                               (nanopass-case (Ltypescript Path-Element) (car else-path)
                                 [,path-index #t]
                                 [else #f])
                               (for-all (lambda (e)
                                          (expr-supported?
                                            e native-id-ht witness-id-ht circuit-id-ht))
                                        then-exprs)
                               (for-all (lambda (e)
                                          (expr-supported?
                                            e native-id-ht witness-id-ht circuit-id-ht))
                                        else-exprs)
                               (expr-supported?
                                 (car parts) native-id-ht witness-id-ht circuit-id-ht))))))]
              ;; Iter 4: terminal `(for var-name tsize0 tsize1 #f body)`
              ;; range loop with literal nat bounds whose body is a
              ;; single non-write public-ledger ADT update call (e.g.
              ;; `c.increment(1);`). Emitted by unrolling the body's
              ;; builder lines (hi - lo) times into a single
              ;; OpProgramVerify chain. Bounds must be literal tsize
              ;; nats — variable bounds and iterable loops are deferred
              ;; to a future iteration.
              [(and (null? (cdr stmts))
                    (stmt->for-range (car stmts))) =>
               (lambda (fr)
                 (let* ([lo (cadr fr)]
                        [hi (caddr fr)]
                        [body-stmt (cadddr fr)]
                        [body-call (branch->single-pl-call body-stmt)])
                   (and body-call
                        ;; Body must be a single non-write public-ledger
                        ;; ADT-update call whose arg expressions are
                        ;; expr-supported?. Reject Cell.write (the
                        ;; emit-body-writes path) so we stick to ADT-update
                        ;; vm-code emission.
                        (not (stmt->public-ledger-write body-stmt))
                        (let ([body-path (caddr body-call)]
                              [body-exprs (cadddr body-call)])
                          (and (fx= (length body-path) 1)
                               (nanopass-case (Ltypescript Path-Element) (car body-path)
                                 [,path-index #t]
                                 [else #f])
                               (for-all (lambda (e)
                                          (expr-supported?
                                            e native-id-ht witness-id-ht circuit-id-ht))
                                        body-exprs))))))]
              ;; Iter 5/6: terminal `(statement-expression (fold ...))`
              ;; from a desugared `for (const x of <static-len iterable>)
              ;; { body }`. Body must be a single non-write public-
              ;; ledger ADT update call with a literal-index path. The
              ;; loop variable may appear in the body's args — Iter 6's
              ;; emit-for-iter-terminal substitutes per-iteration via
              ;; iterable-expr->literals. We require the iterable to be
              ;; a static array literal whose elements are integer
              ;; constants (or safe-cast over the same), so the
              ;; substitution can always materialise a `(quote …)`
              ;; integer at each iteration.
              [(and (null? (cdr stmts))
                    (stmt->for-iter (car stmts))) =>
               (lambda (fi)
                 (let* ([body-stmt (caddr fi)]
                        [iter-expr (cadddr fi)]
                        [literals (iterable-expr->literals iter-expr)]
                        [body-call (branch->single-pl-call body-stmt)])
                   (and body-call
                        literals
                        (fx= (length literals) (car fi))
                        (not (stmt->public-ledger-write body-stmt))
                        (let ([body-path (caddr body-call)]
                              [body-exprs (cadddr body-call)])
                          (and (fx= (length body-path) 1)
                               (nanopass-case (Ltypescript Path-Element) (car body-path)
                                 [,path-index #t]
                                 [else #f])
                               (for-all (lambda (e)
                                          (expr-supported?
                                            e native-id-ht witness-id-ht circuit-id-ht))
                                        body-exprs))))))]
              [else
               (let ([w (stmt->public-ledger-write (car stmts))])
                 (and w
                      (expr-supported?
                        (cdr w) native-id-ht witness-id-ht circuit-id-ht)
                      (loop (cdr stmts))))]))))

      ;; stmt->assert: detect a `(statement-expression (assert expr msg))`
      ;; and return (cons expr msg). The IR exposes `assert` as an Expression
      ;; that lives in statement position via `statement-expression`. Returns
      ;; #f for anything else.
      (define (stmt->assert stmt)
        (nanopass-case (Ltypescript Statement) stmt
          [(statement-expression ,expr)
           (nanopass-case (Ltypescript Expression) expr
             [(assert ,src ,expr^ ,mesg) (cons expr^ mesg)]
             [else #f])]
          [else #f]))

      ;; assert-cond-rust: render the assert condition. Witness / pure-
      ;; circuit / native calls route through ctor-expr-rust. Non-exported
      ;; circuit calls whose body we can inline (currently any circuit
      ;; whose body is a single return-expression — e.g. tiny.compact's
      ;; `in_state(s) => state == s`) get inlined via inline-circuit-call;
      ;; everything else falls back to a `true` placeholder so the assert
      ;; is a no-op rather than a compile error.
      (define (assert-cond-rust expr local-binds
                                native-id-ht witness-id-ht circuit-id-ht)
        (let ([e (expr-strip-cast expr)])
          (nanopass-case (Ltypescript Expression) e
            [(call ,src ,function-name ,expr* ...)
             (let ([ne (eq-hashtable-ref native-id-ht function-name #f)]
                   [w (eq-hashtable-ref witness-id-ht function-name #f)]
                   [c (eq-hashtable-ref circuit-id-ht function-name #f)])
               (cond
                 ;; Witness / pure-circuit / native — use ctor-expr-rust.
                 [(or ne w (and c (id-pure? function-name)))
                  (ctor-expr-rust e local-binds
                                  native-id-ht witness-id-ht circuit-id-ht)]
                 [c
                  ;; Non-exported (or impure) circuit call. Try to inline
                  ;; its body (the I3b/3 trick that turns the in_state
                  ;; placeholder into a semantically real comparison).
                  (or (inline-circuit-call c expr* local-binds
                                           native-id-ht witness-id-ht circuit-id-ht)
                      (format "/* TODO M3: inline ~a in assert */ true"
                              (id-sym function-name)))]
                 [else
                  (format "/* TODO M3: inline ~a in assert */ true"
                          (id-sym function-name))]))]
            [else
             (ctor-expr-rust e local-binds
                             native-id-ht witness-id-ht circuit-id-ht)])))

      ;; inline-circuit-call: attempt to inline a circuit invocation by
      ;; rendering the callee's body as a Rust expression with the
      ;; formals locally bound to the rendered actuals. Returns the
      ;; Rust expression string on success, #f if the body shape isn't a
      ;; single statement-expression we can render.
      ;;
      ;; The callee's body is walked via stmt-flatten — supported shape
      ;; is a single `(statement-expression expr)`. The formal-to-actual
      ;; map injects the rendered actual as the "Rust name" associated
      ;; with the formal id, so any `(var-ref formal)` inside the body
      ;; lowers to the actual's already-rendered Rust text. This works
      ;; because local-binds is consulted by ctor-expr-rust before
      ;; emitting var-ref's snake-cased name.
      (define (inline-circuit-call cdefn actual-expr* outer-local-binds
                                   native-id-ht witness-id-ht circuit-id-ht)
        (nanopass-case (Ltypescript Program-Element) cdefn
          [(circuit ,src ,function-name (,arg* ...) ,type ,stmt)
           (let ([stmts (stmt-flatten stmt)])
             (cond
               [(or (null? stmts) (not (null? (cdr stmts)))) #f]
               [else
                (nanopass-case (Ltypescript Statement) (car stmts)
                  [(statement-expression ,expr)
                   (cond
                     [(not (fx= (length arg*) (length actual-expr*))) #f]
                     [else
                      (let* ([formal-binds
                              (map (lambda (formal actual)
                                     (nanopass-case (Ltypescript Argument) formal
                                       [(,var-name ,type)
                                        ;; Bug-10: when the formal's declared type
                                        ;; is a tenum, render the actual with
                                        ;; current-enum-ref-typed? set so any
                                        ;; bare `EnumName.variant` actual emits
                                        ;; `EnumName::variant` rather than the
                                        ;; raw u8 discriminant. The body then sees
                                        ;; a typed enum on the substituted side,
                                        ;; matching Bug-10's typed decoder on the
                                        ;; ledger-read side (state == STATE.unset
                                        ;; inlined into in_state's body becomes
                                        ;; `decode_via_field_repr::<STATE>(_av)?
                                        ;; == STATE::unset`). Non-tenum formals
                                        ;; retain the existing untyped rendering.
                                        (let* ([formal-tenum? (tenum-name-of-type type)]
                                               [rendered
                                                (parameterize ([current-enum-ref-typed? (if formal-tenum? #t (current-enum-ref-typed?))])
                                                  (ctor-expr-rust actual outer-local-binds
                                                                  native-id-ht witness-id-ht circuit-id-ht))])
                                          (cons var-name rendered))]))
                                   arg* actual-expr*)]
                             [extended-binds (append formal-binds outer-local-binds)])
                        (ctor-expr-rust expr extended-binds
                                        native-id-ht witness-id-ht circuit-id-ht))])]
                  [else #f])]))]))

      ;; emit-body-or-fallback: walk the body of a constructor or circuit and
      ;; emit `let` bindings, optional leading asserts, and an OpProgramVerify
      ;; chain that writes each ledger field. Returns #t on success, #f if the
      ;; body shape isn't one we know how to handle (caller should fall back
      ;; to its placeholder/default return).
      ;;
      ;; The supported shape is the flat sequence
      ;;   (assert <expr> "msg")*
      ;;   (const local (call <witness-or-pure>) ...)*
      ;;   (public-ledger field idx write <expr>)+
      ;; matching tiny.compact's constructor and `set` circuit.
      ;;
      ;; `mode` is 'ctor or 'circuit and controls the witness-context shape
      ;; and final return wrapping (see emit-body-writes).
      ;;
      ;; A27: does a flat body sequence contain a top-level impure-circuit call
      ;; (const-binding RHS or bare statement)? Non-streamed circuit bodies with
      ;; such calls must accumulate the callees' gas (see circuit-gas-acc?).
      (define (body-has-impure-circuit-call? stmt witness-id-ht circuit-id-ht)
        (let loop ([stmts (stmt-flatten stmt)])
          (cond
            [(null? stmts) #f]
            [(const-binding (car stmts)) =>
             (lambda (b)
               (or (eq? (car (classify-const-rhs (cdr b) witness-id-ht circuit-id-ht))
                        'impure-exported)
                   (loop (cdr stmts))))]
            [(stmt->bare-call (car stmts)) =>
             (lambda (c)
               (or (eq? (car (classify-call (car c) (cdr c) witness-id-ht circuit-id-ht))
                        'impure-exported)
                   (loop (cdr stmts))))]
            [else (loop (cdr stmts))])))

      (define (emit-body-or-fallback stmt mode
                                     native-id-ht witness-id-ht circuit-id-ht)
        (let* ([stmts (stmt-flatten stmt)]
               ;; A27: seed a gas accumulator for non-streamed circuit bodies
               ;; that call impure helpers, so their gas is summed into the
               ;; result rather than dropped.
               [gas-acc?
                (and (eq? mode 'circuit)
                     (body-has-impure-circuit-call? stmt witness-id-ht circuit-id-ht))])
          (parameterize ([circuit-gas-acc? gas-acc?])
          (let loop ([stmts stmts]
                     [local-binds '()]     ; (var-name . rust-name)
                     [witness-emitted? #f] ; have we emitted any witness call?
                     [pre-lines (if gas-acc?
                                    (list "        let mut __gas_acc = compact_runtime::RunningCost::default();\n")
                                    '())] ; reverse-accumulated Rust lines
                     [writes '()])         ; reverse-accumulated tagged
                                           ; mutations: each entry is
                                           ; ('cell-write idx . expr) for
                                           ; a Cell.write OR ('pl-call src
                                           ; adt-op path-elt* expr*) for a
                                           ; non-write public-ledger ADT
                                           ; update call (A4).
            (cond
              [(null? stmts)
               ;; A7: bodies with no mutations (e.g. did.compact's
               ;; `assertController()` is a single assert with no
               ;; state writes) still need to emit the pre-lines +
               ;; a unit-return wrapper. emit-body-mutations on an
               ;; empty mutations list emits an empty OpProgramVerify
               ;; chain — query_for_verify on a build()-only chain
               ;; succeeds and threads ctx through unchanged, which
               ;; is the correct "side-effect-free" semantics for an
               ;; assert-only body. Pre-v0.2 this returned #f and
               ;; threw a hard "no walker shape matched" error.
               ;;
               ;; Bodies with absolutely no pre-lines AND no writes
               ;; (an empty function body) still return #f — there's
               ;; literally nothing for the walker to emit. Those
               ;; fall through to the streaming walker / error path
               ;; per the existing chain.
               (cond
                 [(and (null? writes) (null? pre-lines)) #f]
                 [else
                  ;; Prelude emits BEFORE we know whether
                  ;; emit-body-mutations succeeds — we've already
                  ;; committed the function-body opening brace by
                  ;; getting here. If mutations emission fails, the
                  ;; caller (emit-impure-circuit / emit-initial-state)
                  ;; sees #f and triggers `rust-feature-error` rather
                  ;; than silently producing an unclosed body.
                  ;; Propagate the return value directly.
                  (emit-ctor-prelude (reverse pre-lines))
                  (emit-body-mutations (reverse writes) mode local-binds
                                       native-id-ht witness-id-ht circuit-id-ht
                                       witness-emitted?)])]
              ;; E4.3: a TERMINAL `public-ledger` call whose op-class is
              ;; not `write` (e.g. HistoricMerkleTree.insert). The vm-code
              ;; expansion path renders it via expand-vm-code +
              ;; vminstr->builder-call, mirroring emit-public-ledger-call-body
              ;; but with the const-bindings + pre-lines already emitted.
              ;; Only fire when (a) there are no plain writes accumulated
              ;; (mixing Cell.write + ADT-insert in the same body isn't
              ;; supported yet) and (b) this is the last statement.
              [(and (null? (cdr stmts))
                    (null? writes)
                    (let ([parts (stmt->public-ledger-call (car stmts))])
                      (and parts
                           (not (stmt->public-ledger-write (car stmts)))
                           parts))) =>
               (lambda (parts)
                 (let ([src (car parts)]
                       [adt-op (cadr parts)]
                       [path-elt* (caddr parts)]
                       [expr* (cadddr parts)])
                   (and
                     (emit-non-write-public-ledger-terminal
                       src adt-op path-elt* expr* local-binds mode witness-emitted?
                       (reverse pre-lines)
                       native-id-ht witness-id-ht circuit-id-ht))))]
              ;; E6.2: terminal `(if cond then-stmt else-stmt)` whose
              ;; branches are each a single non-write public-ledger
              ;; ADT-update call. Emit Rust `if/else` where each branch
              ;; carries its own OpProgramVerify + query_for_verify; the
              ;; if-expression's QueryResults is threaded into the final
              ;; CircuitResults return. Mirrors the constraint above:
              ;; only fire when no plain writes accumulated and this is
              ;; the body's last statement.
              [(and (null? (cdr stmts))
                    (null? writes)
                    (let ([if-parts (stmt->if-then-else (car stmts))])
                      (and if-parts
                           (let ([then-parts
                                  (if-then-else-branch-pl-call?
                                    (cadr if-parts) local-binds
                                    native-id-ht witness-id-ht circuit-id-ht)]
                                 [else-parts
                                  (if-then-else-branch-pl-call?
                                    (caddr if-parts) local-binds
                                    native-id-ht witness-id-ht circuit-id-ht)])
                             (and then-parts else-parts
                                  (list (car if-parts) then-parts else-parts)))))) =>
               (lambda (bundle)
                 (emit-if-then-else-terminal
                   (car bundle) (cadr bundle) (caddr bundle)
                   local-binds mode witness-emitted? (reverse pre-lines)
                   native-id-ht witness-id-ht circuit-id-ht))]
              ;; Iter 4: terminal `(for var-name tsize0 tsize1 #f body)`
              ;; range loop with literal nat bounds. Body must be a single
              ;; non-write public-ledger ADT update call (e.g. Counter
              ;; increment). Unroll (hi - lo) iterations of the body's
              ;; builder lines into a single OpProgramVerify chain. The
              ;; loop var is not currently substituted into the body —
              ;; bodies that read `i` (e.g. `mp.insert(i)`) are deferred.
              [(and (null? (cdr stmts))
                    (null? writes)
                    (let ([fr (stmt->for-range (car stmts))])
                      (and fr
                           (let ([body-parts
                                  (branch->single-pl-call (cadddr fr))])
                             (and body-parts
                                  (not (stmt->public-ledger-write (cadddr fr)))
                                  (list (cadr fr) (caddr fr) body-parts)))))) =>
               (lambda (bundle)
                 (let ([lo (car bundle)]
                       [hi (cadr bundle)]
                       [body-parts (caddr bundle)])
                   (emit-for-range-terminal
                     lo hi
                     (car body-parts) (cadr body-parts)
                     (caddr body-parts) (cadddr body-parts)
                     local-binds mode witness-emitted? (reverse pre-lines)
                     native-id-ht witness-id-ht circuit-id-ht)))]
              ;; Iter 5/6: terminal `(statement-expression (fold ...))`
              ;; from a desugared `for (const x of <static-len iterable>)
              ;; { body }`. Unroll `len` iterations of the body's
              ;; builder lines into a single OpProgramVerify chain, with
              ;; the loop var substituted to the i-th literal from the
              ;; iterable on each iteration (Iter 6). The literals are
              ;; extracted up front so the loop body's expr* shape we
              ;; need to substitute into is captured once.
              [(and (null? (cdr stmts))
                    (null? writes)
                    (let ([fi (stmt->for-iter (car stmts))])
                      (and fi
                           (let* ([body-parts
                                   (branch->single-pl-call (caddr fi))]
                                  [literals
                                   (iterable-expr->literals (cadddr fi))])
                             (and body-parts
                                  literals
                                  (fx= (length literals) (car fi))
                                  (not (stmt->public-ledger-write (caddr fi)))
                                  (list (car fi) (cadr fi) body-parts literals)))))) =>
               (lambda (bundle)
                 (let ([len (car bundle)]
                       [elt-name (cadr bundle)]
                       [body-parts (caddr bundle)]
                       [literals (cadddr bundle)])
                   (emit-for-iter-terminal
                     len elt-name literals
                     (car body-parts) (cadr body-parts)
                     (caddr body-parts) (cadddr body-parts)
                     local-binds mode witness-emitted? (reverse pre-lines)
                     native-id-ht witness-id-ht circuit-id-ht)))]
              [(stmt->assert (car stmts)) =>
               (lambda (a)
                 (let* ([expr (car a)]
                        [msg (cdr a)]
                        [subcalls (collect-witness-subcalls
                                    expr witness-id-ht)]
                        [hoist (emit-hoisted-witnesses
                                 subcalls (length pre-lines) mode
                                 local-binds witness-emitted?
                                 native-id-ht witness-id-ht circuit-id-ht)]
                        [hoist-lines (car hoist)]
                        [hoist-binds (cadr hoist)]
                        [we2 (caddr hoist)]
                        [cond-str
                         (parameterize ([current-witness-call-binds
                                          (append hoist-binds
                                                  (current-witness-call-binds))])
                           (assert-cond-rust expr local-binds
                                             native-id-ht witness-id-ht circuit-id-ht))]
                        [line
                         (format "        compact_assert!(~a, ~s);\n"
                                 cond-str msg)])
                   (loop (cdr stmts)
                         local-binds
                         we2
                         (cons line (append hoist-lines pre-lines))
                         writes)))]
              [(const-binding (car stmts)) =>
               (lambda (b)
                 (let* ([var-name (car b)]
                        [rhs (cdr b)]
                        ;; Prod-14: uniquify against prior bindings in this
                        ;; body so multi-assignment constructors (each
                        ;; lowered to a fresh `const tmp = ...`) don't
                        ;; shadow earlier `let tmp = …`.
                        [rust-name
                         (uniquify-rust-name
                           (symbol->string (camel->snake (id-sym var-name)))
                           local-binds)]
                        [classified
                         (classify-const-rhs rhs witness-id-ht circuit-id-ht)])
                   ;; M3.5: record the var's declared type so later `==`
                   ;; rendering can detect tenum-typed locals.
                   (record-const-binding-type! var-name rhs
                                               witness-id-ht circuit-id-ht)
                   (case (car classified)
                     [(witness)
                      ;; Witness call. Emit:
                      ;;   let witness_ctx_N = WitnessContext::new(ledger(<state>), <prev-priv>, <qctx-ref>);
                      ;;   let (current_private_state, <name>) = self.witnesses.<m>(&witness_ctx_N, args...);
                      ;; In ctor mode the state/qctx live in the local
                      ;; `qctx` we built from the K1 seed; in circuit mode
                      ;; they come off `ctx.current_query_context`.
                      ;; For the first witness call, the source of the
                      ;; private state is `ctx.initial_private_state` (ctor)
                      ;; or `ctx.current_private_state` (circuit); for
                      ;; subsequent calls it's the `current_private_state`
                      ;; bound by the previous witness call.
                      (let* ([wname (cadr classified)]
                             [wargs (caddr classified)]
                             [ctx-name (format "_witness_ctx_~a" (length pre-lines))]
                             [state-expr (if (eq? mode 'ctor)
                                             "&qctx.state"
                                             "&ctx.current_query_context.state")]
                             [qctx-ref (if (eq? mode 'ctor)
                                           "&qctx"
                                           "&ctx.current_query_context")]
                             [prev-priv
                              (cond
                                [witness-emitted? "current_private_state"]
                                [(eq? mode 'ctor) "ctx.initial_private_state"]
                                [else "ctx.current_private_state"])]
                             [arg-strs
                              (map (lambda (e)
                                     (arg-rust-clone-if-var
                                       e local-binds
                                       native-id-ht witness-id-ht circuit-id-ht))
                                   wargs)]
                             [call-line
                              (format "        let ~a = WitnessContext::new(ledger(~a), ~a, ~a);\n"
                                      ctx-name state-expr prev-priv qctx-ref)]
                             [bind-line
                              (format "        let (current_private_state, ~a) = self.witnesses.~a(&~a~a);\n"
                                      rust-name wname ctx-name
                                      (let join ([xs arg-strs] [acc ""])
                                        (cond
                                          [(null? xs) acc]
                                          [else (join (cdr xs)
                                                      (string-append acc ", " (car xs)))])))])
                        (loop (cdr stmts)
                              (cons (cons var-name rust-name) local-binds)
                              #t
                              (cons bind-line (cons call-line pre-lines))
                              writes))]
                     [(pure-circuit)
                      ;; A6: witness sub-calls inside pure-circuit args.
                      ;; did.compact's `controllerKey(localSecretKey())`
                      ;; lowers to a const-binding RHS whose pargs
                      ;; embeds a witness call. Mirror the assert
                      ;; clause's hoister: collect witness sub-calls
                      ;; first, emit `let _w_<name>_N = ...` lines
                      ;; before the const-binding's own let, and
                      ;; parameterize `current-witness-call-binds` so
                      ;; the arg rendering finds the hoisted names.
                      (let* ([pname (cadr classified)]
                             [pargs (caddr classified)]
                             [subcalls
                              ;; Walk the full RHS expression (not
                              ;; just the args) so a witness call
                              ;; nested anywhere — including inside
                              ;; an elt-ref / cast / arithmetic — is
                              ;; caught.
                              (collect-witness-subcalls rhs witness-id-ht)]
                             [hoist (emit-hoisted-witnesses
                                      subcalls (length pre-lines) mode
                                      local-binds witness-emitted?
                                      native-id-ht witness-id-ht circuit-id-ht)]
                             [hoist-lines (car hoist)]
                             [hoist-binds (cadr hoist)]
                             [we2 (caddr hoist)]
                             ;; F2.2: peek at the callee's formal types so
                             ;; per-arg rendering can coerce tenum ledger
                             ;; reads to the actual enum variant.
                             [callee
                              (nanopass-case (Ltypescript Expression) (expr-strip-cast rhs)
                                [(call ,src ,function-name ,expr* ...)
                                 (eq-hashtable-ref circuit-id-ht function-name #f)]
                                [else #f])]
                             [formal-types (circuit-formal-arg-types callee)]
                             [arg-strs
                              (parameterize ([current-witness-call-binds
                                               (append hoist-binds
                                                       (current-witness-call-binds))])
                                (let loop ([as pargs] [fs formal-types] [acc '()])
                                  (cond
                                    [(null? as) (reverse acc)]
                                    [else
                                     (let* ([ft (and (pair? fs) (car fs))]
                                            [s (if ft
                                                   (render-pure-circuit-arg
                                                     (car as) ft local-binds
                                                     native-id-ht witness-id-ht circuit-id-ht)
                                                   (arg-rust-clone-if-var (car as) local-binds
                                                                          native-id-ht witness-id-ht circuit-id-ht))])
                                       (loop (cdr as)
                                             (if (pair? fs) (cdr fs) '())
                                             (cons s acc)))])))]
                             [bind-line
                              (format "        let ~a = pure_circuits::~a(~a)?;\n"
                                      rust-name pname
                                      (let join ([xs arg-strs] [acc ""])
                                        (cond
                                          [(null? xs) acc]
                                          [(null? (cdr xs)) (string-append acc (car xs))]
                                          [else (join (cdr xs)
                                                      (string-append acc (car xs) ", "))])))])
                        (loop (cdr stmts)
                              (cons (cons var-name rust-name) local-binds)
                              we2
                              (cons bind-line (append hoist-lines pre-lines))
                              writes))]
                     [(impure-exported)
                      ;; E5: const binding whose RHS is a call to an
                      ;; exported impure circuit. Emit
                      ;;     let _cr_N = self.<name>(ctx, args)?;
                      ;;     let ctx = _cr_N.context;
                      ;;     let <rust-name> = _cr_N.result;
                      ;; Subsequent witness / write code reads off the
                      ;; rebound `ctx`, so private state and gas state
                      ;; flow through transparently.
                      (let* ([cname (cadr classified)]
                             [cargs (caddr classified)]
                             [counter (length pre-lines)]
                             [cr-name (format "_cr_~a" counter)]
                             [arg-strs
                              (map (lambda (e)
                                     (arg-rust-clone-if-var
                                       e local-binds
                                       native-id-ht witness-id-ht circuit-id-ht))
                                   cargs)])
                        ;; A-05: hoist ctx-reading args (ledger reads) before
                        ;; the call moves `ctx` — see hoist-ctx-args.
                        (let-values ([(hoist-lines arg-tail)
                                      (hoist-ctx-args arg-strs counter)])
                          ;; A28: in a constructor, flush pending cell-writes to
                          ;; qctx BEFORE this impure call so its arg-reads and
                          ;; the callee's own ledger reads see the written
                          ;; fields (read-your-writes), then keep accumulating.
                          (let* ([flush? (and (eq? mode 'ctor) (pair? writes))]
                                 [flush-lines
                                  (if flush?
                                      (ctor-write-flush-lines writes local-binds
                                                              native-id-ht witness-id-ht
                                                              circuit-id-ht counter)
                                      '())]
                                 [writes-after (if flush? '() writes)]
                                 [thread-lines
                                  (impure-call-thread-lines
                                    cr-name (impure-call-target cname) arg-tail
                                    counter mode witness-emitted?)]
                                 [bind-line
                                  (format "        let ~a = ~a.result;\n"
                                          rust-name cr-name)])
                            (loop (cdr stmts)
                                  (cons (cons var-name rust-name) local-binds)
                                  (if (eq? mode 'ctor) #t witness-emitted?)
                                  (cons bind-line
                                        (append (reverse thread-lines)
                                                (append (reverse hoist-lines)
                                                        (append (reverse flush-lines)
                                                                pre-lines))))
                                  writes-after))))]
                     [else
                      ;; Unknown rhs shape — try a generic ctor-expr-rust
                      ;; render and emit a plain `let`.
                      ;;
                      ;; Prod-9: when the binding's declared type is `Field`
                      ;; (`tfield`) AND the RHS strips down to a bare integer
                      ;; literal, wrap as `Fr::from(<n>u64)` so downstream
                      ;; ledger-write builders (which demand `Into<AlignedValue>`)
                      ;; see the correct `Fr` type rather than an i32. Without
                      ;; this, contracts like
                      ;;     ledger v: Field;
                      ;;     constructor() { v = 42; }
                      ;; emit `let tmp = 42;` and fail to compile.
                      (let* ([decl-type (const-binding-decl-type (car stmts))]
                             [coerced (coerce-literal-rhs-rendered decl-type rhs)]
                             [raw
                              (or coerced
                                  (ctor-expr-rust rhs local-binds
                                                  native-id-ht witness-id-ht circuit-id-ht))]
                             ;; Bug-6: clone non-Copy var-ref / elt-ref RHS so
                             ;; the source struct/local stays usable after the
                             ;; lift. Skip the clone wrap when the coerced
                             ;; literal renamed the RHS (the literal path
                             ;; produced a copy already).
                             [rendered
                              (cond
                                [coerced raw]
                                [else (expr-rust-arg-cloned rhs raw)])])
                        (loop (cdr stmts)
                              (cons (cons var-name rust-name) local-binds)
                              witness-emitted?
                              (cons (format "        let ~a = ~a;\n" rust-name rendered)
                                    pre-lines)
                              writes))])))]
              ;; E4.2: a bare call statement (witness or pure-circuit whose
              ;; return value is discarded). zerocash_mint's
              ;; `private$add_coin(coin);` lands here. We emit a `let _ = ...`
              ;; (re-binding `current_private_state` for witness calls so
              ;; subsequent witness invocations see the updated state).
              [(stmt->bare-call (car stmts)) =>
               (lambda (c)
                 (let* ([fn-id (car c)]
                        [arg* (cdr c)]
                        [classified
                         (classify-call fn-id arg* witness-id-ht circuit-id-ht)])
                   (case (car classified)
                     [(witness)
                      (let* ([wname (cadr classified)]
                             [wargs (caddr classified)]
                             [ctx-name (format "_witness_ctx_~a" (length pre-lines))]
                             [state-expr (if (eq? mode 'ctor)
                                             "&qctx.state"
                                             "&ctx.current_query_context.state")]
                             [qctx-ref (if (eq? mode 'ctor)
                                           "&qctx"
                                           "&ctx.current_query_context")]
                             [prev-priv
                              (cond
                                [witness-emitted? "current_private_state"]
                                [(eq? mode 'ctor) "ctx.initial_private_state"]
                                [else "ctx.current_private_state"])]
                             [arg-strs
                              (map (lambda (e)
                                     (arg-rust-clone-if-var
                                       e local-binds
                                       native-id-ht witness-id-ht circuit-id-ht))
                                   wargs)]
                             [call-line
                              (format "        let ~a = WitnessContext::new(ledger(~a), ~a, ~a);\n"
                                      ctx-name state-expr prev-priv qctx-ref)]
                             [bind-line
                              (format "        let (current_private_state, _) = self.witnesses.~a(&~a~a);\n"
                                      wname ctx-name
                                      (let join ([xs arg-strs] [acc ""])
                                        (cond
                                          [(null? xs) acc]
                                          [else (join (cdr xs)
                                                      (string-append acc ", " (car xs)))])))])
                        (loop (cdr stmts)
                              local-binds
                              #t
                              (cons bind-line (cons call-line pre-lines))
                              writes))]
                     [(pure-circuit)
                      (let* ([pname (cadr classified)]
                             [pargs (caddr classified)]
                             [arg-strs
                              (map (lambda (e)
                                     (arg-rust-clone-if-var
                                       e local-binds
                                       native-id-ht witness-id-ht circuit-id-ht))
                                   pargs)]
                             [bind-line
                              (format "        let _ = pure_circuits::~a(~a)?;\n"
                                      pname
                                      (let join ([xs arg-strs] [acc ""])
                                        (cond
                                          [(null? xs) acc]
                                          [(null? (cdr xs)) (string-append acc (car xs))]
                                          [else (join (cdr xs)
                                                      (string-append acc (car xs) ", "))])))])
                        (loop (cdr stmts)
                              local-binds
                              witness-emitted?
                              (cons bind-line pre-lines)
                              writes))]
                     [(impure-exported)
                      ;; E5: bare statement-position call to an exported
                      ;; impure circuit. Discard the result value, but
                      ;; thread the returned context into a rebound `ctx`
                      ;; so subsequent statements see the updated state.
                      (let* ([cname (cadr classified)]
                             [cargs (caddr classified)]
                             [counter (length pre-lines)]
                             [cr-name (format "_cr_~a" counter)]
                             [arg-strs
                              (map (lambda (e)
                                     (arg-rust-clone-if-var
                                       e local-binds
                                       native-id-ht witness-id-ht circuit-id-ht))
                                   cargs)])
                        ;; A-05: hoist ctx-reading args before the moving call.
                        (let-values ([(hoist-lines arg-tail)
                                      (hoist-ctx-args arg-strs counter)])
                          ;; A28: flush pending ctor writes before the call so it
                          ;; (and its args) read the written fields — the
                          ;; did.compact constructor's
                          ;; assertControllerPublicKeyDistinctFromRecoveryAuthority
                          ;; reads both keys, which must be the witnessed values.
                          (let* ([flush? (and (eq? mode 'ctor) (pair? writes))]
                                 [flush-lines
                                  (if flush?
                                      (ctor-write-flush-lines writes local-binds
                                                              native-id-ht witness-id-ht
                                                              circuit-id-ht counter)
                                      '())]
                                 [writes-after (if flush? '() writes)]
                                 [thread-lines
                                  (impure-call-thread-lines
                                    cr-name (impure-call-target cname) arg-tail
                                    counter mode witness-emitted?)])
                            (loop (cdr stmts)
                                  local-binds
                                  (if (eq? mode 'ctor) #t witness-emitted?)
                                  (append (reverse thread-lines)
                                          (append (reverse hoist-lines)
                                                  (append (reverse flush-lines)
                                                          pre-lines)))
                                  writes-after))))]
                     [else #f])))]
              ;; A4: non-write `public-ledger` call (ADT update op like
              ;; Counter.increment) at ANY position. Accumulated into the
              ;; tagged `writes` chain as ('pl-call src adt-op path-elt*
              ;; expr*). emit-body-mutations dispatches per tag at body
              ;; finalization, emitting one OpProgramVerify chain that
              ;; interleaves cell-writes and pl-calls in source order.
              ;;
              ;; Position-wise this matches both non-terminal and terminal
              ;; non-write PL-calls. The earlier terminal-PL-call clause
              ;; (above) wins for sole-statement bodies via its `(null?
              ;; writes)` guard — preserving counter.compact byte-parity —
              ;; so we reach this clause only when writes is non-empty
              ;; (i.e. the multi-stmt did.compact recordUpdate shape).
              [(let ([parts (stmt->public-ledger-call (car stmts))])
                 (and parts
                      ;; A10: multi-index Cell.writes route here too;
                      ;; only the legacy single-index template path falls
                      ;; through to the cell-write tag below.
                      (not (stmt->public-ledger-write (car stmts)))
                      parts)) =>
               (lambda (parts)
                 (let ([src (car parts)]
                       [adt-op (cadr parts)]
                       [path-elt* (caddr parts)]
                       [expr* (cadddr parts)])
                   (loop (cdr stmts) local-binds witness-emitted?
                         pre-lines
                         (cons (list 'pl-call src adt-op path-elt* expr*)
                               writes))))]
              [else
               ;; Expect a `(statement-expression (public-ledger ... write expr))`.
               (let ([w (stmt->public-ledger-write (car stmts))])
                 (cond
                   [w (loop (cdr stmts) local-binds witness-emitted?
                            pre-lines
                            (cons (cons 'cell-write w) writes))]
                   [else #f]))])))))

      ;; emit-ctor-body-or-fallback: backwards-compatible wrapper that calls
      ;; emit-body-or-fallback in 'ctor mode. Kept for emit-initial-state's
      ;; existing call site.
      (define (emit-ctor-body-or-fallback stmt
                                          native-id-ht witness-id-ht circuit-id-ht)
        ;; Bug-2: ledger reads in the constructor body must read from
        ;; the local `qctx` we built from the K1 seed, not from
        ;; `&ctx.current_query_context` (which doesn't exist on
        ;; ConstructorContext<PS>). Parameterize so any
        ;; emit-ledger-read-expr triggered downstream by an in-expr
        ;; ledger read picks up the right source.
        ;; A25: reset the zswap-threading flag per constructor so
        ;; impure-call-thread-lines / the ConstructorResult emitters agree on
        ;; whether a `_zswap` local was introduced.
        (parameterize ([current-qctx-ref "&qctx"]
                       [ctor-zswap-threaded? #f])
          (emit-body-or-fallback stmt 'ctor
                                 native-id-ht witness-id-ht circuit-id-ht)))
