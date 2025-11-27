mod utils;
use anyhow::anyhow;
use move_core_types::{account_address::AccountAddress, language_storage::StructTag};
use shared_crypto::intent::{Intent, IntentMessage};
use std::{collections::HashMap, str::FromStr};
use sui_sdk::{
    rpc_types::{Coin, SuiTransactionBlockResponseOptions}, types::{
        base_types::ObjectID, programmable_transaction_builder::ProgrammableTransactionBuilder,
        transaction::TransactionData,
    }, SuiClient, SuiClientBuilder
};
use sui_types::{
    base_types::{SuiAddress, SequenceNumber, ObjectDigest},
    crypto::SuiKeyPair,
    crypto::{Signer, SuiSignature},
    signature::GenericSignature,
    transaction::{Command, ObjectArg, ProgrammableMoveCall, ProgrammableTransaction},
    Identifier,
};
use tokio::sync::Mutex; 
use once_cell::sync::Lazy;
use utils::*;

// Momentum DEX constants
const MOMENTUM_TRADE_PACKAGE: &str =
    "0x60e8683e01d5611cd13a69aca2b0c9aace7c6b559734df1b4a7ad9d6bddf007b";
const MOMENTUM_SLIPPAGE_PACKAGE: &str =
    "0x8add2f0f8bc9748687639d7eb59b2172ba09a0172d9e63c029e23a7dbdb6abe6";
const SUI_FRAMEWORK_PACKAGE: &str =
    "0x0000000000000000000000000000000000000000000000000000000000000002";
const SUI_CLOCK_OBJECT: &str = "0x0000000000000000000000000000000000000000000000000000000000000006";

// Token types
const SUI_COIN_TYPE: &str = "0x2::sui::SUI";
const USDC_COIN_TYPE: &str =
    "0xdba34672e30cb065b1f93e3ab55318768fd6fef66c15942c9f7cb846e2f900e7::usdc::USDC";

// Pool objects (from the transaction data)
const MOMENTUM_POOL_OBJECT: &str =
    "0x455cf8d2ac91e7cb883f515874af750ed3cd18195c970b7a2d46235ac2b0c388";
const MOMENTUM_GLOBAL_CONFIG: &str =
    "0x2375a0b1ec12010aaea3b2545acfa2ad34cfbba03ce4b59f4c39e1e25eed1b2a";

#[derive(Debug, Clone)]
pub enum SwapDirection {
    SuiToUsdc, // x_for_y = false in Momentum
    UsdcToSui, // x_for_y = true in Momentum
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
            sqrt_price_limit: Some(79226673515401279992447579050), // Default from transaction
            recipient,
        }
    }

    pub fn new_usdc_to_sui(amount_in_usdc: u64, recipient: SuiAddress) -> Self {
        Self {
            direction: SwapDirection::UsdcToSui,
            amount_in: amount_in_usdc,
            sqrt_price_limit: Some(79226673515401279992447579050), // Default from transaction
            recipient,
        }
    }
}

