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
;;; This file: statement shape extraction and const-binding analysis.

      ;; stmt->if-expression-body: detect the I3b/4 "single if-expression body"
      ;; shape — a flat statement sequence whose only element is
      ;; `(if cond then-stmt else-stmt)`, where each branch is a
      ;; `(statement-expression expr)` carrying a single expression
      ;; representing the circuit's return value. Returns
      ;;   (list cond-expr then-expr else-expr)
      ;; on success, #f otherwise.
      (define (stmt->if-expression-body stmt)
        (let ([stmts (stmt-flatten stmt)])
          (cond
            [(or (null? stmts) (not (null? (cdr stmts)))) #f]
            [else
             (nanopass-case (Ltypescript Statement) (car stmts)
               [(if ,src ,expr0 ,stmt1 ,stmt2)
                (let ([then-expr (stmt->return-expr stmt1)]
                      [else-expr (stmt->return-expr stmt2)])
                  (and then-expr else-expr (list expr0 then-expr else-expr)))]
               [else #f])])))

      ;; stmt->if-chain-body: A19 generalisation of stmt->if-expression-body.
      ;; Admits non-unit-returning impure-circuit bodies of shape:
      ;;   (a) single statement-expression       — `return expr;`
      ;;       (verificationMethodExists)
      ;;   (b) if/else-if/.../else chain         — `if (..) return ... else if ... else return ...;`
      ;;       (tiny.get)
      ;;   (c) if/else-if/...; trailing return    — `if ... else if ...; return default;`
      ;;       (verificationMethodRelationMember)
      ;; Returns
      ;;   (list arms else-expr)
      ;; where arms = ((cond-expr then-expr) ...) (possibly empty) and
      ;; else-expr is always present. Caller dispatches via
      ;; emit-if-chain-body. Returns #f on structural mismatch.
      (define (stmt->if-chain-body stmt)
        (let ([stmts (stmt-flatten stmt)])
          (cond
            [(null? stmts) #f]
            [(null? (cdr stmts))
             ;; Single stmt: either a top-level if (recurse into chain) or
             ;; a statement-expression (no arms, just the expr).
             (nanopass-case (Ltypescript Statement) (car stmts)
               [(if ,src ,expr0 ,stmt1 ,stmt2)
                (collect-if-chain (car stmts) '())]
               [(statement-expression ,expr) (list '() expr)]
               [else #f])]
            [(and (pair? (cdr stmts))
                  (null? (cddr stmts)))
             ;; Two stmts: an if (without final else) plus a trailing
             ;; return-expression. The trailing expr becomes the chain's
             ;; else.
             (let ([trailing (stmt->return-expr (cadr stmts))])
               (and trailing
                    (nanopass-case (Ltypescript Statement) (car stmts)
                      [(if ,src ,expr0 ,stmt1 ,stmt2)
                       (collect-if-chain (car stmts) (list trailing))]
                      [else #f])))]
            [else #f])))

      ;; collect-if-chain: walk a (possibly nested) if-then-else statement,
      ;; collecting (cond, then-expr) pairs and unwinding into the final
      ;; else-expr. `trailing-else` is the override else-expr to use when
      ;; the innermost if has no else of its own (case (c) above).
      (define (collect-if-chain stmt trailing-else)
        (let loop ([s stmt] [arms '()])
          (nanopass-case (Ltypescript Statement) s
            [(if ,src ,expr0 ,stmt1 ,stmt2)
             (let ([then-expr (stmt->return-expr stmt1)])
               (and then-expr
                    (let ([else-stmts (stmt-flatten stmt2)])
                      (cond
                        ;; else is itself a nested if: extend the chain.
                        [(and (pair? else-stmts) (null? (cdr else-stmts))
                              (nanopass-case (Ltypescript Statement) (car else-stmts)
                                [(if ,src ,expr0 ,stmt1 ,stmt2) #t]
                                [else #f]))
                         (loop (car else-stmts) (cons (list expr0 then-expr) arms))]
                        ;; else is a no-op `(tuple)` AND we have a trailing
                        ;; override: use the trailing as else.
                        [(and (pair? trailing-else)
                              (null? else-stmts))
                         (list (reverse (cons (list expr0 then-expr) arms))
                               (car trailing-else))]
                        ;; otherwise else is a return-expr.
                        [else
                         (let ([else-expr (stmt->return-expr stmt2)])
                           (and else-expr
                                (list (reverse (cons (list expr0 then-expr) arms))
                                      else-expr)))]))))]
            [else #f])))

      ;; stmt->return-expr: pull the single return expression out of a
      ;; branch Statement. The branches of a return-value if-expression
      ;; come out of typescript-passes lowering as a single
      ;; `(statement-expression expr)` (possibly wrapped in a `seq` with
      ;; a trailing unit tuple, which stmt-flatten strips). Returns the
      ;; expression on success, #f otherwise.
      (define (stmt->return-expr stmt)
        (let ([stmts (stmt-flatten stmt)])
          (cond
            [(or (null? stmts) (not (null? (cdr stmts)))) #f]
            [else
             (nanopass-case (Ltypescript Statement) (car stmts)
               [(statement-expression ,expr) expr]
               [else #f])])))

      ;; stmt-flatten: collapse nested `seq`s and trailing-`(tuple)` unit
      ;; statements into a flat list of leaf Statements. The unit
      ;; `(statement-expression (tuple src))` at the end of a `seq` is
      ;; pure (returns ()), so dropping it preserves semantics for our
      ;; void-returning circuits. Any other shape is left alone — callers
      ;; treat unexpected leaves as a non-match and fall back.
      ;; lift-seq-prefix-exprs: walk an Expression, find any (seq es ... e)
      ;; nodes in interior positions, collect the prefix `es` as lifted
      ;; assignment-statements, and return two values:
      ;;   (values lifted-stmts cleaned-expr)
      ;; where cleaned-expr has every seq replaced by just its trailing
      ;; expression. Lifting is order-preserving (left-to-right traversal)
      ;; so dependent assignments stay in order.
      ;;
      ;; This is what enables the streaming walker to see `let %tmp = ...;`
      ;; lifted out of complex assert conditions and similar nested
      ;; expressions.
      (define (lift-seq-prefix-exprs expr)
        (define lifted '())
        (define (push-lifted! e)
          (set! lifted
                (cons (with-output-language (Ltypescript Statement)
                        `(statement-expression ,e))
                      lifted)))
        (define (walk e)
          (nanopass-case (Ltypescript Expression) e
            [(seq ,src ,expr* ... ,expr^)
             (for-each (lambda (ex) (push-lifted! (walk ex))) expr*)
             (walk expr^)]
            [(assert ,src ,expr^ ,mesg)
             (let ([new-cond (walk expr^)])
               (with-output-language (Ltypescript Expression)
                 `(assert ,src ,new-cond ,mesg)))]
            [(not ,src ,expr^)
             (let ([n (walk expr^)])
               (with-output-language (Ltypescript Expression)
                 `(not ,src ,n)))]
            [(and ,src ,expr1 ,expr2)
             (let ([a (walk expr1)]
                   [b (walk expr2)])
               (with-output-language (Ltypescript Expression)
                 `(and ,src ,a ,b)))]
            [(or ,src ,expr1 ,expr2)
             (let ([a (walk expr1)]
                   [b (walk expr2)])
               (with-output-language (Ltypescript Expression)
                 `(or ,src ,a ,b)))]
            [(== ,src ,type ,expr1 ,expr2)
             (let ([a (walk expr1)]
                   [b (walk expr2)])
               (with-output-language (Ltypescript Expression)
                 `(== ,src ,type ,a ,b)))]
            [else e]))
        (let ([cleaned (walk expr)])
          (values (reverse lifted) cleaned)))

      (define (stmt-flatten stmt)
        (nanopass-case (Ltypescript Statement) stmt
          [(seq ,src ,stmt* ... ,stmt^)
           (let ([all (append stmt* (list stmt^))])
             (apply append (map stmt-flatten all)))]
          [(statement-expression ,expr)
           ;; Drop a bare unit `(tuple src)` — common terminal of a `seq`
           ;; for void-returning circuits.
           ;;
           ;; Also lift a `(seq src expr* ... expr^)` Expression out into
           ;; separate statement-expressions, so the streaming walker can
           ;; see each assignment and the final body as flat siblings. This
           ;; is what typescript-passes produces for `let*` lifted out of
           ;; expression contexts (e.g. `disclose(merkleTreePathRoot<...>
           ;; (path))` introducing a temp variable for the inner call).
           (nanopass-case (Ltypescript Expression) expr
             [(tuple ,src ,tuple-arg* ...)
              (if (null? tuple-arg*) '() (list stmt))]
             [(seq ,src ,expr* ... ,expr^)
              (apply append
                (map (lambda (e)
                       (stmt-flatten
                         (with-output-language (Ltypescript Statement)
                           `(statement-expression ,e))))
                     (append expr* (list expr^))))]
             [else
              ;; Try lifting any inner seq-prefix assignments from a
              ;; structured expression (assert / and / or / not / ==). If
              ;; lifting produced any prefix statements, return them
              ;; followed by the cleaned statement; otherwise return the
              ;; original statement unchanged.
              (let-values ([(lifted cleaned) (lift-seq-prefix-exprs expr)])
                (cond
                  [(null? lifted) (list stmt)]
                  [else
                   (let ([new-stmt
                          (with-output-language (Ltypescript Statement)
                            `(statement-expression ,cleaned))])
                     (append
                       (apply append (map stmt-flatten lifted))
                       (list new-stmt)))]))])]
          [else (list stmt)]))

      ;; const-binding?: detect a `(const src local expr)` and pull out
      ;; the binder's var-name and the bound expression. Returns
      ;; (cons var-name expr) on a match, #f otherwise.
      (define (const-binding stmt)
        (nanopass-case (Ltypescript Statement) stmt
          [(const ,src ,local ,expr)
           (nanopass-case (Ltypescript Argument) local
             [(,var-name ,type) (cons var-name expr)])]
          [else #f]))

      ;; const-binding-decl-type: like const-binding, but returns the
      ;; binder's declared Type (or #f if `stmt` isn't a const-binding).
      ;; Used by Prod-9 to detect Field-typed integer literals at RHS
      ;; (e.g. `const tmp_0: Field = 42n;`) so the emitter can wrap them
      ;; in `Fr::from(<n>u64)` instead of leaving a bare i32 literal that
      ;; later fails the `Into<AlignedValue>` bound at the ledger-write
      ;; builder call site.
      (define (const-binding-decl-type stmt)
        (nanopass-case (Ltypescript Statement) stmt
          [(const ,src ,local ,expr)
           (nanopass-case (Ltypescript Argument) local
             [(,var-name ,type) type])]
          [else #f]))

      ;; literal-int-expr?: returns the integer datum when `expr` strips
      ;; (through safe-cast layers) down to a `(quote ... <int>)` literal,
      ;; or #f otherwise.
      (define (literal-int-expr? expr)
        (let ([e (expr-strip-cast expr)])
          (nanopass-case (Ltypescript Expression) e
            [(quote ,src ,datum)
             (and (integer? datum) (exact? datum) datum)]
            [else #f])))

      ;; type-is-tfield?: is `type` a (tfield ...)? Peels nominal/transparent
      ;; talias layers so `type Foo = Field;` also matches. Matches every
      ;; Field-Type qualifier (native Fr, JubjubScalar, zkir-v3 fields);
      ;; use `type-tfield-ftype` when the qualifier matters.
      (define (type-is-tfield? type)
        (and (type-tfield-ftype type) #t))

      ;; type-tfield-ftype: the Field-Type qualifier of a (tfield ...)
      ;; (through talias layers), or #f when `type` isn't a tfield.
      (define (type-tfield-ftype type)
        (and type
             (nanopass-case (Ltypescript Type) type
               [(tfield ,src ,ftype) ftype]
               [(talias ,src ,nominal? ,type-name ,type^)
                (type-tfield-ftype type^)]
               [else #f])))

      ;; type-peel-tunsigned: if `type` is a (tunsigned src nat) (possibly
      ;; through a talias chain), return the `nat` upper bound; otherwise #f.
      ;; Companion to `type-is-tfield?` for Prod-13's Uint<N> literal path.
      (define (type-peel-tunsigned type)
        (and type
             (nanopass-case (Ltypescript Type) type
               [(tunsigned ,src ,nat) nat]
               [(talias ,src ,nominal? ,type-name ,type^)
                (type-peel-tunsigned type^)]
               [else #f])))

      ;; uniquify-rust-name: Prod-14 — Compact's frontend lowering produces
      ;; per-statement `const tmp = ...; <ledger> = tmp;` shapes, so a
      ;; constructor body like
      ;;     admin = 42;
      ;;     count = 7;
      ;; lowers into TWO const-bindings both named `tmp`. Each binding maps
      ;; to a distinct id (and `local-binds` keys are eq-compared), but the
      ;; emitted Rust shares the same `let tmp = ...` string and shadows.
      ;; By the time the OpProgram builder consumes each `new_cell(<rust-name>.
      ;; clone())`, only the LAST `let tmp = …` is in scope, so every cell
      ;; reads the wrong value.
      ;;
      ;; Suffix the proposed name with `_N` until it doesn't collide with
      ;; any prior `local-binds` entry's rust-name. First occurrence keeps
      ;; the unsuffixed form to preserve existing snapshots (most fixtures
      ;; have at most one literal-RHS slot per body).
      (define (uniquify-rust-name proposed local-binds)
        (let ([taken (map cdr local-binds)])
          (cond
            [(not (member proposed taken)) proposed]
            [else
             (let loop ([n 0])
               (let ([candidate (format "~a_~a" proposed n)])
                 (cond
                   [(member candidate taken) (loop (fx+ n 1))]
                   [else candidate])))])))

      ;; coerce-literal-rhs-rendered: Prod-9/Prod-13 — typed integer literals
      ;; need to be rendered with the correct Rust type so the ledger-write
      ;; builder's `Into<AlignedValue>` bound is satisfied.
      ;;   - `tfield`: wrap as `Fr::from(<n>u64)`.
      ;;   - `tunsigned`: append the width suffix (`<n>u8` .. `<n>u128`) so
      ;;     the literal types as the same Rust primitive the ledger field
      ;;     uses. Without this, `ledger v: Uint<64>; v = 42;` lowered into
      ;;     `let tmp = 42; new_cell(tmp.clone())` — the bare i32 fails the
      ;;     `Into<AlignedValue>` bound (Prod-13).
      ;; All other RHS shapes return #f so the caller falls back to its
      ;; existing rendering.
      (define (coerce-literal-rhs-rendered decl-type rhs)
        (cond
          [(type-tfield-ftype decl-type) =>
           (lambda (ftype)
             (let ([n (literal-int-expr? rhs)])
               ;; `Fr::from(<n>u64)` for native Field; the JubjubScalar
               ;; builtin (EmbeddedFr) has the same `From<u64>` surface.
               (and n (format "~a::from(~au64)" (field-type-rust ftype) n))))]
          [(type-peel-tunsigned decl-type) =>
           (lambda (nat)
             (let ([n (literal-int-expr? rhs)])
               (and n (format "~a~a" n (uint-rust-width nat)))))]
          [else #f]))

      ;; const-decl-only?: detect a `(const ,src (,local* ...))` Statement —
      ;; the "forward declaration" form produced by typescript-passes when a
      ;; `let* ([%tmp ...])` is lifted out of an expression context. These
      ;; carry no initializer; the actual assignment lands later as a
      ;; `(statement-expression (= ,src ,var-name ,expr))`. In the Rust
      ;; emission this is a no-op — the eventual `(= ...)` becomes a
      ;; plain `let <name> = <expr>;`. Returns #t on match, #f otherwise.
      (define (const-decl-only? stmt)
        (nanopass-case (Ltypescript Statement) stmt
          [(const ,src (,local* ...)) #t]
          [else #f]))

      ;; stmt->assignment: detect a `(statement-expression (= src var-name
      ;; expr))` and return (cons var-name expr). The assignment Expression
      ;; is what typescript-passes emits for `<name> = <expr>` after lifting
      ;; a let* temp out of a containing expression — see lifted-temp
      ;; comments above. Returns #f for anything else.
      (define (stmt->assignment stmt)
        (nanopass-case (Ltypescript Statement) stmt
          [(statement-expression ,expr)
           (nanopass-case (Ltypescript Expression) expr
             [(= ,src ,var-name ,expr^) (cons var-name expr^)]
             [else #f])]
          [else #f]))

      ;; expr-strip-cast: peel safe-cast layers from an Expression. The
      ;; typechecker inserts `safe-cast` to widen the source literal
      ;; (e.g. `1: Uint<1>`) up to the ADT op's declared parameter type
      ;; (e.g. `Uint16` for `Counter.increment`). For literal-int
      ;; arguments the cast is value-preserving, so we look through it
      ;; before extracting the literal.
      (define (expr-strip-cast expr)
        (nanopass-case (Ltypescript Expression) expr
          [(safe-cast ,src ,type ,type^ ,expr^) (expr-strip-cast expr^)]
          [else expr]))

      ;; expr-resolve: chase a `var-ref` through the local-binding alist
      ;; built from preceding `const` statements, then strip any
      ;; cast layers. Returns the underlying Expression or #f if the
      ;; chain hits something we don't recognise.
      (define (expr-resolve expr binds)
        ;; Pass through unknown var-refs unchanged. Originally this
        ;; returned #f to signal "unresolvable", but that prevented
        ;; legitimate Iter 6 use cases (the fold loop variable is bound
        ;; outside `binds` and substituted per-iteration by
        ;; emit-for-iter-terminal). Downstream callers (e.g.
        ;; branch->single-pl-call, stmt->single-public-ledger-call)
        ;; still test for #f via `(memv #f resolved)`; with this change
        ;; that test never fires for var-refs alone. Other shapes (e.g.
        ;; an unsupported expression form via expr-strip-cast) still
        ;; fall through to the `[else e]` arm — they keep their original
        ;; representation, and downstream emission rejects them through
        ;; `expr-supported?` / vm-code expansion rather than via #f here.
        (let ([e (expr-strip-cast expr)])
          (nanopass-case (Ltypescript Expression) e
            [(var-ref ,src ,var-name)
             (cond
               [(assq var-name binds) =>
                (lambda (p) (expr-resolve (cdr p) binds))]
               [else e])]
            [else e])))

      ;; stmt->single-public-ledger-call: detect the narrow I3a shape —
      ;; a flat statement sequence consisting of zero or more `const`
      ;; bindings followed by exactly one `(public-ledger ...)` call
      ;; (e.g. counter.compact's `round.increment(1);` which the
      ;; frontend lowers to `const tmp = safe-cast 1; round.increment(tmp);`).
      ;; On a match returns
      ;;   (list path-elt* adt-op resolved-expr*)
      ;; where each resolved-expr has had var-refs chased through the
      ;; const-binding alist and surrounding safe-casts peeled. Returns
      ;; #f for anything we don't yet support.
      (define (stmt->single-public-ledger-call stmt)
        (let loop ([stmts (stmt-flatten stmt)] [binds '()])
          (cond
            [(null? stmts) #f]
            [(const-binding (car stmts)) =>
             (lambda (b) (loop (cdr stmts) (cons b binds)))]
            [else
             (and
               ;; Exactly one terminal statement-expression.
               (null? (cdr stmts))
               (nanopass-case (Ltypescript Statement) (car stmts)
                 [(statement-expression ,expr)
                  (nanopass-case (Ltypescript Expression) expr
                    [(public-ledger ,src ,ledger-field-name ,sugar? (,path-elt* ...) ,src^ ,adt-op ,expr* ...)
                     (let ([resolved (map (lambda (e) (expr-resolve e binds)) expr*)])
                       (if (memv #f resolved)
                           #f
                           (list path-elt* adt-op resolved)))]
                    [else #f])]
                 [else #f]))])))
