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
;;; This file: if/else branch analysis and public-ledger call lines.

      ;; branch->single-pl-call: walk a single branch of an if-then-else
      ;; and extract a single `(public-ledger ...)` ADT-update call,
      ;; possibly preceded by safe-cast `const` bindings (which the
      ;; frontend inserts for literal-typed args, e.g. lowering
      ;; `tally_yes.increment(1)` to `const _tmp = safe-cast 1;
      ;; tally_yes.increment(_tmp);`).
      ;;
      ;; Returns (list src adt-op path-elt* resolved-expr*) on match
      ;; (mirroring `stmt->public-ledger-call`'s shape), #f otherwise.
      ;; `resolved-expr*` has had var-refs chased through any preceding
      ;; consts in the branch, and safe-cast layers stripped.
      (define (branch->single-pl-call stmt)
        (let loop ([stmts (stmt-flatten stmt)] [binds '()])
          (cond
            [(null? stmts) #f]
            [(const-binding (car stmts)) =>
             (lambda (b) (loop (cdr stmts) (cons b binds)))]
            [(and (null? (cdr stmts))
                  (stmt->public-ledger-call (car stmts))) =>
             (lambda (parts)
               ;; stmt->public-ledger-call returns (src adt-op path-elt*
               ;; expr*); rewrite expr* through `expr-resolve` so any
               ;; var-refs to preceding consts get chased.
               (let* ([src (car parts)]
                      [adt-op (cadr parts)]
                      [path-elt* (caddr parts)]
                      [expr* (cadddr parts)]
                      [resolved-expr* (map (lambda (e) (expr-resolve e binds))
                                           expr*)])
                 (and (not (memv #f resolved-expr*))
                      (list src adt-op path-elt* resolved-expr*))))]
            [else #f])))

      ;; branch->assert-and-pl-call: A12/A14/A17 sibling of branch->single-pl-call.
      ;; Admits a sequence inside an if-branch of:
      ;;   - const-decl-only (skipped — no Rust emission needed)
      ;;   - stmt->assignment (lifted-let initializer; tracked as pre-stmt)
      ;;   - const-binding (tracked as pre-stmt)
      ;;   - one optional `(assert ...)` (captured as assert-pair)
      ;; ...followed by ONE of:
      ;;   (a) a terminal pl-call (A12: setAlsoKnownAs's Insert arm)
      ;;   (b) nothing (A14: setVerificationMethod's Insert arm — assert-only)
      ;;   (c) a terminal bare-call to a non-pure user circuit
      ;;       (A17: setVerificationMethodRelation's branches call
      ;;       insertVerificationMethodRelation / removeVerificationMethodRelationFromLedger).
      ;;
      ;; Returns
      ;;   (list assert-pair pre-stmts src adt-op path-elt* resolved-expr* bare-call-info)
      ;; on success, #f on structural mismatch. `bare-call-info` is
      ;; `(cons fn-id arg-exprs)` for the A17 bare-call case, #f otherwise. Fields:
      ;;   - assert-pair: (cons assert-expr msg) or #f
      ;;   - pre-stmts: ordered list of (var-name . expr) bindings, to be
      ;;     emitted as `let <name> = <expr>;` inside the branch body
      ;;     BEFORE the assert and pl-call chain. Rendering relies on
      ;;     ctor-expr-rust's var-ref fallback to produce the same
      ;;     `(camel->snake id-sym)` name when the assert / pl-call
      ;;     references the lifted local.
      ;;   - src/adt-op/path-elt*/resolved-expr*: pl-call details, or
      ;;     src=#f (and the rest empty) for an assert-only branch.
      ;;
      ;; A14 motivating shape (did.compact setVerificationMethod's Update
      ;; branch):
      ;;   (seq (seq (const ((%tmp.262)))           ; const-decl-only
      ;;             (assert (seq (= %tmp.262 ...)  ; assignment, lifted from
      ;;                          (pl-read member))  ; the assert's cond seq
      ;;                     msg))
      ;;        (seq (const %tmp.264 (elt-ref ...))  ; const-binding
      ;;             (pl-call remove %tmp.264)))
      ;; Returns an 8-element list on success:
      ;;   (assert-pair pre-stmts src adt-op path-elt* resolved-expr*
      ;;    terminal-bare-call mid-calls)
      ;; where `mid-calls` is a list of NON-terminal bare impure-circuit
      ;; calls (each `(fn-id . resolved-args)`) captured between the guard
      ;; assert and the terminal op — did.compact 0.5.0's
      ;; setVerificationMethod / setVerificationMethodRelation interleave a
      ;; compatibility-assert circuit call there.
      (define (branch->assert-and-pl-call stmt)
        (let loop ([stmts (stmt-flatten stmt)]
                   [pre-stmts '()]
                   [binds '()]
                   [assert-pair #f]
                   [mid-calls '()]
                   [body-items '()])
          (cond
            [(null? stmts)
             ;; A14: assert-only branch is valid if we captured one.
             (and assert-pair
                  (list assert-pair (reverse pre-stmts) #f #f '() '() #f
                        (reverse mid-calls) (reverse body-items)))]
            [(const-decl-only? (car stmts))
             (loop (cdr stmts) pre-stmts binds assert-pair mid-calls body-items)]
            [(stmt->assignment (car stmts)) =>
             (lambda (a)
               (loop (cdr stmts)
                     (cons a pre-stmts)
                     (cons a binds)
                     assert-pair mid-calls
                     (cons (cons 'bind a) body-items)))]
            [(const-binding (car stmts)) =>
             (lambda (b)
               (loop (cdr stmts)
                     (cons b pre-stmts)
                     (cons b binds)
                     assert-pair mid-calls
                     (cons (cons 'bind b) body-items)))]
            [(stmt->assert (car stmts)) =>
             ;; A24: accept MULTIPLE asserts per branch. `assert-pair` keeps
             ;; the FIRST assert (back-compat for single-assert consumers);
             ;; `body-items` records every assert + bind in source order so
             ;; the emitter can render an ordered multi-assert branch (e.g.
             ;; did.compact 0.5.0's assertVerificationMethodRelationCompatible:
             ;; assert(member) / const lookup / assert(crv)). Single-assert
             ;; branches keep `body-items` with one assert, so the emitter's
             ;; legacy path still fires and byte-parity is preserved.
             (lambda (a)
               (loop (cdr stmts) pre-stmts binds
                     (or assert-pair a) mid-calls
                     (cons (cons 'assert a) body-items)))]
            [(and (null? (cdr stmts))
                  (stmt->public-ledger-call (car stmts))) =>
             (lambda (parts)
               (let* ([src (car parts)]
                      [adt-op (cadr parts)]
                      [path-elt* (caddr parts)]
                      [expr* (cadddr parts)]
                      [resolved-expr* (map (lambda (e) (expr-resolve e binds))
                                           expr*)])
                 (and (not (memv #f resolved-expr*))
                      (list assert-pair (reverse pre-stmts) src adt-op
                            path-elt* resolved-expr* #f (reverse mid-calls)
                            (reverse body-items)))))]
            [(and (null? (cdr stmts))
                  (stmt->bare-call (car stmts))) =>
             ;; A17: terminal bare-call to a non-pure user circuit
             ;; (e.g. setVerificationMethodRelation's
             ;; `insertVerificationMethodRelation(...)` /
             ;; `removeVerificationMethodRelationFromLedger(...)` calls).
             ;; classify-call must accept it as impure-exported / witness /
             ;; pure-circuit for the emit clause to render it; the helper
             ;; here just admits the shape and lets emission validate.
             (lambda (c)
               (let* ([fn-id (car c)]
                      [arg-exprs (cdr c)]
                      [resolved-args
                       (map (lambda (e) (expr-resolve e binds)) arg-exprs)])
                 (and (not (memv #f resolved-args))
                      (list assert-pair (reverse pre-stmts) #f #f '() '()
                            (cons fn-id resolved-args) (reverse mid-calls)
                            (reverse body-items)))))]
            [(stmt->bare-call (car stmts)) =>
             ;; A-05 (did.compact 0.5.0): a NON-terminal bare-call inside a
             ;; branch — a compatibility-assert circuit interleaved between
             ;; the guard assert and the terminal mutating op
             ;; (setVerificationMethod's
             ;; `assertExistingVerificationMethodRelationsCompatible(...)`,
             ;; setVerificationMethodRelation's
             ;; `assertVerificationMethodRelationCompatible(...)`). These
             ;; are ledger-reading asserts (no mutation), so emission renders
             ;; each as `self.<helper>(ctx.clone(), ...)?` before the op.
             ;; Terminal bare-calls are already caught above (the
             ;; `(null? (cdr stmts))` clause), so this reaches only
             ;; non-terminal ones.
             (lambda (c)
               (let* ([fn-id (car c)]
                      [arg-exprs (cdr c)]
                      [resolved-args
                       (map (lambda (e) (expr-resolve e binds)) arg-exprs)])
                 (and (not (memv #f resolved-args))
                      ;; A26: record the call BOTH in `mid-calls` (legacy) and
                      ;; as an ordered `'call` item in `body-items`, so the
                      ;; ordered emitter can place it at its source position and
                      ;; thread its returned context into later reads / the
                      ;; terminal op (rather than discarding it).
                      (loop (cdr stmts) pre-stmts binds assert-pair
                            (cons (cons fn-id resolved-args) mid-calls)
                            (cons (cons 'call (cons fn-id resolved-args))
                                  body-items)))))]
            [else #f])))

      ;; compute-pl-builder-lines: given a public-ledger ADT-op + path +
      ;; arg expressions + local bindings, compute the list of builder-
      ;; call lines for the OpProgramVerify chain (push/idx/ins/...) via
      ;; expand-vm-code + vminstr->builder-call. Returns the list of
      ;; strings on success, #f on any failure (so caller falls back).
      ;;
      ;; Extracted from emit-non-write-public-ledger-terminal so both
      ;; that emitter and the E6.2 if-branch emitter can share the
      ;; vm-code translation without duplicating logic.
      (define (compute-pl-builder-lines
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
                            ;; Integer literals must stay as plain Scheme
                            ;; integers so `addi`'s `vm-immediate->int`
                            ;; unwraps the `(VMvalue->int n)` cleanly.
                            ;; Non-literal args go through the
                            ;; `vm-rust-expr` carrier (lifts the rendered
                            ;; Rust string into expand-vm-code).
                            (let ([stripped (expr-strip-cast e)])
                              (nanopass-case (Ltypescript Expression) stripped
                                [(quote ,src ,datum)
                                 (if (and (integer? datum) (exact? datum))
                                     datum
                                     (let ([rendered
                                            (guard (c [#t #f])
                                              (arg-rust-clone-if-var e local-binds
                                                                     native-id-ht
                                                                     witness-id-ht
                                                                     circuit-id-ht))])
                                       (and rendered (make-vm-rust-expr rendered))))]
                                [else
                                 (let ([rendered
                                        (guard (c [#t #f])
                                          (arg-rust-clone-if-var e local-binds
                                                                 native-id-ht
                                                                 witness-id-ht
                                                                 circuit-id-ht))])
                                   (and rendered (make-vm-rust-expr rendered)))])))
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

      ;; if-then-else-branch-pl-call?: returns the (list src adt-op
      ;; path-elt* expr*) parts when `branch-stmt` is a single non-write
      ;; public-ledger call, AND the builder lines compute successfully
      ;; against the given local-binds. Returns #f otherwise.
      ;;
      ;; This is the predicate used by both body-walkable? (E6.2
      ;; pre-validation) and emit-body-or-fallback (E6.2 emission) to
      ;; decide if a branch is emittable.
      (define (if-then-else-branch-pl-call?
                branch-stmt local-binds
                native-id-ht witness-id-ht circuit-id-ht)
        (let ([parts (branch->single-pl-call branch-stmt)])
          (and parts
               ;; Reject write-class (Cell.write) — its emission lives in
               ;; the emit-body-writes path and would need a different
               ;; OpProgramVerify chain shape. ADT-update calls (insert,
               ;; increment, etc.) are what E6.2 targets.
               (let ([adt-op (cadr parts)])
                 (not (stmt->public-ledger-write branch-stmt)))
               (let ([lines (compute-pl-builder-lines
                              (car parts) (cadr parts) (caddr parts)
                              (cadddr parts) local-binds
                              native-id-ht witness-id-ht circuit-id-ht)])
                 (and lines parts)))))
