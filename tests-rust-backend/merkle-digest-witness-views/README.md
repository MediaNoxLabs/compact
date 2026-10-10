# Implicit Merkle digest needed by witness views

The Compact source declares one plain and one historic MerkleTree<3,Uint8>. Its only exported circuit calls a zero-argument Boolean witness. There are no Compact root/path calls or explicit MerkleTreeDigest annotations. Thus the digest declaration is needed solely by the generated LedgerView API.

The current backend generates this complete retained IR during the fixture build. The runtime test calls both actual generated witness accessors on empty and populated trees. Inside the callback, official roots convert into the generated canonical digest and back using the generated derive; aligned encodings are identical. Typed generated digests are then used for metered check_root reads, forcing the CellValue contract. Correct and deliberately different queries return true/false, respectively. Native, recorded and independently metered frames compare results, one callback/private-state step, private FAB, state/effects and gas.

Witness-local reads do not create public recorded operations. There is no proof obligation or proof claim for this declaration fix; no network/provider/testkit is involved. Populated state uses existing official typed slots outside the Compact circuit, keeping source dependency isolation explicit.

The frozen before-emitter output is retained in the delivery scratch combined-before/contract/lib.rs, with two crate::types::MerkleTreeDigest references but no types declaration. Source emission succeeded; parent must retain the actual unchanged-native compilation failure separately before integrating the fix. A render success alone is not native compilation evidence.
