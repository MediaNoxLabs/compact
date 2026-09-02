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
;;; This file: struct and Maybe type helpers, struct literals.

      ;; unit-type?: returns #t if a Type IR node is the empty tuple `()`
      ;; (Compact's `Void` / Ltypescript `(ttuple src)` with no element
      ;; types). I3a only emits bodies for unit-returning circuits; richer
      ;; return shapes (e.g. tiny.compact's `get(): Maybe<Fr>`) keep the
      ;; `unimplemented!()` fallback until I3b.
      (define (unit-type? type)
        (nanopass-case (Ltypescript Type) type
          [(ttuple ,src ,type* ...) (null? type*)]
          [else #f]))

      ;; struct-of-type: if `type` is a tstruct (possibly through a talias
      ;; chain), return (list struct-name elt-name* type*); otherwise #f.
      ;; Used by F2.2's struct-literal emission to recover the field-name
      ;; list (the IR's `(new ...)` carries field initialisers in source
      ;; order but not the names — those come off the struct's type).
      (define (struct-of-type type)
        (nanopass-case (Ltypescript Type) type
          [(tstruct ,src ,struct-name (,elt-name* ,type*) ...)
           (list struct-name elt-name* type*)]
          [(talias ,src ,nominal? ,type-name ,type) (struct-of-type type)]
          [else #f]))

      ;; tstruct-node-of-type: peel talias chains and return the underlying
      ;; tstruct Type NODE (not its parts), or #f. Callers pass the result to
      ;; struct-rust-name so a colliding struct resolves to its disambiguated
      ;; name (Name / Name_1) at value / decode / default sites, not just in
      ;; type positions. Returning the node (rather than the bare name) lets
      ;; struct-rust-name key on eq? identity or structural fingerprint.
      (define (tstruct-node-of-type type)
        (nanopass-case (Ltypescript Type) type
          [(tstruct ,src ,struct-name (,elt-name* ,type*) ...) type]
          [(talias ,src ,nominal? ,type-name ,type) (tstruct-node-of-type type)]
          [else #f]))

      ;; struct-rust-name-of: the disambiguated Rust struct name string for a
      ;; (possibly alias-wrapped) struct type. Falls back to the bare
      ;; struct-name symbol when the type isn't a tstruct (defensive; callers
      ;; already know it is a struct).
      (define (struct-rust-name-of type fallback-name)
        (let ([node (tstruct-node-of-type type)])
          (if node
              (symbol->string (struct-rust-name node))
              (symbol->string fallback-name))))

      ;; render-struct-literal: F2.2 — emit a Rust struct construction
      ;; expression for an Ltypescript `(new src type expr*)`. The struct
      ;; name comes off the type (via struct-of-type); for Maybe<T> we emit
      ;; the L1 runtime alias `Maybe` (in scope via the contract module
      ;; preamble), for user structs the bare struct name (H5-H7 emit the
      ;; struct definition into the contract module).
      ;;
      ;; Field initialiser exprs are rendered through ctor-expr-rust so
      ;; var-refs resolve against the current local-binds and nested
      ;; ledger reads / calls / etc. lower correctly.
      (define (render-struct-literal src type expr* local-binds
                                     native-id-ht witness-id-ht circuit-id-ht)
        (let* ([st (struct-of-type type)]
               [struct-name (and st (car st))]
               [elt-name* (and st (cadr st))])
          (cond
            [(not st)
             (rust-feature-error src 'struct-literal-non-tstruct
               "struct-literal of non-tstruct type")]
            [(not (fx= (length expr*) (length elt-name*)))
             (rust-feature-error src 'struct-literal-field-count-mismatch
               "struct-literal field-count mismatch for ~a (expected ~a, got ~a)"
               struct-name (length elt-name*) (length expr*))]
            [else
             (let* ([rust-struct-name (struct-rust-name-of type struct-name)]
                    [field-strs
                     (map (lambda (name e)
                            (format "~a: ~a"
                                    (symbol->string name)
                                    (ctor-expr-rust e local-binds
                                                    native-id-ht witness-id-ht circuit-id-ht)))
                          elt-name* expr*)])
               (string-append
                 rust-struct-name
                 " { "
                 (let join ([xs field-strs] [acc ""])
                   (cond
                     [(null? xs) acc]
                     [(null? (cdr xs)) (string-append acc (car xs))]
                     [else (join (cdr xs)
                                 (string-append acc (car xs) ", "))]))
                 " }"))])))

      ;; maybe-value-type: if `type` is `Maybe<T>` (a tstruct named Maybe with
      ;; a `value` field), return T. Otherwise return #f. Used by the
      ;; I3b/4 if-expression emitter to render `some::<T>` / `none::<T>` with
      ;; an explicit generic argument so Rust's type inference doesn't need
      ;; help from surrounding context.
      (define (maybe-value-type type)
        (nanopass-case (Ltypescript Type) type
          [(tstruct ,src ,struct-name (,elt-name* ,type*) ...)
           (cond
             [(eq? struct-name 'Maybe)
              (let loop ([names elt-name*] [types type*])
                (cond
                  [(null? names) #f]
                  [(eq? (car names) 'value) (car types)]
                  [else (loop (cdr names) (cdr types))]))]
             [else #f])]
          [(talias ,src ,nominal? ,type-name ,type^) (maybe-value-type type^)]
          [else #f]))
