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
;;; This file: the support predicates: can this shape be lowered at all.

      ;; expr-supported?: predicate that returns #t when an Expression is
      ;; in a shape our body emitter can render cleanly (no
      ;; `unimplemented!()` placeholders, no unresolved enum/var refs).
      ;; Used as a pre-validation gate so circuits whose bodies contain
      ;; shapes we don't yet handle (e.g. tiny.compact's `clear` with its
      ;; `apk == authority` comparison and `default<T>` writes) fall back
      ;; to `unimplemented!()` rather than emitting partially-broken code.
      ;;
      ;; Witness/pure-circuit/native call sites are accepted even when the
      ;; underlying function returns from a non-emittable circuit, since
      ;; we render those as method/function invocations.
      (define (expr-supported? expr native-id-ht witness-id-ht circuit-id-ht)
        (let ([e (expr-strip-cast expr)])
          (nanopass-case (Ltypescript Expression) e
            [(var-ref ,src ,var-name) #t]
            [(quote ,src ,datum)
             (or (and (integer? datum) (exact? datum))
                 (boolean? datum)
                 (bytevector? datum))]
            [(enum-ref ,src ,type ,elt-name)
             ;; enum-ref->u8 returns #f on unknown variants.
             (and (enum-ref->u8 e) #t)]
            [(call ,src ,function-name ,expr* ...)
             (let ([ne (eq-hashtable-ref native-id-ht function-name #f)]
                   [w (eq-hashtable-ref witness-id-ht function-name #f)]
                   [c (eq-hashtable-ref circuit-id-ht function-name #f)])
               ;; F2.2: accept ALL user pure-circuit callees (exported
               ;; or not). Non-exported helpers (e.g. election's
               ;; `successor`) now land in the `pure_circuits` mod as
               ;; `pub(crate) fn` (per E4.4 / emit-pure-circuit), so
               ;; `pure_circuits::<name>(...)` is a valid in-crate
               ;; reference from impure-circuit bodies.
               (and (or ne
                        w
                        (and c (id-pure? function-name)))
                    (let loop ([xs expr*])
                      (cond
                        [(null? xs) #t]
                        [(expr-supported? (car xs) native-id-ht
                                          witness-id-ht circuit-id-ht)
                         (loop (cdr xs))]
                        [else #f]))))]
            [(tuple ,src ,tuple-arg* ...)
             (let loop ([xs tuple-arg*])
               (cond
                 [(null? xs) #t]
                 [else
                  (let ([ok?
                         (nanopass-case (Ltypescript Tuple-Argument) (car xs)
                           [(single ,src ,expr)
                            (expr-supported? expr native-id-ht
                                             witness-id-ht circuit-id-ht)]
                           [else #f])])
                    (and ok? (loop (cdr xs))))]))]
            [(default ,src ,type)
             ;; I3b/3: any type the default-value-rust helper can render
             ;; is fine. The helper has a Default::default() fallback so
             ;; this is effectively always supported, but we still gate
             ;; on the helper's recognised shapes to keep the codegen
             ;; faithful.
             (default-supported? type)]
            [(== ,src ,type ,expr1 ,expr2)
             ;; I3b/3: equality. Recurse into both operands.
             (and (expr-supported? expr1 native-id-ht
                                   witness-id-ht circuit-id-ht)
                  (expr-supported? expr2 native-id-ht
                                   witness-id-ht circuit-id-ht))]
            [(!= ,src ,type ,expr1 ,expr2)
             ;; A9: inequality. The IR has `!=` as its own node, not
             ;; lowered to `(not (== ...))`. did.compact's
             ;; `rotateControllerKey` body asserts
             ;; `disclosedNewControllerPublicKey != controllerPublicKey`.
             ;; Same recursion shape as `==`.
             (and (expr-supported? expr1 native-id-ht
                                   witness-id-ht circuit-id-ht)
                  (expr-supported? expr2 native-id-ht
                                   witness-id-ht circuit-id-ht))]
            [(not ,src ,expr)
             ;; F1.2: Boolean negation. Recurse into the operand.
             (expr-supported? expr native-id-ht
                              witness-id-ht circuit-id-ht)]
            [(and ,src ,expr1 ,expr2)
             ;; F1.2: short-circuit AND.
             (and (expr-supported? expr1 native-id-ht
                                   witness-id-ht circuit-id-ht)
                  (expr-supported? expr2 native-id-ht
                                   witness-id-ht circuit-id-ht))]
            [(or ,src ,expr1 ,expr2)
             ;; F1.2: short-circuit OR.
             (and (expr-supported? expr1 native-id-ht
                                   witness-id-ht circuit-id-ht)
                  (expr-supported? expr2 native-id-ht
                                   witness-id-ht circuit-id-ht))]
            [(elt-ref ,src ,expr ,elt-name ,nat)
             ;; F1.2: struct field access. The inner expression must be
             ;; renderable; the field selection itself is unconditionally
             ;; supported (Rust structs use the same `.field` syntax,
             ;; emitter doesn't know whether the field name is a Rust
             ;; reserved word — current zerocash structs use safe names).
             ;;
             ;; F2.2: also accept `(elt-ref (public-ledger ... read) field N)`
             ;; when the read returns a tstruct and the projected field at
             ;; offset 0 has a decoder. The whole-struct path through
             ;; `ledger-read-supported?` would reject (no struct decoder),
             ;; so we provide a narrower acceptance criterion that lines
             ;; up with the special-case in `ctor-expr-rust`. Currently
             ;; only the leading boolean field is supported (e.g.
             ;; `topic.read().is_some` on `Maybe<T>` — `decode_bool` on the
             ;; resulting AlignedValue reads exactly the first atom).
             (or (expr-supported? expr native-id-ht
                                  witness-id-ht circuit-id-ht)
                 (and (fx= nat 0)
                      (elt-ref-of-struct-read? expr)))]
            [(public-ledger ,src ,ledger-field-name ,sugar? (,path-elt* ...) ,src^ ,adt-op ,expr* ...)
             ;; I3b/3: ledger read in expression position. Supported when
             ;; op-class is `read`, the path is a single index, and the
             ;; result type has a decoder.
             (ledger-read-supported? path-elt* adt-op)]
            [(new ,src ,type ,expr* ...)
             ;; F2.2: struct-literal construction (e.g. election.set_topic's
             ;; `Maybe<Opaque<"string">>{ is_some: true, value: t }`).
             ;; Accept when the type is a tstruct (Maybe or user struct) and
             ;; all field initialiser exprs render. The arg count must equal
             ;; the struct's field count (lowered from named field initialisers
             ;; in source order). The Maybe path reuses the L1 runtime alias;
             ;; user structs are referenced by their bare name (H5-H7).
             (let ([st (struct-of-type type)])
               (and st
                    (fx= (length expr*) (length (cadr st)))
                    (for-all (lambda (e)
                               (expr-supported? e native-id-ht
                                                witness-id-ht circuit-id-ht))
                             expr*)))]
            [(map ,src ,len ,fun ,map-arg ,map-arg* ...)
             ;; Iter 7: `map(fn, iterable)` over a static-length literal
             ;; iterable. Single map-arg only (no zip-map); fun must be a
             ;; bare-lambda `(circuit (arg) ret-type body)` whose body is
             ;; an expr-supported? Expression after substituting the
             ;; element parameter with the i-th literal. The iterable
             ;; (map-arg's expr) must be a `(tuple ...)` or `(vector ...)`
             ;; literal whose elements are themselves expr-supported?
             ;; (the body substitution preserves each element verbatim,
             ;; so per-iteration we get an expression of the same shape
             ;; we just validated).
             (and (null? map-arg*)
                  (map-expr-mvp-supported? src fun map-arg
                                           native-id-ht witness-id-ht circuit-id-ht))]
            [(+ ,src ,type ,expr1 ,expr2)
             ;; Iter 7 follow-up: unsigned addition in expression position
             ;; (e.g. inside a map() lambda body). We render via Rust's
             ;; wrapping_add — the Compact typer wraps non-trivial
             ;; arithmetic in a downcast-unsigned which checks the upper
             ;; bound, so wrapping semantics match the bounded-Uint
             ;; contract.
             ;;
             ;; 0.33 replaced the `mbits` (maybe-bits) slot with the full
             ;; result `Type`. The pre-0.33 guard here was `(not mbits)`,
             ;; and `mbits = #f` was emitted by the typer's FIELD branch
             ;; (analysis-passes.ss `(k #f ...)` with
             ;; `result-type = (tfield src)`), so the literal translation
             ;; is `(type-is-tfield? type)`. That is preserved verbatim
             ;; to keep this port byte-parity-neutral.
             ;;
             ;; RESOLVED (was tracked on MediaNoxLabs/compact#17). The
             ;; literal translation admitted only `tfield`, which is
             ;; backwards for a guard in front of unsigned arithmetic.
             ;; Both unsigned and field arithmetic now have correct
             ;; lowerings, so both are admitted here.
             ;;
             ;; Two things measured while fixing it, since the earlier
             ;; note guessed wrong about both:
             ;;
             ;; 1. These guards ARE dead for the corpus — not "expected to
             ;;    move fixture bytes". Flipping tfield->tunsigned left all
             ;;    32 fixtures byte-identical, because every fixture that
             ;;    emits arithmetic reaches the emitter through the
             ;;    constructor / seq-stmt / cond-rust routes, not here.
             ;; 2. Flipping them did NOT fix the trap the note described.
             ;;    Field arithmetic still reached `arith-binop-rust` by
             ;;    another route and still emitted `(a).wrapping_add(b)` on
             ;;    two `Fr`s — which does not compile. The real defect was
             ;;    in `arith-binop-rust`'s fallback branch and is fixed
             ;;    there; see the field case in that function.
             (and (or (type-peel-tunsigned type) (type-is-tfield? type))
                  (expr-supported? expr1 native-id-ht
                                   witness-id-ht circuit-id-ht)
                  (expr-supported? expr2 native-id-ht
                                   witness-id-ht circuit-id-ht))]
            [(- ,src ,type ,expr1 ,expr2)
             ;; Iter 7 follow-up: unsigned subtraction. See the `+` clause
             ;; above for the wrapping-vs-checked rationale and the 0.33
             ;; `mbits`→`type` translation note.
             (and (or (type-peel-tunsigned type) (type-is-tfield? type))
                  (expr-supported? expr1 native-id-ht
                                   witness-id-ht circuit-id-ht)
                  (expr-supported? expr2 native-id-ht
                                   witness-id-ht circuit-id-ht))]
            [(* ,src ,type ,expr1 ,expr2)
             ;; Iter 7 follow-up: unsigned multiplication.
             (and (or (type-peel-tunsigned type) (type-is-tfield? type))
                  (expr-supported? expr1 native-id-ht
                                   witness-id-ht circuit-id-ht)
                  (expr-supported? expr2 native-id-ht
                                   witness-id-ht circuit-id-ht))]
            [(downcast-unsigned ,src ,nat2 ,nat1 ,expr)
             ;; Iter 7 follow-up: the typer inserts `downcast-unsigned`
             ;; around arithmetic results whose declared output type is
             ;; narrower than the natural product/sum width (e.g.
             ;; `(x * 2) as Uint<64>`). The downcast's target width
             ;; (`nat1`, an unsigned upper bound — 0.33 made the source
             ;; bound `nat2` mandatory but left the target in the second
             ;; slot) drives the Rust `as uN` suffix in expr-rust. Only
             ;; widths on the standard 8/16/32/64/128 ladder are
             ;; supported; other widths fall through to #f.
             (and (tunsigned-rust-suffix-for-bound nat1)
                  (expr-supported? expr native-id-ht
                                   witness-id-ht circuit-id-ht))]
            ;; NOTE: no `cast-from-field` clause — `Field as Uint<N>` has
            ;; no Rust lowering (`Fr` is a struct, so `as uN` is E0605),
            ;; so it must reach the `else` arm and be rejected. See the
            ;; `cast-from-field` clause in expr-rust for the full
            ;; rationale and the follow-up.
            [(cast-to-field ,src ,ftype ,type ,expr)
             ;; 0.33: casts whose target is a `(tfield ftype)` distinct
             ;; from the source type. Only the JubjubScalar direction has
             ;; a runtime helper (`jubjub_scalar_from_field`); mirrors
             ;; cast-to-field-rust's accepted shapes.
             (and (field-type-jubjub-scalar? ftype)
                  (or (and (type-tfield-ftype type)
                           (field-type-native? (type-tfield-ftype type)))
                      (let ([nat (type-peel-tunsigned type)])
                        (and nat (<= nat 18446744073709551615))))
                  (expr-supported? expr native-id-ht
                                   witness-id-ht circuit-id-ht))]
            [else #f])))

      ;; map-expr-mvp-supported?: narrow-shape predicate for a
      ;; `(map ,src ,len ,fun ,map-arg)` expression. Accepts the
      ;; literal-iterable MVP: `fun` is a bare-lambda over a single
      ;; param, iterable peels to a static `(tuple ...)` /
      ;; `(vector ...)` literal whose elements survive
      ;; `tuple-arg->literal`, and the lambda body is renderable by
      ;; expr-rust under per-iteration substitution. Iter 7 shipped the
      ;; identity-body case; the follow-up adds arithmetic + downcast
      ;; bodies (`(x * 2) as Uint<64>` and friends) via
      ;; lambda-body-supported?.
      ;;
      ;; Returns #t on a supported shape, #f otherwise. The caller
      ;; (expr-supported?) is the gate; ctor-expr-rust's matching
      ;; map-clause assumes the same shape and renders accordingly.
      (define (map-expr-mvp-supported? src fun map-arg
                                       native-id-ht witness-id-ht circuit-id-ht)
        (let ([param-name (lambda-param-name fun)]
              [body (lambda-body-expr fun)]
              [iter-expr (map-arg->expr map-arg)])
          (and param-name
               body
               iter-expr
               (let ([literals (iterable-expr->literals iter-expr)])
                 (and literals
                      (lambda-body-supported? body param-name
                                              native-id-ht
                                              witness-id-ht
                                              circuit-id-ht))))))

      ;; lambda-param-name: from a Function IR node of shape
      ;; `(circuit src ((var-name type)) ret-type expr)` (single-param
      ;; bare lambda), return the `var-name`. Returns #f for any other
      ;; shape (fref / multi-param / zero-param).
      (define (lambda-param-name fun)
        (nanopass-case (Ltypescript Function) fun
          [(circuit ,src (,arg* ...) ,type ,stmt)
           (and (fx= (length arg*) 1)
                (nanopass-case (Ltypescript Argument) (car arg*)
                  [(,var-name ,type) var-name]))]
          [else #f]))

      ;; lambda-body-expr: from a Function IR node, return the body
      ;; Expression iff the shape is `(circuit src (arg*) ret-type
      ;; expr)`. The map IR's fun slot is always an Expression-valued
      ;; lambda at Ltypescript (per langs.ss line 866 — Ltypescript's
      ;; Function uses the `stmt` slot but with a (statement-expression
      ;; expr) wrapper). Returns the underlying Expression after
      ;; peeling statement-expression wrappers, or #f if the shape
      ;; isn't recognisable.
      ;;
      ;; Note: at the Ltypescript layer, lambdas inside `map`/`fold`
      ;; that arrived from the frontend's pre-statement IR carry their
      ;; body as an Expression directly (see the trace from compactc
      ;; with --trace-passes: the map's fun is `(circuit ((%x ...))
      ;; ret-type %x)` where `%x` is a raw Expression, not a
      ;; Statement). Be defensive: if it's a Statement-shaped lambda,
      ;; peel `statement-expression` once.
      (define (lambda-body-expr fun)
        (nanopass-case (Ltypescript Function) fun
          [(circuit ,src (,arg* ...) ,type ,stmt)
           (nanopass-case (Ltypescript Statement) stmt
             [(statement-expression ,expr) expr]
             [else #f])]
          [else #f]))

      ;; lambda-body-identity?: returns #t when the lambda body is a
      ;; `(var-ref ,param-name)` (possibly with safe-cast wrappers).
      ;; Retained for callers that specifically want to detect the
      ;; identity shape; the broader map-MVP gate uses
      ;; lambda-body-supported? below.
      (define (lambda-body-identity? body param-name)
        (let ([e (expr-strip-cast body)])
          (nanopass-case (Ltypescript Expression) e
            [(var-ref ,src ,var-name)
             (eq? (id-sym var-name) (id-sym param-name))]
            [else #f])))

      ;; lambda-body-supported?: returns #t when the lambda body is
      ;; renderable by expr-rust after substituting `param-name` with an
      ;; element literal. Walks past safe-cast / downcast-unsigned /
      ;; arithmetic to validate the inner shapes — for the substitution
      ;; to be sound we need every recursive position to be either
      ;; expr-supported? in its own right (e.g. an integer literal,
      ;; a struct field access on a closed-over var) or a var-ref to
      ;; `param-name` itself. Recursion bottoms out at var-ref:
      ;; the parameter reference is the only var-ref we accept here
      ;; (the lambda's surface syntax doesn't capture other locals at
      ;; the IR layer we operate on for map(), so any other var-ref
      ;; would be a closure variable we don't yet substitute).
      (define (lambda-body-supported? body param-name
                                       native-id-ht witness-id-ht circuit-id-ht)
        (let walk ([e (expr-strip-cast body)])
          (nanopass-case (Ltypescript Expression) e
            [(var-ref ,src ,var-name)
             (eq? (id-sym var-name) (id-sym param-name))]
            [(quote ,src ,datum)
             (or (and (integer? datum) (exact? datum))
                 (boolean? datum))]
            ;; 0.33 `mbits`→`type`: the pre-0.33 guard was `(not mbits)`,
             ;; which the typer emitted only for FIELD arithmetic, so
             ;; `(type-is-tfield? type)` was the literal translation.
             ;; Now admits unsigned AND field, both of which lower
             ;; correctly — see the resolved note in expr-supported? above.
            [(+ ,src ,type ,expr1 ,expr2)
             (and (or (type-peel-tunsigned type) (type-is-tfield? type))
                  (walk (expr-strip-cast expr1))
                  (walk (expr-strip-cast expr2)))]
            [(- ,src ,type ,expr1 ,expr2)
             (and (or (type-peel-tunsigned type) (type-is-tfield? type))
                  (walk (expr-strip-cast expr1))
                  (walk (expr-strip-cast expr2)))]
            [(* ,src ,type ,expr1 ,expr2)
             (and (or (type-peel-tunsigned type) (type-is-tfield? type))
                  (walk (expr-strip-cast expr1))
                  (walk (expr-strip-cast expr2)))]
            [(downcast-unsigned ,src ,nat2 ,nat1 ,expr)
             (and (tunsigned-rust-suffix-for-bound nat1)
                  (walk (expr-strip-cast expr)))]
            ;; No `cast-from-field` clause: `Field as Uint<N>` has no Rust
            ;; lowering, so it falls through to #f. See expr-rust.
            [else #f])))

      ;; default-supported?: returns #t when default-value-rust would
      ;; produce a faithful Rust expression for `type`. Mirrors the
      ;; helper's case analysis (sans the catch-all `Default::default()`
      ;; fallback) so we don't accept type shapes we'd silently lower
      ;; to a generic default.
      (define (default-supported? type)
        (nanopass-case (Ltypescript Type) type
          [(tunsigned ,src ,nat) #t]
          [(tfield ,src ,ftype)
           ;; Fr and JubjubScalar (EmbeddedFr) both derive Default (zero);
           ;; zkir-v3 field variants are unreachable under --rust.
           (or (field-type-native? ftype) (field-type-jubjub-scalar? ftype))]
          [(tboolean ,src) #t]
          [(tbytes ,src ,len) #t]
          [(tenum ,src ,enum-name ,elt-name ,elt-name* ...) #t]
          [(talias ,src ,nominal? ,type-name ,type) (default-supported? type)]
          [else #f]))

      ;; tenum-name-of-type: if `type` is a tenum (possibly through a
      ;; talias chain), return the enum's name symbol; otherwise #f.
      ;; Used at pure-circuit call sites to detect that the formal arg
      ;; expects a user enum and a runtime coercion from the gathered
      ;; AlignedValue is needed.
      (define (tenum-name-of-type type)
        (nanopass-case (Ltypescript Type) type
          [(tenum ,src ,enum-name ,elt-name ,elt-name* ...) enum-name]
          [(talias ,src ,nominal? ,type-name ,type) (tenum-name-of-type type)]
          [else #f]))

      ;; type-rust-copy?: returns #t when the Rust lowering of `type` is
      ;; `Copy` (so passing it to a callee doesn't move the original).
      ;; Used by `arg-rust-clone-if-var` to suppress redundant `.clone()`
      ;; on primitive locals — keeps counter / tiny snapshots byte-stable
      ;; while still defending non-Copy locals (user structs, MerklePath,
      ;; Vec<u8>, OpaqueString, ...) against move-then-reuse.
      ;;
      ;; Returns #f when we don't have a type (so the caller defaults to
      ;; cloning) — the defensive over-clone is safe for non-Copy types
      ;; and a no-op for Copy types we missed.
      (define (type-rust-copy? type)
        (and type
             (nanopass-case (Ltypescript Type) type
               [(tfield ,src ,ftype)
                ;; Fr and JubjubScalar (EmbeddedFr) are both Copy.
                (or (field-type-native? ftype) (field-type-jubjub-scalar? ftype))]
               [(tboolean ,src) #t]
               [(tunsigned ,src ,nat) #t]
               [(tbytes ,src ,len) #t]
               [(ttuple ,src ,type* ...)
                ;; A tuple is Copy iff every element is.
                (let loop ([ts type*])
                  (cond
                    [(null? ts) #t]
                    [(type-rust-copy? (car ts)) (loop (cdr ts))]
                    [else #f]))]
               [(tvector ,src ,len ,type)
                ;; Fixed-size array `[T; N]` is Copy iff T is. (We don't
                ;; lower tvector to Vec for bounded arrays.)
                (type-rust-copy? type)]
               [(talias ,src ,nominal? ,type-name ,type)
                (type-rust-copy? type)]
               [(tenum ,src ,enum-name ,elt-name ,elt-name* ...)
                ;; User enums derive Clone but not Copy (the emitted
                ;; `#[derive]` at H1 does not include Copy).
                #f]
               [(tstruct ,src ,struct-name (,elt-name* ,type*) ...)
                ;; User structs derive Clone but not Copy.
                #f]
               [(topaque ,src ,opaque-type) #f]
               [(tcontract ,src ,contract-name (,elt-name* ,pure-dcl* (,type** ...) ,type*) ...) #f]
               [(tunknown) #f]
               [else #f])))

      ;; circuit-formal-arg-types: pull the list of formal arg types from a
      ;; circuit Program-Element. Returns '() if cdefn is #f or not a
      ;; circuit. Used by F2.2 to align actual args with their declared
      ;; types when emitting pure-circuit call args.
      (define (circuit-formal-arg-types cdefn)
        (cond
          [(not cdefn) '()]
          [else
           (nanopass-case (Ltypescript Program-Element) cdefn
             [(circuit ,src ,function-name (,arg* ...) ,type ,stmt)
              (map (lambda (a)
                     (nanopass-case (Ltypescript Argument) a
                       [(,var-name ,type) type]))
                   arg*)]
             [else '()])]))

      ;; render-pure-circuit-arg: render a single actual arg expression
      ;; for a pure-circuit call. If the formal type is a tenum AND the
      ;; actual is a `(public-ledger ... read)` returning the matching
      ;; tenum, emit a gather block decoded via the enum's FromFieldRepr
      ;; (decode_via_field_repr::<EnumName>) so the call receives the
      ;; actual enum variant rather than the bare u8 discriminant. Other
      ;; shapes fall through to ctor-expr-rust.
      (define (render-pure-circuit-arg actual formal-type local-binds
                                       native-id-ht witness-id-ht circuit-id-ht)
        (let* ([enum-name (tenum-name-of-type formal-type)]
               [e (expr-strip-cast actual)])
          (cond
            [(and enum-name
                  (nanopass-case (Ltypescript Expression) e
                    [(public-ledger ,src ,ledger-field-name ,sugar? (,path-elt* ...) ,src^ ,adt-op ,expr* ...)
                     (and (null? expr*)
                          (nanopass-case (Ltypescript ADT-Op) adt-op
                            [(,ledger-op ,op-class (,adt-name (,adt-formal* ,adt-arg*) ...) ((,var-name* ,type*) ...) ,type ,vm-code)
                             (and (eq? op-class 'read)
                                  (tenum-name-of-type type))])
                          path-elt*)]
                    [else #f]))
             =>
             (lambda (path-elt*)
               (emit-struct-field-zero-read
                 path-elt*
                 (format "midnight_compact_runtime::std_lib::decode_via_field_repr::<~a>"
                         enum-name)))]
            [else
             (arg-rust-clone-if-var actual local-binds
                                    native-id-ht witness-id-ht circuit-id-ht)])))

      ;; elt-ref-of-struct-read?: predicate for the F2.2 narrow case
      ;; `(elt-ref (public-ledger ... read with tstruct return) field 0)`.
      ;; Returns #t when the inner expression is a public-ledger `read`
      ;; whose return type is a tstruct AND the field at index 0 has a
      ;; decoder-for-type. Used by expr-supported? + ctor-expr-rust to
      ;; light up `topic.read().is_some` on Maybe<Opaque<string>>: the
      ;; whole-struct read has no decoder, but projecting `.is_some`
      ;; only needs to decode the leading boolean field, which
      ;; `decode_bool` does on the gathered AlignedValue.
      (define (elt-ref-of-struct-read? inner-expr)
        (let ([e (expr-strip-cast inner-expr)])
          (nanopass-case (Ltypescript Expression) e
            [(public-ledger ,src ,ledger-field-name ,sugar? (,path-elt* ...) ,src^ ,adt-op ,expr* ...)
             (nanopass-case (Ltypescript ADT-Op) adt-op
               [(,ledger-op ,op-class (,adt-name (,adt-formal* ,adt-arg*) ...) ((,var-name* ,type*) ...) ,type ,vm-code)
                (and
                  (eq? op-class 'read)
                  (null? expr*)
                  (let loop ([xs path-elt*])
                    (cond
                      [(null? xs) #t]
                      [else
                       (and (nanopass-case (Ltypescript Path-Element) (car xs)
                              [,path-index #t]
                              [else #f])
                            (loop (cdr xs)))]))
                  ;; Result type is a tstruct whose first field has a
                  ;; decoder. We grab field-0's type via the tstruct's
                  ;; type list.
                  (nanopass-case (Ltypescript Type) type
                    [(tstruct ,src ,struct-name (,elt-name* ,type*) ...)
                     (and (pair? type*)
                          (decoder-for-type (car type*))
                          #t)]
                    [else #f]))])]
            [else #f])))

      ;; struct-read-first-field-decoder: pull the decoder for the leading
      ;; field of the tstruct returned by `(public-ledger ... read)`. The
      ;; caller has already validated the inner shape via
      ;; elt-ref-of-struct-read?, so we just project here. Returns the
      ;; decoder string, or #f defensively.
      (define (struct-read-first-field-decoder inner-expr)
        (let ([e (expr-strip-cast inner-expr)])
          (nanopass-case (Ltypescript Expression) e
            [(public-ledger ,src ,ledger-field-name ,sugar? (,path-elt* ...) ,src^ ,adt-op ,expr* ...)
             (nanopass-case (Ltypescript ADT-Op) adt-op
               [(,ledger-op ,op-class (,adt-name (,adt-formal* ,adt-arg*) ...) ((,var-name* ,type*) ...) ,type ,vm-code)
                (nanopass-case (Ltypescript Type) type
                  [(tstruct ,src ,struct-name (,elt-name* ,type^*) ...)
                   (and (pair? type^*) (decoder-for-type (car type^*)))]
                  [else #f])])]
            [else #f])))

      ;; struct-read-path-elts: pull the path-elt* out of an inner
      ;; (public-ledger ... read) expression. Used by F2.2's elt-ref
      ;; projection emission to build the gather idx_at_index chain
      ;; against the original cell path.
      (define (struct-read-path-elts inner-expr)
        (let ([e (expr-strip-cast inner-expr)])
          (nanopass-case (Ltypescript Expression) e
            [(public-ledger ,src ,ledger-field-name ,sugar? (,path-elt* ...) ,src^ ,adt-op ,expr* ...)
             path-elt*]
            [else #f])))

      ;; ledger-read-supported?: returns #t for the `(public-ledger ...
      ;; read)` shapes emit-ledger-read-expr can render — i.e. op-class
      ;; is `read`, every path-elt is a path-index, and the result type
      ;; has either a decoder-for-type or is a tbytes alias chain.
      (define (ledger-read-supported? path-elt* adt-op)
        (nanopass-case (Ltypescript ADT-Op) adt-op
          [(,ledger-op ,op-class (,adt-name (,adt-formal* ,adt-arg*) ...) ((,var-name* ,type*) ...) ,type ,vm-code)
           (and
             (eq? op-class 'read)
             (let loop ([xs path-elt*])
               (cond
                 [(null? xs) #t]
                 [else
                  (and
                    (nanopass-case (Ltypescript Path-Element) (car xs)
                      [,path-index #t]
                      [else #f])
                    (loop (cdr xs)))]))
             (and (decoder-for-type type) #t))]))
