use anyhow::Result;
use prost_types::{value::Kind, FieldMask, Value};
use std::time::{Duration, Instant};
use tonic::transport::Channel;

// IMPORTANT: 在 crate root 直接 include 這兩個 package
// 會自動產生：crate::sui::rpc::v2::... 和 crate::google::rpc::...
tonic::include_proto!("sui.rpc.v2");
tonic::include_proto!("google.rpc");

// 對應到 sui.rpc.v2 裡的型別
use sui::rpc::v2::ledger_service_client::LedgerServiceClient;
use sui::rpc::v2::{GetObjectRequest, GetObjectResponse};

#[tokio::main]
async fn main() -> Result<()> {
    let object_id =
        "0x6e35c9f02f1cebb018f8c2b9f157dea6cf5d03bcc63f1addf4c2609be8c29212";

    // 建 TLS channel
    let channel = Channel::from_static("https://fullnode.mainnet.sui.io:443")
        .connect()
        .await?;
    let mut client = LedgerServiceClient::new(channel);

    let rounds = 20usize;
    let mut samples = Vec::with_capacity(rounds);

    for i in 0..rounds {
        // 注意：object_id 是 Option<String>
        // read_mask 也是 Option<FieldMask>
        let req = GetObjectRequest {
            object_id: Some(object_id.to_string()),
            read_mask: Some(FieldMask {
                paths: vec!["json".to_string()],
            }),
            // 其它欄位用預設值
            ..Default::default()
        };

        let start = Instant::now();
        let response = client.get_object(req).await?;
        let elapsed = start.elapsed();

        let resp = response.into_inner();
        let sqrt_price =
            extract_sqrt_price(&resp).unwrap_or_else(|| "UNKNOWN".to_string());

        println!(
            "[{}] sqrt_price = {}, latency = {:.3} ms",
            i,
            sqrt_price,
            duration_to_ms(elapsed)
        );

        samples.push(elapsed);
    }

    let avg_ms: f64 = samples
        .iter()
        .map(|d| duration_to_ms(*d))
        .sum::<f64>()
        / samples.len() as f64;

    println!("=== summary ===");
    println!("rounds: {}", rounds);
    println!("avg latency: {:.3} ms", avg_ms);

    Ok(())
}

fn duration_to_ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}

// 從 GetObjectResponse 裡面把 json.srqt_price 取出來
fn extract_sqrt_price(resp: &GetObjectResponse) -> Option<String> {
    // response.object: Option<Object>
    let obj = resp.object.as_ref()?;

    // object.json: Option<Value>（google.protobuf.Value）
    let json_val: &Value = obj.json.as_ref()?;

    // Value.kind 裡面包真正的 StructValue / StringValue / NumberValue ...
    match json_val.kind.as_ref()? {
        Kind::StructValue(struct_val) => {
            // struct_val.fields: BTreeMap<String, Value>
            let field_val = struct_val.fields.get("sqrt_price")?;

            match field_val.kind.as_ref()? {
                Kind::StringValue(s) => Some(s.clone()),
                Kind::NumberValue(n) => Some(n.to_string()),
                _ => None,
            }
        }
        _ => None,
    }
}
