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
;;; This file: witness and circuit lookup tables, enum coercion.

      ;; witness-pelt?: returns #t if a Program-Element is a witness
      ;; declaration. Used by build-witness-id-ht to index witnesses.
      (define (witness-pelt? pelt)
        (nanopass-case (Ltypescript Program-Element) pelt
          [(witness ,src ,function-name (,arg* ...) ,type) #t]
          [else #f]))

      ;; witness-pelt-function-name: extract function-name id from a witness
      ;; Program-Element.
      (define (witness-pelt-function-name pelt)
        (nanopass-case (Ltypescript Program-Element) pelt
          [(witness ,src ,function-name (,arg* ...) ,type) function-name]))

      ;; build-witness-id-ht: eq-hashtable from each witness function-name
      ;; id to the witness Program-Element itself. Lets call-site emission
      ;; recognise a `(call witness-id args)` and emit the matching
      ;; `self.witnesses.<name>(...)` invocation with the right private-state
      ;; threading shape.
      (define (build-witness-id-ht pelt*)
        (let ([ht (make-eq-hashtable)])
          (for-each
            (lambda (pelt)
              (when (witness-pelt? pelt)
                (eq-hashtable-set! ht
                  (witness-pelt-function-name pelt)
                  pelt)))
            pelt*)
          ht))

      ;; build-circuit-id-ht: eq-hashtable from each circuit function-name id
      ;; to the circuit Program-Element. Lets call-site emission recognise a
      ;; `(call circuit-id args)` and dispatch on `id-pure?` to either
      ;; `pure_circuits::<name>(...)` or (eventually) a circuit invocation.
      (define (build-circuit-id-ht pelt*)
        (let ([ht (make-eq-hashtable)])
          (for-each
            (lambda (pelt)
              (when (circuit? pelt)
                (eq-hashtable-set! ht
                  (circuit-function-name pelt)
                  pelt)))
            pelt*)
          ht))

      ;; enum-ref->u8: render a `(enum-ref type elt-name)` Expression as the
      ;; u8 discriminant of the named variant. Returns the integer or #f if
      ;; the type isn't a tenum we recognise.
      (define (enum-ref->u8 expr)
        (nanopass-case (Ltypescript Expression) expr
          [(enum-ref ,src ,type ,elt-name)
           (nanopass-case (Ltypescript Type) type
             [(tenum ,src ,enum-name ,elt-name^ ,elt-name* ...)
              (let loop ([variants (cons elt-name^ elt-name*)] [i 0])
                (cond
                  [(null? variants) #f]
                  [(eq? (car variants) elt-name) i]
                  [else (loop (cdr variants) (+ i 1))]))]
             [else #f])]
          [else #f]))

      ;; enum-ref->typed-rust: render `(enum-ref type elt-name)` as
      ;; `EnumName::r#variant`. Returns #f if the type isn't a tenum.
      ;; Used when the surrounding context (current-enum-ref-typed?)
      ;; expects a typed enum value rather than the integer discriminant.
      (define (enum-ref->typed-rust expr)
        (nanopass-case (Ltypescript Expression) expr
          [(enum-ref ,src ,type ,elt-name)
           (nanopass-case (Ltypescript Type) type
             [(tenum ,src ,enum-name ,elt-name^ ,elt-name* ...)
              (format "~a::~a"
                      (symbol->string enum-name)
                      (rust-variant-name elt-name))]
             [else #f])]
          [else #f]))

      ;; witness-call-return-tenum?: returns #t when `expr` is a direct call
      ;; into a witness whose declared return type is a tenum (possibly via
      ;; talias). Used by the `==` rendering to detect that one operand
      ;; renders as a typed enum value, so an `enum-ref` on the other side
      ;; needs to render as `EnumName::variant` rather than as the integer
      ;; discriminant. Strips talias chains.
      (define (witness-call-return-tenum? expr witness-id-ht)
        (let ([e (expr-strip-cast expr)])
          (nanopass-case (Ltypescript Expression) e
            [(call ,src ,function-name ,expr* ...)
             (let ([w (eq-hashtable-ref witness-id-ht function-name #f)])
               (and w
                    (let ([ret-type
                           (nanopass-case (Ltypescript Program-Element) w
                             [(witness ,src ,function-name (,arg* ...) ,type) type]
                             [else #f])])
                      (and ret-type (tenum-name-of-type ret-type) #t))))]
            [else #f])))

      ;; operand-typed-enum?: returns #t when `expr` should render as a
      ;; typed enum value in Rust. Currently triggers on:
      ;;   1. a direct witness call whose declared return type is a tenum
      ;;      (e.g. election's `private$state()` returns `PrivateState`)
      ;;   2. a var-ref resolving to a formal arg whose declared type is a
      ;;      tenum (e.g. election.vote_for's `vote: PermissibleVotes`)
      ;; Both flow through the current-formal-arg-types hashtable
      ;; (populated by the impure-circuit emitter) for the var-ref case.
      ;; Used by `ctor-expr-rust`'s `==` rendering to opt into typed
      ;; enum-ref emission on the other operand.
      (define (operand-typed-enum? expr witness-id-ht)
        (or (witness-call-return-tenum? expr witness-id-ht)
            (let ([e (expr-strip-cast expr)]
                  [ht (current-formal-arg-types)])
              (and ht
                   (nanopass-case (Ltypescript Expression) e
                     [(var-ref ,src ,var-name)
                      (let ([t (eq-hashtable-ref ht (id-sym var-name) #f)])
                        (and t (tenum-name-of-type t) #t))]
                     [else #f])))))

      ;; is-ledger-read-expr?: returns #t when `expr` is a ledger read
      ;; (`public-ledger` IR node), possibly wrapped in cast/talias. The
      ;; Rust decoder for an integer-typed ledger cell is decode_bool /
      ;; decode_u8 / decode_uN (rust-passes-emit.ss `decoder-for-type`),
      ;; so the operand renders as a raw integer regardless of how the
      ;; IR's `==`/`!=` `type` field tags it. Bug-8: short-circuit
      ;; typed-enum rendering on the other operand so we don't emit
      ;; `<u8> == EnumName::variant` (which doesn't compile). Strips
      ;; casts the same way the surrounding ctor-expr-rust does.
      ;;
      ;; Bug-10 (2026-06-29): tenum-typed ledger reads now decode via
      ;; `decode_via_field_repr::<EnumName>` (typed) — they no longer
      ;; produce a raw u8, so they must not trigger the short-circuit.
      ;; Use `ledger-read-tenum?` below to peek at the adt-op's result
      ;; type and exclude that case.
      (define (is-ledger-read-expr? expr)
        (let ([e (expr-strip-cast expr)])
          (nanopass-case (Ltypescript Expression) e
            [(public-ledger ,src ,ledger-field-name ,sugar? (,path-elt* ...) ,src^ ,adt-op ,expr* ...)
             (not (adt-op-tenum-result? adt-op))]
            [else #f])))

      ;; adt-op-tenum-result?: returns #t when the read ADT-Op's
      ;; declared result type is a tenum (possibly through talias).
      ;; Used by `is-ledger-read-expr?` so Bug-10's typed decoder
      ;; integrates cleanly with Bug-8's integer-coercion short-circuit:
      ;; integer ledger reads still suppress typed enum-ref rendering on
      ;; the opposite operand, but tenum ledger reads (now typed)
      ;; explicitly do not.
      (define (adt-op-tenum-result? adt-op)
        (nanopass-case (Ltypescript ADT-Op) adt-op
          [(,ledger-op ,op-class (,adt-name (,adt-formal* ,adt-arg*) ...) ((,var-name* ,type*) ...) ,type ,vm-code)
           (and (eq? op-class 'read)
                (tenum-name-of-type type)
                #t)]
          [else #f]))

      ;; ctor-expr-rust: render a constructor-body Expression as a Rust
      ;; expression string. Tracks a local binding alist (var-name id ->
      ;; snake-cased Rust name) so var-refs resolve to the let-bound name
      ;; we emitted earlier. Falls back to expr-rust (which handles natives
      ;; etc.) for the remaining shapes.
      ;;
      ;; native-id-ht / witness-id-ht / circuit-id-ht let call-site
      ;; classification distinguish pure-circuit / witness / native calls.
      ;; coerce-cmp-operand-rust: render a comparison (`==`/`!=`) operand,
      ;; coercing a bare integer literal to `Fr::from(<n>u64)` when the
      ;; comparison `type` is `Field` (tfield). The typer types integer
      ;; literals in a Field-typed comparison as `Field`, but expr-rust
      ;; renders `(quote 0)` as the bare Rust integer `0` — which then
      ;; fails to type-check against the `Fr` produced by the other
      ;; operand (e.g. `jubjub_point_x(...) != 0` in the digital-passport
      ;; holder-binding check). Non-Field types and non-literal operands
      ;; render via ctor-expr-rust unchanged.
      (define (coerce-cmp-operand-rust expr type local-binds
                                        native-id-ht witness-id-ht circuit-id-ht)
        (cond
          [(and (type-tfield-ftype type) (literal-int-expr? expr))
           ;; `Fr::from(<n>u64)` for native Field, `JubjubScalar::from`
           ;; for the 0.33 embedded-scalar builtin (same From<u64> shape).
           (format "~a::from(~au64)"
                   (field-type-rust (type-tfield-ftype type))
                   (literal-int-expr? expr))]
          [else
           (ctor-expr-rust expr local-binds
                           native-id-ht witness-id-ht circuit-id-ht)]))
