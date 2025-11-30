// src/main.rs
mod pb;

use anyhow::{anyhow, Result};
use dotenvy::dotenv;
use once_cell::sync::Lazy;
use std::{collections::HashMap, env, str::FromStr, time::Duration};

use move_core_types::{account_address::AccountAddress, language_storage::StructTag};
use shared_crypto::intent::{Intent, IntentMessage};
use tokio::sync::Mutex;
use tonic::transport::{Channel, ClientTlsConfig, Endpoint};

use pb::sui::rpc::v2::{
    ledger_service_client::LedgerServiceClient,
    state_service_client::StateServiceClient,
    transaction_execution_service_client::TransactionExecutionServiceClient,
    // message types
    Bcs,
    Epoch,
    ExecuteTransactionRequest,
    ExecuteTransactionResponse,
    FieldMask,
    GetEpochRequest,
    GetObjectRequest,
    ListOwnedObjectsRequest,
    Object,
    Owner,
    Transaction as GrpcTransaction,
    UserSignature,
};

use prost_types::FieldMask;

use sui_types::{
    base_types::{ObjectDigest, ObjectID, SequenceNumber, SuiAddress},
    crypto::{Signer, SuiKeyPair},
    signature::GenericSignature,
    transaction::{Command, ObjectArg, ProgrammableMoveCall, ProgrammableTransaction, TransactionData},
    Identifier,
};

use sui_types::transaction::Argument as TxArg;

// ---------------- Momentum constants ----------------

const MOMENTUM_TRADE_PACKAGE: &str =
    "0x60e8683e01d5611cd13a69aca2b0c9aace7c6b559734df1b4a7ad9d6bddf007b";
const MOMENTUM_SLIPPAGE_PACKAGE: &str =
    "0x8add2f0f8bc9748687639d7eb59b2172ba09a0172d9e63c029e23a7dbdb6abe6";
const SUI_FRAMEWORK_PACKAGE: &str =
    "0x0000000000000000000000000000000000000000000000000000000000000002";
const SUI_CLOCK_OBJECT: &str =
    "0x0000000000000000000000000000000000000000000000000000000000000006";

const SUI_COIN_TYPE: &str = "0x2::sui::SUI";
const USDC_COIN_TYPE: &str =
    "0xdba34672e30cb065b1f93e3ab55318768fd6fef66c15942c9f7cb846e2f900e7::usdc::USDC";

const MOMENTUM_POOL_OBJECT: &str =
    "0x455cf8d2ac91e7cb883f515874af750ed3cd18195c970b7a2d46235ac2b0c388";
const MOMENTUM_GLOBAL_CONFIG: &str =
    "0x2375a0b1ec12010aaea3b2545acfa2ad34cfbba03ce4b59f4c39e1e25eed1b2a";

// ----------------------------------------------------
// Swap params
// ----------------------------------------------------

#[derive(Debug, Clone)]
pub enum SwapDirection {
    SuiToUsdc,
    UsdcToSui,
}

pub struct MomentumSwapParams {
    pub direction: SwapDirection,
    pub amount_in: u64,
    pub sqrt_price_limit: Option<u128>,
    pub recipient: SuiAddress,
}

impl MomentumSwapParams {
    pub fn new_sui_to_usdc(amount_in_sui: u64, recipient: SuiAddress) -> Self {
        Self {
            direction: SwapDirection::SuiToUsdc,
            amount_in: amount_in_sui,
            sqrt_price_limit: Some(79226673515401279992447579050),
            recipient,
        }
    }

    pub fn new_usdc_to_sui(amount_in_usdc: u64, recipient: SuiAddress) -> Self {
        Self {
            direction: SwapDirection::UsdcToSui,
            amount_in: amount_in_usdc,
            sqrt_price_limit: Some(79226673515401279992447579050),
            recipient,
        }
    }
}

// ----------------------------------------------------
// Shared global caches (optional, same as your code)
// ----------------------------------------------------

static POOL_INFO: Lazy<Mutex<(ObjectID, SequenceNumber, bool)>> =
    Lazy::new(|| Mutex::new((ObjectID::zero(), SequenceNumber::from_u64(0), true)));

static CLOCK_INFO: Lazy<Mutex<(ObjectID, SequenceNumber, bool)>> =
    Lazy::new(|| Mutex::new((ObjectID::zero(), SequenceNumber::from_u64(0), true)));

