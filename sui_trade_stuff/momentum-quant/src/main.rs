use std::error::Error;

use bech32::FromBase32;
use sui_crypto::ed25519::Ed25519PrivateKey;
use sui_crypto::SuiSigner;
use sui_rpc::Client;
use sui_rpc::proto::sui::rpc::v2::{ListOwnedObjectsRequest, GetObjectRequest};
use sui_sdk_types::{Address, Digest};
use sui_transaction_builder::unresolved::Input;
use prost_types::FieldMask;
use sui_rpc::proto::sui::rpc::v2::Object;
use tokio::time::Instant;
mod momentum;

/// Enable / disable debug logs in main.rs.
const DEBUG_MAIN: bool = true;

/// Default swap amount (in smallest unit of the token).
const DEFAULT_SWAP_AMOUNT: u64 = 1_000_000;

/// Default gas budget and gas price.
const DEFAULT_GAS_BUDGET: u64 = 500_000_00;
const DEFAULT_GAS_PRICE: u64 = 1_000;

/// Hard-coded pool object id and token object id used in the example.
const DEFAULT_POOL_ID: &str =
    "0x455cf8d2ac91e7cb883f515874af750ed3cd18195c970b7a2d46235ac2b0c388";
const DEFAULT_TOKEN_OBJECT_ID: &str =
    "0x024ebdcd5cfee93cf032dd2091fb0a8e570734595577cb10ec6df46e5c11432c";

/// Example private key (bech32 suiprivkey format).
const EXAMPLE_PRIVATE_KEY: &str =
    "";

const VERSIONED_OBJECT_ID: &str =
    "0x2375a0b1ec12010aaea3b2545acfa2ad34cfbba03ce4b59f4c39e1e25eed1b2a";
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    debug_main("[main] start");
    let start = Instant::now();
    // 1. Decode private key from bech32 "suiprivkey..." format.
    let private_key = decode_sui_private_key(EXAMPLE_PRIVATE_KEY)?;
    let public_key = private_key.public_key();
    let owner_address = public_key.derive_address();

    println!("Owner address: {:?}", owner_address);
    debug_main("[main] decoded private key and derived address");

    // 2. Create Sui gRPC client (testnet).
    let mut client = Client::new("http://3.114.103.176:443")?;
    println!("Sui gRPC client connected");
    debug_main("[main] Sui gRPC client created");

    // 3. Query owned SUI coins to get a gas object id (only object id, no version/digest).
    let gas_object_id = fetch_first_sui_gas_object_id(&mut client, &owner_address).await?;
    println!("Selected gas object id: {:?}", gas_object_id);
    debug_main(&format!(
        "[main] fetched gas object id: {gas_object_id}"
    ));
    // 4. Prepare swap parameters.
    let pool_object_id: Address = DEFAULT_POOL_ID.parse()?;
    let token_object_id: Address = DEFAULT_TOKEN_OBJECT_ID.parse()?;
    let versioned_object_id: Address = VERSIONED_OBJECT_ID.parse()?;
    let clock_object_id: Address = "0x6".parse()?; // Sui system clock object id
    
    // Fetch object details
    let gas_obj = fetch_object_details(&mut client, gas_object_id).await?;
    let pool_obj = fetch_object_details(&mut client, pool_object_id).await?;
    let token_obj = fetch_object_details(&mut client, token_object_id).await?;
    let version_obj = fetch_object_details(&mut client, versioned_object_id).await?;
    let clock_obj = fetch_object_details(&mut client, clock_object_id).await?;

    // Construct Inputs
    // Token (Owned)
    let token_version = token_obj.version.ok_or("Missing version for token")?;
    let token_digest_str = token_obj.digest.ok_or("Missing digest for token")?;
    let token_digest: Digest = token_digest_str.parse()?;

    let gas_version = gas_obj.version.ok_or("Missing version for gas object")?;
    let gas_digest_str = gas_obj.digest.ok_or("Missing digest for gas object")?;
    let gas_digest: Digest = gas_digest_str.parse()?;

    let gas_input = Input::by_id(gas_object_id)
        .with_owned_kind()
        .with_version(gas_version)
        .with_digest(gas_digest);

    let token_input = Input::by_id(token_object_id)
        .with_owned_kind()
        .with_version(token_version)
        .with_digest(token_digest);

    // Pool (Shared)
    let initial_shared_version = get_initail_shared_version(&pool_obj)?;
    let clock_version = get_initail_shared_version(&clock_obj)?;
    let version_version = get_initail_shared_version(&version_obj)?;
    println!("Initial shared versions - pool: {}, clock: {}, versioned: {}", initial_shared_version, clock_version, version_version);
    let pool_input = Input::by_id(pool_object_id)
        .with_shared_kind()
        .with_initial_shared_version(initial_shared_version)
        .by_val();

    let clock_input = Input::by_id(clock_object_id)
        .with_shared_kind()
        .with_initial_shared_version(clock_version)
        .by_ref();

    let version_input = Input::by_id(versioned_object_id)
        .with_shared_kind()
        .with_initial_shared_version(version_version)
        .by_val();

    let amount: u64 = DEFAULT_SWAP_AMOUNT;
    let direction: bool = true; // true: A -> B, false: B -> A

    debug_main(&format!(
        "[main] swap params: token={token_object_id}, pool={pool_object_id}, amount={amount}, direction={direction}"
    ));

    // 5. Build transaction (all inputs created via by_id).
    debug_main("[main] before create_swap_transaction");
    let tx = momentum::create_swap_transaction(
        token_input,
        pool_input,
        gas_input,
        amount,
        direction,
        owner_address,
        DEFAULT_GAS_BUDGET,
        DEFAULT_GAS_PRICE,
        clock_input,
        version_input,
    )?;
    debug_main("[main] after create_swap_transaction (tx built)");

    // 6. Sign transaction.
    let signature = private_key.sign_transaction(&tx)?;
    debug_main("[main] transaction signed");

    // 7. Execute transaction.
    let mut exec_client = client.execution_client();

    let mut request = sui_rpc::proto::sui::rpc::v2::ExecuteTransactionRequest::default();
    request.transaction = Some(tx.into());
    request.signatures = vec![signature.into()];

    debug_main("[main] before execute_transaction");
    let response = exec_client.execute_transaction(request).await?;
    debug_main("[main] after execute_transaction");
    let elapsed = start.elapsed();
    println!("Transaction submitted, response: {:?}", response.into_inner());
    println!("Elapsed time: {:.3?}", elapsed);
    Ok(())
}

