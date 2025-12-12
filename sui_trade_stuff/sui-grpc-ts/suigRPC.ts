import { SuiGrpcClient } from '@mysten/sui/grpc';
import { SuiClient, getFullnodeUrl } from '@mysten/sui/client';
import { Ed25519Keypair } from '@mysten/sui/keypairs/ed25519';
import { fromBase64 } from '@mysten/sui/utils';
import { momentumAddLiquidity } from './tx/momentum_add_liquidity.js';
import { create } from 'domain';
import { createPool } from './tx/momentum_create_pool.js';
import { swap } from './tx/momentum_trade_clean.js';

const grpcClient = new SuiGrpcClient({
	network: 'testnet',
	baseUrl: 'https://fullnode.testnet.sui.io:443',
});

// 給 Transaction.build 用的 HTTP client
const httpClient = new SuiClient({
	url: getFullnodeUrl('testnet'),
});

async function getKeypair(): Promise<Ed25519Keypair> {
	return Ed25519Keypair.fromSecretKey(
		"suiprivkey1qrqp6xtphngqg9nh488v6hyvev229wl6w964nuukl9l95c090pkskvuznyd"
	);
}

async function measureLatency() {
	const signer = await getKeypair();
	const tx = swap("0x404a7f46e5473495993ebddb764be27ece654edfe9917fa905424b56719de086", 1000, true, "0xa3593a0e01b6294da826ac24e0d5fdfbd276862fce7e1528136c8fa1f2b9b9c9", signer.toSuiAddress());

	tx.setSender(signer.toSuiAddress());

	// ---------- Fetch Gas Price ----------
	const {
		response: { epoch },
	} = await grpcClient.ledgerService.getEpoch({});
	tx.setGasPrice(epoch?.referenceGasPrice ?? 1000);

	// ---------- Fetch Gas Coins ----------
	const {
		response: { objects },
	} = await grpcClient.stateService.listOwnedObjects({
		owner: signer.toSuiAddress(),
		objectType: '0x2::coin::Coin<0x2::sui::SUI>',
		readMask: { paths: ['object_id', 'version', 'digest', 'balance'] },
	});

	if (!objects || objects.length === 0) throw new Error('No gas coins');
	const coin = objects[0];

	tx.setGasPayment([
		{
			objectId: coin.objectId,
			version: coin.version.toString(),
			digest: coin.digest,
		},
	]);

	tx.setGasBudget(100_000_000);

	// ---------- Measure build time ----------
	const t0 = performance.now();
	const transactionBytes = await tx.build({
		client: httpClient, // 🔴 關鍵：把 client 傳進去
	});
	const t1 = performance.now();

	console.log(`Build latency: ${(t1 - t0).toFixed(2)} ms`);

	// ---------- Measure sign time ----------
	const t2 = performance.now();
	const { signature } = await tx.sign({ signer });
	const t3 = performance.now();

	console.log(`Sign latency: ${(t3 - t2).toFixed(2)} ms`);

	// ---------- Measure network submission latency ----------
	const t4 = performance.now();
	const { response } =
		await grpcClient.transactionExecutionService.executeTransaction({
			transaction: {
				bcs: { value: transactionBytes },
			},
			signatures: [
				{
					bcs: { value: fromBase64(signature) },
					signature: { oneofKind: undefined },
				},
			],
		});
	const t5 = performance.now();

	console.log(`RPC submission latency: ${(t5 - t4).toFixed(2)} ms`);
	console.log('Execution response:', response);
}

measureLatency();
