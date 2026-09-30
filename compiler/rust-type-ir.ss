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

;;; PROTOTYPE (compact#95): a Rust *type* as data, instead of as text.
;;;
;;; The Rust backend renders text. Roughly 200 `format` / `string-append`
;;; sites across 12k lines of emitter produce the output, and whether that
;;; output is valid Rust is decided downstream — by rustfmt for shape and by
;;; `cargo build` for truth. Three of the silent-bad-output defects recorded
;;; in docs/rust-backend-limitations.md are defects of that arrangement rather
;;; than of the lowering: `(x) as uN` where `x: Fr`, `wrapping_*` on two
;;; `Fr`s, and the `sentinel-splice` backstop that exists only because a `#f`
;;; can reach `format` and land in the output as the literal text `#f`.
;;;
;;; This file asks a narrow version of the question on the smallest pass:
;;; if a Rust type is a value rather than a string, which of those defects
;;; become *unrepresentable*, and which merely move?
;;;
;;; The representation is deliberately closed. There is no constructor that
;;; takes arbitrary Rust text except `rt-foreign`, which exists to make the
;;; remaining text dependencies *countable* rather than invisible — every one
;;; carries a reason symbol, and `rust-type-foreign-count` reports how many a
;;; given tree still needs. A prototype that hid them would answer the
;;; question dishonestly.

(define-record-type rt-prim                 ; Fr, bool, u8..u128, ContractAddress
  (fields name))

(define-record-type rt-array                ; [T; N]
  (fields elem len))

(define-record-type rt-tuple                ; (), (T,), (T, U, ...)
  (fields elems))

(define-record-type rt-path                 ; Name, or a::b::Name
  (fields segments))

(define-record-type rt-generic              ; Head<A, B>
  (fields head args))

;; The escape hatch, kept visible on purpose. `why` is a symbol naming the
;; reason this subtree could not be modelled, so the measurement in the issue
;; can be taken by counting rather than by reading.
(define-record-type rt-foreign
  (fields text why))

;; A type the backend cannot lower. Delegates to the rejection helper this
;; line already has (`rust-passes-helpers.ss`), which raises a located
;; diagnostic with a kind symbol — the same contract the vehicle states in
;; docs/rust-backend-limitations.md.
;;
;; Worth noting for the issue: the helper was already here. The type pass
;; simply never used it, and produced `/* TODO ... */` strings instead. So
;; this half of the change is a straight defect fix that does not depend on
;; the typed representation at all, and could be ported to `codegen-rust`
;; on its own.
(define (rust-type-unlowerable src kind msg . args)
  (apply rust-feature-error src kind msg args))

;; rust-type->string: total over the six constructors above. Nothing else is
;; representable, so there is no `else` branch to get wrong and no path by
;; which unlowered text can arrive here.
(define (rust-type->string t)
  (cond
    [(rt-prim? t) (rt-prim-name t)]
    [(rt-array? t)
     (format "[~a; ~a]" (rust-type->string (rt-array-elem t)) (rt-array-len t))]
    [(rt-tuple? t)
     (let ([parts (map rust-type->string (rt-tuple-elems t))])
       (cond
         [(null? parts) "()"]
         ;; Rust 1-tuples need the trailing comma: (T,)
         [(null? (cdr parts)) (format "(~a,)" (car parts))]
         [else (format "(~a)" (comma-join parts))]))]
    [(rt-path? t) (dcolon-join (rt-path-segments t))]
    [(rt-generic? t)
     (format "~a<~a>"
       (rust-type->string (rt-generic-head t))
       (comma-join (map rust-type->string (rt-generic-args t))))]
    [(rt-foreign? t) (rt-foreign-text t)]
    ;; Unreachable by construction: the six constructors above are the whole
    ;; representation. If this fires, a caller built something that is not a
    ;; Rust type — a compiler bug, not a bad input program, which is why it
    ;; is an internal error rather than a located diagnostic.
    [else (internal-errorf "not a Rust type node: ~s" t)]))

;; rust-type-foreign-count: how much of this tree is still text. The number
;; the prototype exists to produce.
(define (rust-type-foreign-count t)
  (cond
    [(rt-foreign? t) 1]
    [(rt-array? t) (rust-type-foreign-count (rt-array-elem t))]
    [(rt-tuple? t) (apply + 0 (map rust-type-foreign-count (rt-tuple-elems t)))]
    [(rt-generic? t)
     (+ (rust-type-foreign-count (rt-generic-head t))
        (apply + 0 (map rust-type-foreign-count (rt-generic-args t))))]
    [else 0]))

(define (comma-join parts)
  (let loop ([xs parts] [acc ""])
    (cond
      [(null? xs) acc]
      [(null? (cdr xs)) (string-append acc (car xs))]
      [else (loop (cdr xs) (string-append acc (car xs) ", "))])))

(define (dcolon-join segs)
  (let loop ([xs segs] [acc ""])
    (cond
      [(null? xs) acc]
      [(null? (cdr xs)) (string-append acc (car xs))]
      [else (loop (cdr xs) (string-append acc (car xs) "::"))])))
