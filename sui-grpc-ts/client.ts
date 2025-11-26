import grpc = require("@grpc/grpc-js");
import protoLoader = require("@grpc/proto-loader");
import path = require("path");

// 1) 指向 ledger_service.proto
const PROTO_PATH = path.join(
  __dirname,
  "protos",
  "sui",
  "rpc",
  "v2",
  "ledger_service.proto"
);

// 2) 載入 proto 定義
const packageDefinition = protoLoader.loadSync(PROTO_PATH, {
  keepCase: true,
  longs: String,
  enums: String,
  defaults: true,
  oneofs: true,
  // 讓 import "sui/rpc/v2/xxx.proto" 這類路徑找得到
  includeDirs: [path.join(__dirname, "protos")],
});

// 3) 取得 package 物件
const suiProto = grpc.loadPackageDefinition(packageDefinition) as any;
const LedgerService = suiProto.sui.rpc.v2.LedgerService;

// 4) 建立 gRPC client（mainnet fullnode）
const client = new LedgerService(
  "fullnode.mainnet.sui.io:443",
  grpc.credentials.createSsl()
);

// 要測試的 object id（你的 pool 物件）
const OBJECT_ID =
  "0x6e35c9f02f1cebb018f8c2b9f157dea6cf5d03bcc63f1addf4c2609be8c29212";

// 小工具：用 hrtime 量毫秒
function measureMs(start: bigint, end: bigint): number {
  const diffNs = Number(end - start); // nanoseconds
  return diffNs / 1e6;
}

// 包一層 Promise 方便用 async/await
function getObjectWithTiming(
  objectId: string
): Promise<{ response: any; latencyMs: number }> {
  const request = {
    object_id: objectId,
    read_mask: {
      paths: ["json"], // 跟你 grpcurl 一樣，只要 json 欄位
    },
  };

  return new Promise((resolve, reject) => {
    const start = process.hrtime.bigint();

    client.GetObject(request, (err: any, response: any) => {
      const end = process.hrtime.bigint();
      const latencyMs = measureMs(start, end);

      if (err) {
        return reject(err);
      }

      resolve({ response, latencyMs });
    });
  });
}

async function main() {
  try {
    console.log("=== Sui gRPC TypeScript client example ===");
    console.log(`Object ID: ${OBJECT_ID}`);
    console.log("-----------------------------------------");

    const rounds = 10;
    const latencies: number[] = [];

    for (let i = 0; i < rounds; i++) {
      const { response, latencyMs } = await getObjectWithTiming(OBJECT_ID);

      // 回傳格式預期：
      // {
      //   object: {
      //     objectId: "...",
      //     version: "...",
      //     json: {
      //       sqrt_price: "5464...",
      //       ...
      //     }
      //   }
      // }
      const sqrtPrice = response?.object?.json?.structValue.fields.sqrt_price.stringValue ?? null;
      console.log(
        `[${i}] sqrt_price = ${sqrtPrice}, latency = ${latencyMs.toFixed(
          3
        )} ms`
      );
      latencies.push(latencyMs);
    }

    if (latencies.length > 0) {
      const avg =
        latencies.reduce((sum, v) => sum + v, 0) / latencies.length;
      const min = Math.min(...latencies);
      const max = Math.max(...latencies);

      console.log("-----------------------------------------");
      console.log(`Rounds: ${latencies.length}`);
      console.log(`Avg latency: ${avg.toFixed(3)} ms`);
      console.log(`Min latency: ${min.toFixed(3)} ms`);
      console.log(`Max latency: ${max.toFixed(3)} ms`);
    }
  } catch (err) {
    console.error("Error while calling GetObject:", err);
  }
}

main();
