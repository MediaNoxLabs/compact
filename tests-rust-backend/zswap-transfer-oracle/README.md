# Current-generated transfer compatibility fixture

`build.rs` renders the complete fresh `zswap-transfer-schema20-ir.json` with the current backend. `RETAINED_IR` is the proof adapter's whole-source pairing input. The existing independent TypeScript rows and runtime assertions are preserved.

This fixture contains two public exports, `transfer` and `read_coin`. Body-owner attribution tests qualify the new recording algebra separately from generated-crate compilation. Proof readiness requires full source/IR/current Rust pairing and the original funded proof adapter; rendering alone does not establish funding or proof validity.

The `transfer` adapter checks the complete artifact IR before keys or deployment. This checks input identity, not proving-key provenance. Source and key receipts must bind the current frontend/ZKIR and actual key files.
