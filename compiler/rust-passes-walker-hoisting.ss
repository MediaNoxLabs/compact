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
;;; This file: hoisting impure and witness calls out of expressions.

      ;; impure-call-bound: A15 alist lookup for current-impure-call-binds.
      ;; Same shape as witness-call-bound — each entry is (list function-name
      ;; arg-expr* rust-name). Returns the rust-name (a let-bound variable
      ;; holding `CircuitResults<PS,T>`) on hit, #f otherwise. ctor-call-rust
      ;; renders the call as `<rust-name>.result.clone()` when matched.
      (define (impure-call-bound function-name expr* binds)
        (let loop ([bs binds])
          (cond
            [(null? bs) #f]
            [(and (eq? (car (car bs)) function-name)
                  (let walk ([as (cadr (car bs))] [bs2 expr*])
                    (cond
                      [(and (null? as) (null? bs2)) #t]
                      [(or (null? as) (null? bs2)) #f]
                      [(eq? (car as) (car bs2))
                       (walk (cdr as) (cdr bs2))]
                      [else #f])))
             (caddr (car bs))]
            [else (loop (cdr bs))])))

      ;; collect-impure-call-subcalls: A15 sibling of collect-witness-subcalls.
      ;; Walk an Expression and return the list of (call <non-pure-user-circuit>
      ;; args*) sub-expressions in source order. Each call must be to a
      ;; user-defined circuit (in circuit-id-ht) that is NOT pure, NOT a
      ;; witness, NOT native. Duplicates dropped via eq? identity.
      (define (collect-impure-call-subcalls expr witness-id-ht circuit-id-ht
                                            native-id-ht)
        (let ([seen '()])
          (let walk ([e expr])
            (let ([e (expr-strip-cast e)])
              (nanopass-case (Ltypescript Expression) e
                [(call ,src ,function-name ,expr* ...)
                 (let ([w (eq-hashtable-ref witness-id-ht function-name #f)]
                       [c (eq-hashtable-ref circuit-id-ht function-name #f)]
                       [ne (eq-hashtable-ref native-id-ht function-name #f)])
                   (when (and c
                              (not (id-pure? function-name))
                              (not w)
                              (not ne))
                     (unless (memq e seen)
                       (set! seen (cons e seen)))))
                 (for-each walk expr*)]
                [(not ,src ,expr) (walk expr)]
                [(and ,src ,expr1 ,expr2) (walk expr1) (walk expr2)]
                [(or ,src ,expr1 ,expr2) (walk expr1) (walk expr2)]
                [(== ,src ,type ,expr1 ,expr2) (walk expr1) (walk expr2)]
                [(elt-ref ,src ,expr ,elt-name ,nat) (walk expr)]
                [else (void)])))
          (reverse seen)))

      ;; emit-hoisted-impure-calls: A15 sibling of emit-hoisted-witnesses.
      ;; For each impure circuit call in `subcalls`, emit the
      ;;   let _cr_h<N> = self.<name>(ctx, args)?;
      ;;   let ctx = _cr_h<N>.context;
      ;; lines and return (list lines binds), where lines is in reverse
      ;; (caller prepends) and binds is the list of
      ;; (list function-name arg-expr* rust-name) entries to feed
      ;; current-impure-call-binds.
      ;;
      ;; The hoisted call's `ctx` rebind threads the post-call QueryContext
      ;; through subsequent statements in the same Rust scope (the if-arm
      ;; body, or the streaming walker step). For read-only impure helpers
      ;; (e.g. did.compact's `verificationMethodExists`), the rebind's
      ;; effect on the result-of-verify is semantically a no-op (no state
      ;; mutation), but the threading keeps the type uniform and the gas
      ;; accounting honest.
      (define (emit-hoisted-impure-calls subcalls counter-start
                                         local-binds
                                         native-id-ht witness-id-ht circuit-id-ht
                                         indent)
        (let loop ([subs subcalls]
                   [counter counter-start]
                   [rev-lines '()]
                   [binds '()])
          (cond
            [(null? subs) (list rev-lines binds)]
            [else
             (let* ([call-expr (car subs)]
                    [function-name
                     (nanopass-case (Ltypescript Expression) call-expr
                       [(call ,src ,function-name ,expr* ...) function-name])]
                    [arg-exprs
                     (nanopass-case (Ltypescript Expression) call-expr
                       [(call ,src ,function-name ,expr* ...) expr*])]
                    [cname (id->rust-name function-name)]
                    [rust-name (format "_cr_h~a" counter)]
                    [arg-strs
                     (map (lambda (e)
                            (arg-rust-clone-if-var
                              e local-binds
                              native-id-ht witness-id-ht circuit-id-ht))
                          arg-exprs)]
                    [arg-tail
                     (let join ([xs arg-strs] [acc ""])
                       (cond
                         [(null? xs) acc]
                         [else (join (cdr xs)
                                     (string-append acc ", " (car xs)))]))]
                    ;; Bug-7: when this hoist runs inside a non-top-level
                     ;; arm (indent depth > 8 spaces), the surrounding code
                     ;; still needs `ctx` for the post-arm rebind. Pass
                     ;; `ctx.clone()` so the outer ctx remains usable. At
                     ;; the top-level body the ctx rebind that follows
                     ;; absorbs the move so the extra clone is harmless.
                     [arm-context? (> (string-length indent) 8)]
                     [ctx-arg (if arm-context? "ctx.clone()" "ctx")]
                    [call-line
                     (format "~alet ~a = self.~a(~a~a)?;\n"
                             indent rust-name cname ctx-arg arg-tail)]
                    [ctx-line
                     (format "~alet ctx = ~a.context;\n" indent rust-name)])
               (loop (cdr subs)
                     (+ counter 1)
                     (cons ctx-line (cons call-line rev-lines))
                     (cons (list function-name arg-exprs rust-name) binds)))])))

      ;; witness-call-bound: alist lookup for current-witness-call-binds.
      ;; Each entry is (list function-name arg-expr* rust-name). Match by
      ;; eq? on function-name and list of eq?-on-each arg expressions.
      ;; Returns the rust-name string on hit, #f otherwise.
      (define (witness-call-bound function-name expr* binds)
        (let loop ([bs binds])
          (cond
            [(null? bs) #f]
            [(and (eq? (car (car bs)) function-name)
                  (let walk ([as (cadr (car bs))] [bs2 expr*])
                    (cond
                      [(and (null? as) (null? bs2)) #t]
                      [(or (null? as) (null? bs2)) #f]
                      [(eq? (car as) (car bs2))
                       (walk (cdr as) (cdr bs2))]
                      [else #f])))
             (caddr (car bs))]
            [else (loop (cdr bs))])))

      ;; collect-witness-subcalls: walk an Expression and return the list
      ;; of (call <witness> args*) sub-expressions in source order. Used
      ;; by the body walker to hoist witness sub-calls out of assert
      ;; conditions before rendering them. Duplicates are dropped — a
      ;; second occurrence of the same eq?-identical node returns the
      ;; binding from the first hoist.
      (define (collect-witness-subcalls expr witness-id-ht)
        (let ([seen '()])
          (let walk ([e expr])
            (let ([e (expr-strip-cast e)])
              (nanopass-case (Ltypescript Expression) e
                [(call ,src ,function-name ,expr* ...)
                 (when (eq-hashtable-ref witness-id-ht function-name #f)
                   (unless (memq e seen)
                     (set! seen (cons e seen))))
                 (for-each walk expr*)]
                [(not ,src ,expr) (walk expr)]
                [(and ,src ,expr1 ,expr2) (walk expr1) (walk expr2)]
                [(or ,src ,expr1 ,expr2) (walk expr1) (walk expr2)]
                [(== ,src ,type ,expr1 ,expr2) (walk expr1) (walk expr2)]
                [(elt-ref ,src ,expr ,elt-name ,nat) (walk expr)]
                [(tuple ,src ,tuple-arg* ...)
                 (for-each
                   (lambda (ta)
                     (nanopass-case (Ltypescript Tuple-Argument) ta
                       [(single ,src ,expr) (walk expr)]
                       [(spread ,src ,nat ,expr) (walk expr)]
                       [else (void)]))
                   tuple-arg*)]
                [else (void)])))
          (reverse seen)))

      ;; emit-hoisted-witnesses: for each witness call expression in
      ;; subcalls, emit the witness-context + bind lines and return a
      ;; list of (lines binds witness-emitted?) tracking the accumulated
      ;; pre-lines, the per-call rust-name bindings (to feed
      ;; current-witness-call-binds), and the updated witness-emitted?
      ;; flag. `counter-start` is the starting index for _witness_ctx_N
      ;; numbering (typically `(length pre-lines)`).
      ;;
      ;; Returns (list rev-lines new-binds new-witness-emitted?), where
      ;; rev-lines is in reverse (so the caller can prepend them onto
      ;; pre-lines in the natural order) and new-binds is the list of
      ;; (list function-name arg-expr* rust-name) entries.
      (define (emit-hoisted-witnesses subcalls counter-start mode
                                      local-binds witness-emitted?
                                      native-id-ht witness-id-ht circuit-id-ht)
        (let loop ([subs subcalls]
                   [counter counter-start]
                   [we? witness-emitted?]
                   [rev-lines '()]
                   [binds '()])
          (cond
            [(null? subs) (list rev-lines binds we?)]
            [else
             (let* ([call-expr (car subs)]
                    [function-name
                     (nanopass-case (Ltypescript Expression) call-expr
                       [(call ,src ,function-name ,expr* ...) function-name])]
                    [arg-exprs
                     (nanopass-case (Ltypescript Expression) call-expr
                       [(call ,src ,function-name ,expr* ...) expr*])]
                    [wname (id->rust-name function-name)]
                    [rust-name (format "_w_~a_~a" wname counter)]
                    [ctx-name (format "_witness_ctx_h~a" counter)]
                    [state-expr (if (eq? mode 'ctor)
                                    "&qctx.state"
                                    "&ctx.current_query_context.state")]
                    [qctx-ref (if (eq? mode 'ctor)
                                  "&qctx"
                                  "&ctx.current_query_context")]
                    [prev-priv
                     (cond
                       [we? "current_private_state"]
                       [(eq? mode 'ctor) "ctx.initial_private_state"]
                       [else "ctx.current_private_state"])]
                    [arg-strs
                     (map (lambda (e)
                            (arg-rust-clone-if-var
                              e local-binds
                              native-id-ht witness-id-ht circuit-id-ht))
                          arg-exprs)]
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
               (loop (cdr subs)
                     (fx+ counter 1)
                     #t
                     (cons bind-line (cons call-line rev-lines))
                     (cons (list function-name arg-exprs rust-name) binds)))])))

      ;; assert-cond-supported?: like expr-supported? but additionally
      ;; accepts a (call ...) into a non-exported / impure circuit — we
      ;; render those as `true` placeholders. This means an assert whose
      ;; sole content is `in_state(STATE.unset)` is supported, but an
      ;; assert containing `apk == authority` is not (no expr-supported?
      ;; branch for `==`).
      (define (assert-cond-supported? expr native-id-ht witness-id-ht circuit-id-ht)
        (let ([e (expr-strip-cast expr)])
          (nanopass-case (Ltypescript Expression) e
            [(call ,src ,function-name ,expr* ...) #t]
            [else
             (expr-supported? e native-id-ht witness-id-ht circuit-id-ht)])))

      ;; impure-circuit-body-walkable?: pre-validate whether
      ;; emit-impure-circuit will succeed on a circuit's body. Mirrors
      ;; the body-shape dispatch inside emit-impure-circuit (rust-passes-
      ;; emit.ss:1068) — the body emits iff one of these predicates
      ;; matches:
      ;;   - if-expression-body (any return type)
      ;;   - unit-type AND (single-public-ledger-call OR body-walkable?
      ;;     OR body-streaming-walkable?)
      ;; Used by rust-passes.ss's emission filter to decide whether a
      ;; non-exported impure circuit is safe to emit as a method.
      (define (impure-circuit-body-walkable?
                cdefn native-id-ht witness-id-ht circuit-id-ht)
        (nanopass-case (Ltypescript Program-Element) cdefn
          [(circuit ,src ,function-name (,arg* ...) ,type ,stmt)
           (or (stmt->if-expression-body stmt)
               ;; A19: multi-arm chain / single-return body.
               (stmt->if-chain-body stmt)
               (and (unit-type? type)
                    (or (stmt->single-public-ledger-call stmt)
                        (body-walkable?
                          stmt native-id-ht witness-id-ht circuit-id-ht)
                        ;; A18: drop the body-needs-streaming? preference
                        ;; gate. Any body the streaming walker accepts is a
                        ;; superset of body-walkable?'s shapes, so when the
                        ;; simpler walker rejects we should still try
                        ;; streaming (e.g. terminal multi-arm if/else-if
                        ;; chains with single pl-call arms).
                        (body-streaming-walkable?
                          stmt native-id-ht witness-id-ht circuit-id-ht))))]
          [else #f]))
