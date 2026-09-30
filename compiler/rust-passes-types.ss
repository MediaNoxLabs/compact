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

;;; Compact type -> Rust type mapping.
;;;
;;; `type-rust` is the core translation. Also owns struct fingerprinting
;;; (used to dedup structurally identical generic instantiations) and the
;;; "problematic type" predicates that mark shapes needing special
;;; handling rather than the default rendering.
;;;
;;; See compiler/README-rust-passes.md for the module map.

      ;; type-rust: the Rust type for an Ltypescript Type node.
      ;;
      ;; PROTOTYPE (compact#95): this used to build the string directly. It
      ;; now builds a `rust-type-ir` value and prints it. The output is
      ;; byte-identical — that is the point of the first step; what changes is
      ;; what the code is *able* to say.
      (define (type-rust type)
        (rust-type->string (type->rust-type type)))

      ;; type->rust-type: Ltypescript Type -> rust-type-ir.
      ;;
      ;; Three arms below used to produce `/* TODO M3-F4: ... */` strings.
      ;; That text is valid-looking Rust in type position: `compactc` exits 0,
      ;; the fixture regenerates and agrees with itself, and the failure
      ;; surfaces at `cargo build` with nothing pointing back at the Compact
      ;; source. They now raise at the lowering site, because the
      ;; representation has no way to carry them: there is no constructor that
      ;; takes arbitrary Rust text.
      (define (type->rust-type type)
        (nanopass-case (Ltypescript Type) type
          [(tfield ,src) (make-rt-prim "Fr")]
          [(tboolean ,src) (make-rt-prim "bool")]
          [(tunsigned ,src ,nat) (make-rt-prim (uint-rust-width nat))]
          [(tbytes ,src ,len) (make-rt-array (make-rt-prim "u8") len)]
          [(ttuple ,src ,type* ...)
           (make-rt-tuple (map type->rust-type type*))]
          [(tvector ,src ,len ,type)
           (make-rt-array (type->rust-type type) len)]
          [(talias ,src ,nominal? ,type-name ,type)
           ;; A nominal alias is an opaque name by design — the user expects
           ;; `MyId` in a signature, not the expanded form. A transparent
           ;; alias expands.
           (if nominal?
               (make-rt-path (list (symbol->string type-name)))
               (type->rust-type type))]
          [(topaque ,src ,opaque-type)
           (cond
             [(equal? opaque-type "string")
              (make-rt-path (list "midnight_compact_runtime" "std_lib" "OpaqueString"))]
             [(equal? opaque-type "Uint8Array")
              (make-rt-generic (make-rt-path (list "Vec")) (list (make-rt-prim "u8")))]
             [(equal? opaque-type "JubjubPoint")
              (make-rt-path (list "JubjubPoint"))]
             [else
              ;; was: "/* TODO M3-F4: topaque ~a */"
              (rust-type-unlowerable src 'opaque-type
                "Opaque<~s> has no Rust lowering" opaque-type)])]
          [(tstruct ,src ,struct-name (,elt-name* ,type*) ...)
           (let ([entry (lookup-stdlib-struct struct-name)])
             (if entry
                 ;; The stdlib mappings render their own text (Maybe<T>,
                 ;; MerkleTreePath<#n, T>). Modelling them properly means
                 ;; giving each a generic shape; until then they are the
                 ;; measured remainder, not a hidden one.
                 (make-rt-foreign ((car entry) elt-name* type*) 'stdlib-struct)
                 (make-rt-path (list (symbol->string (struct-rust-name type))))))]
          [(tenum ,src ,enum-name ,elt-name ,elt-name* ...)
           (make-rt-path (list (symbol->string enum-name)))]
          [(tcontract ,src ,contract-name (,elt-name* ,pure-dcl* (,type** ...) ,type*) ...)
           ;; A reference to another contract lowers to its address, as the
           ;; TypeScript path does.
           (make-rt-prim "ContractAddress")]
          ;; was: "/* TODO M3-F4: tunknown */"
          [(tunknown)
           (rust-type-unlowerable #f 'unknown-type
             "a value reached the Rust backend with no type information")]
          ;; was: "/* TODO M3-F4: unhandled type variant */"
          [else
           (rust-type-unlowerable #f 'type-variant
             "no Rust lowering for the type ~a" (unparse-Ltypescript type))]))

      ;; type-fingerprint: a disambiguation-INDEPENDENT structural key for a
      ;; Type node. Unlike type-rust it never consults the struct rename
      ;; tables, so it renders the same whether computed while the tables are
      ;; empty (during build-struct-rust-name-ht) or full (at a resolution
      ;; site) — the stability struct-rust-name's fp fallback relies on. It
      ;; recurses through nested struct / enum / vector / tuple / transparent-
      ;; alias structure so a nested collision is reflected in the outer key:
      ;; two module-collided `Outer { inner: Inner }` whose `Inner` bodies
      ;; differ must produce different `Outer` fingerprints, which a shallow
      ;; `(type-rust field)` (bare name "Inner" for both) would not. Leaf /
      ;; nominal forms delegate to type-rust (stable — it touches no table for
      ;; primitives, and a nominal alias is an opaque name by design).
      (define (type-fingerprint type)
        (nanopass-case (Ltypescript Type) type
          [(tstruct ,src ,struct-name (,elt-name* ,type*) ...)
           (cons 'struct
                 (cons struct-name
                       (map (lambda (n t) (cons n (type-fingerprint t))) elt-name* type*)))]
          [(tenum ,src ,enum-name ,elt-name ,elt-name* ...)
           (cons 'enum (cons enum-name (cons elt-name elt-name*)))]
          [(tvector ,src ,len ,type) (list 'vec len (type-fingerprint type))]
          [(ttuple ,src ,type* ...) (cons 'tuple (map type-fingerprint type*))]
          [(talias ,src ,nominal? ,type-name ,type)
           ;; Match type-rust: a nominal alias is its own opaque name; a
           ;; transparent alias expands (recurse so a struct underneath is
           ;; seen structurally).
           (if nominal? (list 'alias type-name) (type-fingerprint type))]
          ;; PROTOTYPE (compact#95): was `(type-rust type)`. A fingerprint is a
          ;; structural *key*, not emitted Rust, so it must stay total over
          ;; every Type node — including the ones that have no Rust lowering.
          ;; Delegating to `type-rust` was harmless only while unlowerable
          ;; types produced a placeholder string; once they refuse, computing
          ;; a key for `export {Maybe}` fails on its type variable `T`.
          ;; The vehicle hit the identical interaction during CPT-008 and
          ;; resolved it the same way.
          [else (list 'other (unparse-Ltypescript type))]))

      ;; tstruct-fingerprint: a structural fingerprint of a tstruct/tenum
      ;; Type node, used to distinguish two `import M<...>` instantiations
      ;; that share a struct-name but have field-distinct bodies. Returns
      ;; `(struct-name . (field-name . field-fingerprint) ...)` for tstruct,
      ;; `(enum-name . variants)` for tenum, or #f for other types. Field
      ;; types use type-fingerprint (recursive, table-independent) so two
      ;; variants differing only in a nested colliding struct still
      ;; fingerprint distinctly.
      (define (tstruct-fingerprint type)
        (nanopass-case (Ltypescript Type) type
          [(tstruct ,src ,struct-name (,elt-name* ,type*) ...)
           (cons struct-name
                 (map (lambda (n t) (cons n (type-fingerprint t))) elt-name* type*))]
          [(tenum ,src ,enum-name ,elt-name ,elt-name* ...)
           (cons enum-name (cons elt-name elt-name*))]
          [else #f]))

      ;; -----------------------------------------------------------------
      ;; M3.5 helpers: per-field codegen for Aligned / FieldRepr /
      ;; FromFieldRepr impls of user structs.
      ;;
      ;; Rust's orphan rules forbid us from impl'ing those upstream traits
      ;; on foreign types like `[u8; N]` (N != 32), `Vec<u8>`, or
      ;; `[UserType; N]`. To sidestep that, when a struct field has one of
      ;; these "problematic" types we emit:
      ;;   - the FIELD_SIZE / Aligned::concat / field_size() / field_repr()
      ;;     pieces inline (computing what `<T as Trait>::method` would
      ;;     have returned), and
      ;;   - a call to a free `midnight_compact_runtime::*_from_field_repr`
      ;;     helper for the parse side.
      ;; -----------------------------------------------------------------

      ;; Recognise tbytes with a non-32 length.
      (define (problematic-bytes? type)
        (nanopass-case (Ltypescript Type) type
          [(tbytes ,src ,len) (not (= len 32))]
          [else #f]))

      ;; tbytes-len-over-32?: #t when `type` is a `(tbytes src len)` (peeling
      ;; talias) with `len > 32`. Rust's std impls `Default` for `[T; N]`
      ;; only up to N=32 (the const-generic blanket was never stabilised),
      ;; so a struct with a `Bytes<64>` field cannot `#[derive(Default)]`.
      ;; Used by emit-type-decls to switch to a manual `impl Default`.
      (define (tbytes-len-over-32? type)
        (nanopass-case (Ltypescript Type) type
          [(tbytes ,src ,len) (> len 32)]
          [(talias ,src ,nominal? ,type-name ,type) (tbytes-len-over-32? type)]
          [else #f]))

      ;; tvector-len-over-32?: same as tbytes-len-over-32? but for
      ;; `(tvector src len elt)` — `Vector<50,T>` lowers to `[T; 50]` which
      ;; likewise lacks `Default`.
      (define (tvector-len-over-32? type)
        (nanopass-case (Ltypescript Type) type
          [(tvector ,src ,len ,type) (> len 32)]
          [(talias ,src ,nominal? ,type-name ,type) (tvector-len-over-32? type)]
          [else #f]))

      ;; R5a: Recognise the `JubjubPoint` opaque type. Lowered through
      ;; Ltypescript as `tstruct 'JubjubPoint` with no body fields.
      ;; Upstream `EmbeddedGroupAffine` (= JubjubPoint) has an `Aligned`
      ;; impl but no `FieldRepr` / `FromFieldRepr` / `BinaryHashRepr`,
      ;; and Rust's orphan rules forbid us from supplying them
      ;; downstream. Codegen routes JubjubPoint-typed struct fields
      ;; through midnight_compact_runtime::jubjub_point_* free functions.
      (define (problematic-jubjub-point? type)
        (nanopass-case (Ltypescript Type) type
          [(topaque ,src ,opaque-type)
           (equal? opaque-type "JubjubPoint")]
          [(talias ,src ,nominal? ,type-name ,type)
           (problematic-jubjub-point? type)]
          [else #f]))

      ;; Recognise tvector whose element type has no `[T; N]` repr impls
      ;; available upstream. Covers:
      ;;   - User structs / enums (no upstream `[T; N]` for non-u8 T).
      ;;   - R5b: tfield — `[Fr; N]` (Schnorr digest, etc.) — upstream
      ;;     impls FromFieldRepr/FieldRepr on Fr alone but not on the
      ;;     fixed-size array. Codegen routes through array_from_field_repr
      ;;     and the iter().map().sum() field_size pattern.
      (define (problematic-vector? type)
        (nanopass-case (Ltypescript Type) type
          [(tvector ,src ,len ,type)
           (nanopass-case (Ltypescript Type) type
             [(tstruct ,src ,struct-name (,elt-name* ,type*) ...)
              (not (eq? struct-name 'Maybe))]
             [(tenum ,src ,enum-name ,elt-name ,elt-name* ...) #t]
             [(tfield ,src) #t]
             [else #f])]
          [else #f]))

      ;; Recognise Opaque<"Uint8Array"> which lowers to Vec<u8>.
      (define (problematic-vec-u8? type)
        (nanopass-case (Ltypescript Type) type
          [(topaque ,src ,opaque-type) (equal? opaque-type "Uint8Array")]
          [else #f]))

      ;; field-size-const-expr: compile-time expression for
      ;; `<T as FromFieldRepr>::FIELD_SIZE`. For problematic types,
      ;; substitute a runtime helper / literal.
      (define (field-size-const-expr type)
        (cond
          [(problematic-bytes? type)
           (nanopass-case (Ltypescript Type) type
             [(tbytes ,src ,len)
              (format "midnight_compact_runtime::bytes_field_size(~a)" len)])]
          [(problematic-vec-u8? type)
           ;; Vec<u8> has no fixed FIELD_SIZE; codegen treats this as 0
           ;; (the surrounding ADT carries the byte count).
           "0"]
          [(problematic-jubjub-point? type)
           ;; R5a: orphan-safe const from midnight_compact_runtime.
           "midnight_compact_runtime::JUBJUB_POINT_FIELD_SIZE"]
          [(problematic-vector? type)
           (nanopass-case (Ltypescript Type) type
             [(tvector ,src ,len ,type)
              (format "<~a as FromFieldRepr>::FIELD_SIZE * ~a"
                      (type-rust type) len)])]
          [else
           (format "<~a as FromFieldRepr>::FIELD_SIZE" (type-rust type))]))

      ;; field-from-repr-expr: parse one field from the slice
      ;; `r[_offset.._offset + size]` and bind to `name`. For
      ;; problematic types, call into runtime helpers.
      (define (emit-field-from-repr name type)
        (let ([size-expr (field-size-const-expr type)])
          (cond
            [(problematic-bytes? type)
             (nanopass-case (Ltypescript Type) type
               [(tbytes ,src ,len)
                (out (format "        let ~a = midnight_compact_runtime::bytes_from_field_repr::<~a>(&_repr[_offset.._offset + ~a])?;\n"
                             name len size-expr))
                (out (format "        _offset += ~a;\n" size-expr))])]
            [(problematic-vec-u8? type)
             (out (format "        let ~a = midnight_compact_runtime::vec_u8_from_field_repr(&_repr[_offset.._offset])?;\n"
                          name))]
            [(problematic-jubjub-point? type)
             ;; R5a: orphan-safe parse via midnight_compact_runtime helper.
             (out (format "        let ~a = midnight_compact_runtime::jubjub_point_from_field_repr(&_repr[_offset.._offset + ~a])?;\n"
                          name size-expr))
             (out (format "        _offset += ~a;\n" size-expr))]
            [(problematic-vector? type)
             (nanopass-case (Ltypescript Type) type
               [(tvector ,src ,len ,type)
                (out (format "        let ~a = midnight_compact_runtime::array_from_field_repr::<~a, ~a>(&_repr[_offset.._offset + ~a], <~a as FromFieldRepr>::FIELD_SIZE)?;\n"
                             name (type-rust type) len size-expr (type-rust type)))
                (out (format "        _offset += ~a;\n" size-expr))])]
            [else
             (let ([rust-ty (type-rust type)])
               (out (format "        let ~a = <~a as FromFieldRepr>::from_field_repr(&_repr[_offset.._offset + <~a as FromFieldRepr>::FIELD_SIZE])?;\n"
                            name rust-ty rust-ty))
               (out (format "        _offset += <~a as FromFieldRepr>::FIELD_SIZE;\n" rust-ty)))])))

      ;; alignment-expr: emit a `&Alignment` reference for use inside
      ;; `Alignment::concat([...])`. For `[T; N]` of user types,
      ;; Alignment::concat needs N references which we cannot easily
      ;; produce inline — synthesise a small helper expression that
      ;; builds a temporary Vec<&Alignment>.
      ;;
      ;; Since `Alignment::concat` takes `IntoIterator<Item = &Alignment>`,
      ;; we can build an expression that does the work inline.
      (define (emit-alignment-piece type first?)
        (cond
          [(problematic-vector? type)
           (nanopass-case (Ltypescript Type) type
             [(tvector ,src ,len ,type)
              ;; Emit a Box-leaked vector of N copies of T's alignment.
              ;; Simpler: just emit N comma-separated &T::alignment() calls.
              (let loop ([i 0])
                (when (< i len)
                  (out (format "~a&<~a as Aligned>::alignment()"
                               (if (and first? (= i 0)) "" ", ")
                               (type-rust type)))
                  (loop (+ i 1))))])]
          [else
           (out (format "~a&<~a as Aligned>::alignment()"
                        (if first? "" ", ")
                        (type-rust type)))]))

      ;; field-size-instance-expr: runtime field.field_size() expression
      ;; for use in the per-instance field_size() summation.
      (define (field-size-instance-expr field-name type)
        (cond
          [(problematic-vector? type)
           ;; iter().map(|e| e.field_size()).sum() avoids requiring
           ;; FieldRepr to be impl'd on [T; N].
           (format "self.~a.iter().map(|e| e.field_size()).sum::<usize>()" field-name)]
          [(problematic-jubjub-point? type)
           ;; R5a: orphan-safe field_size.
           (format "midnight_compact_runtime::jubjub_point_field_size(&self.~a)" field-name)]
          [else
           (format "self.~a.field_size()" field-name)]))

      ;; field-repr-emit: write the per-field `self.x.field_repr(writer)`
      ;; or equivalent loop for `[T; N]` of user types.
      (define (emit-field-repr-call field-name type)
        (cond
          [(problematic-vector? type)
           (out (format "        for _e in self.~a.iter() { _e.field_repr(writer); }\n"
                        field-name))]
          [(problematic-jubjub-point? type)
           ;; R5a: orphan-safe field_repr.
           (out (format "        midnight_compact_runtime::jubjub_point_field_repr(&self.~a, writer);\n"
                        field-name))]
          [else
           (out (format "        self.~a.field_repr(writer);\n" field-name))]))
