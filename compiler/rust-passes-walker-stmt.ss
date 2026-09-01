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
;;; This file: statement classification and shape extraction.

      ;; classify-const-rhs: inspect a `const` binding's RHS expression and
      ;; classify the call (or return 'unknown). Returns
      ;;   (list 'witness rust-name args)         for witness calls
      ;;   (list 'pure-circuit rust-name args)    for pure circuit calls
      ;;   (list 'impure-exported rust-name args) for exported impure circuit
      ;;                                          method calls (`self.<name>`)
      ;;   (list 'unknown)                        otherwise
      (define (classify-const-rhs rhs witness-id-ht circuit-id-ht)
        (let ([e (expr-strip-cast rhs)])
          (nanopass-case (Ltypescript Expression) e
            [(call ,src ,function-name ,expr* ...)
             (cond
               [(eq-hashtable-ref witness-id-ht function-name #f)
                (list 'witness
                      (id->rust-name function-name)
                      expr*)]
               [(and (eq-hashtable-ref circuit-id-ht function-name #f)
                     (id-pure? function-name))
                (list 'pure-circuit
                      (id->rust-name function-name)
                      expr*)]
               ;; E5 / A17: impure circuit (exported or internal). The
               ;; callee is emitted as a method on the Contract impl
               ;; (visibility per emit-impure-circuit), so the call shape
               ;; is `self.<snake>(ctx, args)?` returning CircuitResults.
               ;; Tag stays 'impure-exported to keep downstream emit
               ;; clauses uniform.
               [(and (eq-hashtable-ref circuit-id-ht function-name #f)
                     (not (id-pure? function-name)))
                (list 'impure-exported
                      (id->rust-name function-name)
                      expr*)]
               [else (list 'unknown)])]
            [else (list 'unknown)])))

      ;; stmt->public-ledger-write: detect a single statement of shape
      ;; `(statement-expression (public-ledger field (idx) write expr))` and
      ;; return (cons path-idx expr). The path must be a single path-index;
      ;; ledger-op must be `write`. Returns #f for anything else.
      (define (stmt->public-ledger-write stmt)
        (nanopass-case (Ltypescript Statement) stmt
          [(statement-expression ,expr)
           (nanopass-case (Ltypescript Expression) expr
             [(public-ledger ,src ,ledger-field-name ,sugar? (,path-elt* ...) ,src^ ,adt-op ,expr* ...)
              (nanopass-case (Ltypescript ADT-Op) adt-op
                [(,ledger-op ,op-class (,adt-name (,adt-formal* ,adt-arg*) ...) ((,var-name* ,type*) ...) ,type ,vm-code)
                 (cond
                   [(not (eq? ledger-op 'write)) #f]
                   [(not (fx= (length path-elt*) 1)) #f]
                   [(not (fx= (length expr*) 1)) #f]
                   [else
                    (let ([path-idx
                           (nanopass-case (Ltypescript Path-Element) (car path-elt*)
                             [,path-index path-index]
                             [else #f])])
                      (and path-idx (cons path-idx (car expr*))))])])]
             [else #f])]
          [else #f]))

      ;; stmt->public-ledger-call: detect a single statement of shape
      ;; `(statement-expression (public-ledger field (idx) <op> expr*))` for
      ;; any ledger-op (including non-write ADT update ops like `insert`).
      ;; Returns (list src adt-op path-elt* expr*) on a match, #f otherwise.
      ;; Distinct from stmt->public-ledger-write — this terminal-call helper
      ;; is used by the body walker to dispatch the vm-code-driven emission
      ;; path (matching emit-public-ledger-call-body) for ADT inserts etc.
      (define (stmt->public-ledger-call stmt)
        (nanopass-case (Ltypescript Statement) stmt
          [(statement-expression ,expr)
           (nanopass-case (Ltypescript Expression) expr
             [(public-ledger ,src ,ledger-field-name ,sugar? (,path-elt* ...) ,src^ ,adt-op ,expr* ...)
              (list src adt-op path-elt* expr*)]
             [else #f])]
          [else #f]))

      ;; stmt->bare-call: detect a single statement of shape
      ;; `(statement-expression (call <fn-id> <arg-expr>*))` and return
      ;; (cons fn-id args*). Used for witness or pure-circuit calls in
      ;; statement position whose return value is discarded (e.g.
      ;; zerocash_mint's `private$add_coin(coin);`). Returns #f otherwise.
      (define (stmt->bare-call stmt)
        (nanopass-case (Ltypescript Statement) stmt
          [(statement-expression ,expr)
           (let ([e (expr-strip-cast expr)])
             (nanopass-case (Ltypescript Expression) e
               [(call ,src ,function-name ,expr* ...)
                (cons function-name expr*)]
               [else #f]))]
          [else #f]))

      ;; classify-call: same shape as classify-const-rhs but for a bare
      ;; (call <fn-id> args*) at statement position. Returns
      ;;   (list 'witness rust-name args)
      ;;   (list 'pure-circuit rust-name args)
      ;;   (list 'impure-exported rust-name args)
      ;;   (list 'unknown)
      (define (classify-call fn-id arg* witness-id-ht circuit-id-ht)
        (cond
          [(eq-hashtable-ref witness-id-ht fn-id #f)
           (list 'witness (id->rust-name fn-id) arg*)]
          [(and (eq-hashtable-ref circuit-id-ht fn-id #f)
                (id-pure? fn-id))
           (list 'pure-circuit (id->rust-name fn-id) arg*)]
          ;; E5 / DID-walker-A: bare-call to an impure circuit (exported
          ;; or not — both are emitted as methods on the contract impl,
          ;; with `pub` vs `pub(crate)` visibility per emit-impure-circuit).
          ;; Emitted as `self.<snake>(ctx, ...)?` with the returned context
          ;; threaded into a rebound `ctx`. The tag stays 'impure-exported
          ;; to keep the downstream emit clauses unchanged.
          [(and (eq-hashtable-ref circuit-id-ht fn-id #f)
                (not (id-pure? fn-id)))
           (list 'impure-exported (id->rust-name fn-id) arg*)]
          [else (list 'unknown)]))

      ;; stmt->if-then-else: detect a `(if cond then-stmt else-stmt)`
      ;; statement and return (list cond then-stmt else-stmt). Used by
      ;; E6.2's impure if-mid-body walker extension.
      ;;
      ;; A13: Ltypescript Statement has BOTH a 3-arg `(if cond stmt1 stmt2)`
      ;; form AND a 2-arg `(if cond stmt1)` no-else form (langs.ss:875).
      ;; Normalise the 2-arg form by synthesising a unit `(tuple)`
      ;; statement-expression as the else, so downstream code can rely
      ;; on a uniform 3-element shape.
      (define (stmt->if-then-else stmt)
        (nanopass-case (Ltypescript Statement) stmt
          [(if ,src ,expr0 ,stmt1 ,stmt2) (list expr0 stmt1 stmt2)]
          [(if ,src ,expr0 ,stmt1)
           (list expr0
                 stmt1
                 (with-output-language (Ltypescript Statement)
                   `(statement-expression
                      ,(with-output-language (Ltypescript Expression)
                         `(tuple ,src)))))]
          [else #f]))

      ;; tsize->int / stmt->for-range: HISTORICAL — the original Iter 4
      ;; path dispatched on Ltypescript Type-Size + Statement `for` forms,
      ;; but those non-terminals/forms don't exist at the Ltypescript
      ;; layer. Type-Size is removed at the Lexpanded boundary
      ;; (langs.ss:611-613), and the Statement `for` form is removed at
      ;; Lexpr (langs.ss:367-376). The frontend lowers range loops
      ;;   `for (const i of N..M) { body }`
      ;; to an iterable form `(for src var-name (tuple ... lits ...) body)`
      ;; in expand-modules-and-types (analysis-passes.ss:1247-1261), and
      ;; infer-types then desugars the iterable form to a `(fold ...)`
      ;; expression (analysis-passes.ss:2878-2894). By the time we reach
      ;; Ltypescript every for-loop — range or iterable — is a `fold`.
      ;;
      ;; The dispatch sites in body-walkable? / emit-body-or-fallback
      ;; still reference these helpers; they're kept as always-#f stubs
      ;; so the call sites are no-ops and the fold-based Iter 5/6 path
      ;; (extended below to also recognise the lowered range form's
      ;; `(tuple src ...)` iterable shape) handles every case.
      (define (tsize->int t) #f)
      (define (stmt->for-range stmt) #f)

      ;; Iter 5: detect a Compact `for (const _ of <static-len iterable>)
      ;; { body }` after frontend desugaring. The Ltypescript IR has
      ;; already lowered the `for...of` form to a `fold`:
      ;;   (statement-expression
      ;;     (fold src len (circuit ((acc-var atype) (elt-var etype)) atype
      ;;                            (seq body-stmt acc-var-ref))
      ;;           init-expr
      ;;           map-arg))
      ;; with `len` known statically. We accept the shape only when the
      ;; body's accumulator is threaded unchanged (the trailing
      ;; statement-expression is a var-ref to `acc-var`), so the fold
      ;; degenerates into N side-effecting body executions — the same
      ;; semantics Iter 4's for-range covers.
      ;;
      ;; Returns (list len elt-var body-stmt iterable-expr) on match,
      ;; #f otherwise. `elt-var` is the loop-variable name (the element
      ;; binding in the desugared fold lambda). `iterable-expr` is the
      ;; iterable Expression extracted from the fold's single Map-
      ;; Argument slot — Iter 6 consumers (emit-for-iter-terminal) walk
      ;; it via `iterable-expr->literals` to recover the per-iteration
      ;; literal values used to substitute `elt-var` in the body.
      (define (stmt->for-iter stmt)
        (nanopass-case (Ltypescript Statement) stmt
          [(statement-expression ,expr)
           (nanopass-case (Ltypescript Expression) expr
             [(fold ,src ,len ,fun (,expr0 ,type0) ,map-arg ,map-arg* ...)
              ;; Single map-arg only — multi-zip folds (multiple
              ;; iterables) aren't covered by the MVP. The `(expr0
              ;; type0)` grouping is the fold's initial-accumulator
              ;; value + its type; we don't use either (Iter 6 only
              ;; substitutes the element binding) but the pattern must
              ;; destructure them so `map-arg` lines up.
              (and (null? map-arg*)
                   (nanopass-case (Ltypescript Function) fun
                     [(circuit ,src (,arg* ...) ,type ,stmt^)
                      ;; Expect exactly two args: (acc, elt).
                      (cond
                        [(not (fx= (length arg*) 2)) #f]
                        [else
                         (let ([acc-arg (car arg*)]
                               [elt-arg (cadr arg*)])
                           (nanopass-case (Ltypescript Argument) acc-arg
                             [(,var-name ,type)
                              (let ([acc-name var-name])
                                (nanopass-case (Ltypescript Argument) elt-arg
                                  [(,var-name ,type)
                                   (let ([elt-name var-name]
                                         [stripped
                                          (fold-body-strip-acc-return
                                            stmt^ acc-name)]
                                         [iter-expr
                                          (map-arg->expr map-arg)])
                                     (and stripped
                                          iter-expr
                                          (list len elt-name stripped iter-expr)))]))]))])]
                     [else #f]))]
             [else #f])]
          [else #f]))

      ;; map-arg->expr: peel the leading Expression out of a fold's
      ;; Map-Argument node. The Map-Argument shape is `(expr type type^)`
      ;; per langs.ss's Ltypescript Map-Argument definition. Returns the
      ;; inner Expression, or #f if the node isn't recognised.
      (define (map-arg->expr m)
        (nanopass-case (Ltypescript Map-Argument) m
          [(,expr ,type ,type^) expr]
          [else #f]))

      ;; iterable-expr->literals: returns a list of N literal Expression
      ;; nodes when `expr` is a statically-extractable iterable, #f
      ;; otherwise. Recognises two shapes:
      ;;
      ;;   (vector src (single src lit) ...)   ; user-written `[1,2,3]`
      ;;   (tuple  src (single src lit) ...)   ; synthesised by
      ;;                                       ; expand-modules-and-types
      ;;                                       ; for range loops
      ;;                                       ; `for (const i of N..M)`
      ;;                                       ; (analysis-passes.ss:1257-1258)
      ;;
      ;; Every element must be a `(quote src datum)` with an exact-
      ;; integer datum (optionally wrapped in safe-cast layers, peeled
      ;; by `tuple-arg->literal`).
      ;;
      ;; The returned list is in iteration order: the i-th element is
      ;; what `elt-name` binds to during iteration i.
      (define (iterable-expr->literals expr)
        (let ([e (expr-strip-cast expr)])
          (define (peel-tuple-args xs)
            (let loop ([xs xs] [acc '()])
              (cond
                [(null? xs) (reverse acc)]
                [else
                 (let ([elt (tuple-arg->literal (car xs))])
                   (and elt (loop (cdr xs) (cons elt acc))))])))
          (nanopass-case (Ltypescript Expression) e
            [(vector ,src ,tuple-arg* ...) (peel-tuple-args tuple-arg*)]
            [(tuple ,src ,tuple-arg* ...) (peel-tuple-args tuple-arg*)]
            [else #f])))

      ;; tuple-arg->literal: peel a `(single src expr)` Tuple-Argument
      ;; and return the inner Expression iff it strips down to a
      ;; `(quote src <int>)` literal. Spread args (`(spread src nat
      ;; expr)`) are rejected — Iter 6 only handles flat array
      ;; literals. Returns the original Expression (with casts intact)
      ;; on success, #f otherwise.
      (define (tuple-arg->literal t)
        (nanopass-case (Ltypescript Tuple-Argument) t
          [(single ,src ,expr)
           (let ([stripped (expr-strip-cast expr)])
             (nanopass-case (Ltypescript Expression) stripped
               [(quote ,src ,datum)
                (and (integer? datum) (exact? datum) expr)]
               [else #f]))]
          [else #f]))

      ;; expr-subst-var-ref: walk an Expression and replace every
      ;; `(var-ref src target-name)` with `replacement` (also an
      ;; Expression). Recurses through safe-cast layers, leaves all
      ;; other shapes alone — the Iter 6 MVP only needs to handle the
      ;; loop variable appearing directly (or under a safe-cast) as a
      ;; top-level `c.increment(x)` arg.
      ;;
      ;; Returns the (possibly identical) Expression. Used by
      ;; emit-for-iter-terminal to specialise the body's expr-list
      ;; per iteration before feeding it to compute-pl-builder-lines.
      (define (expr-subst-var-ref e target-name replacement)
        (nanopass-case (Ltypescript Expression) e
          [(var-ref ,src ,var-name)
           (if (eq? (id-sym var-name) (id-sym target-name))
               replacement
               e)]
          [(safe-cast ,src ,type ,type^ ,expr)
           (let ([sub (expr-subst-var-ref expr target-name replacement)])
             (with-output-language (Ltypescript Expression)
               `(safe-cast ,src ,type ,type^ ,sub)))]
          [(+ ,src ,type ,expr1 ,expr2)
           ;; Iter 7 follow-up: substitute through arithmetic so non-identity
           ;; lambda bodies like `(x + 1) as Uint<N>` survive the
           ;; render-map-mvp per-element specialisation. 0.33 replaced the
           ;; `mbits` slot with the result `Type`; it is threaded through
           ;; unchanged, exactly as `mbits` was.
           (let ([sub1 (expr-subst-var-ref expr1 target-name replacement)]
                 [sub2 (expr-subst-var-ref expr2 target-name replacement)])
             (with-output-language (Ltypescript Expression)
               `(+ ,src ,type ,sub1 ,sub2)))]
          [(- ,src ,type ,expr1 ,expr2)
           (let ([sub1 (expr-subst-var-ref expr1 target-name replacement)]
                 [sub2 (expr-subst-var-ref expr2 target-name replacement)])
             (with-output-language (Ltypescript Expression)
               `(- ,src ,type ,sub1 ,sub2)))]
          [(* ,src ,type ,expr1 ,expr2)
           (let ([sub1 (expr-subst-var-ref expr1 target-name replacement)]
                 [sub2 (expr-subst-var-ref expr2 target-name replacement)])
             (with-output-language (Ltypescript Expression)
               `(* ,src ,type ,sub1 ,sub2)))]
          [(downcast-unsigned ,src ,nat2 ,nat1 ,expr)
           (let ([sub (expr-subst-var-ref expr target-name replacement)])
             (with-output-language (Ltypescript Expression)
               `(downcast-unsigned ,src ,nat2 ,nat1 ,sub)))]
          [(cast-from-field ,src ,nat ,ftype ,expr)
           ;; 0.33: split out of downcast-unsigned's `nat? = #f` case;
           ;; substitute through it the same way.
           (let ([sub (expr-subst-var-ref expr target-name replacement)])
             (with-output-language (Ltypescript Expression)
               `(cast-from-field ,src ,nat ,ftype ,sub)))]
          [(cast-to-field ,src ,ftype ,type ,expr)
           (let ([sub (expr-subst-var-ref expr target-name replacement)])
             (with-output-language (Ltypescript Expression)
               `(cast-to-field ,src ,ftype ,type ,sub)))]
          [else e]))

      ;; fold-body-strip-acc-return: given a fold body (Statement) and
      ;; the accumulator's var-name, peel off a trailing
      ;; `(statement-expression (var-ref acc-name))` that the desugar
      ;; emits to thread the accumulator through unchanged. Returns the
      ;; body Statement with that tail removed, or #f if no such tail.
      ;; The returned Statement is what we feed to branch->single-pl-call
      ;; to extract the side-effecting public-ledger call.
      (define (fold-body-strip-acc-return stmt acc-name)
        ;; Extract the outer stmt's src once; we reuse it as the
        ;; rebuilt-seq's src below. The `seq` form's src field is
        ;; constructor-validated as source-object?, so passing #f
        ;; would error at IR-construction time.
        (let ([outer-src
               (nanopass-case (Ltypescript Statement) stmt
                 [(seq ,src ,stmt* ... ,stmt^) src]
                 [else #f])]
              [flat (stmt-flatten stmt)])
          (cond
            [(null? flat) #f]
            [else
             ;; Last element should be a statement-expression wrapping a
             ;; var-ref to acc-name. Drop it and rebuild a seq from the
             ;; remaining stmts.
             (let ([rev (reverse flat)])
               (let ([tail (car rev)]
                     [rest (reverse (cdr rev))])
                 (and (stmt-is-var-ref? tail acc-name)
                      (cond
                        [(null? rest) #f]
                        [(null? (cdr rest)) (car rest)]
                        [(not outer-src) #f]
                        [else
                         ;; rest = (stmt0 stmt1 ... stmtN). seq's shape is
                         ;; (seq src stmt* ... stmt) — last in tail
                         ;; position.
                         (let ([rest-rev (reverse rest)])
                           (let ([last-stmt (car rest-rev)]
                                 [stmt* (reverse (cdr rest-rev))])
                             (with-output-language (Ltypescript Statement)
                               `(seq ,outer-src ,stmt* ... ,last-stmt))))]))))])))

      ;; stmt-is-var-ref?: detect `(statement-expression (var-ref name))`
      ;; matching `target-name`.
      (define (stmt-is-var-ref? stmt target-name)
        (nanopass-case (Ltypescript Statement) stmt
          [(statement-expression ,expr)
           (nanopass-case (Ltypescript Expression) expr
             [(var-ref ,src ,var-name)
              (eq? (id-sym var-name) (id-sym target-name))]
             [else #f])]
          [else #f]))