static CONFIG_INFO: Lazy<Mutex<(ObjectID, SequenceNumber, bool)>> =
    Lazy::new(|| Mutex::new((ObjectID::zero(), SequenceNumber::from_u64(0), true)));

// ----------------------------------------------------
// gRPC helpers
// ----------------------------------------------------

async fn make_channel() -> Result<Channel> {
    let endpoint = Endpoint::from_static("https://fullnode.mainnet.sui.io:443")
        .tls_config(ClientTlsConfig::new())?
        .tcp_keepalive(Some(Duration::from_secs(30)));

    let channel = endpoint.connect().await?;
    Ok(channel)
}

// Get shared object ref via gRPC (id + initial_shared_version + mutable flag)
async fn get_shared_object_ref_grpc(
    ledger: &mut LedgerServiceClient<Channel>,
    object_id_str: &str,
) -> Result<(ObjectID, SequenceNumber, bool)> {
    let req = GetObjectRequest {
        object_id: Some(object_id_str.to_string()),
        read_mask: Some(FieldMask {
            paths: vec!["object_id".into(), "version".into(), "owner".into()],
        }),
    };

    let resp = ledger.get_object(req).await?.into_inner();

    let obj: Object = resp
        .object
        .ok_or_else(|| anyhow!("GetObject: empty object for {}", object_id_str))?;

    let id = ObjectID::from_str(&obj.object_id.ok_or_else(|| anyhow!("missing object_id"))?)?;
    let version = obj.version;
    let owner: Owner = obj.owner.ok_or_else(|| anyhow!("missing owner"))?;

    // For shared objects, owner.kind == Shared and owner.version is initial_shared_version.
    let (initial_shared_version, mutable) = if let Some(owner_enum) = owner.owner {
        match owner_enum {
            // FIXME: Adjust variant name based on generated code
            pb::sui::rpc::v2::owner::owner::Owner::Shared(shared_owner) => {
                (shared_owner.version, shared_owner.mutable)
            }
            _ => (version, true),
        }
    } else {
        (version, true)
    };

    Ok((
        id,
        SequenceNumber::from_u64(initial_shared_version),
        mutable,
    ))
}

// Get reference gas price via GetEpoch
async fn get_reference_gas_price_grpc(
    ledger: &mut LedgerServiceClient<Channel>,
) -> Result<u64> {
    let req = GetEpochRequest {
        epoch: None,
        read_mask: Some(FieldMask {
            paths: vec!["reference_gas_price".into()],
        }),
    };

    let resp = ledger.get_epoch(req).await?.into_inner();
    let epoch: Epoch = resp
        .epoch
        .ok_or_else(|| anyhow!("GetEpoch: empty epoch"))?;

    Ok(epoch.reference_gas_price)
}

// SUI / USDC coins via StateService.ListOwnedObjects
#[derive(Clone, Debug)]
pub struct CoinRef {
    pub object_id: ObjectID,
    pub version: SequenceNumber,
    pub digest: ObjectDigest,
    pub coin_type: String,
    pub balance: u64,
}

async fn list_coins_grpc(
    state: &mut StateServiceClient<Channel>,
    owner: &SuiAddress,
    coin_type_filter: Option<&str>,
) -> Result<Vec<CoinRef>> {
    // In gRPC, owner is the Sui address string (0x...)
    let owner_str = format!("{owner}");

    // FIXME: adjust fields according to your generated `ListOwnedObjectsRequest`
    let req = ListOwnedObjectsRequest {
        owner: Some(owner_str),
        filter: None, // or Some(ObjectFilter { object_type: coin_type_filter.map(str::to_string) })
        cursor: None,
        limit: Some(100),
        read_mask: Some(FieldMask {
            paths: vec![
                "object_id".into(),
                "version".into(),
                "digest".into(),
                "object_type".into(),
                "balance".into(),
            ],
        }),
    };

    let mut coins = Vec::new();
    let mut cursor = None;

    loop {
        let mut req_clone = req.clone();
        req_clone.cursor = cursor.clone();

        let resp = state
            .list_owned_objects(req_clone)
            .await?
            .into_inner();

        for obj in resp.objects {
            let obj_type = obj.object_type.clone().unwrap_or_default();
            if let Some(filter) = coin_type_filter {
                if obj_type != filter {
                    continue;
                }
            }

            let object_id = ObjectID::from_str(&obj.object_id.unwrap())?;
            let version = SequenceNumber::from_u64(obj.version);
            let digest = ObjectDigest::from_str(&obj.digest.unwrap())?;
            let balance = obj.balance.unwrap_or(0);

            coins.push(CoinRef {
                object_id,
                version,
                digest,
                coin_type: obj_type,
                balance,
            });
        }

        if !resp.has_next_page {
            break;
        }
        cursor = resp.next_cursor;
    }

    Ok(coins)
}

