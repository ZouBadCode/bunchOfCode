use std::time::Instant;

use anyhow::Result;
use prost_types::{value, FieldMask, Value};
use tonic::transport::{Channel, ClientTlsConfig};
use tonic::Request;
// --- Generated modules via tonic::include_proto! ---
// 對應 proto package: google.rpc
pub mod google {
    pub mod rpc {
        tonic::include_proto!("google.rpc");
    }
}

// 對應 proto package: sui.rpc.v2
pub mod sui {
    pub mod rpc {
        pub mod v2 {
            tonic::include_proto!("sui.rpc.v2");
        }
    }
}

use sui::rpc::v2::ledger_service_client::LedgerServiceClient;
use sui::rpc::v2::GetObjectRequest;

const OBJECT_ID: &str =
    "0x6e35c9f02f1cebb018f8c2b9f157dea6cf5d03bcc63f1addf4c2609be8c29212";

async fn create_client() -> Result<LedgerServiceClient<Channel>> {
    // 1) 建 TLS 設定，載入系統信任的 root CA
    let tls_config = ClientTlsConfig::new()
        .with_native_roots()                    // 🔑 關鍵：啟用系統 root cert
        .domain_name("fullnode.mainnet.sui.io"); // SNI / 憑證主體驗證用

    // 2) 建 Endpoint 並套上 TLS
    let endpoint = Channel::from_static("https://fullnode.mainnet.sui.io:443")
        .tls_config(tls_config)?;               // 把 TLS config 掛上去

    // 3) 建連線
    let channel = endpoint.connect().await?;
    Ok(LedgerServiceClient::new(channel))
}
/// Extract "sqrt_price" from the JSON field of the object, if present.
fn extract_sqrt_price(json: &Value) -> Option<String> {
    let struct_value = match &json.kind {
        Some(value::Kind::StructValue(s)) => s,
        _ => return None,
    };

    let field_value = struct_value.fields.get("sqrt_price")?;

    match &field_value.kind {
        Some(value::Kind::StringValue(s)) => Some(s.clone()),
        Some(value::Kind::NumberValue(n)) => Some(n.to_string()),
        _ => None,
    }
}

async fn get_object_with_timing(
    client: &mut LedgerServiceClient<Channel>,
    object_id: &str,
) -> Result<(Option<String>, f64)> {
    let request = GetObjectRequest {
        // 注意：proto3 optional string -> Option<String>
        object_id: Some(object_id.to_string()),
        // 不指定 version -> latest
        version: None,
        read_mask: Some(FieldMask {
            paths: vec!["json".to_string()],
        }),
    };

    let start = Instant::now();

    let response = client
        .get_object(Request::new(request))
        .await?
        .into_inner();

    let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;

    let sqrt_price = response
        .object
        .as_ref()
        .and_then(|obj| obj.json.as_ref())
        .and_then(extract_sqrt_price);

    Ok((sqrt_price, elapsed_ms))
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Sui gRPC Rust client example ===");
    println!("Object ID: {OBJECT_ID}");
    println!("-----------------------------------------");

    let rounds = 10usize;
    let mut latencies = Vec::with_capacity(rounds);

    let mut client = create_client().await?;

    for i in 0..rounds {
        match get_object_with_timing(&mut client, OBJECT_ID).await {
            Ok((sqrt_price, latency_ms)) => {
                println!(
                    "[{}] sqrt_price = {:?}, latency = {:.3} ms",
                    i, sqrt_price, latency_ms
                );
                latencies.push(latency_ms);
            }
            Err(e) => {
                eprintln!("[{}] Error while calling GetObject: {:#}", i, e);
            }
        }
    }

    if !latencies.is_empty() {
        let sum: f64 = latencies.iter().copied().sum();
        let avg = sum / (latencies.len() as f64);
        let min = latencies
            .iter()
            .copied()
            .fold(f64::INFINITY, f64::min);
        let max = latencies
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max);

        println!("-----------------------------------------");
        println!("Rounds: {}", latencies.len());
        println!("Avg latency: {:.3} ms", avg);
        println!("Min latency: {:.3} ms", min);
        println!("Max latency: {:.3} ms", max);
    }

    Ok(())
}
