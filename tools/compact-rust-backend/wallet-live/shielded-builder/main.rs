// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Acceptance-only standalone consumer. No synthetic ledger or wallet secrets.
use compact_contract_shielded::{ledger_contract as contract, types};
use midnight_base_crypto::{
    data_provider::{FetchMode, MidnightDataProvider, OutputMode},
    hash::persistent_hash,
    signatures::Signature,
    time::Timestamp,
};
use midnight_compact_runtime as runtime;
use midnight_ledger::{
    dust::{DUST_EXPECTED_FILES, DustResolver},
    prove::Resolver as LedgerResolver,
    structure::{
        ContractDeploy, INITIAL_PARAMETERS, Intent, ProofMarker, ProofPreimageMarker, Transaction,
    },
};
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::{
    Deserializable, Serializable, Tagged, tagged_deserialize, tagged_serialize,
};
use midnight_storage::storage::HashMap;
use midnight_transient_crypto::{
    commitment::{PedersenRandomness, PureGeneratorPedersen},
    curve::Fr,
    encryption::PublicKey as EncryptionPublicKey,
    merkle_tree::MerkleTreeDigest,
    proofs::{KeyLocation, ProofPreimage, ProvingKeyMaterial, Resolver, VerifierKey},
};
use midnight_zkir::LocalProvingProvider;
use midnight_zswap::{Input, Offer, Output, prove::ZswapResolver};
use rand::{SeedableRng, rngs::StdRng};
use runtime::context::{CircuitContext, ConstructorContext};
use runtime::ledger::{
    CoinInfo, CoinNonce, CoinPublicKey, ContractAddress, DefaultDB, HashOutput, ShieldedTokenType,
};
use runtime::transaction::{
    CheckpointMetadata, Observation, ObservationalOfferBackedState, ObservedContractState,
    TrustedObservationCheckpoint, TrustedObservationParts, WalletCheckpointMetadata,
    WalletFundingInputs,
};
use runtime::{BoundedUint, FixedBytes};
use serde_json::{Value, json};
use std::io::Write;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::{
    error::Error,
    fs, io,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

type Result<T> = std::result::Result<T, Box<dyn Error>>;
type Sealed = Transaction<Signature, ProofMarker, PureGeneratorPedersen, DefaultDB>;
type Unproven = Transaction<Signature, ProofPreimageMarker, PedersenRandomness, DefaultDB>;
const FORMAT: &str = "compact-shielded-live/v1";

fn text<'a>(v: &'a Value, field: &str) -> Result<&'a str> {
    v[field]
        .as_str()
        .ok_or_else(|| format!("missing string {field}").into())
}
fn number(v: &Value, field: &str) -> Result<u64> {
    Ok(text(v, field)?.parse()?)
}
fn hex32(v: &Value, field: &str) -> Result<[u8; 32]> {
    hex::decode(text(v, field)?)?
        .try_into()
        .map_err(|_| format!("{field} must contain exactly 32 bytes").into())
}
fn hash(bytes: &[u8]) -> String {
    hex::encode(persistent_hash(bytes).0)
}
fn private_read(path: &Path) -> Result<Vec<u8>> {
    let parent = path.parent().ok_or("missing private handoff directory")?;
    if fs::metadata(parent)?.permissions().mode() & 0o077 != 0 {
        return Err("handoff directory must be private".into());
    }
    let meta = fs::symlink_metadata(path)?;
    if !meta.is_file() || meta.permissions().mode() & 0o077 != 0 {
        return Err("handoff file must be a private regular file".into());
    }
    if meta.len() > 256 * 1024 * 1024 {
        return Err("handoff file exceeds bounded acceptance size".into());
    }
    Ok(fs::read(path)?)
}
fn referenced(v: &Value, encoding: &str) -> Result<Vec<u8>> {
    if text(v, "encoding")? != encoding {
        return Err("unexpected binary encoding".into());
    }
    let path = Path::new(text(v, "path")?);
    if !path.is_absolute() {
        return Err("handoff paths must be absolute".into());
    }
    let bytes = private_read(path)?;
    if v["bytes"].as_u64() != Some(bytes.len() as u64) || text(v, "sha256")? != hash(&bytes) {
        return Err("handoff length or SHA256 mismatch".into());
    }
    Ok(bytes)
}
fn decode<T: Deserializable + Tagged>(bytes: &[u8]) -> Result<T> {
    let mut source = bytes;
    let value = tagged_deserialize(&mut source)?;
    if !source.is_empty() {
        return Err("trailing tagged bytes".into());
    }
    Ok(value)
}
fn raw_hex<T: Deserializable>(s: &str) -> Result<T> {
    let bytes = hex::decode(s)?;
    let mut source = &bytes[..];
    let value = T::deserialize(&mut source, 0)?;
    if !source.is_empty() {
        return Err("trailing raw hex bytes".into());
    }
    Ok(value)
}
fn encode<T: Serializable + Tagged>(value: &T) -> Result<Vec<u8>> {
    let mut bytes = vec![];
    tagged_serialize(value, &mut bytes)?;
    Ok(bytes)
}
fn write_file(root: &Path, name: &str, bytes: &[u8], encoding: &str) -> Result<Value> {
    let path = root.join(name);
    let mut file = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(0o600)
        .open(&path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(json!({"path":path,"sha256":hash(bytes),"bytes":bytes.len(),"encoding":encoding}))
}
fn check_format(v: &Value, kind: &str) -> Result<()> {
    if text(v, "format")? != FORMAT || text(v, "kind")? != kind {
        return Err("unsupported manifest format/kind".into());
    }
    Ok(())
}
fn coin(v: &Value) -> Result<CoinInfo> {
    Ok(CoinInfo {
        nonce: CoinNonce(HashOutput(hex32(v, "nonceHex")?)),
        type_: ShieldedTokenType(HashOutput(hex32(v, "colorHex")?)),
        value: text(v, "value")?.parse()?,
    })
}
fn compact(coin: CoinInfo) -> Result<types::ShieldedCoinInfo> {
    Ok(types::ShieldedCoinInfo {
        nonce: FixedBytes::new(coin.nonce.0.0),
        color: FixedBytes::new(coin.type_.0.0),
        value: BoundedUint::new(coin.value)?,
    })
}
fn qualified(coin: CoinInfo, index: u64) -> Result<types::QualifiedShieldedCoinInfo> {
    Ok(types::QualifiedShieldedCoinInfo {
        nonce: FixedBytes::new(coin.nonce.0.0),
        color: FixedBytes::new(coin.type_.0.0),
        value: BoundedUint::new(coin.value)?,
        mt_index: BoundedUint::new(index.into())?,
    })
}
fn coin_json(coin: CoinInfo, index: u64) -> Value {
    json!({"nonceHex":hex::encode(coin.nonce.0.0),"colorHex":hex::encode(coin.type_.0.0),"value":coin.value.to_string(),"mtIndex":index.to_string()})
}
fn key(root: &Path, name: &str) -> Result<VerifierKey> {
    decode(&fs::read(root.join(format!("keys/{name}.verifier")))?)
}
struct ArtifactResolver {
    root: PathBuf,
}
impl Resolver for ArtifactResolver {
    async fn resolve_key(&self, location: KeyLocation) -> io::Result<Option<ProvingKeyMaterial>> {
        let name = location.0.as_ref();
        if !["bootstrap", "accept", "release"].contains(&name) {
            return Ok(None);
        }
        Ok(Some(ProvingKeyMaterial {
            prover_key: fs::read(self.root.join(format!("keys/{name}.prover")))?,
            verifier_key: fs::read(self.root.join(format!("keys/{name}.verifier")))?,
            ir_source: fs::read(self.root.join(format!("zkir/{name}.bzkir")))?,
        }))
    }
}
fn resolver(root: &Path) -> Result<LedgerResolver> {
    let zswap = ZswapResolver(MidnightDataProvider::new(
        FetchMode::Synchronous,
        OutputMode::Log,
        midnight_zswap::ZSWAP_EXPECTED_FILES.to_vec(),
    )?);
    let dust = DustResolver(MidnightDataProvider::new(
        FetchMode::Synchronous,
        OutputMode::Log,
        DUST_EXPECTED_FILES.to_vec(),
    )?);
    let root = root.to_owned();
    Ok(LedgerResolver::new(
        zswap,
        dust,
        Box::new(move |location| {
            let resolver = ArtifactResolver { root: root.clone() };
            Box::pin(async move { resolver.resolve_key(location).await })
        }),
    ))
}

fn prove(tx: Unproven, root: &Path, rng: &mut StdRng) -> Result<Sealed> {
    let resolver = resolver(root)?;
    let params = MidnightDataProvider::new(FetchMode::OnDemand, OutputMode::Log, vec![])?;
    let provider = LocalProvingProvider {
        rng: StdRng::from_rng(&mut *rng)?,
        resolver: &resolver,
        params: &params,
    };
    Ok(futures_executor::block_on(
        tx.prove(provider, &INITIAL_PARAMETERS.cost_model.runtime_cost_model),
    )?
    .seal(StdRng::from_rng(rng)?))
}
struct Checkpoint {
    observed: ObservedContractState,
    checked: TrustedObservationCheckpoint,
    projection: midnight_zswap::ledger::State<DefaultDB>,
    frontier: u64,
    wallet: midnight_zswap::local::State<DefaultDB>,
    node_profile: Value,
}
fn validate_node_profile(metadata: &Value) -> Result<()> {
    if text(metadata, "ledgerVersion")? != "8.0.3" {
        return Err("unsupported decoder version".into());
    }
    if text(metadata, "nodeLedgerConstraint")? == "=8.0.3" {
        return Ok(());
    }
    let pins = [
        (
            "compatibilityProfile",
            "node-0.22.3/indexer-4.0.1/ledger-8.0.2/codec-8.0.3",
        ),
        ("nodeSoftwareVersion", "0.22.3-6f0ef437"),
        ("indexerVersion", "4.0.1"),
        ("nodeLedgerConstraint", "=8.0.2"),
        ("compatibilityStatus", "candidate-live-validation-pending"),
    ];
    if pins
        .iter()
        .all(|(name, expected)| metadata[*name].as_str() == Some(*expected))
    {
        Ok(())
    } else {
        Err("unsupported node/codec profile; explicit compatibility review required".into())
    }
}

fn checkpoint(v: &Value, network: &str) -> Result<Checkpoint> {
    check_format(v, "checkpoint")?;
    let m = &v["metadata"];
    let w = &v["wallet"];
    validate_node_profile(m)?;
    if text(m, "networkId")? != network {
        return Err("request/checkpoint network mismatch".into());
    }
    let address = ContractAddress(HashOutput(hex32(m, "addressHex")?));
    let observation = Observation {
        transaction_hash: hex32(m, "transactionHash")?,
        block_hash: hex32(m, "blockHash")?,
        block_height: number(m, "blockHeight")?,
    };
    let node_contract: ContractState<DefaultDB> =
        decode(&referenced(&v["files"]["nodeContract"], "ContractState")?)?;
    let projection: midnight_zswap::ledger::State<DefaultDB> =
        decode(&referenced(&v["files"]["contractTree"], "ZswapChainState")?)?;
    let wallet_bytes = referenced(&v["files"]["walletState"], "ZswapLocalState")?;
    if text(w, "walletStateSha256")? != hash(&wallet_bytes) {
        return Err("acquired wallet snapshot digest mismatch".into());
    }
    let wallet: midnight_zswap::local::State<DefaultDB> = decode(&wallet_bytes)?;
    let frontier = number(m, "firstFree")?;
    let observed = ObservedContractState::new(address, node_contract.clone(), observation);
    let checked = TrustedObservationCheckpoint::from_trusted_sources(TrustedObservationParts {
        metadata: CheckpointMetadata {
            observation,
            address,
            network_id: network.into(),
            ledger_version: text(m, "ledgerVersion")?.into(),
            zswap_root: raw_hex::<MerkleTreeDigest>(text(m, "nodeZswapRootHex")?)?,
            first_free: frontier,
            final_zswap_event_id: number(m, "finalZswapEventId")?,
        },
        node_contract,
        contract_tree: projection.coin_coms.clone(),
        wallet: wallet.clone(),
        wallet_checkpoint: WalletCheckpointMetadata {
            block_hash: hex32(w, "blockHash")?,
            block_height: number(w, "blockHeight")?,
            network_id: text(w, "networkId")?.into(),
            ledger_version: text(w, "ledgerVersion")?.into(),
            applied_event_id: number(w, "appliedEventId")?,
            wallet_state_sha256: persistent_hash(&wallet_bytes).0,
        },
    })?;
    Ok(Checkpoint {
        observed,
        checked,
        projection,
        frontier,
        wallet,
        node_profile: json!({"nodeLedgerConstraint":m["nodeLedgerConstraint"],"ledgerVersion":m["ledgerVersion"],"compatibilityProfile":m["compatibilityProfile"],"nodeSoftwareVersion":m["nodeSoftwareVersion"],"indexerVersion":m["indexerVersion"],"compatibilityStatus":m["compatibilityStatus"]}),
    })
}
fn run() -> Result<()> {
    let args = std::env::args_os().skip(1).collect::<Vec<_>>();
    if args.len() != 3 {
        return Err("usage: shielded-builder <deploy|bootstrap|accept|release> <request.json> <new-output-directory>".into());
    }
    let action = args[0].to_str().ok_or("invalid action")?;
    if !["deploy", "bootstrap", "accept", "release"].contains(&action) {
        return Err("unsupported action".into());
    }
    let request: Value = serde_json::from_slice(&private_read(Path::new(&args[1]))?)?;
    execute(
        action,
        &request,
        Path::new(&args[2]),
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
    )
}

fn execute(action: &str, request: &Value, output: &Path, now: u64) -> Result<()> {
    check_format(request, "action-request")?;
    let network = text(request, "networkId")?;
    if network.trim() != network || network.is_empty() {
        return Err("invalid network".into());
    }
    let expiry = number(request, "ttlEpochSeconds")?;
    if expiry < now + 60 || expiry > now + 3600 {
        return Err("TTL must be 60 seconds to one hour in the future".into());
    }
    let ttl = Timestamp::from_secs(expiry);
    let root = Path::new(text(request, "artifactsRoot")?);
    let provenance: Value = serde_json::from_str(include_str!("build-provenance.json"))?;
    if provenance["generated"]["lib.rs"] != hash(&fs::read(root.join("contract/lib.rs"))?)
        || provenance["generated"]["Cargo.toml"]
            != hash(&fs::read(root.join("contract/Cargo.toml"))?)
    {
        return Err("artifact contract differs from unedited compiled generated crate".into());
    }
    let mut rng = StdRng::from_entropy();
    let (tx, address, result_coin, checkpoint_hash, node_profile) = if action == "deploy" {
        let initial = contract::initial_state(ConstructorContext::new(()))?;
        let mut ops = HashMap::new();
        for name in ["bootstrap", "accept", "release"] {
            ops = ops.insert(
                EntryPointBuf(name.as_bytes().to_vec()),
                ContractOperation::new(Some(key(root, name)?)),
            );
        }
        let state = ContractState::new(
            initial.ledger_state.get_ref().clone(),
            ops,
            ContractMaintenanceAuthority::default(),
        );
        let deploy = ContractDeploy::new(&mut rng, state);
        let address = deploy.address();
        let intent: Intent<Signature, ProofPreimageMarker, PedersenRandomness, DefaultDB> =
            Intent::empty(&mut rng, ttl).add_deploy(deploy);
        (
            Transaction::from_intents(network, HashMap::new().insert(1_u16, intent)),
            address,
            None,
            None,
            Value::Null,
        )
    } else {
        let checkpoint_bytes = referenced(&request["checkpoint"], "checkpoint-json")?;
        let checkpoint_json: Value = serde_json::from_slice(&checkpoint_bytes)?;
        let cp = checkpoint(&checkpoint_json, network)?;
        let address = cp.observed.address();
        let frontier = cp.frontier;
        let node_profile = cp.node_profile.clone();
        let (offer, funding, output_coin) = match action {
            "bootstrap" => {
                let domain = HashOutput(hex32(request, "domainHex")?);
                let amount = number(request, "amount")?;
                let coin = CoinInfo {
                    nonce: CoinNonce(HashOutput(hex32(request, "nonceHex")?)),
                    type_: address.custom_shielded_token_type(domain),
                    value: amount.into(),
                };
                let pk = CoinPublicKey(HashOutput(hex32(request, "walletCoinPublicKeyHex")?));
                let enc: EncryptionPublicKey =
                    raw_hex(text(request, "walletEncryptionPublicKeyHex")?)?;
                let out = Output::new(&mut rng, &coin, None, &pk, Some(enc))?;
                (
                    Offer::new(vec![], vec![out], vec![]).ok_or("empty bootstrap")?,
                    None,
                    coin,
                )
            }
            "accept" => {
                let input: Input<ProofPreimage, DefaultDB> =
                    decode(&referenced(&request["input"], "Input<ProofPreimage>")?)?;
                let coin = coin(&request["coin"])?;
                let selected = coin.qualify(number(&request["coin"], "mtIndex")?);
                let known = cp
                    .wallet
                    .coins
                    .get(&input.nullifier)
                    .or_else(|| cp.wallet.pending_spends.get(&input.nullifier));
                if known != Some(&selected) {
                    return Err("selected input does not match checkpoint wallet coin".into());
                }
                let funding = WalletFundingInputs::from_inputs(vec![input.clone()])
                    .map_err(|_| "invalid explicit wallet input selection")?;
                let out = Output::new_contract_owned(&mut rng, &coin, None, address)?;
                (
                    Offer::new(vec![input], vec![out], vec![]).ok_or("empty accept")?,
                    Some(funding),
                    coin,
                )
            }
            "release" => {
                let coin = coin(&request["coin"])?;
                let index = number(&request["coin"], "mtIndex")?;
                let pk = CoinPublicKey(HashOutput(hex32(request, "walletCoinPublicKeyHex")?));
                let enc: EncryptionPublicKey =
                    raw_hex(text(request, "walletEncryptionPublicKeyHex")?)?;
                // The filtered upstream tree may collapse unrelated leaves;
                // index() panics there. Enumerate visible leaves before asking
                // the upstream constructor for this exact membership witness.
                let expected = coin.commitment(&runtime::ledger::CoinRecipient::Contract(address));
                let visible = cp
                    .projection
                    .coin_coms
                    .iter_aux()
                    .find(|(at, _)| *at == index);
                if index >= frontier
                    || !matches!(visible, Some((_, (commitment, Some(owner)))) if commitment == expected.0 && *owner == address)
                {
                    return Err(
                        "qualified coin is not an owned visible leaf at its claimed index".into(),
                    );
                }
                let input = Input::new_contract_owned(
                    &mut rng,
                    &coin.qualify(index),
                    None,
                    address,
                    &cp.projection.coin_coms,
                )?;
                let mut preview =
                    CircuitContext::from_contract_state((), address, cp.observed.contract());
                preview.set_zswap_output_start(frontier)?;
                let native = contract::release(
                    preview,
                    qualified(coin, index)?,
                    types::ZswapCoinPublicKey {
                        bytes: FixedBytes::new(pk.0.0),
                    },
                    BoundedUint::new(coin.value)?,
                )?;
                if native.result.change.is_some {
                    return Err("full-value release unexpectedly produced change".into());
                }
                let sent = native.result.sent;
                let sent = runtime::ledger::coin_info_from_compact(
                    sent.nonce,
                    sent.color,
                    sent.value.value(),
                );
                let out = Output::new(&mut rng, &sent, None, &pk, Some(enc))?;
                (
                    Offer::new(vec![input], vec![out], vec![]).ok_or("empty release")?,
                    None,
                    sent,
                )
            }
            _ => unreachable!(),
        };
        let bound = ObservationalOfferBackedState::bind(cp.observed, cp.checked, offer, funding)?;
        let tx = match action {
            "bootstrap" => {
                let pk = CoinPublicKey(HashOutput(hex32(request, "walletCoinPublicKeyHex")?));
                let recipient = runtime::ledger::CoinRecipient::User(pk);
                let call = contract::Contract::default().recording().bootstrap_call(
                    bound.observed(),
                    (),
                    FixedBytes::new(hex32(request, "domainHex")?),
                    BoundedUint::new(number(request, "amount")?.into())?,
                    compact(output_coin)?,
                    types::Either {
                        is_left: true,
                        left: types::ZswapCoinPublicKey {
                            bytes: FixedBytes::new(pk.0.0),
                        },
                        right: types::ContractAddress {
                            bytes: FixedBytes::new([0; 32]),
                        },
                    },
                    FixedBytes::new(output_coin.commitment(&recipient).0.0),
                )?;
                bound
                    .prepare(call, key(root, action)?, Fr::from(0))?
                    .into_transaction(&mut rng, ttl)
            }
            "accept" => {
                let call = contract::Contract::default().recording().accept_call(
                    bound.observed(),
                    (),
                    compact(output_coin)?,
                )?;
                bound
                    .prepare(call, key(root, action)?, Fr::from(0))?
                    .into_transaction(&mut rng, ttl)
            }
            "release" => {
                let coin = coin(&request["coin"])?;
                let call = contract::Contract::default().recording().release_call(
                    bound.observed(),
                    (),
                    qualified(coin, number(&request["coin"], "mtIndex")?)?,
                    types::ZswapCoinPublicKey {
                        bytes: FixedBytes::new(hex32(request, "walletCoinPublicKeyHex")?),
                    },
                    BoundedUint::new(coin.value)?,
                )?;
                let sent = &call.recorded().execution.result;
                if sent.change.is_some
                    || runtime::ledger::coin_info_from_compact(
                        sent.sent.nonce,
                        sent.sent.color,
                        sent.sent.value.value(),
                    ) != output_coin
                {
                    return Err("recorded release differs from generated preview".into());
                }
                bound
                    .prepare(call, key(root, action)?, Fr::from(0))?
                    .into_transaction(&mut rng, ttl)
            }
            _ => unreachable!(),
        };
        let recipient = if action == "accept" {
            runtime::ledger::CoinRecipient::Contract(address)
        } else {
            runtime::ledger::CoinRecipient::User(CoinPublicKey(HashOutput(hex32(
                request,
                "walletCoinPublicKeyHex",
            )?)))
        };
        let mut result_coin = coin_json(output_coin, frontier);
        result_coin["commitmentHex"] = json!(hex::encode(output_coin.commitment(&recipient).0.0));
        (
            tx,
            address,
            Some(result_coin),
            Some(hash(&checkpoint_bytes)),
            node_profile,
        )
    };
    let sealed = prove(tx, root, &mut rng)?;
    fs::create_dir(output)?;
    fs::set_permissions(output, fs::Permissions::from_mode(0o700))?;
    let output = output.canonicalize()?;
    let transaction = write_file(
        &output,
        "transaction.bin",
        &encode(&sealed)?,
        "Transaction<Signature,Proof,Binding>",
    )?;
    let contract_address = write_file(
        &output,
        "contract-address.bin",
        &encode(&address)?,
        "ContractAddress",
    )?;
    let Transaction::Standard(standard) = &sealed else {
        return Err("expected standard transaction".into());
    };
    if !standard.fallible_coins.is_empty() {
        return Err("unexpected fallible offer".into());
    }
    let guaranteed = standard
        .guaranteed_coins
        .as_ref()
        .map(|offer| -> Result<Value> {
            write_file(
                &output,
                "guaranteed-offer.bin",
                &encode(offer)?,
                "Offer<Proof>",
            )
        })
        .transpose()?;
    let mut artifacts = serde_json::Map::new();
    for name in ["bootstrap", "accept", "release"] {
        for ext in ["prover", "verifier"] {
            let name = format!("keys/{name}.{ext}");
            artifacts.insert(name.clone(), json!(hash(&fs::read(root.join(name))?)));
        }
    }
    for name in ["bootstrap", "accept", "release"] {
        for ext in ["zkir", "bzkir"] {
            let name = format!("zkir/{name}.{ext}");
            artifacts.insert(name.clone(), json!(hash(&fs::read(root.join(name))?)));
        }
    }
    let receipt = json!({"format":FORMAT,"kind":"action-result","action":action,"networkId":network,"addressHex":hex::encode(address.0.0),"contractAddress":contract_address,"transaction":transaction,"guaranteedOffer":guaranteed,"offerFingerprint":{"guaranteed":guaranteed.as_ref().map(|v|v["sha256"].clone()),"fallible":[]},"coin":result_coin,"checkpointSha256":checkpoint_hash,"policy":if action=="deploy"{"deployment"}else{"trusted-observation"},"artifacts":artifacts,"buildProvenance":provenance,"nodeProfile":node_profile,"ledgerVersion":"8.0.3"});
    write_file(
        &output,
        "result.json",
        &serde_json::to_vec_pretty(&receipt)?,
        "action-result-json",
    )?;
    println!(
        "action artifact ready: {}",
        output.join("result.json").display()
    );
    Ok(())
}
fn main() {
    if let Err(error) = run() {
        eprintln!("shielded builder rejected: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests;