// ----------------------------------------------------
// Build Momentum swap PT (mostly your original code)
// ----------------------------------------------------

async fn build_momentum_swap_transaction(
    ledger: &mut LedgerServiceClient<Channel>,
    state: &mut StateServiceClient<Channel>,
    sender: SuiAddress,
    swap_params: &MomentumSwapParams,
    gas_coins: Vec<(ObjectID, SequenceNumber, ObjectDigest)>,
) -> Result<ProgrammableTransaction> {
    let start = std::time::Instant::now();
    let mut builder = sui_types::types::programmable_transaction_builder::ProgrammableTransactionBuilder::new();
    let d1 = start.elapsed().as_micros();

    let x_for_y = matches!(swap_params.direction, SwapDirection::UsdcToSui);
    let by_amount_in = true;
    let d2 = start.elapsed().as_micros();

    // Build input coins according to direction
    let input_coins = if matches!(swap_params.direction, SwapDirection::SuiToUsdc) {
        gas_coins
            .iter()
            .take(1)
            .cloned()
            .collect::<Vec<_>>()
    } else {
        // USDC coins via gRPC
        let all_usdc = list_coins_grpc(state, &sender, Some(USDC_COIN_TYPE)).await?;
        if all_usdc.is_empty() {
            return Err(anyhow!("No USDC coins found for sender"));
        }
        all_usdc
            .into_iter()
            .take(3)
            .map(|c| (c.object_id, c.version, c.digest))
            .collect::<Vec<_>>()
    };
    let d3 = start.elapsed().as_micros();

    // Command0: merge coins (optional)
    if input_coins.len() > 1 {
        let primary = builder.obj(ObjectArg::ImmOrOwnedObject(input_coins[0]))?;
        let merge_vec = input_coins[1..]
            .iter()
            .map(|r| builder.obj(ObjectArg::ImmOrOwnedObject(*r)))
            .collect::<Result<Vec<_>, _>>()?;

        builder.command(Command::MergeCoins(primary, merge_vec));
    }
    let d4 = start.elapsed().as_micros();

    let primary_coin = builder.obj(ObjectArg::ImmOrOwnedObject(input_coins[0]))?;
    let amount_arg = builder.pure(swap_params.amount_in)?;
    builder.command(Command::SplitCoins(primary_coin, vec![amount_arg.clone()]));
    let d5 = start.elapsed().as_micros();

    // Shared object refs via gRPC
    let (pool_id, pool_version, pool_mut) =
        get_shared_object_ref_grpc(ledger, MOMENTUM_POOL_OBJECT).await?;
    let (clock_id, clock_version, clock_mut) =
        get_shared_object_ref_grpc(ledger, SUI_CLOCK_OBJECT).await?;
    let (config_id, config_version, config_mut) =
        get_shared_object_ref_grpc(ledger, MOMENTUM_GLOBAL_CONFIG).await?;
    let d6 = start.elapsed().as_micros();

    // Optionally cache to POOL_INFO, CLOCK_INFO, CONFIG_INFO
    {
        let mut pool = POOL_INFO.lock().await;
        *pool = (pool_id, pool_version, pool_mut);
        let mut clock = CLOCK_INFO.lock().await;
        *clock = (clock_id, clock_version, clock_mut);
        let mut cfg = CONFIG_INFO.lock().await;
        *cfg = (config_id, config_version, config_mut);
    }

    let pool_arg = builder.obj(ObjectArg::SharedObject {
        id: pool_id,
        initial_shared_version: pool_version,
        mutable: true,
    })?;
    let d7 = start.elapsed().as_micros();

    let clock_arg = builder.obj(ObjectArg::SharedObject {
        id: clock_id,
        initial_shared_version: clock_version,
        mutable: true,
    })?;
    let d8 = start.elapsed().as_micros();

    let config_arg = builder.obj(ObjectArg::SharedObject {
        id: config_id,
        initial_shared_version: config_version,
        mutable: true,
    })?;
    let d9 = start.elapsed().as_micros();

    let x_for_y_arg = builder.pure(x_for_y)?;
    let by_amount_in_arg = builder.pure(by_amount_in)?;
    let sqrt_price_limit_arg = builder.pure(
        swap_params
            .sqrt_price_limit
            .unwrap_or(79226673515401279992447579050),
    )?;
    let recipient_arg1 = builder.pure(swap_params.recipient)?;
    let recipient_arg2 = builder.pure(swap_params.recipient)?;
    let d10 = start.elapsed().as_micros();

    // Command 2: flash_swap(...)
    builder.command(Command::MoveCall(Box::new(ProgrammableMoveCall {
        package: ObjectID::from_str(MOMENTUM_TRADE_PACKAGE)?,
        module: "trade".parse()?,
        function: "flash_swap".parse()?,
        type_arguments: vec![
            {
                let address = AccountAddress::from_str(SUI_FRAMEWORK_PACKAGE)?;
                sui_types::TypeTag::Struct(Box::new(StructTag {
                    address,
                    module: Identifier::new("sui")?,
                    name: Identifier::new("SUI")?,
                    type_params: vec![],
                }))
                .into()
            },
            {
                let address = AccountAddress::from_str(
                    "0xdba34672e30cb065b1f93e3ab55318768fd6fef66c15942c9f7cb846e2f900e7",
                )?;
                sui_types::TypeTag::Struct(Box::new(StructTag {
                    address,
                    module: Identifier::new("usdc")?,
                    name: Identifier::new("USDC")?,
                    type_params: vec![],
                }))
                .into()
            },
        ],
        arguments: vec![
            pool_arg,
            x_for_y_arg.clone(),
            by_amount_in_arg,
            amount_arg.clone(),
            sqrt_price_limit_arg.clone(),
            clock_arg,
            config_arg,
        ],
    })));
    let d11 = start.elapsed().as_micros();

    // .... 這裡開始你原本 Commands 3 ~ 14 的邏輯完全可以照搬
    // 為了篇幅，我就保留你原來那一大段（destroy_zero, zero, swap_receipt_debts, split, into_balance, repay_flash_swap, assert_slippage, from_balance, value, transferObjects）
    // 你直接把你目前的 Command 3 ~ 14 貼進來即可，程式與舊版相同，只是 `sui_client` 改成 gRPC 來源而已。

    // At the end:
    println!(
        "d1: {}, d2: {}, d3: {}, d4: {}, d5: {}, d6: {}, d7: {}, d8: {}, d9: {}, d10: {}, d11: {}",
        d1,
        d2 - d1,
        d3 - d2,
        d4 - d3,
        d5 - d4,
        d6 - d5,
        d7 - d6,
        d8 - d7,
        d9 - d8,
        d10 - d9,
        d11 - d10,
    );

    Ok(builder.finish())
}

