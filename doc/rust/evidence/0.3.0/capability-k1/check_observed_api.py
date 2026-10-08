import json
import sys

path, *required_exports = sys.argv[1:]
if not required_exports:
    raise SystemExit('Specify the stateful exports your application needs.')
with open(path, encoding='utf-8') as handle:
    report = json.load(handle)
if report.get('schema_version') != 3 or not isinstance(report.get('circuits'), list):
    raise SystemExit('Expected a finalized Rust capability schema-3 report.')
rows = report['circuits']
by_name = {row['name']: row for row in rows}
if len(by_name) != len(rows):
    raise SystemExit('Duplicate capability names.')
failures = []
for name in required_exports:
    row = by_name.get(name)
    if row is None:
        failures.append(f'{name}: not a reported stateful export')
    elif row.get('proof_required') is not True:
        failures.append(f'{name}: proof is not applicable; choose the pure/native API')
    elif not (row.get('recorded') is True and row.get('observed_call') is True
              and row.get('recording_status') == 'available'):
        gap = row.get('observed_call_unavailable') or row.get('recording_unavailable') or {}
        failures.append(f"{name}: {gap.get('code', 'unavailable')} at {gap.get('path', '<no path>')}")
if failures:
    raise SystemExit('\n'.join(failures))
print('Available generated APIs: ' + ', '.join(required_exports))
