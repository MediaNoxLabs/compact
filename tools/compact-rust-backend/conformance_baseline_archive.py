# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#  	http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.
"""Explicit historical inputs for the one retained repository-baseline law.

No reads are projected or cached. The original checker reads the supplied files;
all other tests retain their current roots. This is not current conformance.
"""
from contextlib import contextmanager
import hashlib
import os
from pathlib import Path

import specification_conformance as conformance

ENVIRONMENT = "COMPACT_CONFORMANCE_BASELINE_ROOT"
BASELINE_SHA256 = "593d6b6c5da8abdb7e977e26b483fe83464d5f602d10caeaa1d8e36c3ee81261"
CASE_ID = ("test_specification_conformance.RepositoryBaselineTests."
           "test_checked_in_baseline_matches_current_sources_and_mappings")
BUNDLED_ROOT = Path(__file__).parent / "tests/retained-conformance-6e37443b"


def selected_test_root():
    """Choose explicit test data; the authenticator itself has no default."""
    return os.environ.get(ENVIRONMENT, BUNDLED_ROOT)


def authenticate_archive(value):
    """Check the immutable baseline and every source/anchor it already requires."""
    if value is None or not str(value):
        raise ValueError("explicit retained conformance inputs required: supply "
                         "--conformance-baseline-root or " + ENVIRONMENT)
    root = Path(value)
    if not root.is_absolute():
        raise ValueError("conformance baseline root must be absolute")
    root = root.resolve(strict=True)
    if not root.is_dir():
        raise ValueError("conformance baseline root must be a directory")
    raw = Path(conformance.__file__).with_suffix(".json").read_bytes()
    if hashlib.sha256(raw).hexdigest() != BASELINE_SHA256:
        raise ValueError("retained conformance baseline authority changed")
    baseline = conformance.load_json(raw.decode("utf-8"))
    result = conformance.check(root, baseline["grammar_inventory"], baseline)
    if result["status"] != "passed" or result["errors"]:
        raise ValueError("retained conformance archive refused: "
                         f"{len(result['errors'])} source/mapping errors")
    return {"root": str(root), "baseline_sha256": BASELINE_SHA256,
            "case_id": CASE_ID,
            "scope": "Retained 6e37443b baseline inputs; not current-source conformance "
                     "or the separate complete historical-law qualification."}


@contextmanager
def repository_baseline_root(value=None):
    """Select only the original case's root, restore it even on test failure."""
    value = os.environ.get(ENVIRONMENT) if value is None else value
    receipt = authenticate_archive(value)
    previous = conformance.ROOT
    conformance.ROOT = Path(receipt["root"])
    try:
        yield receipt
    finally:
        conformance.ROOT = previous
        authenticate_archive(receipt["root"])