/// Decode Sui Ed25519 private key from bech32 "suiprivkey..." string.
fn decode_sui_private_key(key_str: &str) -> Result<Ed25519PrivateKey, Box<dyn Error>> {
    let (_hrp, data, _variant) = bech32::decode(key_str)?;
    let bytes = Vec::<u8>::from_base32(&data)?;

    if bytes.len() != 33 || bytes[0] != 0 {
        return Err("Invalid Sui private key format".into());
    }

    let pk_bytes: [u8; 32] = bytes[1..]
        .try_into()
        .map_err(|_| "Invalid Sui private key length")?;

    Ok(Ed25519PrivateKey::new(pk_bytes))
}

/// Fetch the first owned SUI coin object id for the given address.
/// Only the object id is used; version and digest are not needed when using Input::by_id.
async fn fetch_first_sui_gas_object_id(
    client: &mut Client,
    owner: &Address,
) -> Result<Address, Box<dyn Error>> {
    let mut state_client = client.state_client();

    let mut request = ListOwnedObjectsRequest::default();
    request.owner = Some(owner.to_string());
    request.page_size = Some(1000);
    request.object_type = Some("0x2::coin::Coin<0x2::sui::SUI>".to_string());

    let mut mask = prost_types::FieldMask::default();
    mask.paths = vec!["object_id".to_string()];
    request.read_mask = Some(mask);

    let response = state_client.list_owned_objects(request).await?.into_inner();
    println!("Owned SUI objects response: {:?}", response);
    if response.objects.is_empty() {
        return Err("No SUI gas objects found for this address".into());
    }

    // Use the first SUI coin object as gas.
    let obj = &response.objects[0];

    let oid_str = obj
        .object_id
        .as_ref()
        .ok_or("Missing object_id field in ListOwnedObjectsResponse")?;

    let oid: Address = oid_str.parse()?;
    Ok(oid)
}

fn debug_main(msg: &str) {
    if DEBUG_MAIN {
        eprintln!("{msg}");
    }
}

async fn fetch_object_details(
    client: &mut Client,
    object_id: Address,
) -> Result<Object, Box<dyn std::error::Error>> {
    let mut ledger_client = client.ledger_client();

    // 如果你原本有 GetObjectRequest::new，可以這樣用：
    let mut request = GetObjectRequest::new(&object_id);

    // 覆寫 read_mask，要求所有欄位
    request.read_mask = Some(FieldMask {
        paths: vec![
        "object_id".to_string(),
        "version".to_string(),
        "digest".to_string(),
        "owner".to_string(),
        ],
    });

    let response = ledger_client.get_object(request).await?.into_inner();
    response.object.ok_or_else(|| "Object not found".into())
}
fn get_initail_shared_version(
    obj: &sui_rpc::proto::sui::rpc::v2::Object,
) -> Result<u64, Box<dyn Error>> {
    println!("Object details: {:?}", obj);
    if let Some(ref owner) = obj.owner {
            return Ok(owner.version());
    }
    Err("Object is not shared or missing owner field".into())
}