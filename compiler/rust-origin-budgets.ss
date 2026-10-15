#!chezscheme

;;; This file is part of Compact.
;;; Copyright (C) 2026 Midnight Foundation
;;; SPDX-License-Identifier: Apache-2.0

;;; Transport containment only. These do not change Compact source admission.
;;; A future budget change needs its own analysis and measured qualification.
(library (rust-origin-budgets)
  (export address-limit pending-limit path-character-limit companion-byte-limit origin-u64-max)
  (import (chezscheme))
  (define address-limit 131072)
  (define pending-limit 4096)
  (define path-character-limit 262144)
  (define companion-byte-limit (* 4 1024 1024))
  (define origin-u64-max 18446744073709551615))