// ----------------------------------------------------
// Execute transaction via gRPC
// ----------------------------------------------------

async fn execute_momentum_swap_grpc(
    ledger: &mut LedgerServiceClient<Channel>,
    state: &mut StateServiceClient<Channel>,
    tx_exec: &mut TransactionExecutionServiceClient<Channel>,
    keypair: &SuiKeyPair,
    sender: SuiAddress,
    swap_params: &MomentumSwapParams,
) -> Result<String> {
    println!("Executing Momentum swap: {:?}, amount_in = {}", swap_params.direction, swap_params.amount_in);

    // Gas coins from gRPC (SUI coins)
    let sui_coins = list_coins_grpc(state, &sender, Some(SUI_COIN_TYPE)).await?;
    if sui_coins.is_empty() {
        return Err(anyhow!("No SUI coins for gas"));
    }
    let gas_ref = sui_coins[0].clone();

    let gas_object_ref = (
        gas_ref.object_id,
        gas_ref.version,
        gas_ref.digest,
    );

    // Build PT
    let pt = build_momentum_swap_transaction(
        ledger,
        state,
        sender,
        swap_params,
        vec![gas_object_ref],
    )
    .await?;

    // Gas price via gRPC
    let gas_price = get_reference_gas_price_grpc(ledger).await?;
    let gas_budget = 30_000_000u64;

    let gas_payment = vec![gas_object_ref];

    let tx_data = TransactionData::new_programmable(
        sender,
        gas_payment,
        pt,
        gas_budget,
        gas_price,
    );

    // Intent message + BCS
    let intent_msg = IntentMessage::new(Intent::sui_transaction(), tx_data);
    let raw_tx = bcs::to_bytes(&intent_msg)?;

    // Blake2b-256 hash
    use blake2::{Blake2b, Digest as Blake2Digest};
    let mut hasher = Blake2b::<typenum::U32>::new();
    hasher.update(&raw_tx);
    let hash_bytes: [u8; 32] = hasher.finalize().into();

    let sui_sig = keypair.sign(&hash_bytes);

    // Verify locally
    sui_sig
        .verify_secure(
            &intent_msg,
            sender,
            sui_types::crypto::SignatureScheme::ED25519,
        )
        .map_err(|e| anyhow!("Local signature verification failed: {e}"))?;

    // Wrap into GenericSignature for Sui types (if needed)
    let generic_sig = GenericSignature::Signature(sui_sig.clone());

    // ---- gRPC TransactionExecutionService ----
    // You have to map Sui's `Transaction` (types crate) into gRPC `Transaction` (proto).
    // Typically, only `bcs` is required (name="Transaction", value=bcs_tx_bytes)
    let tx_bcs = bcs::to_bytes(
        &sui_types::transaction::Transaction::from_generic_sig_data(
            intent_msg.value,
            vec![generic_sig],
        ),
    )?;

    let grpc_tx = GrpcTransaction {
        bcs: Some(Bcs {
            name: "Transaction".to_string(),
            data: tx_bcs,
        }),
        ..Default::default() // FIXME: if your proto requires more fields, fill here
    };

    // Convert your SuiKeyPair signature to gRPC `UserSignature`
    // `sui_sig.as_ref()` usually returns [flag || sig || pubkey]
    let sui_sig_bytes = sui_sig.as_ref().to_vec();

    let user_sig = UserSignature {
        // FIXME: adjust according to your proto:
        // often something like: UserSignature { signature: Some(Signature::Single(SingleSignature { flag: ..., bytes: ... })) }
        signature: Some(
            pb::sui::rpc::v2::user_signature::Signature::Ed25519(
                pb::sui::rpc::v2::Ed25519Signature {
                    bytes: sui_sig_bytes,
                },
            ),
        ),
    };

    let req = ExecuteTransactionRequest {
        transaction: Some(grpc_tx),
        signatures: vec![user_sig],
        read_mask: None, // or Some(FieldMask { paths: vec!["effects.status".into(), "checkpoint".into()] })
    };

    let resp: ExecuteTransactionResponse =
        tx_exec.execute_transaction(req).await?.into_inner();

    // FIXME: adjust how to take digest from response, depends on proto
    let executed = resp
        .transaction
        .ok_or_else(|| anyhow!("No transaction in ExecuteTransactionResponse"))?;

    let digest = executed
        .digest
        .ok_or_else(|| anyhow!("No digest field in ExecutedTransaction"))?;

    Ok(digest)
}

