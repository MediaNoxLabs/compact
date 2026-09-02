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
;;; This file: pure circuit bodies.

      ;; stmt-pure-body-rust: render the body of a pure circuit as a Rust
      ;; block (a sequence of `let`-bindings / statements terminated by a
      ;; tail expression, or a single expression in tail position).
      ;;   - `(const src local expr)` and lifted `(statement-expression
      ;;     (= src var expr))` bindings become `let <name> = <rhs>;` and
      ;;     thread into `local-binds` so later references resolve.
      ;;   - `(const src (local* ...))` forward declarations are no-ops
      ;;     (the assignment lands later as a `(=)`).
      ;;   - `if/else`, `assert`, and tail `statement-expression` forms
      ;;     render via pure-stmt-rust.
      ;; `local-binds` is an alist `(var-name . rust-name)` threaded
      ;; through the statement walk and bound to `current-var-substitution`
      ;; around expression rendering so expr-rust resolves locals.
      ;; `witness-id-ht` / `circuit-id-ht` are passed to cond-rust so
      ;; if/assert conditions that call user pure circuits route to
      ;; `pure_circuits::...` (and witness calls, if any, resolve).
      ;; Returns the Rust source string, or #f to signal the caller
      ;; should fall back to `unimplemented!()`.
      ;;
      ;; pure-stmt-rust: render one VALUE-producing statement of a pure
      ;; circuit body (if / assert / tail expression). Const-bindings and
      ;; lifted assignments are intercepted by stmt-pure-body-rust's loop
      ;; before reaching here. Returns a Rust source string (terminated by
      ;; `;` for non-tail statements, no terminator for a tail
      ;; expression), or #f on unsupported shapes.
      (define (pure-stmt-rust stmt native-id-ht witness-id-ht circuit-id-ht
                               local-binds last?)
        (parameterize ([current-var-substitution local-binds])
        (nanopass-case (Ltypescript Statement) stmt
          [(if ,src ,expr0 ,stmt1 ,stmt2)
           (let ([cond-str (guard (c [#t #f])
                             (cond-rust expr0 local-binds
                                        native-id-ht
                                        witness-id-ht
                                        circuit-id-ht))]
                 [then-str (stmt-pure-body-rust stmt1 native-id-ht
                                                witness-id-ht circuit-id-ht
                                                local-binds #f)]
                 [else-str (stmt-pure-body-rust stmt2 native-id-ht
                                                witness-id-ht circuit-id-ht
                                                local-binds #f)])
             (cond
               [(or (not cond-str) (not then-str) (not else-str)) #f]
               [(or (rendered-has-todo? cond-str)
                    (rendered-has-todo? then-str)
                    (rendered-has-todo? else-str)) #f]
               [else
                (format "if ~a {\n            ~a\n        } else {\n            ~a\n        }"
                        cond-str then-str else-str)]))]
          [(if ,src ,expr0 ,stmt1)
           (let ([cond-str (guard (c [#t #f])
                             (cond-rust expr0 local-binds
                                        native-id-ht
                                        witness-id-ht
                                        circuit-id-ht))]
                 [then-str (stmt-pure-body-rust stmt1 native-id-ht
                                                witness-id-ht circuit-id-ht
                                                local-binds #f)])
             (cond
               [(or (not cond-str) (not then-str)) #f]
               [(or (rendered-has-todo? cond-str)
                    (rendered-has-todo? then-str)) #f]
               [else
                (format "if ~a {\n            ~a\n        }"
                        cond-str then-str)]))]
          [(statement-expression ,expr)
           (nanopass-case (Ltypescript Expression) expr
             [(assert ,src ,expr0 ,mesg)
              (let ([cond-str
                     (guard (c [#t #f])
                       (cond-rust expr0 local-binds
                                  native-id-ht
                                  witness-id-ht
                                  circuit-id-ht))])
                (cond
                  [(or (not cond-str) (rendered-has-todo? cond-str)) #f]
                  [else
                   (format "compact_assert!(~a, ~s);" cond-str
                           (if (string? mesg) mesg ""))]))]
             [else
              (let ([s (guard (c [#t #f]) (expr-rust expr native-id-ht))])
                (cond
                  [(or (not s) (rendered-has-todo? s)) #f]
                  [last? s]
                  [else (string-append s ";")]))])]
          [else #f])))

      ;; wrap-ok?: when #t (the default, used by emit-pure-circuit) the
      ;; rendered body's tail is wrapped as `Ok(...)` so the block is a
      ;; `Result<_, CompactError>`; assert-only / unit bodies end with
      ;; `Ok(())`. Recursion into if-branches (pure-stmt-rust's if arms)
      ;; passes #f so the branches are bare expressions — the enclosing
      ;; `if` expression is then wrapped by the top-level call.
      (define (stmt-pure-body-rust stmt native-id-ht witness-id-ht circuit-id-ht
                                   local-binds . maybe-wrap-ok?)
        (let ([wrap-ok? (or (null? maybe-wrap-ok?) (car maybe-wrap-ok?))])
          (let ([stmts (stmt-flatten stmt)])
          (let loop ([xs stmts] [binds local-binds] [acc '()])
            (cond
              [(null? xs)
               ;; A body of only declarations (no value-statement) — either
               ;; a truly empty body (assertValidSchemaCapabilities: a
               ;; placeholder pure circuit with only comments) or a
               ;; unit-returning circuit whose every stmt is a const /
               ;; assignment / decl-only. Emit the accumulated `let;` lines
               ;; and a trailing `()` so the block type-checks as `()`; a
               ;; truly empty body emits just `()`.
               (let ([joined
                      (let join ([rs (reverse acc)] [out ""])
                        (cond
                          [(null? rs) out]
                          [(null? (cdr rs)) (string-append out (car rs))]
                          [else (join (cdr rs) (string-append out (car rs) "\n        "))]))])
                 (let ([unit-tail (if wrap-ok? "Ok(())" "()")])
                   (cond
                     [(string=? joined "") unit-tail]
                     [else (string-append joined "\n        " unit-tail)])))]
              [(const-binding (car xs))
               =>
               (lambda (b)
                 (let* ([var-name (car b)]
                        [rhs (cdr b)]
                        [proposed (symbol->string (camel->snake (id-sym var-name)))]
                        [rust-name (uniquify-rust-name proposed binds)]
                        ;; G1: reserve rust-name while rendering the RHS so a
                        ;; temp lifted from inside it (seq-stmt-rust's `(=)`
                        ;; clause) uniquifies instead of shadowing this
                        ;; binder. The RHS can't reference var-name itself, so
                        ;; adding the entry early only affects name selection.
                        [rhs-binds (cons (cons var-name rust-name) binds)])
                   (let ([s (guard (c [#t #f])
                              (parameterize ([current-var-substitution rhs-binds])
                                (expr-rust rhs native-id-ht)))])
                     (cond
                       [(or (not s) (rendered-has-todo? s)) #f]
                       [else
                        (loop (cdr xs)
                              (cons (cons var-name rust-name) binds)
                              (cons (format "let ~a = ~a;" rust-name s) acc))]))))]
              [(const-decl-only? (car xs))
               ;; Forward declaration `(const src (local* ...))` — no-op;
               ;; the assignment lands later as a `(statement-expression
               ;; (= ...))` which the next clause renders as `let`.
               (loop (cdr xs) binds acc)]
              [(stmt->assignment (car xs))
               =>
               (lambda (a)
                 (let* ([var-name (car a)]
                        [rhs (cdr a)]
                        [proposed (symbol->string (camel->snake (id-sym var-name)))]
                        [rust-name (uniquify-rust-name proposed binds)]
                        ;; G1: see the const-binding clause above — reserve
                        ;; rust-name for the duration of the RHS render.
                        [rhs-binds (cons (cons var-name rust-name) binds)])
                   (let ([s (guard (c [#t #f])
                              (parameterize ([current-var-substitution rhs-binds])
                                (expr-rust rhs native-id-ht)))])
                     (cond
                       [(or (not s) (rendered-has-todo? s)) #f]
                       [else
                        (loop (cdr xs)
                              (cons (cons var-name rust-name) binds)
                              (cons (format "let ~a = ~a;" rust-name s) acc))]))))]
              [else
               ;; A value-producing statement. The last one renders as a
               ;; tail expression (no `;`); earlier ones as `<expr>;`.
               (let ([last? (null? (cdr xs))])
                 (let ([s (pure-stmt-rust (car xs) native-id-ht
                                           witness-id-ht circuit-id-ht
                                           binds last?)])
                   (cond
                     [(not s) #f]
                     [last?
                      ;; tail element. `s` is either a value expression
                      ;; (rendered without `;`) or an assert statement
                      ;; (`compact_assert!(...);`, ends with `;`). When
                      ;; wrap-ok?: wrap a value tail in `Ok(...)`, and
                      ;; follow an assert tail with `Ok(())` so the block
                      ;; yields `Result<_, CompactError>`.
                      (let* ([n (string-length s)]
                             [value-tail?
                              (or (= n 0)
                                  (not (char=? (string-ref s (- n 1)) #\;)))])
                        (let ([acc
                               (cond
                                 [(and wrap-ok? value-tail?)
                                  (cons (format "Ok(~a)" s) acc)]
                                 [(and wrap-ok? (not value-tail?))
                                  (cons "Ok(())" (cons s acc))]
                                 [else (cons s acc)])])
                          (let join ([rs (reverse acc)] [out ""])
                            (cond
                              [(null? rs) out]
                              [(null? (cdr rs)) (string-append out (car rs))]
                              [else (join (cdr rs) (string-append out (car rs) "\n        "))]))))]
                     [else (loop (cdr xs) binds (cons s acc))])))])))))

      ;; emit-pure-circuit: emit a pure circuit as a free function inside
      ;; `mod pure_circuits`. No ctx — just the declared args and a direct
      ;; return type. For the narrow tiny.compact-style shape (a single
      ;; expression in statement position) we render the expression
      ;; directly; richer bodies (sequenced asserts / tail calls to other
      ;; user pure circuits / if-guarded asserts / const bindings) are
      ;; walked by stmt-pure-body-rust. Anything the walker can't handle
      ;; keeps an `unimplemented!()` placeholder via rust-feature-error.
      ;;
      ;; Exported circuits become `pub fn` (part of the crate's public
      ;; surface). Non-exported user pure circuits (e.g. zerocash's
      ;; `commitment_from_coin_info`, `derive_nullifier`) become
      ;; `pub(crate) fn` — callable from anywhere in the generated
      ;; contract crate (notably impure-circuit bodies via
      ;; `pure_circuits::foo(...)`) but not part of the contract's
      ;; downstream API.
      ;;
      ;; current-circuit-id-ht / current-witness-id-ht are parameterised
      ;; around the body so expr-rust/call-rust (which don't take the id
      ;; hashtables explicitly) can route calls to user pure circuits as
      ;; `pure_circuits::<snake>(...)`.
      (define (emit-pure-circuit cdefn native-id-ht witness-id-ht circuit-id-ht)
        (nanopass-case (Ltypescript Program-Element) cdefn
          [(circuit ,src ,function-name (,arg* ...) ,type ,stmt)
           (out (format "    ~a fn ~a("
                        (if (id-exported? function-name) "pub" "pub(crate)")
                        (id->rust-name function-name)))
           (let loop ([arg* arg*] [first? #t])
             (cond
               [(null? arg*) (void)]
               [else
                (nanopass-case (Ltypescript Argument) (car arg*)
                  [(,var-name ,type)
                   (out (format "~a~a: ~a"
                                (if first? "" ", ")
                                (camel->snake (id-sym var-name))
                                (type-rust type)))])
                (loop (cdr arg*) #f)]))
           (out (format ") -> Result<~a, CompactError> {\n" (type-rust type)))
           (parameterize ([current-formal-arg-types (build-formal-arg-type-ht arg*)]
                          [current-circuit-id-ht circuit-id-ht]
                          [current-witness-id-ht witness-id-ht])
             (let ([body (stmt-pure-body-rust stmt native-id-ht
                                              witness-id-ht circuit-id-ht '()
                                              #t)])
               (cond
                 [body (out (format "        ~a\n" body))]
                 [else
                  (rust-feature-error src 'pure-circuit-body-emission
                    "no walker shape matched pure circuit body for ~a"
                    (id-sym function-name))])))
           (out "    }\n\n")]))
