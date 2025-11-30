use sui_rpc::Client;
use sui_crypto::ed25519::Ed25519PrivateKey;
use sui_crypto::Signer;
use rand::rngs::OsRng;
use sui_rpc::proto::sui::rpc::v2::GetObjectRequest;
use sui_transaction_builder::{TransactionBuilder, Serialized};
use sui_sdk_types::{Address};
use sui_rpc::proto::sui::rpc::v2::ExecuteTransactionRequest;
use sui_sdk_types::Intent;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 產一個隨機地址（目前只是印出來用）
    let mut rng = OsRng;
    let pk = Ed25519PrivateKey::generate(&mut rng);
    let public_key = pk.public_key();
    let address = public_key.derive_address();

    // 連線到 Testnet fullnode
    let mut client = Client::new(Client::MAINNET_FULLNODE)?;

    // ⚠️ 這裡要放「Object ID」，不是 type tag
    // 用你之前 grpcurl 拿到的 objectId 來測試
    let object_id = "0x6e35c9f02f1cebb018f8c2b9f157dea6cf5d03bcc63f1addf4c2609be8c29212".parse()?;

    // 這個版本的 API 是這樣設計：
    // pub fn new(object_id: &Address) -> Self
    let request = GetObjectRequest::new(&object_id);

    let mut ledger_service = client.ledger_client();
    let res = ledger_service.get_object(request).await?;

    println!("我的地址: {:?}", address);
    println!("object_info: {:?}", res.into_inner());
    println!("Sui Client Connected");
    Ok(())
}

async fn send_sui(
    client: &mut sui_rpc::Client, 
    signer: &Ed25519PrivateKey, 
    recipient: Address
) -> Result<(), Box<dyn std::error::Error>> {
    let sender_address = signer.public_key().derive_address();
    
    // 1. 初始化 TransactionBuilder
    let mut tx_builder = TransactionBuilder::new();
    tx_builder.set_sender(sender_address);
    
    // 設定 Gas 參數 (實際開發中通常需要先估算或查詢)
    tx_builder.set_gas_budget(50_000_000); // 0.05 SUI
    tx_builder.set_gas_price(1000);

    // 2. 尋找並加入 Gas Object
    // (這裡簡化略過尋找 Gas 的過程，你需要用 client.ledger_client() 找到你擁有的 Coin)
    // let gas_coin = ...; 
    // tx_builder.add_gas_objects(vec![gas_coin]);

    // 3. 建構交易指令：轉帳
    // 從 Gas Coin 中切分出 1 SUI (10^9 MIST)
    let amount = tx_builder.input(Serialized(&1_000_000_000u64));
    let coin = tx_builder.split_coins(tx_builder.gas(), vec![amount]);
    
    // 轉移給接收者
    let recipient_arg = tx_builder.input(Serialized(&recipient));
    tx_builder.transfer_objects(vec![coin], recipient_arg);

    // 4. 完成交易建構
    let transaction = tx_builder.finish()?;
    // 5. 簽名
    let signature = signer.sign(&transaction);

    // 6. 發送執行
    let mut exec_service = client.execution_client();
    let request = ExecuteTransactionRequest::new(sui_rpc::proto::sui::rpc::v2::Transaction {
        transaction: Some(transaction.try_into()?),
        signatures: vec![signature.into()],
        ..Default::default()
    });
    let response = exec_service.execute_transaction(request).await?;

    println!("交易已發送，Digest: {:?}", response.into_inner());

    Ok(())
}