#!chezscheme

;;; This file is part of Compact.
;;; Copyright (C) 2026 Midnight Foundation
;;; SPDX-License-Identifier: Apache-2.0
;;; Licensed under the Apache License, Version 2.0 (the "License");
;;; you may not use this file except in compliance with the License.
;;; You may obtain a copy of the License at
;;;
;;; http://www.apache.org/licenses/LICENSE-2.0
;;;
;;; Unless required by applicable law or agreed to in writing, software
;;; distributed under the License is distributed on an "AS IS" BASIS,
;;; WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
;;; See the License for the specific language governing permissions and
;;; limitations under the License.

;;; The Rust target starts from Lnodisclose, before any TypeScript-specific
;;; pass. Scheme owns Compact semantics and emits a closed JSON domain model;
;;; compact-rust-backend constructs and prints Rust syntax from that model.

(library (rust-ir-passes)
  (export rust-ir-passes)
  (import (except (chezscheme) errorf)
          (utils)
          (nanopass)
          (json)
          (langs)
          (pass-helpers))

  (define-pass emit-rust-ir : Lnodisclose (ir) -> Lnodisclose ()
    (definitions
      (define (object . fields) fields)

      (define (kind name)
        (object (cons "kind" name)))

      (define (type-ir ty owner-src)
        (nanopass-case (Lnodisclose Type) ty
          [(tboolean ,src) (kind "boolean")]
          [(tfield ,src) (kind "field")]
          [(ttuple ,src ,type* ...)
           (if (null? type*)
               (kind "unit")
               (object (cons "kind" "tuple")
                       (cons "elements" (list->vector (map (lambda (ty) (type-ir ty owner-src)) type*)))))]
          [(tvector ,src ,len ,type)
           (object (cons "kind" "vector")
                   (cons "length" len)
                   (cons "element" (type-ir type owner-src)))]
          [(talias ,src ,nominal? ,type-name ,type)
           (type-ir type owner-src)]
          [else (source-errorf owner-src "Rust backend does not yet support this Compact type")]))

      (define (tuple-argument-ir arg owner-src)
        (nanopass-case (Lnodisclose Tuple-Argument) arg
          [(single ,src ,expr) (expression-ir expr owner-src)]
          [else (source-errorf owner-src "Rust backend does not yet support tuple spreads")]))

      (define (expression-ir expr owner-src)
        (nanopass-case (Lnodisclose Expression) expr
          [(return ,src ,expr) (expression-ir expr owner-src)]
          [(var-ref ,src ,var-name)
           (object (cons "kind" "parameter")
                   (cons "name" (symbol->string (id-sym var-name))))]
          [(quote ,src ,datum)
           (if (boolean? datum)
               (object (cons "kind" "boolean") (cons "value" datum))
               (source-errorf src "Rust backend does not yet support this literal"))]
          [(tuple ,src ,tuple-arg* ...)
           (if (null? tuple-arg*)
               (kind "unit")
               (object (cons "kind" "tuple")
                       (cons "elements"
                             (list->vector
                               (map (lambda (arg) (tuple-argument-ir arg owner-src)) tuple-arg*)))))]
          [(+ ,src ,mbits ,expr1 ,expr2)
           (when mbits
             (source-errorf src "Rust backend does not yet support bounded unsigned arithmetic"))
           (object (cons "kind" "add")
                   (cons "left" (expression-ir expr1 owner-src))
                   (cons "right" (expression-ir expr2 owner-src)))]
          [else (source-errorf owner-src "Rust backend does not yet support this circuit expression")]))

      (define (argument-ir arg owner-src)
        (nanopass-case (Lnodisclose Argument) arg
          [(,var-name ,type)
           (object (cons "name" (symbol->string (id-sym var-name)))
                   (cons "ty" (type-ir type owner-src)))]))

      (define (exported-names function-name export-alist)
        (fold-right
          (lambda (entry names)
            (if (eq? (cdr entry) function-name)
                (cons (symbol->string (car entry)) names)
                names))
          '()
          export-alist))

      (define (circuit-ir pelt export-alist circuits)
        (nanopass-case (Lnodisclose Program-Element) pelt
          [(circuit ,src ,function-name (,arg* ...) ,type ,expr)
           (let ([names (exported-names function-name export-alist)])
             (if (null? names)
                 circuits
                 (begin
                   (unless (id-pure? function-name)
                     (source-errorf src "Rust backend currently supports pure circuits only"))
                   (append
                     (map
                       (lambda (name)
                         (object (cons "name" name)
                                 (cons "parameters" (list->vector (map (lambda (arg) (argument-ir arg src)) arg*)))
                                 (cons "result" (type-ir type src))
                                 (cons "body" (expression-ir expr src))))
                       names)
                     circuits))))]
          [else circuits]))

      (define (check-supported-declaration pelt owner-src)
        (nanopass-case (Lnodisclose Program-Element) pelt
          [(witness ,src ,function-name (,arg* ...) ,type)
           (source-errorf src "Rust backend does not yet support witnesses")]
          [(public-ledger-declaration ,pl-array ,lconstructor)
           (nanopass-case (Lnodisclose Public-Ledger-Array) pl-array
             [(public-ledger-array ,pl-array-elt* ...)
              (unless (null? pl-array-elt*)
                (source-errorf owner-src
                               "Rust backend does not yet support ledger fields"))])]
          [else (void)])))

    (Program : Program (ir) -> Program ()
      [(program ,src (,contract-name* ...) ((,export-name* ,name*) ...) ,pelt* ...)
       (for-each (lambda (pelt) (check-supported-declaration pelt src)) pelt*)
       (let ([export-alist (map cons export-name* name*)])
         (print-json
           (get-target-port 'rust.ir.json)
           (object (cons "schema_version" 1)
                   (cons "circuits"
                         (list->vector
                           (fold-right
                             (lambda (pelt circuits) (circuit-ir pelt export-alist circuits))
                             '()
                             pelt*))))))
       ir]))

  (define-passes rust-ir-passes
    (emit-rust-ir Lnodisclose)))
