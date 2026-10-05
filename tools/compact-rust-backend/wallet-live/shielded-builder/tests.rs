// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;
use midnight_ledger::{
    semantics::TransactionResult, test_utilities::TestState, verify::WellFormedStrictness,
};
use midnight_zswap::keys::SecretKeys;

fn directory(label: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "compact-builder-{label}-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&path).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
    path
}
#[test]
fn exact_private_binary_refs_and_decoders() -> Result<()> {
    let root = directory("guards");
    let original = write_file(&root, "value", b"test", "fixture")?;
    assert_eq!(referenced(&original, "fixture")?, b"test");
    let mut bad = original.clone();
    bad["sha256"] = json!("00".repeat(32));
    assert!(referenced(&bad, "fixture").is_err());
    bad = original.clone();
    bad["bytes"] = json!(5);
    assert!(referenced(&bad, "fixture").is_err());
    assert!(referenced(&original, "other").is_err());
    fs::set_permissions(root.join("value"), fs::Permissions::from_mode(0o644))?;
    assert!(referenced(&original, "fixture").is_err());
    let address = ContractAddress(HashOutput([4; 32]));
    let mut encoded = encode(&address)?;
    assert_eq!(decode::<ContractAddress>(&encoded)?, address);
    encoded.push(0);
    assert!(decode::<ContractAddress>(&encoded).is_err());
    assert!(raw_hex::<EncryptionPublicKey>("00").is_err());
    Ok(())
}
#[test]
fn candidate_profile_is_exact_and_remains_a_candidate() -> Result<()> {
    let valid = json!({"ledgerVersion":"8.0.3","nodeLedgerConstraint":"=8.0.2","nodeSoftwareVersion":"0.22.3-6f0ef437","indexerVersion":"4.0.1","compatibilityProfile":"node-0.22.3/indexer-4.0.1/ledger-8.0.2/codec-8.0.3","compatibilityStatus":"candidate-live-validation-pending"});
    validate_node_profile(&valid)?;
    for name in [
        "ledgerVersion",
        "nodeLedgerConstraint",
        "nodeSoftwareVersion",
        "indexerVersion",
        "compatibilityProfile",
        "compatibilityStatus",
    ] {
        let mut changed = valid.clone();
        changed[name] = json!("unknown");
        assert!(validate_node_profile(&changed).is_err());
    }
    let mut accepted = valid.clone();
    accepted["compatibilityStatus"] = json!("accepted");
    assert!(validate_node_profile(&accepted).is_err());
    Ok(())
}
fn checkpoint_file(
    root: &Path,
    state: &TestState<DefaultDB>,
    wallet: &midnight_zswap::local::State<DefaultDB>,
    address: ContractAddress,
    step: u8,
) -> Result<Value> {
    let dir = root.join(format!("checkpoint-{step}"));
    fs::create_dir(&dir)?;
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o700))?;
    let node = state
        .ledger
        .contract
        .get(&address)
        .ok_or("missing deployed state")?;
    let projection = midnight_zswap::ledger::State {
        coin_coms: state.ledger.zswap.filter(&[address]),
        ..midnight_zswap::ledger::State::new()
    };
    let node_ref = write_file(&dir, "contract.bin", &encode(node)?, "ContractState")?;
    let tree_ref = write_file(&dir, "tree.bin", &encode(&projection)?, "ZswapChainState")?;
    let wallet_ref = write_file(&dir, "wallet.bin", &encode(wallet)?, "ZswapLocalState")?;
    let mut root_bytes = vec![];
    state
        .ledger
        .zswap
        .coin_coms
        .rehash()
        .root()
        .ok_or("missing root")?
        .serialize(&mut root_bytes)?;
    let block = hex::encode([step; 32]);
    let height = step.to_string();
    let value = json!({"format":FORMAT,"kind":"checkpoint","metadata":{"networkId":"local-test","ledgerVersion":"8.0.3","nodeLedgerConstraint":"=8.0.3","addressHex":hex::encode(address.0.0),"transactionHash":block,"blockHash":block,"blockHeight":height,"parentBlockHash":hex::encode([step.saturating_sub(1);32]),"nodeZswapRootHex":hex::encode(root_bytes),"firstFree":state.ledger.zswap.first_free.to_string(),"finalZswapEventId":height},"wallet":{"networkId":"local-test","ledgerVersion":"8.0.3","blockHash":block,"blockHeight":height,"appliedEventId":height,"walletStateSha256":wallet_ref["sha256"]},"files":{"nodeContract":node_ref,"contractTree":tree_ref,"walletState":wallet_ref}});
    // Negative transport consistency controls use actual upstream bytes.
    let mut wrong = value.clone();
    wrong["metadata"]["nodeLedgerConstraint"] = json!("=8.0.2");
    assert!(checkpoint(&wrong, "local-test").is_err());
    wrong = value.clone();
    wrong["metadata"]["firstFree"] = json!((state.ledger.zswap.first_free + 1).to_string());
    assert!(checkpoint(&wrong, "local-test").is_err());
    wrong = value.clone();
    wrong["wallet"]["walletStateSha256"] = json!("00".repeat(32));
    assert!(checkpoint(&wrong, "local-test").is_err());
    assert!(checkpoint(&value, "wrong-network").is_err());
    checkpoint(&value, "local-test")?;
    let mut changed_tree = projection.clone();
    changed_tree.coin_coms = midnight_zswap::ledger::State::<DefaultDB>::new()
        .coin_coms
        .update_hash(0, HashOutput([99; 32]), None)
        .rehash();
    wrong = value.clone();
    wrong["files"]["contractTree"] = write_file(
        &dir,
        "changed-tree.bin",
        &encode(&changed_tree)?,
        "ZswapChainState",
    )?;
    assert!(
        checkpoint(&wrong, "local-test")
            .err()
            .ok_or("changed tree accepted")?
            .to_string()
            .contains("RootMismatch")
    );
    let mut changed_wallet = wallet.clone();
    changed_wallet.merkle_tree = midnight_zswap::local::State::<DefaultDB>::new()
        .merkle_tree
        .update_hash(0, HashOutput([98; 32]), ())
        .rehash();
    wrong = value.clone();
    wrong["files"]["walletState"] = write_file(
        &dir,
        "changed-wallet.bin",
        &encode(&changed_wallet)?,
        "ZswapLocalState",
    )?;
    wrong["wallet"]["walletStateSha256"] = wrong["files"]["walletState"]["sha256"].clone();
    assert!(
        checkpoint(&wrong, "local-test")
            .err()
            .ok_or("changed wallet tree accepted")?
            .to_string()
            .contains("RootMismatch")
    );
    write_file(
        &dir,
        "checkpoint.json",
        &serde_json::to_vec(&value)?,
        "checkpoint-json",
    )
}
fn apply_result(
    root: &Path,
    state: &mut TestState<DefaultDB>,
    wallet: &mut midnight_zswap::local::State<DefaultDB>,
    keys: &SecretKeys,
    artifacts: &Path,
    rng: &mut StdRng,
) -> Result<Value> {
    let result: Value = serde_json::from_slice(&private_read(&root.join("result.json"))?)?;
    let tx: Sealed = decode(&referenced(
        &result["transaction"],
        "Transaction<Signature,Proof,Binding>",
    )?)?;
    let Transaction::Standard(before) = &tx else {
        return Err("not standard".into());
    };
    let offer = before
        .guaranteed_coins
        .as_ref()
        .map(|offer| encode(offer))
        .transpose()?;
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let balanced =
        rt.block_on(state.balance_tx(StdRng::from_rng(rng)?, tx, &resolver(artifacts)?))?;
    let Transaction::Standard(after) = &balanced else {
        return Err("not standard".into());
    };
    assert_eq!(
        offer,
        after
            .guaranteed_coins
            .as_ref()
            .map(|offer| encode(offer))
            .transpose()?,
        "fee balancing must preserve exact offer proof identity"
    );
    if let Some(offer) = &after.guaranteed_coins {
        *wallet = wallet.apply(keys, offer);
    }
    assert!(matches!(
        state.apply(&balanced, WellFormedStrictness::default())?,
        TransactionResult::Success(_)
    ));
    if result["action"] == "accept" || result["action"] == "release" {
        assert!(
            balanced
                .well_formed(&state.ledger, WellFormedStrictness::default(), state.time)
                .is_err(),
            "spent input replay must fail"
        );
    }
    assert_eq!(wallet.first_free, state.ledger.zswap.first_free);
    assert_eq!(
        wallet.merkle_tree.rehash().root(),
        state.ledger.zswap.coin_coms.rehash().root()
    );
    Ok(result)
}
#[test]
#[ignore = "requires exact acceptance keys and pinned proof parameters; offline strict acceptance"]
fn offline_generated_actions_strict() -> Result<()> {
    let artifacts = PathBuf::from(std::env::var("COMPACT_SHIELDED_ARTIFACTS")?);
    let root = directory("strict");
    let mut rng = StdRng::seed_from_u64(217_2026);
    let mut state = TestState::new(&mut rng);
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(state.give_fee_token(&mut rng, 10));
    assert_eq!(
        state.ledger.zswap.first_free, 0,
        "no shielded genesis coin insertion"
    );
    let keys = SecretKeys::from_rng_seed(&mut rng);
    let mut wallet = midnight_zswap::local::State::new();
    let request = |state: &TestState<DefaultDB>| json!({"format":FORMAT,"kind":"action-request","networkId":"local-test","ttlEpochSeconds":(state.time.to_secs()+3600).to_string(),"artifactsRoot":artifacts});
    let deploy_dir = root.join("deploy");
    execute(
        "deploy",
        &request(&state),
        &deploy_dir,
        state.time.to_secs(),
    )?;
    let deployed = apply_result(
        &deploy_dir,
        &mut state,
        &mut wallet,
        &keys,
        &artifacts,
        &mut rng,
    )?;
    let address = ContractAddress(HashOutput(hex32(&deployed, "addressHex")?));
    let deployed_state = state
        .ledger
        .contract
        .get(&address)
        .ok_or("missing deployment")?;
    for name in ["bootstrap", "accept", "release"] {
        assert!(
            deployed_state
                .operations
                .get(&EntryPointBuf(name.as_bytes().to_vec()))
                .is_some()
        );
    }
    let mut enc = vec![];
    keys.enc_public_key().serialize(&mut enc)?;
    let mut bootstrap = request(&state);
    bootstrap["checkpoint"] = checkpoint_file(&root, &state, &wallet, address, 1)?;
    bootstrap["domainHex"] = json!(hex::encode([0x17; 32]));
    bootstrap["nonceHex"] = json!(hex::encode([9; 32]));
    bootstrap["amount"] = json!("42");
    bootstrap["walletCoinPublicKeyHex"] = json!(hex::encode(keys.coin_public_key().0.0));
    bootstrap["walletEncryptionPublicKeyHex"] = json!(hex::encode(enc));
    let bootstrap_dir = root.join("bootstrap");
    execute(
        "bootstrap",
        &bootstrap,
        &bootstrap_dir,
        state.time.to_secs(),
    )?;
    let minted = apply_result(
        &bootstrap_dir,
        &mut state,
        &mut wallet,
        &keys,
        &artifacts,
        &mut rng,
    )?;
    let owned = *wallet
        .coins
        .iter()
        .next()
        .ok_or("encrypted bootstrap not recovered")?
        .1;
    assert_eq!(owned, coin(&minted["coin"])?.qualify(0));
    assert_eq!(wallet.coins.iter().count(), 1);
    let (_, input) = wallet.spend(&mut rng, &keys, &owned, None)?;
    let mut accept = request(&state);
    accept["checkpoint"] = checkpoint_file(&root, &state, &wallet, address, 2)?;
    accept["input"] = write_file(
        &root,
        "wallet-input.bin",
        &encode(&input)?,
        "Input<ProofPreimage>",
    )?;
    accept["coin"] = minted["coin"].clone();
    let mut wrong = accept.clone();
    wrong["coin"]["value"] = json!("41");
    assert!(
        execute(
            "accept",
            &wrong,
            &root.join("wrong-coin"),
            state.time.to_secs()
        )
        .unwrap_err()
        .to_string()
        .contains("selected input")
    );
    let accept_dir = root.join("accept");
    execute("accept", &accept, &accept_dir, state.time.to_secs())?;
    let accepted = apply_result(
        &accept_dir,
        &mut state,
        &mut wallet,
        &keys,
        &artifacts,
        &mut rng,
    )?;
    assert_eq!(wallet.coins.iter().count(), 0);
    let mut release = request(&state);
    release["checkpoint"] = checkpoint_file(&root, &state, &wallet, address, 3)?;
    release["coin"] = accepted["coin"].clone();
    release["walletCoinPublicKeyHex"] = bootstrap["walletCoinPublicKeyHex"].clone();
    release["walletEncryptionPublicKeyHex"] = bootstrap["walletEncryptionPublicKeyHex"].clone();
    let mut wrong = release.clone();
    wrong["coin"]["mtIndex"] = json!("0");
    assert!(
        execute(
            "release",
            &wrong,
            &root.join("wrong-index"),
            state.time.to_secs()
        )
        .is_err()
    );
    wrong = release.clone();
    wrong["coin"]["mtIndex"] = json!((1_u64 << 32).to_string());
    assert!(
        execute(
            "release",
            &wrong,
            &root.join("out-of-range"),
            state.time.to_secs()
        )
        .unwrap_err()
        .to_string()
        .contains("owned visible leaf")
    );
    let checkpoint_bytes = referenced(&release["checkpoint"], "checkpoint-json")?;
    let mut changed: Value = serde_json::from_slice(&checkpoint_bytes)?;
    let tree_bytes = referenced(&changed["files"]["contractTree"], "ZswapChainState")?;
    let mut projection: midnight_zswap::ledger::State<DefaultDB> = decode(&tree_bytes)?;
    let index = number(&release["coin"], "mtIndex")?;
    let expected =
        coin(&release["coin"])?.commitment(&runtime::ledger::CoinRecipient::Contract(address));
    projection.coin_coms = projection
        .coin_coms
        .update_hash(
            index,
            expected.0,
            Some(midnight_storage::arena::Sp::new(ContractAddress(
                HashOutput([88; 32]),
            ))),
        )
        .rehash();
    changed["files"]["contractTree"] = write_file(
        &root,
        "wrong-owner-tree.bin",
        &encode(&projection)?,
        "ZswapChainState",
    )?;
    wrong = release.clone();
    wrong["checkpoint"] = write_file(
        &root,
        "wrong-owner-checkpoint.json",
        &serde_json::to_vec(&changed)?,
        "checkpoint-json",
    )?;
    assert!(
        execute(
            "release",
            &wrong,
            &root.join("wrong-owner"),
            state.time.to_secs()
        )
        .unwrap_err()
        .to_string()
        .contains("owned visible leaf")
    );
    let release_dir = root.join("release");
    execute("release", &release, &release_dir, state.time.to_secs())?;
    let released = apply_result(
        &release_dir,
        &mut state,
        &mut wallet,
        &keys,
        &artifacts,
        &mut rng,
    )?;
    let recovered = *wallet
        .coins
        .iter()
        .next()
        .ok_or("encrypted release not recovered")?
        .1;
    assert_eq!(recovered, coin(&released["coin"])?.qualify(2));
    assert_eq!(recovered.value, 42);
    assert_eq!(wallet.coins.iter().count(), 1);
    println!(
        "offline strict deploy/bootstrap/accept/release passed; encrypted wallet ownership 0→1→0→1, exact offer identity, selected-coin/membership/metadata refusals and spent-input replay refusal; artifacts {}",
        root.display()
    );
    Ok(())
}
