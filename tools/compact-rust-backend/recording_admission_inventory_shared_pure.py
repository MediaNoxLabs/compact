# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Shared pure policies/signature carriers; body certification remains pending."""
import copy
import json
import tomllib

from recording_admission_inventory_jubjub import canonical, digest
from recording_admission_inventory_aligned_test import _TextSnapshot

MANIFEST_SHA = '5f813b360fdb28ee170d4c24b811bb22a442e973740933f5a12a8826012bc368'
BASELINE_COMMIT = '807bed34aa0937883cfc085aa060b41b3ba825b7'
BASE = 'tools/compact-rust-backend/src'
OWNERS = ('Cargo.lock', 'tools/compact-rust-backend/Cargo.lock',
          'tools/compact-rust-backend/Cargo.toml', BASE + '/lib.rs',
          BASE + '/naming.rs', BASE + '/value_lowering.rs',
          BASE + '/recorded/effect_plan/pure_certification/native.rs')
SELECTOR = BASE + '/recorded/profile_selection.rs'


def predecessor_sources(root, extension):
    if (canonical(extension) != MANIFEST_SHA or extension.get('schema_version') != 1
            or extension.get('issue') != 622 or extension.get('slice') != 'A'
            or extension.get('baseline_commit') != BASELINE_COMMIT
            or extension.get('new_retired_route_ids') != []):
        raise ValueError('shared622A: unreviewed extension')
    if set(extension['owners']) != set(OWNERS):
        raise ValueError('shared622A: all seven complete owners required')
    sources = {}
    for owner, row in extension['owners'].items():
        source = (root / owner).read_bytes().decode()
        if digest(source.encode()) != row['current_sha256']:
            raise ValueError('shared622A: current complete owner changed: ' + owner)
        for edit in row['predecessor_reconstruction']:
            if not edit['current'] or source.count(edit['current']) != 1:
                raise ValueError('shared622A: inverse is not unique: ' + owner)
            source = source.replace(edit['current'], edit['previous'], 1)
        if digest(source.encode()) != row['previous_sha256']:
            raise ValueError('shared622A: complete signed predecessor differs: ' + owner)
        sources[owner] = source
    for label in ('new_and_shared_sources', 'historical_manifests', 'oracle_authorities'):
        for owner, expected in extension[label].items():
            if digest((root / owner).read_bytes()) != expected:
                raise ValueError('shared622A: ' + label + ' changed: ' + owner)
    if len(extension['historical_manifests']) != 67:
        raise ValueError('shared622A: all 67 scoped historical manifests required')
    return sources


class _Checkpoint:
    def __init__(self, root, extension):
        self._root = root
        self._sources = predecessor_sources(root, extension)

    def __truediv__(self, owner):
        return _TextSnapshot(self._sources[owner]) if owner in self._sources else self._root / owner


def checkpoint(root, extension):
    return _Checkpoint(root, extension)


def latest_checkpoint(root, baseline_path):
    """Historical laws use only an authenticated latest predecessor."""
    path = baseline_path.with_name('recording_admission_inventory.shared-pure-semantics-622a.json')
    return checkpoint(root, json.loads(path.read_text())) if path.exists() else root


def require(source, anchor, count=1):
    if source.count(anchor) != count:
        raise ValueError('shared622A: semantic/adapter boundary changed: ' + anchor)


def dependency_closure(root, previous):
    manifest_owner = 'tools/compact-rust-backend/Cargo.toml'
    manifest = tomllib.loads((root / manifest_owner).read_bytes().decode())
    old_manifest = tomllib.loads(previous[manifest_owner])
    if manifest['dependencies'].pop('unicode-ident', None) != '=1.0.24' or manifest != old_manifest:
        raise ValueError('shared622A: manifest change exceeds existing locked unicode edge')
    locks = {}
    for owner in ('Cargo.lock', 'tools/compact-rust-backend/Cargo.lock'):
        current = tomllib.loads((root / owner).read_bytes().decode())
        old = tomllib.loads(previous[owner])
        backend = [p for p in current['package'] if p['name'] == 'compact-rust-backend']
        unicode = [p for p in current['package'] if p['name'] == 'unicode-ident']
        old_unicode = [p for p in old['package'] if p['name'] == 'unicode-ident']
        if (len(backend) != 1 or len(unicode) != 1 or unicode != old_unicode
                or unicode[0]['version'] != '1.0.24'
                or backend[0]['dependencies'].count('unicode-ident') != 1):
            raise ValueError('shared622A: unicode package cohort changed: ' + owner)
        backend[0]['dependencies'].remove('unicode-ident')
        if current != old:
            raise ValueError('shared622A: package/version/source/checksum/alias drift: ' + owner)
        locks[owner] = {'package_count': len(current['package']),
                        'unicode_version': '1.0.24', 'packages_and_cohort_unchanged': True}
    return locks