// ----------------------------------------------------
// main
// ----------------------------------------------------

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Momentum DEX gRPC swap ===");

    dotenv().ok();

    // Build gRPC channel and clients
    let channel = make_channel().await?;
    let mut ledger = LedgerServiceClient::new(channel.clone());
    let mut state = StateServiceClient::new(channel.clone());
    let mut tx_exec = TransactionExecutionServiceClient::new(channel);

    // Load keypair
    let pk_str = env::var("PRIVATE_KEY")
        .expect("PRIVATE_KEY env var is required (Base64-encoded sui.keypair)");

    let keypair = SuiKeyPair::decode(&pk_str).map_err(|_| anyhow!("Invalid PRIVATE_KEY format"))?;
    let sender = SuiAddress::from(&keypair.public());

    println!("Sender: {sender}");

    // Prepare swap params (example: USDC -> SUI)
    let swap_params = MomentumSwapParams::new_usdc_to_sui(100_000, sender); // 0.1 USDC if 6 decimals

    match execute_momentum_swap_grpc(
        &mut ledger,
        &mut state,
        &mut tx_exec,
        &keypair,
        sender,
        &swap_params,
    )
    .await
    {
        Ok(digest) => println!("Swap success, digest = {digest}"),
        Err(e) => eprintln!("Swap failed: {e:#}"),
    }

    Ok(())
}
