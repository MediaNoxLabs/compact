# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Explicit ordinary Unit return expansion; no selector or retirement change."""
import re
from recording_admission_inventory_context import function
from recording_admission_inventory_jubjub import canonical, digest
from recording_admission_inventory_receive import SELECTOR

PRIOR = '7650b6f40bd07439284b56c50399017ed2d921c1954683e9b56d110224007835'
OLD_HELPER = '855c7d271402b25d428739cb352336dc9d0f76fb250889b2f54ec4e193a4fd23'
NEW_HELPER = 'd50dc8e9632ea1ebf6ff6ca3ce2128364ade0fabbe7494e1db14552f6737c11f'
MARKER = 'fn checked_unit_domain('
OLD_RETURN = '!matches!(circuit.return_value, StateReturn::Expression { .. })'
NEW_RETURN = '''!matches!(
            circuit.return_value,
            StateReturn::Unit | StateReturn::Expression { .. }
        )'''
OLD_COMMENT = '/// Unit expression interfaces enter at the former unit-action priority. Body'
NEW_COMMENT = '/// Ordinary Unit interfaces enter at the former unit-action priority. Body'


def validate(root, extension, receive):
    try:
        source = (root / SELECTOR).read_text()
    except (OSError, UnicodeError) as error:
        return ['Unit domain: unreadable helper: ' + str(error)]
    return validate_source(source, extension, receive)


def validate_source(source, extension, receive):
    """Validate an exact source snapshot; later transitions must authenticate it."""
    errors = []
    if (extension.get('schema_version') != 1 or extension.get('issue') != 540
            or extension.get('classification') != 'ordinary-unit-return-domain-no-retirement'):
        errors.append('Unit domain: unsupported review manifest')
    if (receive is None or canonical(receive) != PRIOR
            or extension.get('prior_receive_transition_canonical_sha256') != PRIOR):
        return errors + ['Unit domain: frozen receive predecessor changed or unavailable']
    if extension.get('prior_source_fingerprints') != receive['reviewed_source_fingerprints']:
        errors.append('Unit domain: prior owner is not bound to receive evidence')
    if (extension.get('helper') != MARKER or extension.get('prior_helper_sha256') != OLD_HELPER
            or extension.get('reviewed_helper_sha256') != NEW_HELPER):
        errors.append('Unit domain: unreviewed helper change')
    for key in ['instrumented_route_ids', 'retired_route_ids', 'reviewed_selector']:
        if extension.get(key) != receive[key]:
            errors.append('Unit domain: unchanged selector/routes required: ' + key)
    if (extension.get('prior_selector') != receive['reviewed_selector']
            or extension.get('new_retired_routes') != []):
        errors.append('Unit domain: no new selector or retirement authorized')
    owners = extension.get('reviewed_source_fingerprints', {})
    if (set(owners) != {SELECTOR}
            or not isinstance(owners.get(SELECTOR), str)
            or not re.fullmatch('[0-9a-f]{64}', owners[SELECTOR])):
        errors.append('Unit domain: exactly one valid reviewed owner required')
    try:
        if digest(source.encode()) != owners.get(SELECTOR):
            errors.append('Unit domain: reviewed observation owner changed')
        body = function(source, MARKER)
        if source.count(MARKER) != 1 or digest(body.encode()) != NEW_HELPER:
            errors.append('Unit domain: current helper differs from reviewed body')
        # Reconstruct the precise predecessor, not just its selector call list.
        # This permits only the reviewed return predicate and its comment.
        original_body = body.replace(NEW_RETURN, OLD_RETURN, 1)
        if digest(original_body.encode()) != OLD_HELPER:
            errors.append('Unit domain: original helper reconstruction failed')
        original = source.replace(body, original_body, 1).replace(NEW_COMMENT, OLD_COMMENT, 1)
        if digest(original.encode()) != receive['reviewed_source_fingerprints'][SELECTOR]:
            errors.append('Unit domain: unrelated selector owner content changed')
    except (OSError, UnicodeError, ValueError) as error:
        errors.append('Unit domain: unreadable helper: ' + str(error))
    return errors