async fn build_momentum_swap_transaction(
    sui_client: &sui_sdk::SuiClient,
    sender: SuiAddress,
    swap_params: &MomentumSwapParams,
    gas_coins: Vec<sui_sdk::types::base_types::ObjectRef>,
) -> Result<ProgrammableTransaction, anyhow::Error> {
    let start = std::time::Instant::now();
    let mut builder = ProgrammableTransactionBuilder::new();
    let d1 = start.elapsed().as_micros();

    // Determine swap direction parameters
    let x_for_y = match swap_params.direction {
        SwapDirection::SuiToUsdc => true,  // SUI->USDC
        SwapDirection::UsdcToSui => false, // USDC->SUI
    };
    let by_amount_in = true; // Always swap by input amount
    let d2 = start.elapsed().as_micros();

    // Get input coins based on direction
    let input_coins = if matches!(swap_params.direction, SwapDirection::SuiToUsdc) {
        // For SUI->USDC, use gas coins
        gas_coins
            .iter()
            .take(1)
            .map(|coin| *coin)
            .collect::<Vec<_>>()
    } else {
        // For USDC->SUI, get USDC coins
        let usdc_coins = sui_client
            .coin_read_api()
            .get_coins(sender, Some(USDC_COIN_TYPE.to_string()), None, None)
            .await?;

        if usdc_coins.data.is_empty() {
            return Err(anyhow!("No USDC coins found"));
        }

        usdc_coins
            .data
            .iter()
            .take(3)
            .map(|coin| coin.object_ref())
            .collect()
    };
    let d3 = start.elapsed().as_micros();

    // Command 0: Merge coins (if we have multiple coins)
    if input_coins.len() > 1 {
        let primary_coin = builder.obj(ObjectArg::ImmOrOwnedObject(input_coins[0]))?;
        let merge_coins: Vec<_> = input_coins[1..]
            .iter()
            .map(|coin_ref| builder.obj(ObjectArg::ImmOrOwnedObject(*coin_ref)))
            .collect::<Result<Vec<_>, _>>()?;

        builder.command(Command::MergeCoins(primary_coin, merge_coins));
    }
    let d4 = start.elapsed().as_micros();

    // Command 1: Split coins for the swap amount
    let primary_coin = builder.obj(ObjectArg::ImmOrOwnedObject(input_coins[0]))?;
    let amount_arg = builder.pure(swap_params.amount_in)?;
    builder.command(Command::SplitCoins(primary_coin, vec![amount_arg.clone()]));
    let d5 = start.elapsed().as_micros();

    let (pool_id, pool_version, pool_mutable) =
        get_shared_object_ref(sui_client, MOMENTUM_POOL_OBJECT).await?;
    let (clock_id, clock_version, clock_mutable) =
        get_shared_object_ref(sui_client, SUI_CLOCK_OBJECT).await?;
    let (config_id, config_version, config_mutable) =
        get_shared_object_ref(sui_client, MOMENTUM_GLOBAL_CONFIG).await?;
    let d6 = start.elapsed().as_micros();
    // println!("pool_id: {}, pool_version: {}, pool_mutable: {}", pool_id, pool_version, pool_mutable);
    // println!("clock_id: {}, clock_version: {}, clock_mutable: {}", clock_id, clock_version, clock_mutable);
    // println!("config_id: {}, config_version: {}, config_mutable: {}", config_id, config_version, config_mutable);

    // let (pool_id, pool_version, pool_mutable, clock_id, clock_version, clock_mutable, config_id, config_version, config_mutable) = {
    //     let pool_info_locked = POOL_INFO.lock().await;
    //     let clock_info_locked = CLOCK_INFO.lock().await;
    //     let config_info_locked = CONFIG_INFO.lock().await;
    //     (
    //         pool_info_locked.0, pool_info_locked.1, pool_info_locked.2,
    //         clock_info_locked.0, clock_info_locked.1, clock_info_locked.2,
    //         config_info_locked.0, config_info_locked.1, config_info_locked.2
    //     )
    // };
    // let d6 = start.elapsed().as_micros();

    let pool_arg = builder.obj(ObjectArg::SharedObject {
        id: pool_id,
        initial_shared_version: pool_version,
        mutable: pool_mutable,
    })?;
    let d7 = start.elapsed().as_micros();

    let clock_arg = builder.obj(ObjectArg::SharedObject {
        id: clock_id,
        initial_shared_version: clock_version,
        mutable: clock_mutable,
    })?;
    let d8 = start.elapsed().as_micros();

    let global_config_arg = builder.obj(ObjectArg::SharedObject {
        id: config_id,
        initial_shared_version: config_version,
        mutable: config_mutable,
    })?;
    let d9 = start.elapsed().as_micros();

    // 預先創建所有純值參數
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

    // Command 2: Flash swap
    builder.command(Command::MoveCall(Box::new(ProgrammableMoveCall {
        package: ObjectID::from_str(MOMENTUM_TRADE_PACKAGE)?,
        module: "trade".parse()?,
        function: "flash_swap".parse()?,
        type_arguments: vec![
            // SUI type
            {
                let address = AccountAddress::from_str(
                    "0x0000000000000000000000000000000000000000000000000000000000000002",
                )?;
                sui_types::TypeTag::Struct(Box::new(StructTag {
                    address,
                    module: Identifier::new("sui")?,
                    name: Identifier::new("SUI")?,
                    type_params: vec![],
                }))
                .into()
            },
            // USDC type
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
            amount_arg,
            sqrt_price_limit_arg.clone(),
            clock_arg,
            global_config_arg,
        ],
    })));
    let d11 = start.elapsed().as_micros();

    // Command 3: Destroy zero balance
    builder.command(Command::MoveCall(Box::new(ProgrammableMoveCall {
        package: ObjectID::from_str(SUI_FRAMEWORK_PACKAGE)?,
        module: "balance".parse()?,
        function: "destroy_zero".parse()?,
        type_arguments: vec![{
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
        }],
        arguments: vec![sui_types::transaction::Argument::NestedResult(2, 1)], // flash_swap_result.1
    })));
    let d12 = start.elapsed().as_micros();

    // Command 4: Create zero coin for SUI
    builder.command(Command::MoveCall(Box::new(ProgrammableMoveCall {
        package: ObjectID::from_str(SUI_FRAMEWORK_PACKAGE)?,
        module: "coin".parse()?,
        function: "zero".parse()?,
        type_arguments: vec![{
            let address = AccountAddress::from_str(
                "0x0000000000000000000000000000000000000000000000000000000000000002",
            )?;
            sui_types::TypeTag::Struct(Box::new(StructTag {
                address,
                module: Identifier::new("sui")?,
                name: Identifier::new("SUI")?,
                type_params: vec![],
            }))
            .into()
        }],
        arguments: vec![],
    })));
    let d13 = start.elapsed().as_micros();

    // Command 5: Get swap receipt debts
    builder.command(Command::MoveCall(Box::new(ProgrammableMoveCall {
        package: ObjectID::from_str(MOMENTUM_TRADE_PACKAGE)?,
        module: "trade".parse()?,
        function: "swap_receipt_debts".parse()?,
        type_arguments: vec![],
        arguments: vec![sui_types::transaction::Argument::NestedResult(2, 2)], // flash_swap_result.2
    })));
    let d14 = start.elapsed().as_micros();

    // Command 6: Split coin for repayment
    builder.command(Command::MoveCall(Box::new(ProgrammableMoveCall {
        package: ObjectID::from_str(SUI_FRAMEWORK_PACKAGE)?,
        module: "coin".parse()?,
        function: "split".parse()?,
        type_arguments: vec![{
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
        }],
        arguments: vec![
            sui_types::transaction::Argument::NestedResult(1, 0), // split result from command 1
            sui_types::transaction::Argument::NestedResult(5, 1), // receipt_debts.1 from command 5
        ],
    })));
    let d15 = start.elapsed().as_micros();

    // Command 7: Convert SUI coin to balance
    builder.command(Command::MoveCall(Box::new(ProgrammableMoveCall {
        package: ObjectID::from_str(SUI_FRAMEWORK_PACKAGE)?,
        module: "coin".parse()?,
        function: "into_balance".parse()?,
        type_arguments: vec![{
            let address = AccountAddress::from_str(
                "0x0000000000000000000000000000000000000000000000000000000000000002",
            )?;
            sui_types::TypeTag::Struct(Box::new(StructTag {
                address,
                module: Identifier::new("sui")?,
                name: Identifier::new("SUI")?,
                type_params: vec![],
            }))
            .into()
        }],
        arguments: vec![sui_types::transaction::Argument::NestedResult(4, 0)], // zero_sui_coin from command 4
    })));
    let d16 = start.elapsed().as_micros();

    // Command 8: Convert USDC coin to balance
    builder.command(Command::MoveCall(Box::new(ProgrammableMoveCall {
        package: ObjectID::from_str(SUI_FRAMEWORK_PACKAGE)?,
        module: "coin".parse()?,
        function: "into_balance".parse()?,
        type_arguments: vec![{
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
        }],
        arguments: vec![sui_types::transaction::Argument::Result(6)], // coin_split result from command 6
    })));
    let d17 = start.elapsed().as_micros();

    // Command 9: Repay flash swap
    builder.command(Command::MoveCall(Box::new(ProgrammableMoveCall {
        package: ObjectID::from_str(MOMENTUM_TRADE_PACKAGE)?,
        module: "trade".parse()?,
        function: "repay_flash_swap".parse()?,
        type_arguments: vec![
            // SUI type
            {
                let address = AccountAddress::from_str(
                    "0x0000000000000000000000000000000000000000000000000000000000000002",
                )?;
                sui_types::TypeTag::Struct(Box::new(StructTag {
                    address,
                    module: Identifier::new("sui")?,
                    name: Identifier::new("SUI")?,
                    type_params: vec![],
                }))
                .into()
            },
            // USDC type
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
            sui_types::transaction::Argument::NestedResult(2, 2), // flash_swap_result.2
            sui_types::transaction::Argument::Result(7),          // sui_balance from command 7
            sui_types::transaction::Argument::Result(8),          // usdc_balance from command 8
            global_config_arg,
        ],
    })));
    let d18 = start.elapsed().as_micros();

    // Command 10: Slippage check
    builder.command(Command::MoveCall(Box::new(ProgrammableMoveCall {
        package: ObjectID::from_str(MOMENTUM_SLIPPAGE_PACKAGE)?,
        module: "slippage_check".parse()?,
        function: "assert_slippage".parse()?,
        type_arguments: vec![
            // SUI type
            {
                let address = AccountAddress::from_str(
                    "0x0000000000000000000000000000000000000000000000000000000000000002",
                )?;
                sui_types::TypeTag::Struct(Box::new(StructTag {
                    address,
                    module: Identifier::new("sui")?,
                    name: Identifier::new("SUI")?,
                    type_params: vec![],
                }))
                .into()
            },
            // USDC type
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
        arguments: vec![pool_arg, sqrt_price_limit_arg, x_for_y_arg],
    })));
    let d19 = start.elapsed().as_micros();

    // Command 11: Convert balance back to coin
    builder.command(Command::MoveCall(Box::new(ProgrammableMoveCall {
        package: ObjectID::from_str(SUI_FRAMEWORK_PACKAGE)?,
        module: "coin".parse()?,
        function: "from_balance".parse()?,
        type_arguments: vec![{
            let address = AccountAddress::from_str(
                "0x0000000000000000000000000000000000000000000000000000000000000002",
            )?;
            sui_types::TypeTag::Struct(Box::new(StructTag {
                address,
                module: Identifier::new("sui")?,
                name: Identifier::new("SUI")?,
                type_params: vec![],
            }))
            .into()
        }],
        arguments: vec![sui_types::transaction::Argument::NestedResult(2, 0)], // flash_swap_result.0
    })));
    let d20 = start.elapsed().as_micros();

    // Command 12: Get coin value (像原始交易一樣)
    builder.command(Command::MoveCall(Box::new(ProgrammableMoveCall {
        package: ObjectID::from_str(SUI_FRAMEWORK_PACKAGE)?,
        module: "coin".parse()?,
        function: "value".parse()?,
        type_arguments: vec![{
            let address = AccountAddress::from_str(
                "0x0000000000000000000000000000000000000000000000000000000000000002",
            )?;
            sui_types::TypeTag::Struct(Box::new(StructTag {
                address,
                module: Identifier::new("sui")?,
                name: Identifier::new("SUI")?,
                type_params: vec![],
            }))
            .into()
        }],
        arguments: vec![sui_types::transaction::Argument::Result(11)], // final_coin from command 11
    })));
    let d21 = start.elapsed().as_micros();

    // Command 13 & 14: Transfer objects
    builder.command(Command::TransferObjects(
        vec![sui_types::transaction::Argument::NestedResult(1, 0)], // split result from command 1
        recipient_arg1,
    ));
    let d22 = start.elapsed().as_micros();

    builder.command(Command::TransferObjects(
        vec![sui_types::transaction::Argument::Result(11)], // final_coin from command 11
        recipient_arg2,
    ));
    let d23 = start.elapsed().as_micros();

    println!("d1: {}, d2: {}, d3: {}, d4: {}, d5: {}, d6: {}, d7: {}, d8: {}, d9: {}, d10: {}, d11: {}, d12: {}, d13: {}, d14: {}, d15: {}, d16: {}, d17: {}, d18: {}, d19: {}, d20: {}, d21: {}, d22: {}, d23: {}", 
        d1, d2-d1, d3-d2, d4-d3, d5-d4, d6-d5, d7-d6, d8-d7, d9-d8, d10-d9, d11-d10, d12-d11, d13-d12, d14-d13, d15-d14, d16-d15, d17-d16, d18-d17, d19-d18, d20-d19, d21-d20, d22-d21, d23-d22
    );

    Ok(builder.finish())
}

async fn merge_account_balances(sui_client: &SuiClient, signer: SuiAddress, all_coins: Vec<Coin>) -> HashMap<String, (ObjectID, SequenceNumber, ObjectDigest)> {
    let mut balances_map: HashMap<String, (ObjectID, SequenceNumber, ObjectDigest)> = HashMap::new();

    for coin in all_coins.iter() {
        let entry: &mut (ObjectID, SequenceNumber, ObjectDigest) = balances_map.entry(coin.coin_type.clone()).or_insert_with(|| (coin.coin_object_id, coin.version, coin.digest));
        // entry.3 += coin.balance; // Sum the balances
        // Update the representative coin if the current coin has a smaller object_id
        if coin.coin_object_id < entry.0 {
            entry.0 = coin.coin_object_id;
            entry.1 = coin.version;
            entry.2 = coin.digest;
        }
    }

    for (currency, currency_info) in balances_map.iter() {
        let mut builder = ProgrammableTransactionBuilder::new();
        let primary_coin = currency_info;

        for coin in all_coins.iter() {
            if &coin.coin_type == currency && coin.coin_object_id != currency_info.0 {
                let coin_to_merge = coin;
                let tx = sui_client
                    .transaction_builder()
                    .merge_coins(
                        signer,
                        primary_coin.0,
                        coin_to_merge.coin_object_id,
                        Some(primary_coin.0),
                        10000, // gas budget
                    )
                    .await.unwrap();
            }
        }

        // builder.command(Command::MergeCoins(primary_coin, merge_coins));
        // builder.finish();
    }

    balances_map
}

// 定義全局變量
static POOL_INFO: Lazy<Mutex<(ObjectID, SequenceNumber, bool)>> = Lazy::new(|| Mutex::new((ObjectID::random(), SequenceNumber::new(), false)));
static CLOCK_INFO: Lazy<Mutex<(ObjectID, SequenceNumber, bool)>> = Lazy::new(|| Mutex::new((ObjectID::random(), SequenceNumber::new(), false)));
static CONFIG_INFO: Lazy<Mutex<(ObjectID, SequenceNumber, bool)>> = Lazy::new(|| Mutex::new((ObjectID::random(), SequenceNumber::new(), false)));


#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    println!("=== Momentum DEX 交換程序 ===");

    // Set up sui client
    let sui_client = SuiClientBuilder::default().build_mainnet().await?;

    let (pool_id, pool_version, pool_mutable) =
        get_shared_object_ref(&sui_client, MOMENTUM_POOL_OBJECT).await?;
    let (clock_id, clock_version, clock_mutable) =
        get_shared_object_ref(&sui_client, SUI_CLOCK_OBJECT).await?;
    let (config_id, config_version, config_mutable) =
        get_shared_object_ref(&sui_client, MOMENTUM_GLOBAL_CONFIG).await?;

    println!("pool_id: {}, pool_version: {}, pool_mutable: {}", pool_id, pool_version, pool_mutable);
    println!("clock_id: {}, clock_version: {}, clock_mutable: {}", clock_id, clock_version, clock_mutable);
    println!("config_id: {}, config_version: {}, config_mutable: {}", config_id, config_version, config_mutable);

    {
        let mut pool_info = POOL_INFO.lock().await;
        *pool_info = (pool_id, pool_version, pool_mutable);

        let mut clock_info = CLOCK_INFO.lock().await;
        *clock_info = (clock_id, clock_version, clock_mutable);

        let mut config_info = CONFIG_INFO.lock().await;
        *config_info = (config_id, config_version, config_mutable);
    }

    {
        let pool_info_locked = POOL_INFO.lock().await;
        println!("{:?}", pool_info_locked);
    }
    dotenv().ok();
    // Import keypair
    let skp = SuiKeyPair::decode(
        &env::var("PRIVATE_KEY").unwrap()
    )
    .map_err(|_| anyhow!("Invalid private key format"))?;

    let pk = skp.public();
    let sender = SuiAddress::from(&pk);
    println!("Sender: {:?}", sender);

    // Get gas coins
    let gas_coins = sui_client
        .coin_read_api()
        .get_coins(sender, None, None, None)
        .await?
        .data;
    
    if gas_coins.is_empty() {
        return Err(anyhow!("No coins found for sender"));
    }

    let all_coins = sui_client
        .coin_read_api()
        .get_all_coins(sender, None, None)
        .await?
        .data;

    let obj_id = ObjectID::from_str("0x2ca5ae70d8b86b3bcfa7882c3b2bb3a86afe8cffb5a62c6bb35ec75df2d736d7").unwrap();
    let obj_id2 = ObjectID::from_str("0x3ebf9624d59cd35a9b58d3c206e78b3d63da9c741c8c01ea787bef357d0d9476").unwrap();
    let result = sui_client.transaction_builder().merge_coins(sender, obj_id, obj_id2, None, 740).await?;
    // println!("{:#?}", result);
    println!("all_coins ({}): {:#?}", all_coins.len(), all_coins);


    // let merged_balances = merge_account_balances(&sui_client, sender, all_coins).await;
    // println!("{:#?}", merged_balances);
    // for (coin_type, (object_id, version, digest, total_balance)) in merged_balances {
    //     println!("Coin Type: {}, Object ID: {:?}, Version: {:?}, Digest: {:?}, Total Balance: {}", coin_type, object_id, version, digest, total_balance);
    // }

    // println!("Available gas coins ({}): {:#?}", gas_coins.len(), gas_coins);

    // Example: Swap USDC to SUI (like in the transaction data)
    // println!("\n=== Swapping USDC to SUI on Momentum ===");
    // let usdc_to_sui_params = MomentumSwapParams::new_usdc_to_sui(
    //     100_000, // 10 USDC (in micro USDC)
    //     sender,  // Recipient address
    // );

    // let swap_result =
    //     execute_momentum_swap(&sui_client, &skp, sender, &usdc_to_sui_params, &gas_coins).await;
    // match swap_result {
    //     Ok(digest) => println!("USDC->SUI swap successful. Digest: {}", digest),
    //     Err(e) => println!("USDC->SUI swap failed: {}", e),
    // }

    // tokio::time::sleep(tokio::time::Duration::from_secs(20)).await;

    // let swap_result2 =
    //     execute_momentum_swap(&sui_client, &skp, sender, &usdc_to_sui_params, &gas_coins).await;
    // match swap_result2 {
    //     Ok(digest) => println!("USDC->SUI swap successful. Digest: {}", digest),
    //     Err(e) => println!("USDC->SUI swap failed: {}", e),
    // }

    Ok(())
}