def source_closure(root, extension, previous):
    text = lambda owner: (root / owner).read_bytes().decode()
    policies = [BASE + '/pure_semantics/' + name + '.rs'
                for name in ('identifier', 'numeric', 'carrier')]
    for owner in policies:
        source = text(owner)
        for forbidden in ('syn::', 'proc_macro2::', 'quote::', 'parse_quote!', 'parse_str::<'):
            if forbidden in source:
                raise ValueError('shared622A: neutral policy constructs syntax: ' + owner)
        require(source, '#[cfg(test)]\nmod tests;')
    identifier = text(policies[0])
    for anchor in ('pub(crate) struct CheckedName {', '    text: String,', '    raw: bool,',
                   'pub(crate) fn check_name(', 'unicode_ident::is_xid_start(first)',
                   'unicode_ident::is_xid_continue(*ch)', '.or_else(|| parse_raw_retry(&normalized))'):
        require(identifier, anchor)
    naming = text(BASE + '/naming.rs')
    require(naming, 'pub(crate) fn checked_ident(')
    require(naming, 'crate::pure_semantics::identifier::check_name(name)', 2)
    for forbidden in ('syn::parse_str::<syn::Ident>', 'name.replace(\'$\', "_")'):
        if forbidden in naming:
            raise ValueError('shared622A: naming adapter repeats lexical admission')
    carrier = text(policies[2])
    for anchor in ('pub(crate) trait CarrierSink {', 'pub(crate) struct CheckSink;',
                   'type Value = ();', 'type Tuple = ();', 'fold_carrier(ty, &mut CheckSink)',
                   'drop(recording_type_nodes(ty)?);', 'fold_bounded(ty, sink)',
                   'Type::Struct { name, .. } | Type::Enum { name, .. } => Leaf::Named(check_name(name)?)',
                   'return Err(RenderError::UnsupportedLedgerValueType(ty.clone()));'):
        require(carrier, anchor)
    sink = carrier[carrier.index('pub(crate) trait CarrierSink {'):carrier.index('/// A real no-syntax consumer')]
    if 'Result<' in sink or 'RenderError' in sink:
        raise ValueError('shared622A: output sink acquired refusal authority')
    if carrier.index('drop(recording_type_nodes(ty)?);') >= carrier.index('    fold_bounded(ty, sink)'):
        raise ValueError('shared622A: carrier recursion precedes resource bounds')
    value = text(BASE + '/value_lowering.rs')
    for anchor in ('checked_unsigned_cast(source_max, target_max)?',
                   'checked_unsigned_arithmetic(kind, left_max, right_max, result_max)?',
                   'crate::pure_semantics::carrier::fold_carrier(ty, &mut carrier::NativeCarrierSink)'):
        require(value, anchor)
    native = text(BASE + '/value_lowering/carrier.rs')
    require(native, 'impl CarrierSink for NativeCarrierSink {')
    if 'RenderError' in native or 'Result<' in native:
        raise ValueError('shared622A: native carrier adapter acquired refusal authority')
    bridge = text(OWNERS[-1])
    for anchor in ('crate::pure_semantics::identifier::check_name(&source.name)?;',
                   'crate::pure_semantics::identifier::check_name(&parameter.name)?;',
                   'crate::pure_semantics::carrier::check_carrier(&parameter.ty)?;',
                   'crate::pure_semantics::carrier::check_carrier(&source.result)?;',
                   'crate::naming::parameter_idents(&source.parameters, &[])',
                   'crate::expression_with_calls(&source.body, &scope, pure)?;'):
        require(bridge, anchor)
    if 'crate::rust_type(' in bridge or 'crate::ident(' in bridge:
        raise ValueError('shared622A: signature certification still constructs carrier/name syntax')
    require(text(BASE + '/lib.rs'), 'mod pure_semantics;')
    return {'neutral_policies': {owner: digest(text(owner).encode()) for owner in policies},
            'carrier_fold_resource_preflight': True, 'sink_refusal_authority': False,
            'signature_carrier_consumer': 'CheckSink', 'native_carrier_consumer': 'NativeCarrierSink',
            'body_syntax_bridge': 'still pending; existing expression_with_calls and binding symbols retained',
            'locks': dependency_closure(root, previous), 'new_retired_routes': [],
            'claim': 'slice A shared neutral policies/signatures; not full pure body typing or epic closure'}


def verify_extension(root, baseline, extension, historical_verify, selector_signature):
    report = {'schema_version': 1, 'claim': 'source inventory; not semantic coverage',
              'route_count': len(baseline.get('routes', [])), 'retired_routes': [],
              'route_checks': [], 'source_changes': [], 'errors': [], 'passed': False}
    try:
        checked = checkpoint(root, extension)
        prior = historical_verify(checked)
        if not prior['passed']:
            report['errors'] = ['shared622A: predecessor checks failed', *prior['errors']]
            return report
        if prior.get('reviewed_stateful_tuple_extension', {}).get('issue') != 624:
            raise ValueError('shared622A: required stateful624 predecessor authority missing')
        report = copy.deepcopy(prior)
        report['passed'] = False
        if prior['route_count'] != 84 or len(prior['retired_routes']) != 69:
            raise ValueError('shared622A: existing denominator/retirements changed')
        selector = selector_signature((root / SELECTOR).read_bytes().decode())
        if selector != extension['current_selector'] or selector != prior['current_selector']:
            raise ValueError('shared622A: recording authority changed')
        closure = source_closure(root, extension, checked._sources)
        if closure != extension['production_closure']:
            raise ValueError('shared622A: reviewed semantic closure changed')
        report['reviewed_shared_pure_semantics_extension'] = {
            'issue': 622, 'slice': 'A', 'claim': extension['claim'], 'new_retired_routes': [],
            'predecessor_commit': extension['baseline_commit'], 'historical_manifest_count': 67,
            'original_oracles': 49, 'production_closure': closure,
            'owners': copy.deepcopy(extension['owners'])}
        report['passed'] = True
    except (OSError, ValueError, KeyError, UnicodeError, StopIteration) as error:
        report['errors'].append(str(error))
    return report
