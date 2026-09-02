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
;;; This file: arithmetic, casts and width selection.

      ;; bytevector->rust-array-literal: render a Scheme bytevector as a Rust
      ;; array literal `[N1u8, N2, ..., NK]`. The first element carries the
      ;; explicit `u8` suffix so the literal infers as `[u8; K]` without an
      ;; ascription. Used by expr-rust for `(quote src #vu8(...))`.
      (define (bytevector->rust-array-literal bv)
        (let* ([n (bytevector-length bv)]
               [parts
                (let loop ([i 0] [acc '()])
                  (cond
                    [(fx= i n) (reverse acc)]
                    [else
                     (let ([byte (bytevector-u8-ref bv i)])
                       (loop (fx+ i 1)
                             (cons
                               (if (fx= i 0)
                                   (format "~au8" byte)
                                   (format "~a" byte))
                               acc)))]))])
          (string-append
            "["
            (let join ([xs parts] [acc ""])
              (cond
                [(null? xs) acc]
                [(null? (cdr xs)) (string-append acc (car xs))]
                [else (join (cdr xs) (string-append acc (car xs) ", "))]))
            "]")))

      ;; arith-operand-rust: render an arithmetic operand and, when
      ;; `current-arith-suffix` is set and the rendered output is a
      ;; bare integer literal, append the suffix so Rust resolves the
      ;; surrounding wrapping_* inherent method against a concrete
      ;; integer type. Non-literal operands (var-refs, method calls,
      ;; nested arithmetic with their own suffix) are returned unmodified
      ;; — they already carry typing information through the enclosing
      ;; `as u<width>` cast emitted by the downcast-unsigned clause.
      ;;
      ;; Iter 7 follow-up: introduced to support non-identity lambdas
      ;; in `map()` (`x * 2 as Uint<64>` and friends).
      (define (arith-operand-rust expr native-id-ht)
        (let ([rendered (expr-rust expr native-id-ht)]
              [s (current-arith-suffix)])
          (cond
            [(and s (integer-literal-rendering? rendered))
             (string-append rendered s)]
            [else rendered])))

      ;; uint-target-rust-width: the Rust unsigned type a Uint cast
      ;; TARGET with maximum value `nat` narrows to, or #f when `nat` is
      ;; not a usable bound (negative, non-integer, or wider than u128).
      ;; Shared by the `downcast-unsigned` (Uint→Uint) and
      ;; `cast-from-field` (Field→Uint) renderers, which 0.33 split out
      ;; of a single production. Bug-11: the width is the smallest one
      ;; that HOLDS `nat` (mirroring `uint-rust-width` and
      ;; `tunsigned-rust-suffix-for-bound`), not just the power-of-2-minus-1
      ;; ladder, so `Uint<L..U>` arithmetic with a non-power-of-2 upper
      ;; bound lowers instead of erroring.
      (define (uint-target-rust-width nat)
        (and (integer? nat) (exact? nat) (not (negative? nat))
             (<= nat 340282366920938463463374607431768211455)
             (uint-rust-width nat)))

      ;; cast-to-field-rust: render 0.33's `cast-to-field` production —
      ;; a cast whose TARGET is a `(tfield ftype)` distinct from the
      ;; source `type`. `render-inner` is a thunk producing the already
      ;; rendered operand (so callers can pick expr-rust vs
      ;; ctor-expr-rust).
      ;;
      ;; Supported on the default (ZKIR v2) path:
      ;;   Field       as JubjubScalar → jubjub_scalar_from_field(x)
      ;;   Uint<N≤u64> as JubjubScalar → jubjub_scalar_from_field(Fr::from(x as u64))
      ;; Everything else (JubjubScalar as Field, u128-width sources, the
      ;; zkir-v3 field variants) has no runtime helper and reports the
      ;; gap rather than guessing an encoding.
      (define (cast-to-field-rust src ftype type render-inner)
        (cond
          [(not (field-type-jubjub-scalar? ftype))
           (rust-feature-error src 'cast-to-field-target
             "cast-to-field: no Rust cast to ~a" (field-type-rust ftype))]
          [(type-tfield-ftype type) =>
           (lambda (src-ftype)
             (cond
               [(field-type-native? src-ftype)
                (format "midnight_compact_runtime::jubjub_scalar_from_field(~a)"
                        (render-inner))]
               [else
                (rust-feature-error src 'cast-to-field-source
                  "cast-to-field: no Rust cast from ~a to JubjubScalar"
                  (field-type-rust src-ftype))]))]
          [(type-peel-tunsigned type) =>
           (lambda (nat)
             (cond
               [(<= nat 18446744073709551615)
                (format "midnight_compact_runtime::jubjub_scalar_from_field(Fr::from((~a) as u64))"
                        (render-inner))]
               [else
                (rust-feature-error src 'cast-to-field-source
                  "cast-to-field: Uint (max ~s) is wider than u64; no JubjubScalar cast"
                  nat)]))]
          [else
           (rust-feature-error src 'cast-to-field-source
             "cast-to-field: unsupported source type for JubjubScalar cast")]))

      ;; arith-result-rust-width: map the RESULT type of a
      ;; `(+/−/* ,src ,type ,e1 ,e2)` node to the smallest Rust unsigned
      ;; int type that holds it, or #f when there is no unsigned width to
      ;; cast to.
      ;;
      ;; 0.33 replaced the old `mbits` (maybe-bits) slot with the full
      ;; result `Type` the typer computed (infer-types.ss `condense` /
      ;; `arithmetic-binop`). The old encoding was:
      ;;   mbits = (max 1 (integer-length result-nat))  for Uint results
      ;;   mbits = #f                                   for Field results
      ;; so `(tunsigned src nat)` here reproduces the old integer branch
      ;; and `(tfield src ftype)` reproduces the old `#f` branch.
      ;;
      ;; `uint-rust-width` (rust-passes-helpers.ss) is the existing ladder
      ;; from a tunsigned MAX VALUE to u8/u16/u32/u64/u128; it agrees with
      ;; the retired `mbits->rust-width` bit-width ladder at every
      ;; boundary. The one difference is the tail: `mbits->rust-width`
      ;; returned #f above 128 bits while `uint-rust-width` saturates at
      ;; u128, so the explicit u128::MAX guard below preserves the old
      ;; "fall back to the operand-native width" behaviour rather than
      ;; silently truncating a wider result.
      ;;
      ;; The width is used to cast BOTH operands to the result width so
      ;; e.g. `ageThresholdYears: Uint<8> * 365` (result max 93075, u32)
      ;; renders as `((...) as u32).wrapping_mul((...) as u32)` — without
      ;; the cast `wrapping_mul` runs at the u8 receiver width and the u32
      ;; comparison side mismatches (digital-passport age-predicate).
      (define (arith-result-rust-width type)
        (cond
          [(type-peel-tunsigned type) =>
           (lambda (nat) (uint-target-rust-width nat))]
          [else #f]))

      ;; arith-binop-rust: render a `(+/−/* ,src ,type ,e1 ,e2)` node,
      ;; casting both operands to the result width (from `type`) when it
      ;; maps to a Rust unsigned type. The cast is a no-op when the
      ;; operands are already that width (e.g. Uint<32> - Uint<32>) and a
      ;; widening cast when an operand is narrower (Uint<8> * literal).
      ;; Field-typed results take the uncast `wrapping_*` fall-through,
      ;; exactly as the pre-0.33 `mbits = #f` case did.
      ;; field-arith-rust-operator: Rust operator for FIELD arithmetic.
      ;; Field arithmetic is modular in the field's own characteristic, so
      ;; the plain operators are the correct lowering — `Fr` implements
      ;; Add/Sub/Mul (and NOT the `wrapping_*` family, which exists only
      ;; on Rust's fixed-width integers).
      (define (field-arith-rust-operator op)
        (cond
          [(string=? op "add") "+"]
          [(string=? op "sub") "-"]
          [(string=? op "mul") "*"]
          [else #f]))

      (define (arith-binop-rust src op type expr1 expr2 native-id-ht)
        (let ([e1 (arith-operand-rust expr1 native-id-ht)]
              [e2 (arith-operand-rust expr2 native-id-ht)]
              [w (arith-result-rust-width type)])
          (cond
            [w
             (format "((~a) as ~a).wrapping_~a((~a) as ~a)" e1 w op e2 w)]
            ;; FIELD arithmetic. This branch previously fell through to
            ;; `(~a).wrapping_~a(~a)`, emitting e.g. `(a).wrapping_add(b)`
            ;; on two `Fr`s — which does not compile, because `Fr` has no
            ;; `wrapping_add`. compactc exited 0 and the failure surfaced
            ;; only at `cargo build`, from Compact as ordinary as
            ;;     export pure circuit addFields(a: Field, b: Field): Field {
            ;;       return a + b;
            ;;     }
            ;; No fixture exercised the shape, so nothing caught it: the
            ;; corpus only ever reached the `w` branch above.
            [(type-is-tfield? type)
             (let ([rust-op (field-arith-rust-operator op)])
               (unless rust-op
                 (rust-feature-error src 'field-arith-operator
                   "field arithmetic operator `~a` has no Rust lowering" op))
               (format "(~a) ~a (~a)" e1 rust-op e2))]
            ;; Neither a known unsigned width nor a field: refuse rather
            ;; than emit `wrapping_*` against a type that may not have it.
            ;; A guess here is exactly the silent-bad-output path the
            ;; field case above spent a release demonstrating.
            [else
             (rust-feature-error src 'arith-result-type
               "unsigned/field arithmetic on an unsupported result type has no Rust lowering")])))

      ;; expr-rust: emit a Rust expression string for an Ltypescript
      ;; Expression. I3b/1 covers the variants needed by tiny.compact's
      ;; public_key body — bytevector literal, var-ref, tuple (array
      ;; literal), and call. Unknown variants emit a TODO placeholder so
      ;; the gap is visible in the generated code rather than crashing.
      ;;
      ;; `native-id-ht` is the eq-hashtable built by build-native-id-ht;
      ;; consulted at every `call` site to resolve the function-name id
      ;; back to its native-entry (and thus its Rust binding name).
      ;; seq-stmt-rust: render one effect statement of a `(seq ...)`
      ;; expression block (the guard asserts the typer wraps around
      ;; trapping unsigned arithmetic). Used by expr-rust's `seq` case.
      ;; Asserts render via cond-rust (with current-var-substitution +
      ;; current-witness/circuit-id-ht) so user pure-circuit calls and
      ;; field accesses in the guard lower correctly; other expressions
      ;; render via expr-rust as `<expr>;`.
      ;;
      ;; Returns two values: the rendered statement string and either #f
      ;; or a `(var-name . rust-name)` pair naming a binding the statement
      ;; introduced, so expr-rust's `seq` clause can thread it into
      ;; current-var-substitution for the statements that follow.
      (define (seq-stmt-rust e native-id-ht)
        (nanopass-case (Ltypescript Expression) e
          [(assert ,src ,expr0 ,mesg)
           (let ([cstr (guard (c [#t #f])
                         (cond-rust expr0 (current-var-substitution)
                                    native-id-ht
                                    (current-witness-id-ht)
                                    (current-circuit-id-ht)))])
             (cond
               [(or (not cstr) (rendered-has-todo? cstr))
                (rust-feature-error src 'pure-circuit-body-emission
                  "seq guard assert unrenderable")]
               [else (values (format "compact_assert!(~a, ~s);" cstr
                                     (if (string? mesg) mesg ""))
                             #f)]))]
          [(= ,src ,var-name ,expr^)
           ;; G1: a let*-lifted temp assignment nested inside a `seq`
           ;; block. The statement-level renderer already lowers these —
           ;; see stmt-pure-body-rust's stmt->assignment clause — but a
           ;; `seq` prefix reached this renderer and fell through to
           ;; expr-rust, which has no `(=)` clause, so the whole body
           ;; bailed with `no walker shape matched`.
           ;;
           ;; The shape arises whenever a trapping arithmetic operand is
           ;; itself let*-lifted, which the typer does for struct-field
           ;; projections: `now - att.createdAt <= policy.maxAge`
           ;; lowers to `(= %t.6 (seq (= %t.7 (elt-ref att createdAt))
           ;; (seq (assert (>= now %t.7) ...) (- now %t.7))))` — two
           ;; nested assignment levels, where a plain scalar operand
           ;; produces only one. Plain `-`/`+`/`*` on locals, and the
           ;; same projection under `==`, only ever hit the outer level,
           ;; which is why every ingredient compiled in isolation.
           ;;
           ;; Rendering reuses the same helpers as the statement-level
           ;; path (uniquify-rust-name over current-var-substitution, then
           ;; expr-rust for the RHS) rather than introducing a second
           ;; projection-aware renderer. The RHS renders under the
           ;; pre-binding substitution — a lifted temp never references
           ;; itself.
           (let* ([binds (current-var-substitution)]
                  [proposed (symbol->string (camel->snake (id-sym var-name)))]
                  [rust-name (uniquify-rust-name proposed binds)]
                  [rhs (expr-rust expr^ native-id-ht)])
             (values (format "let ~a = ~a;" rust-name rhs)
                     (cons var-name rust-name)))]
          [else
           (values (string-append (expr-rust e native-id-ht) ";") #f)]))
