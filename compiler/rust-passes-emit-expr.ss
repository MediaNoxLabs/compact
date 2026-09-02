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
;;; This file: the expression renderer.

      (define (expr-rust expr native-id-ht)
        (nanopass-case (Ltypescript Expression) expr
          [(safe-cast ,src ,type ,type^ ,expr^)
           ;; Iter 7: peel safe-cast layers transparently. The IR uses
           ;; safe-cast to widen literals (e.g. `1: Uint<1>` → `Uint<64>`)
           ;; inside tuple/vector arguments and map iterables. For our
           ;; rendering purposes the cast is value-preserving — the
           ;; underlying integer literal carries the right Rust integer
           ;; type once we ascribe it at the surrounding context (or
           ;; rely on Rust's inference from the array element type).
           ;; Mirrors `expr-strip-cast` but for the rendering path.
           (expr-rust expr^ native-id-ht)]
          [(quote ,src ,datum)
           (cond
             [(bytevector? datum) (bytevector->rust-array-literal datum)]
             [(boolean? datum) (if datum "true" "false")]
             [(and (integer? datum) (exact? datum)) (format "~a" datum)]
             [else (rust-feature-error src 'quote-variant
                     "unsupported quote datum: ~s" datum)])]
          [(var-ref ,src ,var-name)
           ;; Bug-1 fix: consult current-var-substitution before falling
           ;; back to the default snake-case rendering. ctor-expr-rust
           ;; dynamically binds the parameter from its local-binds so
           ;; inlined sub-expressions reaching this path (via
           ;; emit-ledger-read-expr → expr->vm-value → expr-rust) still
           ;; resolve formal→actual substitutions from inline-circuit-call.
           (cond
             [(assq var-name (current-var-substitution)) => cdr]
             [else (symbol->string (camel->snake (id-sym var-name)))])]
          [(tuple ,src ,tuple-arg* ...)
           ;; Compact's `Vector<N, T>` lowers to a Rust `[T; N]`. The IR
           ;; uses `tuple` for both tuples and vectors at the value level;
           ;; the immediate caller (e.g. a native taking a Vector) knows
           ;; which form is wanted. For I3b/1 we render every `tuple` as a
           ;; Rust array literal — the only consumer is persistent_hash,
           ;; where the elements have identical type ([u8; 32]).
           (let ([parts
                  (map (lambda (ta) (tuple-arg-rust ta native-id-ht))
                       tuple-arg*)])
             (string-append
               "["
               (let join ([xs parts] [acc ""])
                 (cond
                   [(null? xs) acc]
                   [(null? (cdr xs)) (string-append acc (car xs))]
                   [else (join (cdr xs) (string-append acc (car xs) ", "))]))
               "]"))]
          [(call ,src ,function-name ,expr* ...)
           (call-rust src function-name expr* native-id-ht)]
          [(default ,src ,type)
           ;; I3b/3: `default<T>` lowers to the type's zero value. Reuses
           ;; the K1 helper so tunsigned/tfield/tbytes/tenum/talias are
           ;; handled consistently with initial_state's per-field seed.
           (default-value-rust type)]
          [(== ,src ,type ,expr1 ,expr2)
           ;; I3b/3: equality comparison. Parenthesised so it composes
           ;; safely inside larger expressions (e.g. inside a Rust assert
           ;; macro call without surrounding parens being implicit).
           (format "(~a == ~a)"
                   (expr-rust expr1 native-id-ht)
                   (expr-rust expr2 native-id-ht))]
          [(not ,src ,expr)
           ;; F1.2: Boolean negation.
           (format "(!(~a))" (expr-rust expr native-id-ht))]
          [(and ,src ,expr1 ,expr2)
           ;; F1.2: short-circuit Boolean AND.
           (format "(~a && ~a)"
                   (expr-rust expr1 native-id-ht)
                   (expr-rust expr2 native-id-ht))]
          [(or ,src ,expr1 ,expr2)
           ;; F1.2: short-circuit Boolean OR.
           (format "(~a || ~a)"
                   (expr-rust expr1 native-id-ht)
                   (expr-rust expr2 native-id-ht))]
          [(elt-ref ,src ,expr ,elt-name ,nat)
           ;; F1.2: struct field access.
           (format "~a.~a"
                   (expr-rust expr native-id-ht)
                   (symbol->string elt-name))]
          [(public-ledger ,src ,ledger-field-name ,sugar? (,path-elt* ...) ,src^ ,adt-op ,expr* ...)
           ;; I3b/3: ledger read in expression position (e.g. inside an
           ;; `(==)` or as the RHS of a const-binding). Emits an inline
           ;; gather query against (current-qctx-ref) and decodes the
           ;; resulting AlignedValue using the same decoder table the
           ;; Ledger view uses. F1.2/2: passes expr* + native-id-ht to
           ;; cover ADT read-with-arg (Set.member etc).
           (emit-ledger-read-expr path-elt* adt-op expr* native-id-ht)]
          [(enum-ref ,src ,type ,elt-name)
           ;; E6: enum variant in expression position (pure circuit body).
           ;; Render as `EnumName::variant` (with Rust keyword escaping
           ;; via rust-variant-name). Used by election.successor — the
           ;; declared return type is the enum itself, so we must emit
           ;; typed variants rather than the bare u8 discriminant.
           (nanopass-case (Ltypescript Type) type
             [(tenum ,src^ ,enum-name ,elt-name^ ,elt-name* ...)
              (format "~a::~a"
                      (symbol->string enum-name)
                      (rust-variant-name elt-name))]
             [else
              (rust-feature-error src 'enum-ref-non-tenum
                "enum-ref ~a on non-tenum type"
                elt-name)])]
          [(+ ,src ,type ,expr1 ,expr2)
           ;; Iter 7 follow-up: unsigned addition. Renders via Rust's
           ;; `wrapping_add` so the bounded-Uint contract holds even if
           ;; the operation would overflow at the inferred Rust width —
           ;; Compact's typer requires an explicit `downcast-unsigned`
           ;; or wider target type around any expression that could
           ;; produce a value outside the source-side type's range, so
           ;; the wrap matches the post-downcast semantics. Both operands
           ;; are cast to the result width (from `type`, 0.33's
           ;; replacement for the old `mbits` slot) via arith-binop-rust.
           (arith-binop-rust src "add" type expr1 expr2 native-id-ht)]
          [(- ,src ,type ,expr1 ,expr2)
           ;; Iter 7 follow-up: unsigned subtraction. See `+` clause.
           (arith-binop-rust src "sub" type expr1 expr2 native-id-ht)]
          [(* ,src ,type ,expr1 ,expr2)
           ;; Iter 7 follow-up: unsigned multiplication. See `+` clause.
           (arith-binop-rust src "mul" type expr1 expr2 native-id-ht)]
          [(downcast-unsigned ,src ,nat2 ,nat1 ,expr)
           ;; 0.33 slot rename: the source-side bound went from
           ;; `(maybe nat?)` to a mandatory `nat2`, and the TARGET bound
           ;; is still the second slot (`nat1`). The old `nat? = #f`
           ;; case (source was a Field) moved out into its own
           ;; `cast-from-field` production, handled below.
           ;;
           ;; Iter 7 follow-up: cast the inner expression to the Rust
           ;; unsigned type whose upper bound is `nat1`. The downcast
           ;; appears around arithmetic whose declared output type is
           ;; narrower than its operands' inferred type (Compact's typer
           ;; inserts it for `(x * 2) as Uint<64>` and similar). The
           ;; expr-supported? gate already rejected non-ladder widths.
           ;;
           ;; The target width is also pushed down via `current-arith-suffix`
           ;; so nested arithmetic operands can apply a type suffix to
           ;; integer literals — Rust's `wrapping_mul` etc. are inherent
           ;; methods on `uN`, so the receiver must be a concrete `uN`
           ;; type (an unsuffixed `1.wrapping_mul(2)` would be rejected
           ;; with "can't call method on ambiguous numeric type").
           ;; Bug-11 (2026-06-29): pick the smallest Rust unsigned width
           ;; that can hold `nat`, mirroring uint-rust-width and the
           ;; generalised tunsigned-rust-suffix-for-bound. Pre-Bug-11
           ;; this rejected anything off the power-of-2-minus-1 ladder
           ;; (255/65535/u32::MAX/u64::MAX/u128::MAX), which surfaced as
           ;; `downcast-unsigned: unsupported target width` for any
           ;; `Uint<L..U>` arithmetic with a non-power-of-2 upper bound.
           ;; The on-state byte-width still tracks `uint-byte-length`,
           ;; so the cell-builder path (new_cell vs new_cell_bounded_uint)
           ;; routes the value through the right alignment descriptor.
           (let ([w (uint-target-rust-width nat1)])
             (cond
               ;; No Field guard is needed here, unlike on the ledger-8 line.
               ;; There, `Field as Uint<N>` was spelled
               ;; `(downcast-unsigned src #f nat expr)` and shared this
               ;; clause, so a missing source bound had to be caught or the
               ;; emitter rendered `(x) as uN` on an `Fr` — a non-primitive
               ;; cast (E0605) from a compile that exited 0.
               ;;
               ;; 0.33 split that cast into its own `cast-from-field`
               ;; production (handled below) and made the source bound
               ;; mandatory here, with the typer asserting an unsigned source
               ;; — see the `downcast-unsigned` rule in
               ;; compiler/circuit-passes/check-types-Linlined.ss. A Field
               ;; cannot reach this clause at all, so the guard would be dead
               ;; code rather than defence.
               [(not w)
                (rust-feature-error src 'downcast-unsigned-width
                  "downcast-unsigned: unsupported target width ~s" nat1)]
               [else
                ;; Range-checked, not `as` (MediaNoxLabs/compact#51).
                ;;
                ;; TypeScript lowers this cast to a bounds check that throws
                ;; (see `downcast-unsigned` in
                ;; compiler/typescript-passes/print-typescript.ss). Rust's
                ;; `as` checks nothing, and disagreed with it twice over:
                ;;
                ;;   * `w` is the smallest Rust width holding `nat1`, so for
                ;;     `Uint<0..100>` it is u8 and 100..=255 was accepted
                ;;     unchanged — outside the declared Compact type, and not
                ;;     even truncated, so nothing made it visible.
                ;;   * past that it truncates: `300 as u8` is 44, turning an
                ;;     out-of-range value into a plausible in-range one.
                ;;
                ;; TypeScript is normative, so both were Rust bugs. The
                ;; helper takes the Compact bound rather than the Rust one and
                ;; reproduces TS's message verbatim, so the two backends
                ;; report the same failure.
                ;;
                ;; `?` is safe in every position this renders into: circuit,
                ;; pure-circuit and constructor bodies all return
                ;; `Result<_, CompactError>` — `initial_state` included.
                (format "midnight_compact_runtime::std_lib::narrow::<~a>((~a) as u128, ~a_u128, ~s)?"
                        w
                        (parameterize ([current-arith-suffix w])
                          (expr-rust expr native-id-ht))
                        nat1
                        (format-source-object src))]))]
          [(cast-from-field ,src ,nat ,ftype ,expr)
           ;; 0.33: `Field as Uint<N>` split out of `downcast-unsigned`
           ;; (which previously spelled it `(downcast-unsigned src #f nat
           ;; expr)`) into its own production.
           ;;
           ;; There is NO Rust lowering for this cast, and reporting the
           ;; gap here fixes a latent bug in the old spelling: the shared
           ;; `downcast-unsigned` clause rendered `(<expr>) as uN`
           ;; regardless of whether the source was a Uint or a Field, so a
           ;; `Field as Uint<N>` emitted `(f) as u64` with `f: Fr`. `Fr` is
           ;; a struct, and Rust's `as` only casts between primitives, so
           ;; that is E0605 "non-primitive cast" — compactc exited 0 and
           ;; the breakage only surfaced at `cargo build`. No fixture
           ;; exercises the shape (all 32 codegen_regression fixtures are
           ;; byte-identical across this change), so failing loudly here
           ;; costs nothing and removes a silent-bad-output path.
           ;;
           ;; Lowering it properly needs a `midnight_compact_runtime` helper that
           ;; range-checks an `Fr` and narrows it to `uN` (the TS runtime
           ;; does the equivalent with a bigint bounds check). Tracked as a
           ;; follow-up on MediaNoxLabs/compact#17.
           (rust-feature-error src 'cast-from-field
             "Field-to-Uint cast (`as Uint<~s>`) has no Rust lowering; ~a"
             nat
             "a range-checking midnight_compact_runtime helper is needed")]
          [(cast-to-field ,src ,ftype ,type ,expr)
           ;; 0.33: `X as Field`-family casts where the TARGET is a
           ;; (tfield ftype) distinct from the source type. On the
           ;; default (ZKIR v2) path the reachable shapes are
           ;; `Field as JubjubScalar`, `Uint<N> as JubjubScalar` and
           ;; `JubjubScalar as Field` (`Uint<N> as Field` still lowers
           ;; to `safe-cast`).
           ;;
           ;; Field/Uint → JubjubScalar reduces mod the embedded scalar
           ;; order via the runtime helper. The reverse direction has no
           ;; runtime helper yet (EmbeddedFr → Fr is not a reduction and
           ;; needs a deliberate encoding decision), so report the gap.
           (cast-to-field-rust src ftype type
                               (lambda () (expr-rust expr native-id-ht)))]
          [(new ,src ,type ,expr* ...)
           ;; F2.2: struct-literal in pure-expression context (e.g. nested
           ;; inside a quote/tuple consumer). Renders each field via the
           ;; raw expr-rust path; for the body-walker context the
           ;; corresponding case in ctor-expr-rust is used instead.
           (let* ([st (struct-of-type type)]
                  [struct-name (and st (car st))]
                  [elt-name* (and st (cadr st))])
             (cond
               [(or (not st)
                    (not (fx= (length expr*) (length elt-name*))))
                (rust-feature-error src 'struct-literal-mismatch
                  "struct-literal mismatch (st=~s, expected=~a, got=~a)"
                  (and st (car st))
                  (and st (length elt-name*))
                  (length expr*))]
               [else
                (let* ([field-strs
                        (map (lambda (name e)
                               (format "~a: ~a"
                                       (symbol->string name)
                                       (expr-rust e native-id-ht)))
                             elt-name* expr*)])
                  (string-append
                    (struct-rust-name-of type struct-name)
                    " { "
                    (let join ([xs field-strs] [acc ""])
                      (cond
                        [(null? xs) acc]
                        [(null? (cdr xs)) (string-append acc (car xs))]
                        [else (join (cdr xs)
                                    (string-append acc (car xs) ", "))]))
                    " }"))]))]
          [(!= ,src ,type ,expr1 ,expr2)
           ;; F1.3: inequality. Parenthesised so it composes safely inside
           ;; larger expressions (assert macro args, && operands). Mirrors
           ;; the `==` rendering; structs derive PartialEq/Eq so `!=` is
           ;; structural for user types just as `==` is.
           (format "(~a != ~a)"
                   (expr-rust expr1 native-id-ht)
                   (expr-rust expr2 native-id-ht))]
          [(< ,src ,bits ,expr1 ,expr2)
           ;; F1.3: ordering comparisons on Uint<N> (Rust unsigned ints).
           ;; Operands render through expr-rust so downcast-unsigned /
           ;; arithmetic / field-access lower correctly; the typer inserts
           ;; downcast-unsigned around literals so both sides share the
           ;; same Rust unsigned width (no i32/u32 mismatch).
           (format "(~a < ~a)"
                   (expr-rust expr1 native-id-ht)
                   (expr-rust expr2 native-id-ht))]
          [(<= ,src ,bits ,expr1 ,expr2)
           (format "(~a <= ~a)"
                   (expr-rust expr1 native-id-ht)
                   (expr-rust expr2 native-id-ht))]
          [(> ,src ,bits ,expr1 ,expr2)
           (format "(~a > ~a)"
                   (expr-rust expr1 native-id-ht)
                   (expr-rust expr2 native-id-ht))]
          [(>= ,src ,bits ,expr1 ,expr2)
           (format "(~a >= ~a)"
                   (expr-rust expr1 native-id-ht)
                   (expr-rust expr2 native-id-ht))]
          [(seq ,src ,expr* ... ,expr)
           ;; F1.4: a guarded expression block. The Compact typer wraps
           ;; trapping unsigned arithmetic (e.g. `currentDay - dateOfBirthDays`
           ;; whose result must be non-negative) in
           ;; `(seq (assert <underflow-guard> ...) <expr>)` and let*-lifts
           ;; it into a const temp. Render as a Rust block
           ;; `{ <guards>; <expr> }` so the assert fires before the value
           ;; is produced, matching Compact's trap-on-underflow semantics
           ;; (Rust's `wrapping_sub` would otherwise silently wrap). The
           ;; guard assert's condition renders via cond-rust using the
           ;; current var-substitution + circuit/witness id hashtables so
           ;; calls to user pure circuits / field accesses lower correctly.
           ;;
           ;; G1: the prefix statements are folded rather than mapped so a
           ;; binding one of them introduces (seq-stmt-rust's `(=)` clause)
           ;; is in scope for the statements after it and for the tail —
           ;; the same left-to-right substitution threading
           ;; stmt-pure-body-rust does at statement level. Prefixes that
           ;; bind nothing leave the substitution untouched, so bodies
           ;; without a nested assignment render exactly as before.
           (let loop ([xs expr*] [binds (current-var-substitution)] [rev-pre '()])
             (cond
               [(pair? xs)
                (let-values ([(line bind)
                              (parameterize ([current-var-substitution binds])
                                (seq-stmt-rust (car xs) native-id-ht))])
                  (loop (cdr xs)
                        (if bind (cons bind binds) binds)
                        (cons line rev-pre)))]
               [else
                (let ([pre (reverse rev-pre)]
                      [tail (guard (c [#t #f])
                              (parameterize ([current-var-substitution binds])
                                (expr-rust expr native-id-ht)))])
                  (cond
                    [(or (not tail) (rendered-has-todo? tail))
                     (rust-feature-error src 'pure-circuit-body-emission
                       "seq tail expression unrenderable")]
                    [else
                     (string-append
                       "{ "
                       (let join ([xs pre] [acc ""])
                         (cond
                           [(null? xs) acc]
                           [(null? (cdr xs)) (string-append acc (car xs))]
                           [else (join (cdr xs) (string-append acc (car xs) " "))]))
                       (if (pair? pre) " " "")
                       tail
                       " }")]))]))]
          [else
           (rust-feature-error #f 'expr-variant
             "unhandled Expression variant in expr-rust")]))
