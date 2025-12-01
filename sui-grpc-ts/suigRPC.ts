import { SuiGrpcClient } from '@mysten/sui/grpc';
import { Ed25519Keypair } from '@mysten/sui/keypairs/ed25519';
import { send_sui }  from './tx/test_tx.js';
import { fromBase64 } from '@mysten/sui/utils';

const grpcClient = new SuiGrpcClient({
	network: 'testnet',
	baseUrl: 'https://fullnode.testnet.sui.io:443',
});

async function getKeypair(): Promise<Ed25519Keypair> {
	return Ed25519Keypair.fromSecretKey(
		"suiprivkey1qrqp6xtphngqg9nh488v6hyvev229wl6w964nuukl9l95c090pkskvuznyd"
	);
}

async function measureLatency() {
	const signer = await getKeypair();
	const tx = send_sui("0x5e3f6b1d3c4e8f7a9b2c3d4e5f60718293a4b5c6d7e8f9a0b1c2d3e4f5061728");
	tx.setSender(signer.toSuiAddress());

	// ---------- Fetch Gas Price ----------
	const { response: { epoch } } = await grpcClient.ledgerService.getEpoch({});
	tx.setGasPrice(epoch?.referenceGasPrice ?? 1000);

	// ---------- Fetch Gas Coins ----------
	const { response: { objects } } = await grpcClient.stateService.listOwnedObjects({
		owner: signer.toSuiAddress(),
		objectType: '0x2::coin::Coin<0x2::sui::SUI>',
		readMask: { paths: ['object_id', 'version', 'digest', 'balance'] }
	});

	if (!objects || objects.length === 0) throw new Error("No gas coins");
	const coin = objects[0];

	tx.setGasPayment([{
		objectId: coin.objectId,
		version: coin.version.toString(),
		digest: coin.digest,
	}]);

	tx.setGasBudget(10_000_000);


	// ---------- Measure build time ----------
	const t0 = performance.now();
	const transactionBytes = await tx.build({});
	const t1 = performance.now();

	console.log(`Build latency: ${(t1 - t0).toFixed(2)} ms`);


	// ---------- Measure sign time ----------
	const t2 = performance.now();
	const { signature } = await tx.sign({ signer });
	const t3 = performance.now();

	console.log(`Sign latency: ${(t3 - t2).toFixed(2)} ms`);


	// ---------- Measure network submission latency ----------
	const t4 = performance.now();
	const { response } = await grpcClient.transactionExecutionService.executeTransaction({
		transaction: {
			bcs: { value: transactionBytes },
		},
		signatures: [{
			bcs: { value: fromBase64(signature) },
			signature: { oneofKind: undefined },
		}],
	});
	const t5 = performance.now();

	console.log(`RPC submission latency: ${(t5 - t4).toFixed(2)} ms`);
	console.log("Execution response:", response);
}

measureLatency();
