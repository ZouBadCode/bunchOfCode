use anyhow::{anyhow, Result};
use serde_json::Value;
use std::process::Command;
use std::str;
use std::time::{Duration, Instant};

fn main() -> Result<()> {
    // 你要測的 Pool 物件
    let object_id =
        "0x6e35c9f02f1cebb018f8c2b9f157dea6cf5d03bcc63f1addf4c2609be8c29212";

    // 跟你 grpcurl 一樣的 payload，只是用 Rust 字串組出來
    let payload = format!(
        r#"{{
  "object_id": "{object_id}",
  "read_mask": {{
    "paths": ["json"]
  }}
}}"#
    );

    let rounds = 20;
    let mut samples = Vec::with_capacity(rounds);

    for i in 0..rounds {
        let start = Instant::now();

        let output = Command::new("grpcurl")
            .arg("-d")
            .arg(&payload)
            .arg("fullnode.mainnet.sui.io:443")
            .arg("sui.rpc.v2.LedgerService/GetObject")
            .output()?;

        let elapsed = start.elapsed();

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("grpcurl failed: {}", stderr));
        }

        let stdout = str::from_utf8(&output.stdout)?;
        let sqrt_price = extract_sqrt_price(stdout).unwrap_or_else(|| "UNKNOWN".to_string());

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

fn extract_sqrt_price(json_str: &str) -> Option<String> {
    let v: Value = serde_json::from_str(json_str).ok()?;
    // 對應你貼出來的回傳格式：
    // {
    //   "object": {
    //     "json": {
    //       "sqrt_price": "5464238785..."
    //     }
    //   }
    // }
    v.get("object")?
        .get("json")?
        .get("sqrt_price")?
        .as_str()
        .map(|s| s.to_string())
}