async fn execute_momentum_swap(
    sui_client: &sui_sdk::SuiClient,
    keypair: &SuiKeyPair,
    sender: SuiAddress,
    swap_params: &MomentumSwapParams,
    gas_coins: &[sui_sdk::rpc_types::Coin],
) -> Result<String, anyhow::Error> {
    println!("Executing Momentum swap: {:?}", swap_params.direction);
    println!("Amount in: {}", swap_params.amount_in);

    // Build the programmable transaction
    let gas_coin_refs: Vec<_> = gas_coins
        .iter()
        .take(1)
        .map(|coin| coin.object_ref())
        .collect();
    let pt =
        build_momentum_swap_transaction(sui_client, sender, swap_params, gas_coin_refs.clone())
            .await?;

    let gas_budget = 30_000_000; // 0.03 SUI - Momentum transactions are more complex
    let gas_price = sui_client.read_api().get_reference_gas_price().await?;

    // Create the transaction data
    let tx_data =
        TransactionData::new_programmable(sender, gas_coin_refs, pt, gas_budget, gas_price);

    // Sign and execute the transaction
    let intent_msg = IntentMessage::new(Intent::sui_transaction(), tx_data);
    let raw_tx = bcs::to_bytes(&intent_msg).expect("bcs should not fail");

    // Hash the transaction
    use blake2::{Blake2b, Digest as Blake2Digest};
    let mut hasher = Blake2b::<typenum::U32>::new();
    hasher.update(&raw_tx);
    let hash_result = hasher.finalize();
    let hash_array: [u8; 32] = hash_result.into();

    let sui_sig = keypair.sign(&hash_array);

    // Verify signature locally
    let verification_result = sui_sig.verify_secure(
        &intent_msg,
        sender,
        sui_types::crypto::SignatureScheme::ED25519,
    );

    if verification_result.is_err() {
        return Err(anyhow!("Signature verification failed locally"));
    }

    // Execute the transaction
    let transaction_response = sui_client
        .quorum_driver_api()
        .execute_transaction_block(
            sui_types::transaction::Transaction::from_generic_sig_data(
                intent_msg.value,
                vec![GenericSignature::Signature(sui_sig)],
            ),
            SuiTransactionBlockResponseOptions::full_content(),
            None,
        )
        .await?;

    // Check transaction status
    if let Some(_effects) = &transaction_response.effects {
        println!("Transaction executed successfully");
    } else {
        println!("No effects in transaction response");
    }

    if let Some(balance_changes) = &transaction_response.balance_changes {
        println!("Balance changes:");
        for change in balance_changes {
            println!("  {}: {}", change.coin_type, change.amount);
        }
    }

    Ok(transaction_response.digest.base58_encode())
}
