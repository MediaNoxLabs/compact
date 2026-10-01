;;; This file is part of Compact.
;;; Copyright (C) 2026 Midnight Foundation
;;; SPDX-License-Identifier: Apache-2.0
;;; Licensed under the Apache License, Version 2.0 (the "License");
;;; you may not use this file except in compliance with the License.
;;; You may obtain a copy of the License at
;;;
;;; 	http://www.apache.org/licenses/LICENSE-2.0
;;;
;;; Unless required by applicable law or agreed to in writing, software
;;; distributed under the License is distributed on an "AS IS" BASIS,
;;; WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
;;; See the License for the specific language governing permissions and
;;; limitations under the License.

;;; The version of the *Rust* runtime, for the `check_runtime_version!` pin
;;; the Rust backend stamps into every generated crate.
;;;
;;; This is deliberately separate from `(runtime-version)`, which reads the
;;; TypeScript runtime's `runtime/package.json`. The two runtimes version
;;; independently, and they have diverged in practice. The pin exists so a
;;; generated crate refuses to compile against a `midnight-compact-runtime`
;;; it was not emitted for, so it must track the crate the generated code
;;; actually links — `runtime-rs/Cargo.toml` — and not the npm package, which
;;; the Rust crate never sees.
;;;
;;; Reading the TypeScript version here is not a cosmetic mismatch: the pin
;;; is `const`-evaluated, so stamping a version the crate does not have turns
;;; every generated crate into a compile error ("midnight-compact-runtime
;;; version mismatch") the moment the npm package is bumped on its own.

#!chezscheme

(library (rust-runtime-version)
  (export rust-runtime-version-string)
  (import (chezscheme))
  (define rust-runtime-version-string
    (let-syntax ([a (lambda (x)
                      (let ([fn "runtime-rs/Cargo.toml"])
                        (#%$require-include fn)
                        ((include "runtime-rs/extract-version.ss") fn)))])
      a)))
