# Capability guide execution receipt

Only the documentation checker was executed here; real source/API fields were inspected in the signed codebase and frozen ADR0288 report. No compiler or runtime change.

```json
{
  "scope": "Executed documentation checker on actual frozen schema3 DID report; no generated Rust or proof gate rerun.",
  "head": "40047fb86cac845c5706ab404bea444d6ac71762",
  "report_path": "/tmp/rust030-adr288/strict-original-gate-final/did/contract/rust-capabilities.json",
  "report_sha256": "8d3dc57a405875ecce5974e945719b6ae7bbb00b3543de1a40ded0756c15d5d2",
  "checker_sha256": "d74106475ee986f3773768c04b15476f0143d2a25150e4c0d405c7e132857179",
  "runs": [
    {
      "command": [
        "python3",
        "/tmp/rust030-capability-guide/check_observed_api.py",
        "/tmp/rust030-adr288/strict-original-gate-final/did/contract/rust-capabilities.json",
        "verifySchnorrJubjubDigestSignature"
      ],
      "expected_exit": 0,
      "exit": 0,
      "stdout": "Available generated APIs: verifySchnorrJubjubDigestSignature\n",
      "stderr": ""
    },
    {
      "command": [
        "python3",
        "/tmp/rust030-capability-guide/check_observed_api.py",
        "/tmp/rust030-adr288/strict-original-gate-final/did/contract/rust-capabilities.json",
        "setVerificationMethodRelation"
      ],
      "expected_exit": 1,
      "exit": 1,
      "stdout": "",
      "stderr": "setVerificationMethodRelation: recording_unavailable at actions[0].action.action.action.actions[0]\n"
    }
  ]
}
```
