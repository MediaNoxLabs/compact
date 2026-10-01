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

      (define call-argument-types (make-eq-hashtable))
      (define current-variable-types (make-parameter #f))
      (define struct-shape-names '())
      (define used-struct-names '())

      ;; Module expansion can leave distinct structs with the same source
      ;; name. Use the complete field structure, including nested structs,
      ;; to choose one stable Rust name at every type and value site.
      (define (struct-rust-name source-name fields)
        (let* ([source-name (symbol->string source-name)]
               [fingerprint (cons source-name fields)]
               [existing (assoc fingerprint struct-shape-names)])
          (if existing
              (cdr existing)
              (let ([assigned
                      (if (member source-name used-struct-names)
                          (let loop ([suffix 1])
                            (let ([candidate (format "~aCompact~a" source-name suffix)])
                              (if (member candidate used-struct-names)
                                  (loop (+ suffix 1))
                                  candidate)))
                          source-name)])
                (set! struct-shape-names
                      (cons (cons fingerprint assigned) struct-shape-names))
                (set! used-struct-names (cons assigned used-struct-names))
                assigned))))

      (define (kind name)
        (object (cons "kind" name)))

      (define (type-ir ty owner-src)
        (nanopass-case (Lnodisclose Type) ty
          [(tboolean ,src) (kind "boolean")]
          [(tfield ,src) (kind "field")]
          [(topaque ,src ,opaque-type)
           (if (string=? opaque-type "JubjubPoint")
               (kind "jubjub_point")
               (source-errorf src "Rust backend does not yet support this opaque type"))]
          [(tbytes ,src ,len)
           (object (cons "kind" "bytes")
                   (cons "length" len))]
          [(tstruct ,src ,struct-name (,elt-name* ,type*) ...)
           (let ([fields
                   (list->vector
                     (map (lambda (name ty)
                            (object (cons "name" (symbol->string name))
                                    (cons "ty" (type-ir ty owner-src))))
                          elt-name* type*))])
             (object (cons "kind" "struct")
                     (cons "name" (struct-rust-name struct-name fields))
                     (cons "fields" fields)))]
          [(tenum ,src ,enum-name ,elt-name ,elt-name* ...)
           (object (cons "kind" "enum")
                   (cons "name" (symbol->string enum-name))
                   (cons "variants"
                         (list->vector (map symbol->string (cons elt-name elt-name*)))))]
          [(tunsigned ,src ,nat)
           (object (cons "kind" "unsigned")
                   (cons "max" (number->string nat)))]
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

      (define (maybe-nonnegative-integer-literal expr owner-src)
        (nanopass-case (Lnodisclose Expression) expr
          [(return ,src ,expr) (maybe-nonnegative-integer-literal expr src)]
          [(quote ,src ,datum)
           (if (and (integer? datum) (<= 0 datum))
               datum
               (source-errorf src "Rust backend supports nonnegative numeric literals only"))]
          [else #f]))

      (define (field-argument-ir expr owner-src)
        (nanopass-case (Lnodisclose Expression) expr
          [(quote ,src ,datum)
           (if (and (integer? datum) (<= 0 datum))
               (object (cons "kind" "field_literal")
                       (cons "value" (number->string datum)))
               (source-errorf src "Rust backend Field literal must be nonnegative"))]
          [else (expression-ir expr owner-src)]))

      (define (let-expression-ir local* expr* body expected-type src)
        (when (current-variable-types)
          (for-each (lambda (local)
                      (nanopass-case (Lnodisclose Argument) local
                        [(,var-name ,type)
                         (eq-hashtable-set! (current-variable-types) var-name type)]))
                    local*))
        (object (cons "kind" "let")
                (cons "bindings"
                      (list->vector
                        (map (lambda (local value)
                               (nanopass-case (Lnodisclose Argument) local
                                 [(,var-name ,type)
                                  (object (cons "name" (symbol->string (id-sym var-name)))
                                          (cons "ty" (type-ir type src))
                                          (cons "value" (typed-expression-ir value type src)))]))
                             local* expr*)))
                (cons "body"
                      (if expected-type
                          (typed-expression-ir body expected-type src)
                          (expression-ir body src)))))

      ;; Analysis inserts this guard for Uint subtraction. The native checked
      ;; subtraction performs the same comparison, so only this exact guard
      ;; may be folded into the arithmetic expression.
      (define (checked-unsigned-subtraction? statements tail)
        (and (= (length statements) 1)
             (nanopass-case (Lnodisclose Expression) tail
               [(- ,src ,mbits ,expr1 ,expr2)
                (and mbits
                     (nanopass-case (Lnodisclose Expression) (car statements)
                       [(assert ,src1 ,expr5 ,mesg)
                        (and (equal? mesg "result of subtraction would be negative")
                             (nanopass-case (Lnodisclose Expression) expr5
                               [(>= ,src2 ,bits ,expr3 ,expr4)
                                (and (= bits mbits)
                                     (equal? (expression-ir expr3 src)
                                             (expression-ir expr1 src))
                                     (equal? (expression-ir expr4 src)
                                             (expression-ir expr2 src)))]
                               [else #f]))]
                       [else #f]))]
               [else #f])))

      (define (expression-ir expr owner-src)
        (nanopass-case (Lnodisclose Expression) expr
          [(return ,src ,expr) (expression-ir expr owner-src)]
          [(var-ref ,src ,var-name)
           (object (cons "kind" "parameter")
                   (cons "name" (symbol->string (id-sym var-name))))]
          [(elt-ref ,src ,expr ,elt-name ,nat)
           (object (cons "kind" "struct_field")
                   (cons "value" (expression-ir expr src))
                   (cons "field" (symbol->string elt-name))
                   (cons "index" nat))]
          [(new ,src ,type ,expr* ...)
           (nanopass-case (Lnodisclose Type) type
             [(tstruct ,src^ ,struct-name (,elt-name* ,type*) ...)
              (unless (= (length expr*) (length type*))
                (source-errorf src "Rust struct literal field count does not match its type"))
              (object (cons "kind" "struct_literal")
                      (cons "ty" (type-ir type src))
                      (cons "fields"
                            (list->vector
                              (map (lambda (value field-type)
                                     (typed-expression-ir value field-type src))
                                   expr* type*))))]
             [else (source-errorf src "Rust backend does not yet support this struct literal")])]
          [(quote ,src ,datum)
           (cond
             [(boolean? datum)
              (object (cons "kind" "boolean") (cons "value" datum))]
             [(bytevector? datum)
              (object (cons "kind" "bytes_literal")
                      (cons "bytes"
                            (list->vector
                              (let loop ([index 0] [bytes '()])
                                (if (= index (bytevector-length datum))
                                    (reverse bytes)
                                    (loop (+ index 1)
                                          (cons (bytevector-u8-ref datum index) bytes)))))))]
             [else (source-errorf src "Rust backend does not yet support this literal")])]
          [(default ,src ,type)
           (nanopass-case (Lnodisclose Type) type
             [(tboolean ,src^) (object (cons "kind" "boolean") (cons "value" #f))]
             [(tfield ,src^) (object (cons "kind" "field_literal") (cons "value" "0"))]
             [(tunsigned ,src^ ,nat)
              (object (cons "kind" "unsigned_literal")
                      (cons "value" "0")
                      (cons "max" (number->string nat)))]
             [else (object (cons "kind" "default")
                           (cons "ty" (type-ir type src)))])]
          [(safe-cast ,src ,type ,type^ ,expr)
           (nanopass-case (Lnodisclose Type) type
             [(tfield ,src^)
              (let ([value (maybe-nonnegative-integer-literal expr src)])
                (if value
                    (object (cons "kind" "field_literal")
                            (cons "value" (number->string value)))
                    (nanopass-case (Lnodisclose Type) type^
                      [(tunsigned ,src1 ,nat)
                       (object (cons "kind" "field_cast")
                               (cons "value" (typed-expression-ir expr type^ src)))]
                      [else (source-errorf src "Rust backend does not yet support this Field cast")])))]
             [(tunsigned ,src^ ,nat)
              (let ([value (maybe-nonnegative-integer-literal expr src)])
                (if value
                    (begin
                      (unless (<= value nat)
                        (source-errorf src "Rust backend Uint literal exceeds its maximum"))
                      (object (cons "kind" "unsigned_literal")
                              (cons "value" (number->string value))
                              (cons "max" (number->string nat))))
                    (object (cons "kind" "unsigned_cast")
                            (cons "max" (number->string nat))
                            (cons "value" (typed-expression-ir expr type^ src)))))]
             [(tvector ,src^ ,len ,type^)
              (typed-expression-ir expr type src)]
             [(ttuple ,src^ ,type* ...)
              (typed-expression-ir expr type src)]
             [else (source-errorf src "Rust backend does not yet support this cast")])]
          [(tuple ,src ,tuple-arg* ...)
           (if (null? tuple-arg*)
               (kind "unit")
               (object (cons "kind" "tuple")
                       (cons "elements"
                             (list->vector
                               (map (lambda (arg) (tuple-argument-ir arg owner-src)) tuple-arg*)))))]
          [(if ,src ,expr0 ,expr1 ,expr2)
           (object (cons "kind" "if")
                   (cons "condition" (expression-ir expr0 src))
                   (cons "then" (expression-ir expr1 src))
                   (cons "otherwise" (expression-ir expr2 src)))]
          [(== ,src ,type ,expr1 ,expr2)
           (object (cons "kind" "equal")
                   (cons "left" (typed-expression-ir expr1 type src))
                   (cons "right" (typed-expression-ir expr2 type src)))]
          [(< ,src ,bits ,expr1 ,expr2)
           (object (cons "kind" "compare") (cons "operator" "less")
                   (cons "left" (expression-ir expr1 src))
                   (cons "right" (expression-ir expr2 src)))]
          [(<= ,src ,bits ,expr1 ,expr2)
           (object (cons "kind" "compare") (cons "operator" "less_equal")
                   (cons "left" (expression-ir expr1 src))
                   (cons "right" (expression-ir expr2 src)))]
          [(> ,src ,bits ,expr1 ,expr2)
           (object (cons "kind" "compare") (cons "operator" "greater")
                   (cons "left" (expression-ir expr1 src))
                   (cons "right" (expression-ir expr2 src)))]
          [(>= ,src ,bits ,expr1 ,expr2)
           (object (cons "kind" "compare") (cons "operator" "greater_equal")
                   (cons "left" (expression-ir expr1 src))
                   (cons "right" (expression-ir expr2 src)))]
          [(!= ,src ,type ,expr1 ,expr2)
           (object (cons "kind" "not_equal")
                   (cons "left" (typed-expression-ir expr1 type src))
                   (cons "right" (typed-expression-ir expr2 type src)))]
          [(let* ,src ([,local* ,expr*] ...) ,expr)
           (let-expression-ir local* expr* expr #f src)]
          [(seq ,src ,expr* ... ,expr)
           (if (checked-unsigned-subtraction? expr* expr)
               (expression-ir expr src)
               (object (cons "kind" "sequence")
                       (cons "steps" (list->vector (map (lambda (step) (expression-ir step src)) expr*)))
                       (cons "value" (expression-ir expr src))))]
          [(assert ,src ,expr ,mesg)
           (object (cons "kind" "assert")
                   (cons "condition" (expression-ir expr src))
                   (cons "message" mesg))]
          [(call ,src ,function-name ,expr* ...)
           (let ([name (id-sym function-name)])
             (cond
               [(eq? name 'transientHash)
                (unless (= (length expr*) 1)
                  (source-errorf src "transientHash expects one argument"))
                (object (cons "kind" "transient_hash")
                        (cons "value" (expression-ir (car expr*) src)))]
               [(eq? name 'transientCommit)
                (unless (= (length expr*) 2)
                  (source-errorf src "transientCommit expects two arguments"))
               (object (cons "kind" "transient_commit")
                        (cons "value" (expression-ir (car expr*) src))
                        (cons "opening" (expression-ir (cadr expr*) src)))]
               [(eq? name 'persistentHash)
                (unless (= (length expr*) 1)
                  (source-errorf src "persistentHash expects one argument"))
               (object (cons "kind" "persistent_hash")
                        (cons "value" (expression-ir (car expr*) src)))]
               [(eq? name 'keccak256)
                (unless (= (length expr*) 1)
                  (source-errorf src "keccak256 expects one argument"))
                (object (cons "kind" "keccak256")
                        (cons "value" (expression-ir (car expr*) src)))]
               [(eq? name 'persistentCommit)
                (unless (= (length expr*) 2)
                  (source-errorf src "persistentCommit expects two arguments"))
                (object (cons "kind" "persistent_commit")
                        (cons "value" (expression-ir (car expr*) src))
                        (cons "opening" (expression-ir (cadr expr*) src)))]
               [(eq? name 'degradeToTransient)
                (unless (= (length expr*) 1)
                  (source-errorf src "degradeToTransient expects one argument"))
                (object (cons "kind" "degrade_to_transient")
                        (cons "value" (expression-ir (car expr*) src)))]
               [(eq? name 'upgradeFromTransient)
                (unless (= (length expr*) 1)
                  (source-errorf src "upgradeFromTransient expects one argument"))
                (object (cons "kind" "upgrade_from_transient")
                        (cons "value" (expression-ir (car expr*) src)))]
               [(eq? name 'hashToCurve)
                (unless (= (length expr*) 1)
                  (source-errorf src "hashToCurve expects one argument"))
                (object (cons "kind" "hash_to_curve")
                        (cons "value" (expression-ir (car expr*) src)))]
               [(memq name '(jubjubPointX jubjubPointY))
                (unless (= (length expr*) 1)
                  (source-errorf src "Jubjub coordinate function expects one argument"))
                (object (cons "kind" (if (eq? name 'jubjubPointX)
                                          "jubjub_point_x"
                                          "jubjub_point_y"))
                        (cons "value" (expression-ir (car expr*) src)))]
               [(eq? name 'ecAdd)
                (unless (= (length expr*) 2)
                  (source-errorf src "ecAdd expects two arguments"))
                (object (cons "kind" "ec_add")
                        (cons "left" (expression-ir (car expr*) src))
                        (cons "right" (expression-ir (cadr expr*) src)))]
               [(eq? name 'constructJubjubPoint)
                (unless (= (length expr*) 2)
                  (source-errorf src "constructJubjubPoint expects two arguments"))
                (object (cons "kind" "construct_jubjub_point")
                        (cons "x" (expression-ir (car expr*) src))
                        (cons "y" (expression-ir (cadr expr*) src)))]
               [(eq? name 'ecNeg)
                (unless (= (length expr*) 1)
                  (source-errorf src "ecNeg expects one argument"))
                (object (cons "kind" "ec_neg")
                        (cons "value" (expression-ir (car expr*) src)))]
               [(eq? name 'ecMul)
                (unless (= (length expr*) 2)
                  (source-errorf src "ecMul expects two arguments"))
                (object (cons "kind" "ec_mul")
                        (cons "point" (expression-ir (car expr*) src))
                        (cons "scalar" (field-argument-ir (cadr expr*) src)))]
               [(eq? name 'ecMulGenerator)
                (unless (= (length expr*) 1)
                  (source-errorf src "ecMulGenerator expects one argument"))
                (object (cons "kind" "ec_mul_generator")
                        (cons "scalar" (field-argument-ir (car expr*) src)))]
               [(eq? name 'jubjubScalarFromNative)
                (unless (= (length expr*) 1)
                  (source-errorf src "jubjubScalarFromNative expects one argument"))
                (object (cons "kind" "jubjub_scalar_from_native")
                        (cons "value" (expression-ir (car expr*) src)))]
               [else
                (let ([formal-types (eq-hashtable-ref call-argument-types function-name #f)])
                  (object (cons "kind" "call")
                          (cons "name" (symbol->string name))
                          (cons "arguments"
                                (list->vector
                                  (if formal-types
                                      (begin
                                        (unless (= (length expr*) (length formal-types))
                                          (source-errorf src "Rust call argument count differs from its declaration"))
                                        (map (lambda (arg formal-type)
                                               (typed-expression-ir arg formal-type src))
                                             expr* formal-types))
                                      (map (lambda (arg) (expression-ir arg src)) expr*))))))]))]
          [(+ ,src ,mbits ,expr1 ,expr2)
           (if mbits
               (object (cons "kind" "unsigned_add")
                       (cons "max" (number->string (- (expt 2 mbits) 1)))
                       (cons "left" (expression-ir expr1 src))
                       (cons "right" (expression-ir expr2 src)))
               (object (cons "kind" "add")
                       (cons "left" (field-argument-ir expr1 owner-src))
                       (cons "right" (field-argument-ir expr2 owner-src))))]
          [(- ,src ,mbits ,expr1 ,expr2)
           (if mbits
               (object (cons "kind" "unsigned_subtract")
                       (cons "max" (number->string (- (expt 2 mbits) 1)))
                       (cons "left" (expression-ir expr1 src))
                       (cons "right" (expression-ir expr2 src)))
               (object (cons "kind" "subtract")
                       (cons "left" (field-argument-ir expr1 src))
                       (cons "right" (field-argument-ir expr2 src))))]
          [(* ,src ,mbits ,expr1 ,expr2)
           (if mbits
               (object (cons "kind" "unsigned_multiply")
                       (cons "max" (number->string (- (expt 2 mbits) 1)))
                       (cons "left" (expression-ir expr1 src))
                       (cons "right" (expression-ir expr2 src)))
               (object (cons "kind" "multiply")
                       (cons "left" (field-argument-ir expr1 src))
                       (cons "right" (field-argument-ir expr2 src))))]
          [else (source-errorf owner-src "Rust backend does not yet support this circuit expression")]))

      (define (typed-expression-ir expr expected-type owner-src)
        (nanopass-case (Lnodisclose Expression) expr
          [(return ,src ,expr) (typed-expression-ir expr expected-type src)]
          [(var-ref ,src ,var-name)
           (let ([actual-type (and (current-variable-types)
                                   (eq-hashtable-ref (current-variable-types) var-name #f))])
             (if actual-type
                 (nanopass-case (Lnodisclose Type) expected-type
                   [(tfield ,src^)
                    (nanopass-case (Lnodisclose Type) actual-type
                      [(tunsigned ,src1 ,nat)
                       (object (cons "kind" "field_cast")
                               (cons "value" (expression-ir expr src)))]
                      [else (expression-ir expr owner-src)])]
                   [(tunsigned ,src^ ,nat)
                    (nanopass-case (Lnodisclose Type) actual-type
                      [(tunsigned ,src1 ,nat1)
                       (if (= nat nat1)
                           (expression-ir expr owner-src)
                           (object (cons "kind" "unsigned_cast")
                                   (cons "value" (expression-ir expr src))
                                   (cons "max" (number->string nat))))]
                      [else (expression-ir expr owner-src)])]
                   [(tvector ,src^ ,len ,type)
                    (nanopass-case (Lnodisclose Type) actual-type
                      [(tvector ,src1 ,len1 ,type1)
                       (object (cons "kind" "coerce")
                               (cons "value" (expression-ir expr src))
                               (cons "ty" (type-ir expected-type src)))]
                      [(ttuple ,src1 ,type* ...)
                       (object (cons "kind" "coerce")
                               (cons "value" (expression-ir expr src))
                               (cons "ty" (type-ir expected-type src)))]
                      [else (expression-ir expr owner-src)])]
                   [(ttuple ,src^ ,type* ...)
                    (nanopass-case (Lnodisclose Type) actual-type
                      [(ttuple ,src1 ,type1* ...)
                       (object (cons "kind" "coerce")
                               (cons "value" (expression-ir expr src))
                               (cons "ty" (type-ir expected-type src)))]
                      [(tvector ,src1 ,len1 ,type1)
                       (object (cons "kind" "coerce")
                               (cons "value" (expression-ir expr src))
                               (cons "ty" (type-ir expected-type src)))]
                      [else (expression-ir expr owner-src)])]
                   [else (expression-ir expr owner-src)])
                 (expression-ir expr owner-src)))]
          [(let* ,src ([,local* ,expr*] ...) ,expr)
           (let-expression-ir local* expr* expr expected-type src)]
          [(tuple ,src ,tuple-arg* ...)
           (nanopass-case (Lnodisclose Type) expected-type
             [(tvector ,src^ ,len ,type)
              (unless (= (length tuple-arg*) len)
                (source-errorf src "Rust vector literal length does not match its type"))
              (object (cons "kind" "vector")
                      (cons "element" (type-ir type src))
                      (cons "elements"
                            (list->vector
                              (map (lambda (arg)
                                     (nanopass-case (Lnodisclose Tuple-Argument) arg
                                       [(single ,src1 ,expr)
                                        (typed-expression-ir expr type src1)]
                                       [else (source-errorf src "Rust backend does not yet support vector spreads")]))
                                   tuple-arg*))))]
             [(ttuple ,src^ ,type* ...)
              (unless (= (length tuple-arg*) (length type*))
                (source-errorf src "Rust tuple literal length does not match its type"))
              (if (null? type*)
                  (kind "unit")
                  (object (cons "kind" "tuple")
                          (cons "elements"
                                (list->vector
                                  (map (lambda (arg ty)
                                         (nanopass-case (Lnodisclose Tuple-Argument) arg
                                           [(single ,src1 ,expr)
                                            (typed-expression-ir expr ty src1)]
                                           [else (source-errorf src "Rust backend does not yet support tuple spreads")]))
                                       tuple-arg* type*)))))]
             [else (expression-ir expr owner-src)])]
          [(if ,src ,expr0 ,expr1 ,expr2)
           (object (cons "kind" "if")
                   (cons "condition" (expression-ir expr0 src))
                   (cons "then" (typed-expression-ir expr1 expected-type src))
                   (cons "otherwise" (typed-expression-ir expr2 expected-type src)))]
          [(seq ,src ,expr* ... ,expr)
           (if (checked-unsigned-subtraction? expr* expr)
               (typed-expression-ir expr expected-type src)
               (object (cons "kind" "sequence")
                       (cons "steps" (list->vector (map (lambda (step) (expression-ir step src)) expr*)))
                       (cons "value" (typed-expression-ir expr expected-type src))))]
          [(+ ,src ,mbits ,expr1 ,expr2)
           (if mbits
               (nanopass-case (Lnodisclose Type) expected-type
                 [(tunsigned ,src^ ,nat)
                  (object (cons "kind" "unsigned_add")
                          (cons "max" (number->string nat))
                          (cons "left" (expression-ir expr1 src))
                          (cons "right" (expression-ir expr2 src)))]
                 [else (expression-ir expr owner-src)])
               (expression-ir expr owner-src))]
          [(- ,src ,mbits ,expr1 ,expr2)
           (if mbits
               (nanopass-case (Lnodisclose Type) expected-type
                 [(tunsigned ,src^ ,nat)
                  (object (cons "kind" "unsigned_subtract")
                          (cons "max" (number->string nat))
                          (cons "left" (expression-ir expr1 src))
                          (cons "right" (expression-ir expr2 src)))]
                 [else (expression-ir expr owner-src)])
               (expression-ir expr owner-src))]
          [(* ,src ,mbits ,expr1 ,expr2)
           (if mbits
               (nanopass-case (Lnodisclose Type) expected-type
                 [(tunsigned ,src^ ,nat)
                  (object (cons "kind" "unsigned_multiply")
                          (cons "max" (number->string nat))
                          (cons "left" (expression-ir expr1 src))
                          (cons "right" (expression-ir expr2 src)))]
                 [else (expression-ir expr owner-src)])
               (expression-ir expr owner-src))]
          [(quote ,src ,datum)
           (if (and (integer? datum) (<= 0 datum))
               (nanopass-case (Lnodisclose Type) expected-type
                 [(tunsigned ,src^ ,nat)
                  (unless (<= datum nat)
                    (source-errorf src "Rust backend Uint literal exceeds its maximum"))
                  (object (cons "kind" "unsigned_literal")
                          (cons "value" (number->string datum))
                          (cons "max" (number->string nat)))]
                 [(tfield ,src^)
                  (object (cons "kind" "field_literal")
                          (cons "value" (number->string datum)))]
                 [else (expression-ir expr owner-src)])
               (expression-ir expr owner-src))]
          [else (expression-ir expr owner-src)]))

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

      (define (witness-declaration-ir pelt declarations)
        (nanopass-case (Lnodisclose Program-Element) pelt
          [(witness ,src ,function-name (,arg* ...) ,type)
           (cons (object (cons "name" (symbol->string (id-sym function-name)))
                         (cons "parameters" (list->vector (map (lambda (arg) (argument-ir arg src)) arg*)))
                         (cons "result" (type-ir type src)))
                 declarations)]
          [else declarations]))

      (define (witness-id-table pelt*)
        (let ([table (make-eq-hashtable)])
          (for-each
            (lambda (pelt)
              (nanopass-case (Lnodisclose Program-Element) pelt
                [(witness ,src ,function-name (,arg* ...) ,type)
                 (eq-hashtable-set! table function-name #t)]
                [else (void)]))
            pelt*)
          table))

      (define (index-call-argument-types pelt)
        (nanopass-case (Lnodisclose Program-Element) pelt
          [(circuit ,src ,function-name (,arg* ...) ,type ,expr)
           (eq-hashtable-set! call-argument-types function-name
             (map (lambda (arg)
                    (nanopass-case (Lnodisclose Argument) arg
                      [(,var-name ,type) type]))
                  arg*))]
          [(witness ,src ,function-name (,arg* ...) ,type)
           (eq-hashtable-set! call-argument-types function-name
             (map (lambda (arg)
                    (nanopass-case (Lnodisclose Argument) arg
                      [(,var-name ,type) type]))
                  arg*))]
          [else (void)]))

      (define (circuit-ir pelt export-alist circuits)
        (nanopass-case (Lnodisclose Program-Element) pelt
          [(circuit ,src ,function-name (,arg* ...) ,type ,expr)
           (let* ([names (exported-names function-name export-alist)]
                  [internal-name (symbol->string (id-sym function-name))])
             (if (id-pure? function-name)
                 (let ([variable-types (make-eq-hashtable)])
                   (for-each (lambda (arg)
                               (nanopass-case (Lnodisclose Argument) arg
                                 [(,var-name ,type)
                                  (eq-hashtable-set! variable-types var-name type)]))
                             arg*)
                   (parameterize ([current-variable-types variable-types])
                     (append
                   (map
                     (lambda (name)
                       (object (cons "name" name)
                               (cons "parameters" (list->vector (map (lambda (arg) (argument-ir arg src)) arg*)))
                               (cons "result" (type-ir type src))
                               (cons "body" (typed-expression-ir expr type src))))
                     names)
                   (if (member internal-name names)
                       '()
                       (list (object (cons "name" internal-name)
                                     (cons "internal" #t)
                                     (cons "parameters" (list->vector (map (lambda (arg) (argument-ir arg src)) arg*)))
                                     (cons "result" (type-ir type src))
                                     (cons "body" (typed-expression-ir expr type src)))))
                   circuits)))
                 circuits))]
          [else circuits]))

      (define (ledger-binding-ir binding owner-src)
        (nanopass-case (Lnodisclose Public-Ledger-Array-Element) binding
          [(,src ,ledger-field-name (,path-index* ...) ,type)
           (unless (and (= (length path-index*) 1)
                        (< (car path-index*) 16))
             (source-errorf src "Rust backend supports root ledger fields only"))
           (nanopass-case (Lnodisclose Type) type
             [(tadt ,src^ ,adt-name ([,adt-formal* ,adt-arg*] ...) ,vm-expr (,adt-op* ...) (,adt-rt-op* ...))
              (cond
                [(eq? adt-name 'Counter)
                 (object (cons "id" (symbol->string (id-sym ledger-field-name)))
                         (cons "index" (car path-index*))
                         (cons "declaration" (kind "counter")))]
                [(and (eq? adt-name '__compact_Cell) (= (length adt-arg*) 1))
                 (object (cons "id" (symbol->string (id-sym ledger-field-name)))
                         (cons "index" (car path-index*))
                         (cons "declaration"
                               (object (cons "kind" "cell")
                                       (cons "ty" (type-ir (car adt-arg*) src)))))]
                [(and (eq? adt-name 'Set) (= (length adt-arg*) 1))
                 (object (cons "id" (symbol->string (id-sym ledger-field-name)))
                         (cons "index" (car path-index*))
                         (cons "declaration"
                               (object (cons "kind" "set")
                                       (cons "ty" (type-ir (car adt-arg*) src)))))]
                [(and (eq? adt-name 'List) (= (length adt-arg*) 1))
                 (object (cons "id" (symbol->string (id-sym ledger-field-name)))
                         (cons "index" (car path-index*))
                         (cons "declaration"
                               (object (cons "kind" "list")
                                       (cons "ty" (type-ir (car adt-arg*) src)))))]
                [(and (eq? adt-name 'Map) (= (length adt-arg*) 2))
                 (object (cons "id" (symbol->string (id-sym ledger-field-name)))
                         (cons "index" (car path-index*))
                         (cons "declaration"
                               (object (cons "kind" "map")
                                       (cons "key" (type-ir (car adt-arg*) src))
                                       (cons "value" (type-ir (cadr adt-arg*) src)))))]
                [else (source-errorf src "Rust backend does not yet support this ledger ADT")])]
             [else (source-errorf src "Rust backend does not yet support this ledger field type")])]
          [else (source-errorf owner-src "Rust backend does not yet support nested ledger fields")]))

      (define (ledger-fields-ir pelt fields owner-src)
        (nanopass-case (Lnodisclose Program-Element) pelt
          [(public-ledger-declaration ,pl-array ,lconstructor)
           (nanopass-case (Lnodisclose Public-Ledger-Array) pl-array
             [(public-ledger-array ,pl-array-elt* ...)
              (append (map (lambda (binding) (ledger-binding-ir binding owner-src)) pl-array-elt*) fields)])]
          [else fields]))

      (define (counter-amount-ir expr environment owner-src)
        (nanopass-case (Lnodisclose Expression) expr
          [(quote ,src ,datum)
           (unless (and (integer? datum) (<= 0 datum 65535))
             (source-errorf src "Counter increment amount must fit Uint<16>"))
           (object (cons "kind" "literal") (cons "value" datum))]
          [(safe-cast ,src ,type ,type^ ,expr)
           (counter-amount-ir expr environment owner-src)]
          [(var-ref ,src ,var-name)
           (let ([entry (assq (id-sym var-name) environment)])
             (if entry
                 (cdr entry)
                 (source-errorf src "Rust backend cannot resolve Counter increment amount")))]
          [else (source-errorf owner-src "Rust backend supports literal or parameter Counter increments only")]))

      (define (state-action-ir expr owner-src environment witness-ids)
        (nanopass-case (Lnodisclose Expression) expr
          [(call ,src ,function-name ,expr* ...)
           (if (eq-hashtable-ref witness-ids function-name #f)
               (object (cons "kind" "expression")
                       (cons "value" (stateful-expression-ir expr src witness-ids)))
               (object (cons "kind" (if (id-pure? function-name) "pure_call" "circuit_call"))
                       (cons "name" (symbol->string (id-sym function-name)))
                       (cons "arguments" (list->vector (map (lambda (arg) (stateful-expression-ir arg src witness-ids)) expr*)))))]
          [(assert ,src ,expr ,mesg)
           (object (cons "kind" "assert")
                   (cons "condition" (stateful-expression-ir expr src witness-ids))
                   (cons "message" mesg))]
          [(let* ,src ([,local* ,expr*] ...) ,expr)
           (let ([environment^
                   (fold-left
                     (lambda (environment local value)
                       (nanopass-case (Lnodisclose Argument) local
                         [(,var-name ,type)
                          (cons (cons (id-sym var-name)
                                      (object (cons "kind" "parameter")
                                              (cons "name" (symbol->string (id-sym var-name)))))
                                environment)]))
                     environment local* expr*)])
             (object (cons "kind" "let")
                     (cons "bindings"
                           (list->vector
                             (map (lambda (local value)
                                    (nanopass-case (Lnodisclose Argument) local
                                      [(,var-name ,type)
                                       (object (cons "name" (symbol->string (id-sym var-name)))
                                               (cons "ty" (type-ir type src))
                                               (cons "value" (stateful-expression-ir value src witness-ids)))]))
                                  local* expr*)))
                     (cons "action" (state-action-ir expr owner-src environment^ witness-ids))))]
          [(public-ledger ,src ,ledger-field-name ,sugar? (,path-elt* ...) ,src^ ,adt-op ,expr* ...)
           (unless (and (= (length path-elt*) 1)
                        (integer? (car path-elt*)))
             (source-errorf src "Rust backend supports root ledger paths only"))
           (nanopass-case (Lnodisclose ADT-Op) adt-op
             [(,ledger-op ,op-class (,adt-name (,adt-formal* ,adt-arg*) ...) ((,var-name* ,type*) ...) ,type ,vm-code)
              (cond
                [(and (eq? adt-name 'Counter)
                      (eq? ledger-op 'increment)
                      (= (length expr*) 1))
                 (object (cons "kind" "counter_increment")
                         (cons "field" (symbol->string (id-sym ledger-field-name)))
                         (cons "index" (car path-elt*))
                         (cons "amount" (counter-amount-ir (car expr*) environment src)))]
                [(and (eq? adt-name 'Counter)
                      (eq? ledger-op 'decrement)
                      (= (length expr*) 1))
                 (object (cons "kind" "counter_decrement")
                         (cons "field" (symbol->string (id-sym ledger-field-name)))
                         (cons "index" (car path-elt*))
                         (cons "amount" (counter-amount-ir (car expr*) environment src)))]
                [(and (eq? adt-name 'Counter)
                      (eq? ledger-op 'resetToDefault)
                      (null? expr*))
                 (object (cons "kind" "counter_reset")
                         (cons "field" (symbol->string (id-sym ledger-field-name)))
                         (cons "index" (car path-elt*)))]
                [(and (eq? adt-name '__compact_Cell)
                      (eq? ledger-op 'write)
                      (= (length expr*) 1))
                 (object (cons "kind" "cell_write")
                         (cons "field" (symbol->string (id-sym ledger-field-name)))
                         (cons "index" (car path-elt*))
                         (cons "value" (stateful-expression-ir (car expr*) src witness-ids)))]
                [(and (eq? adt-name 'Set)
                      (eq? ledger-op 'insert)
                      (= (length expr*) 1))
                 (object (cons "kind" "set_insert")
                         (cons "field" (symbol->string (id-sym ledger-field-name)))
                         (cons "index" (car path-elt*))
                         (cons "value" (expression-ir (car expr*) src)))]
                [(and (eq? adt-name 'Set)
                      (eq? ledger-op 'remove)
                      (= (length expr*) 1))
                 (object (cons "kind" "set_remove")
                         (cons "field" (symbol->string (id-sym ledger-field-name)))
                         (cons "index" (car path-elt*))
                         (cons "value" (expression-ir (car expr*) src)))]
                [(and (eq? adt-name 'Set)
                      (eq? ledger-op 'resetToDefault)
                      (null? expr*))
                 (object (cons "kind" "set_reset")
                         (cons "field" (symbol->string (id-sym ledger-field-name)))
                         (cons "index" (car path-elt*)))]
                [(and (eq? adt-name 'List)
                      (eq? ledger-op 'pushFront)
                      (= (length expr*) 1))
                 (object (cons "kind" "list_push_front")
                         (cons "field" (symbol->string (id-sym ledger-field-name)))
                         (cons "index" (car path-elt*))
                         (cons "value" (expression-ir (car expr*) src)))]
                [(and (eq? adt-name 'List)
                      (eq? ledger-op 'popFront)
                      (null? expr*))
                 (object (cons "kind" "list_pop_front")
                         (cons "field" (symbol->string (id-sym ledger-field-name)))
                         (cons "index" (car path-elt*)))]
                [(and (eq? adt-name 'List)
                      (eq? ledger-op 'resetToDefault)
                      (null? expr*))
                 (object (cons "kind" "list_reset")
                         (cons "field" (symbol->string (id-sym ledger-field-name)))
                         (cons "index" (car path-elt*)))]
                [(and (eq? adt-name 'Map)
                      (eq? ledger-op 'insert)
                      (= (length expr*) 2))
                 (object (cons "kind" "map_insert")
                         (cons "field" (symbol->string (id-sym ledger-field-name)))
                         (cons "index" (car path-elt*))
                         (cons "key" (expression-ir (car expr*) src))
                         (cons "value" (expression-ir (cadr expr*) src)))]
                [(and (eq? adt-name 'Map)
                      (eq? ledger-op 'insertDefault)
                      (= (length expr*) 1))
                 (object (cons "kind" "map_insert_default")
                         (cons "field" (symbol->string (id-sym ledger-field-name)))
                         (cons "index" (car path-elt*))
                         (cons "key" (expression-ir (car expr*) src)))]
                [(and (eq? adt-name 'Map)
                      (eq? ledger-op 'remove)
                      (= (length expr*) 1))
                 (object (cons "kind" "map_remove")
                         (cons "field" (symbol->string (id-sym ledger-field-name)))
                         (cons "index" (car path-elt*))
                         (cons "key" (expression-ir (car expr*) src)))]
                [(and (eq? adt-name 'Map)
                      (eq? ledger-op 'resetToDefault)
                      (null? expr*))
                 (object (cons "kind" "map_reset")
                         (cons "field" (symbol->string (id-sym ledger-field-name)))
                         (cons "index" (car path-elt*)))]
                [else (source-errorf src "Rust backend does not yet support this ledger operation")])])]
          [else (source-errorf owner-src "Rust backend does not yet support this state action")]))

      (define (stateful-body-ir expr src environment witness-ids)
        (nanopass-case (Lnodisclose Expression) expr
          [(seq ,src1 ,expr* ... ,expr)
           (list->vector (map (lambda (action) (state-action-ir action src environment witness-ids)) expr*))]
          [else (vector)]))

      ;; Stateful expressions keep witness calls explicit so Rust can evaluate
      ;; them in order and append each private transcript value exactly once.
      (define (stateful-expression-ir value-expr owner-src witness-ids)
        (nanopass-case (Lnodisclose Expression) value-expr
          [(return ,src ,expr) (stateful-expression-ir expr src witness-ids)]
          [(call ,src ,function-name ,expr* ...)
           (let ([name (id-sym function-name)])
             (cond
               [(eq-hashtable-ref witness-ids function-name #f)
                (let ([formal-types (eq-hashtable-ref call-argument-types function-name #f)])
                  (unless (and formal-types (= (length expr*) (length formal-types)))
                    (source-errorf src "Rust witness argument count differs from its declaration"))
                  (object (cons "kind" "witness_call")
                          (cons "name" (symbol->string name))
                          (cons "arguments"
                                (list->vector
                                  (map (lambda (arg formal-type)
                                         (object (cons "kind" "coerce")
                                                 (cons "value" (stateful-expression-ir arg src witness-ids))
                                                 (cons "ty" (type-ir formal-type src))))
                                       expr* formal-types)))))]
               [(memq name '(transientHash persistentHash keccak256 degradeToTransient upgradeFromTransient
                              hashToCurve jubjubPointX jubjubPointY ecNeg jubjubScalarFromNative))
                (unless (= (length expr*) 1)
                  (source-errorf src "Rust backend native expects one argument"))
                (object (cons "kind" (case name
                                        [(transientHash) "transient_hash"]
                                        [(persistentHash) "persistent_hash"]
                                        [(keccak256) "keccak256"]
                                        [(degradeToTransient) "degrade_to_transient"]
                                        [(upgradeFromTransient) "upgrade_from_transient"]
                                        [(hashToCurve) "hash_to_curve"]
                                        [(jubjubPointX) "jubjub_point_x"]
                                        [(jubjubPointY) "jubjub_point_y"]
                                        [(ecNeg) "ec_neg"]
                                        [else "jubjub_scalar_from_native"]))
                        (cons "value" (stateful-expression-ir (car expr*) src witness-ids)))]
               [(eq? name 'ecMulGenerator)
                (unless (= (length expr*) 1)
                  (source-errorf src "ecMulGenerator expects one argument"))
                (object (cons "kind" "ec_mul_generator")
                        (cons "scalar" (stateful-expression-ir (car expr*) src witness-ids)))]
               [(memq name '(transientCommit persistentCommit))
                (unless (= (length expr*) 2)
                  (source-errorf src "Rust backend native expects two arguments"))
                (object (cons "kind" (if (eq? name 'transientCommit)
                                          "transient_commit"
                                          "persistent_commit"))
                        (cons "value" (stateful-expression-ir (car expr*) src witness-ids))
                        (cons "opening" (stateful-expression-ir (cadr expr*) src witness-ids)))]
               [(memq name '(ecAdd ecMul constructJubjubPoint))
                (unless (= (length expr*) 2)
                  (source-errorf src "Rust backend curve native expects two arguments"))
                (if (eq? name 'constructJubjubPoint)
                    (object (cons "kind" "construct_jubjub_point")
                            (cons "x" (stateful-expression-ir (car expr*) src witness-ids))
                            (cons "y" (stateful-expression-ir (cadr expr*) src witness-ids)))
                (if (eq? name 'ecAdd)
                    (object (cons "kind" "ec_add")
                            (cons "left" (stateful-expression-ir (car expr*) src witness-ids))
                            (cons "right" (stateful-expression-ir (cadr expr*) src witness-ids)))
                    (object (cons "kind" "ec_mul")
                            (cons "point" (stateful-expression-ir (car expr*) src witness-ids))
                            (cons "scalar" (stateful-expression-ir (cadr expr*) src witness-ids)))))]
               [else (expression-ir value-expr owner-src)]))]
          [(public-ledger ,src ,ledger-field-name ,sugar? (,path-elt* ...) ,src^ ,adt-op ,expr* ...)
           (unless (and (= (length path-elt*) 1)
                        (integer? (car path-elt*)))
             (source-errorf src "Rust backend supports root ledger query paths only"))
           (nanopass-case (Lnodisclose ADT-Op) adt-op
             [(,ledger-op ,op-class (,adt-name (,adt-formal* ,adt-arg*) ...) ((,var-name* ,type*) ...) ,type ,vm-code)
              (cond
                [(and (eq? adt-name '__compact_Cell) (eq? ledger-op 'read) (null? expr*))
                 (object (cons "kind" "cell_read")
                         (cons "field" (symbol->string (id-sym ledger-field-name)))
                         (cons "index" (car path-elt*)))]
                [(and (eq? adt-name 'Set) (eq? ledger-op 'member) (= (length expr*) 1))
                 (object (cons "kind" "set_member")
                         (cons "field" (symbol->string (id-sym ledger-field-name)))
                         (cons "index" (car path-elt*))
                         (cons "value" (stateful-expression-ir (car expr*) src witness-ids)))]
                [(and (eq? adt-name 'Set) (eq? ledger-op 'isEmpty) (null? expr*))
                 (object (cons "kind" "set_is_empty")
                         (cons "field" (symbol->string (id-sym ledger-field-name)))
                         (cons "index" (car path-elt*)))]
                [(and (eq? adt-name 'Map) (eq? ledger-op 'isEmpty) (null? expr*))
                 (object (cons "kind" "map_is_empty")
                         (cons "field" (symbol->string (id-sym ledger-field-name)))
                         (cons "index" (car path-elt*)))]
                [else (source-errorf src "Rust backend does not yet support this nested ledger query")])])]
          [(safe-cast ,src ,type ,type^ ,expr)
           (nanopass-case (Lnodisclose Type) type
             [(tunsigned ,src^ ,nat)
              (if (maybe-nonnegative-integer-literal expr src)
                  (expression-ir value-expr owner-src)
                  (object (cons "kind" "unsigned_cast")
                          (cons "max" (number->string nat))
                          (cons "value" (stateful-expression-ir expr src witness-ids))))]
             [else (expression-ir value-expr owner-src)])]
          [(downcast-unsigned ,src ,nat? ,nat ,expr)
           (unless nat?
             (source-errorf src "Rust backend does not yet support Field-to-Uint downcasts"))
           (object (cons "kind" "unsigned_cast")
                   (cons "max" (number->string nat))
                   (cons "value" (stateful-expression-ir expr src witness-ids)))]
          [(if ,src ,expr0 ,expr1 ,expr2)
           (object (cons "kind" "if")
                   (cons "condition" (stateful-expression-ir expr0 src witness-ids))
                   (cons "then" (stateful-expression-ir expr1 src witness-ids))
                   (cons "otherwise" (stateful-expression-ir expr2 src witness-ids)))]
          [(== ,src ,type ,expr1 ,expr2)
           (object (cons "kind" "equal")
                   (cons "left" (stateful-expression-ir expr1 src witness-ids))
                   (cons "right" (stateful-expression-ir expr2 src witness-ids)))]
          [(< ,src ,bits ,expr1 ,expr2)
           (object (cons "kind" "compare") (cons "operator" "less")
                   (cons "left" (stateful-expression-ir expr1 src witness-ids))
                   (cons "right" (stateful-expression-ir expr2 src witness-ids)))]
          [(<= ,src ,bits ,expr1 ,expr2)
           (object (cons "kind" "compare") (cons "operator" "less_equal")
                   (cons "left" (stateful-expression-ir expr1 src witness-ids))
                   (cons "right" (stateful-expression-ir expr2 src witness-ids)))]
          [(> ,src ,bits ,expr1 ,expr2)
           (object (cons "kind" "compare") (cons "operator" "greater")
                   (cons "left" (stateful-expression-ir expr1 src witness-ids))
                   (cons "right" (stateful-expression-ir expr2 src witness-ids)))]
          [(>= ,src ,bits ,expr1 ,expr2)
           (object (cons "kind" "compare") (cons "operator" "greater_equal")
                   (cons "left" (stateful-expression-ir expr1 src witness-ids))
                   (cons "right" (stateful-expression-ir expr2 src witness-ids)))]
          [(!= ,src ,type ,expr1 ,expr2)
           (object (cons "kind" "not_equal")
                   (cons "left" (stateful-expression-ir expr1 src witness-ids))
                   (cons "right" (stateful-expression-ir expr2 src witness-ids)))]
          [(let* ,src ([,local* ,expr*] ...) ,expr)
           (object (cons "kind" "let")
                   (cons "bindings"
                         (list->vector
                           (map (lambda (local value)
                                  (nanopass-case (Lnodisclose Argument) local
                                    [(,var-name ,type)
                                     (object (cons "name" (symbol->string (id-sym var-name)))
                                             (cons "ty" (type-ir type src))
                                             (cons "value" (stateful-expression-ir value src witness-ids)))]))
                                local* expr*)))
                   (cons "body" (stateful-expression-ir expr src witness-ids)))]
          [(seq ,src ,expr* ... ,expr)
           (if (checked-unsigned-subtraction? expr* expr)
               (stateful-expression-ir expr src witness-ids)
               (source-errorf src "Rust backend does not yet support this stateful expression sequence"))]
          [(tuple ,src ,tuple-arg* ...)
           (if (null? tuple-arg*)
               (kind "unit")
               (object (cons "kind" "tuple")
                       (cons "elements"
                             (list->vector
                               (map (lambda (arg)
                                      (nanopass-case (Lnodisclose Tuple-Argument) arg
                                        [(single ,src ,expr)
                                         (stateful-expression-ir expr src witness-ids)]
                                        [else (source-errorf src "Rust backend does not yet support tuple spreads")]))
                                    tuple-arg*)))))]
          [(+ ,src ,mbits ,expr1 ,expr2)
           (if mbits
               (object (cons "kind" "unsigned_add")
                       (cons "max" (number->string (- (expt 2 mbits) 1)))
                       (cons "left" (stateful-expression-ir expr1 src witness-ids))
                       (cons "right" (stateful-expression-ir expr2 src witness-ids)))
               (object (cons "kind" "add")
                       (cons "left" (stateful-expression-ir expr1 src witness-ids))
                       (cons "right" (stateful-expression-ir expr2 src witness-ids))))]
          [(- ,src ,mbits ,expr1 ,expr2)
           (if mbits
               (object (cons "kind" "unsigned_subtract")
                       (cons "max" (number->string (- (expt 2 mbits) 1)))
                       (cons "left" (stateful-expression-ir expr1 src witness-ids))
                       (cons "right" (stateful-expression-ir expr2 src witness-ids)))
               (object (cons "kind" "subtract")
                       (cons "left" (stateful-expression-ir expr1 src witness-ids))
                       (cons "right" (stateful-expression-ir expr2 src witness-ids))))]
          [(* ,src ,mbits ,expr1 ,expr2)
           (if mbits
               (object (cons "kind" "unsigned_multiply")
                       (cons "max" (number->string (- (expt 2 mbits) 1)))
                       (cons "left" (stateful-expression-ir expr1 src witness-ids))
                       (cons "right" (stateful-expression-ir expr2 src witness-ids)))
               (object (cons "kind" "multiply")
                       (cons "left" (stateful-expression-ir expr1 src witness-ids))
                       (cons "right" (stateful-expression-ir expr2 src witness-ids))))]
          [else (expression-ir value-expr owner-src)]))

      (define (stateful-return-ir return-expr owner-src witness-ids)
        (nanopass-case (Lnodisclose Expression) return-expr
          [(return ,src ,expr) (stateful-return-ir expr src witness-ids)]
          [(seq ,src ,expr* ... ,expr) (stateful-return-ir expr src witness-ids)]
          [(var-ref ,src ,var-name)
           (object (cons "kind" "expression")
                   (cons "value" (expression-ir return-expr src)))]
          [(call ,src ,function-name ,expr* ...)
           (if (or (eq-hashtable-ref witness-ids function-name #f)
                   (memq (id-sym function-name)
                         '(transientHash transientCommit persistentHash persistentCommit keccak256
                           degradeToTransient upgradeFromTransient hashToCurve
                           jubjubPointX jubjubPointY ecAdd ecNeg ecMul ecMulGenerator constructJubjubPoint
                           jubjubScalarFromNative)))
               (object (cons "kind" "expression")
                       (cons "value" (stateful-expression-ir return-expr src witness-ids)))
               (object (cons "kind" "expression")
                       (cons "value" (stateful-expression-ir return-expr src witness-ids))))]
          [(tuple ,src ,tuple-arg* ...)
           (if (null? tuple-arg*)
               (kind "unit")
               (object (cons "kind" "expression")
                       (cons "value" (stateful-expression-ir return-expr src witness-ids))))]
          [(if ,src ,expr0 ,expr1 ,expr2)
           (object (cons "kind" "expression")
                   (cons "value" (stateful-expression-ir return-expr src witness-ids)))]
          [(== ,src ,type ,expr1 ,expr2)
           (object (cons "kind" "expression")
                   (cons "value" (stateful-expression-ir return-expr src witness-ids)))]
          [(< ,src ,bits ,expr1 ,expr2)
           (object (cons "kind" "expression")
                   (cons "value" (stateful-expression-ir return-expr src witness-ids)))]
          [(<= ,src ,bits ,expr1 ,expr2)
           (object (cons "kind" "expression")
                   (cons "value" (stateful-expression-ir return-expr src witness-ids)))]
          [(> ,src ,bits ,expr1 ,expr2)
           (object (cons "kind" "expression")
                   (cons "value" (stateful-expression-ir return-expr src witness-ids)))]
          [(>= ,src ,bits ,expr1 ,expr2)
           (object (cons "kind" "expression")
                   (cons "value" (stateful-expression-ir return-expr src witness-ids)))]
          [(!= ,src ,type ,expr1 ,expr2)
           (object (cons "kind" "expression")
                   (cons "value" (stateful-expression-ir return-expr src witness-ids)))]
          [(let* ,src ([,local* ,expr*] ...) ,expr)
           (object (cons "kind" "expression")
                   (cons "value" (stateful-expression-ir return-expr src witness-ids)))]
          [(safe-cast ,src ,type ,type^ ,expr)
           (object (cons "kind" "expression")
                   (cons "value" (stateful-expression-ir return-expr src witness-ids)))]
          [(downcast-unsigned ,src ,nat? ,nat ,expr)
           (object (cons "kind" "expression")
                   (cons "value" (stateful-expression-ir return-expr src witness-ids)))]
          [(+ ,src ,mbits ,expr1 ,expr2)
           (object (cons "kind" "expression")
                   (cons "value" (stateful-expression-ir return-expr src witness-ids)))]
          [(- ,src ,mbits ,expr1 ,expr2)
           (object (cons "kind" "expression")
                   (cons "value" (stateful-expression-ir return-expr src witness-ids)))]
          [(* ,src ,mbits ,expr1 ,expr2)
           (object (cons "kind" "expression")
                   (cons "value" (stateful-expression-ir return-expr src witness-ids)))]
          [(public-ledger ,src ,ledger-field-name ,sugar? (,path-elt* ...) ,src^ ,adt-op ,expr* ...)
           (unless (and (= (length path-elt*) 1)
                        (integer? (car path-elt*)))
             (source-errorf src "Rust backend supports root ledger paths only"))
           (nanopass-case (Lnodisclose ADT-Op) adt-op
             [(,ledger-op ,op-class (,adt-name (,adt-formal* ,adt-arg*) ...) ((,var-name* ,type*) ...) ,type ,vm-code)
              (cond
                [(and (eq? adt-name '__compact_Cell)
                      (eq? ledger-op 'read)
                      (null? expr*))
                 (object (cons "kind" "cell_read")
                         (cons "field" (symbol->string (id-sym ledger-field-name)))
                         (cons "index" (car path-elt*)))]
                [(and (eq? adt-name 'Counter)
                      (eq? ledger-op 'read)
                      (null? expr*))
                 (object (cons "kind" "counter_read")
                         (cons "field" (symbol->string (id-sym ledger-field-name)))
                         (cons "index" (car path-elt*)))]
                [(and (eq? adt-name 'Set)
                      (eq? ledger-op 'member)
                      (= (length expr*) 1))
                 (object (cons "kind" "set_member")
                         (cons "field" (symbol->string (id-sym ledger-field-name)))
                         (cons "index" (car path-elt*))
                         (cons "value" (expression-ir (car expr*) src)))]
                [(and (eq? adt-name 'Set)
                      (eq? ledger-op 'size)
                      (null? expr*))
                 (object (cons "kind" "set_size")
                         (cons "field" (symbol->string (id-sym ledger-field-name)))
                         (cons "index" (car path-elt*)))]
                [(and (eq? adt-name 'Set)
                      (eq? ledger-op 'isEmpty)
                      (null? expr*))
                 (object (cons "kind" "set_is_empty")
                         (cons "field" (symbol->string (id-sym ledger-field-name)))
                         (cons "index" (car path-elt*)))]
                [(and (eq? adt-name 'Map)
                      (eq? ledger-op 'member)
                      (= (length expr*) 1))
                 (object (cons "kind" "map_member")
                         (cons "field" (symbol->string (id-sym ledger-field-name)))
                         (cons "index" (car path-elt*))
                         (cons "key" (expression-ir (car expr*) src)))]
                [(and (eq? adt-name 'Map)
                      (eq? ledger-op 'lookup)
                      (= (length expr*) 1))
                 (object (cons "kind" "map_lookup")
                         (cons "field" (symbol->string (id-sym ledger-field-name)))
                         (cons "index" (car path-elt*))
                         (cons "key" (expression-ir (car expr*) src)))]
                [(and (eq? adt-name 'Map)
                      (eq? ledger-op 'size)
                      (null? expr*))
                 (object (cons "kind" "map_size")
                         (cons "field" (symbol->string (id-sym ledger-field-name)))
                         (cons "index" (car path-elt*)))]
                [(and (eq? adt-name 'Map)
                      (eq? ledger-op 'isEmpty)
                      (null? expr*))
                 (object (cons "kind" "map_is_empty")
                         (cons "field" (symbol->string (id-sym ledger-field-name)))
                         (cons "index" (car path-elt*)))]
                [(and (eq? adt-name 'List)
                      (eq? ledger-op 'length)
                      (null? expr*))
                 (object (cons "kind" "list_length")
                         (cons "field" (symbol->string (id-sym ledger-field-name)))
                         (cons "index" (car path-elt*)))]
                [(and (eq? adt-name 'List)
                      (eq? ledger-op 'isEmpty)
                      (null? expr*))
                 (object (cons "kind" "list_is_empty")
                         (cons "field" (symbol->string (id-sym ledger-field-name)))
                         (cons "index" (car path-elt*)))]
                [(and (eq? adt-name 'List)
                      (eq? ledger-op 'head)
                      (null? expr*))
                 (object (cons "kind" "list_head")
                         (cons "field" (symbol->string (id-sym ledger-field-name)))
                         (cons "index" (car path-elt*)))]
                [else (source-errorf src "Rust backend does not yet support this ledger return operation")])])]
          [else (source-errorf owner-src "Rust backend does not yet support this stateful return value")]))

      (define (stateful-circuit-ir pelt export-alist witness-ids circuits)
        (nanopass-case (Lnodisclose Program-Element) pelt
          [(circuit ,src ,function-name (,arg* ...) ,type ,expr)
           (if (id-pure? function-name)
               circuits
               (let* ([names (exported-names function-name export-alist)]
                      [internal-name (symbol->string (id-sym function-name))]
                      [all-names (if (member internal-name names)
                                     names
                                     (append names (list internal-name)))])
                 (append
                   (map (lambda (name)
                          (append
                            (object (cons "name" name)
                                    (cons "parameters" (list->vector (map (lambda (arg) (argument-ir arg src)) arg*)))
                                    (cons "result" (type-ir type src))
                                    (cons "return_value" (stateful-return-ir expr src witness-ids))
                                    (cons "actions"
                                          (stateful-body-ir expr src
                                            (map (lambda (arg)
                                                   (nanopass-case (Lnodisclose Argument) arg
                                                     [(,var-name ,type)
                                                      (cons (id-sym var-name)
                                                            (object (cons "kind" "parameter")
                                                                    (cons "name" (symbol->string (id-sym var-name)))))]))
                                                 arg*) witness-ids)))
                            (if (member name names) '() (list (cons "internal" #t)))))
                        all-names)
                   circuits)))]
          [else circuits]))

      (define (empty-constructor-expression? expr)
        (nanopass-case (Lnodisclose Expression) expr
          [(return ,src ,expr) (empty-constructor-expression? expr)]
          [(tuple ,src ,tuple-arg* ...) (null? tuple-arg*)]
          [else #f]))

      (define (constructor-counter-environment parameters bindings)
        (append
          (fold-left
            (lambda (environment binding)
              (let* ([value (cdr binding)]
                     [kind-entry (assoc "kind" value)])
                (if (and kind-entry (string=? (cdr kind-entry) "unsigned_literal"))
                    (let ([amount (string->number (cdr (assoc "value" value)))])
                      (if (and (integer? amount) (<= 0 amount 65535))
                          (cons (cons (car binding)
                                      (object (cons "kind" "literal") (cons "value" amount)))
                                environment)
                          environment))
                    environment)))
            '() bindings)
          (map (lambda (name)
                 (cons name (object (cons "kind" "parameter")
                                    (cons "name" (symbol->string name)))))
               parameters)))

      (define (constructor-value-ir expr expected-type parameters bindings owner-src)
        (nanopass-case (Lnodisclose Expression) expr
          [(var-ref ,src ,var-name)
           (let ([binding (assq (id-sym var-name) bindings)])
             (cond
               [binding (cdr binding)]
               [(memq (id-sym var-name) parameters) (expression-ir expr src)]
               [else (source-errorf src "Rust constructor value must be a parameter or typed literal")]))]
          [else (typed-expression-ir expr expected-type owner-src)]))

      (define (constructor-fold-body-ir expr accumulator parameters owner-src)
        (nanopass-case (Lnodisclose Expression) expr
          [(seq ,src ,expr* ... ,expr)
           (append
             (apply append
               (map (lambda (step) (constructor-fold-body-ir step accumulator parameters src)) expr*))
             (constructor-fold-body-ir expr accumulator parameters src))]
          [(tuple ,src ,tuple-arg* ...)
           (if (null? tuple-arg*)
               '()
               (source-errorf src "Rust constructor fold body must have unit effects"))]
          [(var-ref ,src ,var-name)
           (if (eq? (id-sym var-name) accumulator)
               '()
               (source-errorf src "Rust constructor fold must return its accumulator"))]
          [else (list (constructor-step-ir expr parameters '() owner-src))]))

      ;; Both literal ranges and array iteration arrive here as a unit-accumulator fold.
      ;; Preserve its item binding and ordered effects instead of emitting Rust syntax.
      (define (constructor-step-ir expr parameters bindings owner-src)
        (nanopass-case (Lnodisclose Expression) expr
          [(fold ,src ,len ,fun (,expr0 ,type0) ,map-arg ,map-arg* ...)
           (unless (and (null? map-arg*) (empty-constructor-expression? expr0))
             (source-errorf src "Rust backend supports one iterable with a unit fold accumulator"))
           (nanopass-case (Lnodisclose Function) fun
             [(circuit ,src1 (,arg* ...) ,type ,expr1)
              (unless (= (length arg*) 2)
                (source-errorf src1 "Rust constructor fold expects accumulator and item parameters"))
              (nanopass-case (Lnodisclose Argument) (car arg*)
                [(,var-name ,type)
                 (let ([acc-name var-name])
                 (nanopass-case (Lnodisclose Argument) (cadr arg*)
                   [(,var-name ,type)
                    (let ([item-name var-name] [item-type type])
                    (nanopass-case (Lnodisclose Map-Argument) map-arg
                      [(,expr2 ,type ,type^)
                       (nanopass-case (Lnodisclose Expression) expr2
                         [(tuple ,src2 ,tuple-arg* ...)
                          (unless (= (length tuple-arg*) len)
                            (source-errorf src2 "Rust constructor fold length differs from its iterable"))
                          (object (cons "kind" "for_each")
                                  (cons "binding"
                                        (object (cons "name" (symbol->string (id-sym item-name)))
                                                (cons "ty" (type-ir item-type src))))
                                  (cons "values"
                                        (list->vector
                                          (map (lambda (arg)
                                                 (nanopass-case (Lnodisclose Tuple-Argument) arg
                                                   [(single ,src3 ,expr)
                                                    (typed-expression-ir expr item-type src3)]
                                                   [else (source-errorf src2 "Rust constructor fold does not support iterable spreads")]))
                                               tuple-arg*)))
                                  (cons "steps"
                                        (list->vector
                                          (constructor-fold-body-ir expr1 (id-sym acc-name)
                                                                    (cons (id-sym item-name) parameters)
                                                                    src1))))]
                         [else (source-errorf src "Rust constructor fold needs a literal iterable")])]))]))])]
             [else (source-errorf src "Rust constructor fold needs an inline circuit body")])]
          [(let* ,src ([,local* ,expr*] ...) ,expr)
           (let ([bindings^
                   (fold-left
                     (lambda (bindings local value)
                       (nanopass-case (Lnodisclose Argument) local
                         [(,var-name ,type)
                          (cons (cons (id-sym var-name) (typed-expression-ir value type src)) bindings)]))
                     bindings local* expr*)])
             (constructor-step-ir expr parameters bindings^ src))]
          [(public-ledger ,src ,ledger-field-name ,sugar? (,path-elt* ...) ,src^ ,adt-op ,expr* ...)
           (unless (and (= (length path-elt*) 1)
                        (integer? (car path-elt*)))
             (source-errorf src "Rust backend supports root constructor ledger paths only"))
           (nanopass-case (Lnodisclose ADT-Op) adt-op
             [(,ledger-op ,op-class (,adt-name (,adt-formal* ,adt-arg*) ...) ((,var-name* ,type*) ...) ,type ,vm-code)
              (cond
                [(and (eq? adt-name 'Counter) (eq? ledger-op 'increment) (= (length expr*) 1))
                 (object (cons "kind" "counter_increment")
                         (cons "field" (symbol->string (id-sym ledger-field-name)))
                         (cons "index" (car path-elt*))
                         (cons "amount" (counter-amount-ir (car expr*)
                                                           (constructor-counter-environment parameters bindings)
                                                           src)))]
                [(and (eq? adt-name 'Counter) (eq? ledger-op 'decrement) (= (length expr*) 1))
                 (object (cons "kind" "counter_decrement")
                         (cons "field" (symbol->string (id-sym ledger-field-name)))
                         (cons "index" (car path-elt*))
                         (cons "amount" (counter-amount-ir (car expr*)
                                                           (constructor-counter-environment parameters bindings)
                                                           src)))]
                [(and (eq? adt-name 'Counter) (eq? ledger-op 'resetToDefault) (null? expr*))
                 (object (cons "kind" "counter_reset")
                         (cons "field" (symbol->string (id-sym ledger-field-name)))
                         (cons "index" (car path-elt*)))]
                [(and (eq? adt-name 'Set) (eq? ledger-op 'insert) (= (length expr*) 1))
                 (object (cons "kind" "set_insert")
                         (cons "field" (symbol->string (id-sym ledger-field-name)))
                         (cons "index" (car path-elt*))
                         (cons "value" (constructor-value-ir (car expr*) (car adt-arg*)
                                                             parameters bindings src)))]
                [(and (eq? adt-name 'Set) (eq? ledger-op 'remove) (= (length expr*) 1))
                 (object (cons "kind" "set_remove")
                         (cons "field" (symbol->string (id-sym ledger-field-name)))
                         (cons "index" (car path-elt*))
                         (cons "value" (constructor-value-ir (car expr*) (car adt-arg*)
                                                             parameters bindings src)))]
                [(and (eq? adt-name 'Set) (eq? ledger-op 'resetToDefault) (null? expr*))
                 (object (cons "kind" "set_reset")
                         (cons "field" (symbol->string (id-sym ledger-field-name)))
                         (cons "index" (car path-elt*)))]
                [(and (eq? adt-name 'List) (eq? ledger-op 'pushFront) (= (length expr*) 1))
                 (object (cons "kind" "list_push_front")
                         (cons "field" (symbol->string (id-sym ledger-field-name)))
                         (cons "index" (car path-elt*))
                         (cons "value" (constructor-value-ir (car expr*) (car adt-arg*)
                                                             parameters bindings src)))]
                [(and (eq? adt-name 'List) (eq? ledger-op 'popFront) (null? expr*))
                 (object (cons "kind" "list_pop_front")
                         (cons "field" (symbol->string (id-sym ledger-field-name)))
                         (cons "index" (car path-elt*)))]
                [(and (eq? adt-name 'List) (eq? ledger-op 'resetToDefault) (null? expr*))
                 (object (cons "kind" "list_reset")
                         (cons "field" (symbol->string (id-sym ledger-field-name)))
                         (cons "index" (car path-elt*)))]
                [(and (eq? adt-name 'Map) (eq? ledger-op 'insert) (= (length expr*) 2))
                 (object (cons "kind" "map_insert")
                         (cons "field" (symbol->string (id-sym ledger-field-name)))
                         (cons "index" (car path-elt*))
                         (cons "key" (constructor-value-ir (car expr*) (car adt-arg*)
                                                           parameters bindings src))
                         (cons "value" (constructor-value-ir (cadr expr*) (cadr adt-arg*)
                                                             parameters bindings src)))]
                [(and (eq? adt-name 'Map) (eq? ledger-op 'insertDefault) (= (length expr*) 1))
                 (object (cons "kind" "map_insert_default")
                         (cons "field" (symbol->string (id-sym ledger-field-name)))
                         (cons "index" (car path-elt*))
                         (cons "key" (constructor-value-ir (car expr*) (car adt-arg*)
                                                           parameters bindings src)))]
                [(and (eq? adt-name 'Map) (eq? ledger-op 'remove) (= (length expr*) 1))
                 (object (cons "kind" "map_remove")
                         (cons "field" (symbol->string (id-sym ledger-field-name)))
                         (cons "index" (car path-elt*))
                         (cons "key" (constructor-value-ir (car expr*) (car adt-arg*)
                                                           parameters bindings src)))]
                [(and (eq? adt-name 'Map) (eq? ledger-op 'resetToDefault) (null? expr*))
                 (object (cons "kind" "map_reset")
                         (cons "field" (symbol->string (id-sym ledger-field-name)))
                         (cons "index" (car path-elt*)))]
                [(and (eq? adt-name '__compact_Cell) (eq? ledger-op 'write) (= (length expr*) 1))
                 (nanopass-case (Lnodisclose Expression) (car expr*)
                   [(var-ref ,src1 ,var-name)
                    (let ([binding (assq (id-sym var-name) bindings)])
                      (unless (or binding (memq (id-sym var-name) parameters))
                        (source-errorf src1 "Rust constructor Cell initializer must be a constructor parameter or literal"))
                      (object (cons "kind" "cell_write")
                              (cons "field" (symbol->string (id-sym ledger-field-name)))
                              (cons "index" (car path-elt*))
                              (cons "value" (if binding (cdr binding) (expression-ir (car expr*) src)))))]
                   [else
                    (object (cons "kind" "cell_write")
                            (cons "field" (symbol->string (id-sym ledger-field-name)))
                            (cons "index" (car path-elt*))
                            (cons "value" (if (null? adt-arg*)
                                              (expression-ir (car expr*) src)
                                              (typed-expression-ir (car expr*) (car adt-arg*) src))))])]
                [else (source-errorf src "Rust backend does not yet support this constructor ledger operation")])])]
          [else (source-errorf owner-src "Rust backend does not yet support this constructor action")]))

      (define (constructor-steps-ir expr parameters owner-src)
        (nanopass-case (Lnodisclose Expression) expr
          [(return ,src ,expr) (constructor-steps-ir expr parameters src)]
          [(seq ,src ,expr* ... ,expr)
           (unless (empty-constructor-expression? expr)
             (source-errorf src "Rust backend does not yet support constructor return values"))
           (map (lambda (action) (constructor-step-ir action parameters '() src)) expr*)]
          [(tuple ,src ,tuple-arg* ...)
           (if (null? tuple-arg*)
               '()
               (source-errorf src "Rust backend does not yet support constructor return values"))]
          [else (source-errorf owner-src "Rust backend does not yet support this constructor body")]))

      (define (constructor-ir pelt)
        (nanopass-case (Lnodisclose Program-Element) pelt
          [(public-ledger-declaration ,pl-array ,lconstructor)
           (nanopass-case (Lnodisclose Ledger-Constructor) lconstructor
             [(constructor ,src ((,var-name* ,type*) ...) ,expr)
              (let ([steps (constructor-steps-ir expr (map id-sym var-name*) src)])
                (if (and (null? var-name*) (null? steps))
                    #f
                    (object (cons "parameters"
                                  (list->vector
                                    (map (lambda (name ty)
                                           (object (cons "name" (symbol->string (id-sym name)))
                                                   (cons "ty" (type-ir ty src))))
                                         var-name* type*)))
                            (cons "steps" (list->vector steps)))))])]
          [else #f])))

    (Program : Program (ir) -> Program ()
      [(program ,src (,contract-name* ...) ((,export-name* ,name*) ...) ,pelt* ...)
       (hashtable-clear! call-argument-types)
       (set! struct-shape-names '())
       (set! used-struct-names '())
       (for-each index-call-argument-types pelt*)
       (let ([constructor* (filter (lambda (value) value) (map constructor-ir pelt*))]
             [export-alist (map cons export-name* name*)]
             [witness-ids (witness-id-table pelt*)])
         (when (> (length constructor*) 1)
           (source-errorf src "Rust backend found multiple constructors"))
         (print-json
           (get-target-port 'rust.ir.json)
           (append (object (cons "schema_version" 5)
                   (cons "ledger_fields"
                         (list->vector
                           (fold-right (lambda (pelt fields) (ledger-fields-ir pelt fields src)) '() pelt*)))
                   (cons "witnesses"
                         (list->vector
                           (fold-right witness-declaration-ir '() pelt*)))
                   (cons "circuits"
                         (list->vector
                           (fold-right
                             (lambda (pelt circuits) (circuit-ir pelt export-alist circuits))
                             '()
                             pelt*)))
                   (cons "stateful_circuits"
                         (list->vector
                           (fold-right
                             (lambda (pelt circuits) (stateful-circuit-ir pelt export-alist witness-ids circuits))
                             '()
                             pelt*))))
                   (if (null? constructor*) '() (list (cons "constructor" (car constructor*)))))))
       ir]))

  (define-passes rust-ir-passes
    (emit-rust-ir Lnodisclose)))
