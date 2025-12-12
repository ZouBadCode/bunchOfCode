import {
    Transaction,
    coinWithBalance,
    type TransactionObjectArgument,
    type TransactionResult,
} from '@mysten/sui/transactions';
import { SuiClient } from '@mysten/sui/client';

// ===== Constants =====
export const MOMENTUM_TRADE_PACKAGE =
    "0xd7c99e1546b1fc87a6489afdc08bcece4ae1340cbd8efd2ab152ad71dea0f0f2";

export const MOMENTUM_SLIPPAGE_PACKAGE =
    "0xfd6a45c396a90811fd93efaf585cc95c29aecd079c87822893f1e97e3fee8c50";

export const SUI_FRAMEWORK_PACKAGE =
    "0x0000000000000000000000000000000000000000000000000000000000000002";

export const SUI_CLOCK_OBJECT =
    "0x0000000000000000000000000000000000000000000000000000000000000006";

// Coin types
export const SUI_COIN_TYPE = "0x2::sui::SUI";
export const USDC_COIN_TYPE =
    "0xdba34672e30cb065b1f93e3ab55318768fd6fef66c15942c9f7cb846e2f900e7::usdc::USDC";

// Pool objects
export const MOMENTUM_POOL_OBJECT =
    "0x455cf8d2ac91e7cb883f515874af750ed3cd18195c970b7a2d46235ac2b0c388";
export const MOMENTUM_GLOBAL_CONFIG =
    "0x2375a0b1ec12010aaea3b2545acfa2ad34cfbba03ce4b59f4c39e1e25eed1b2a";

// Metadata
export type SwapDirection = "SUI_TO_USDC" | "USDC_TO_SUI";

export interface MomentumSwapParams {
    direction: SwapDirection;
    amountIn: bigint;
    sqrtPriceLimit?: bigint;
    recipient: string;
}

export async function buildMomentumSwapPTB(
    client: SuiClient,
    sender: string,
    params: MomentumSwapParams,
    gasCoins: { objectId: string; version: string; digest: string }[],
) {
    const tx = new Transaction();

    // Pick 1 gas coin
    if (gasCoins.length === 0) throw new Error("No gas coins provided");
    const gasCoin = gasCoins[0]!;

    // ---
    // A. Determine swap direction
    // Rust: x_for_y = SuiToUsdc ? true : false
    // ---

    const xForY = params.direction === "SUI_TO_USDC";
    const byAmountIn = true;

    // ---
    // B. Select input coins
    // ---

    let inputCoin: TransactionResult;

    if (params.direction === "SUI_TO_USDC") {
        inputCoin = coinWithBalance(
            { balance: params.amountIn, useGasCoin: false }
        )(tx);
    } else {
        inputCoin = coinWithBalance(
            { balance: params.amountIn, useGasCoin: false, type: USDC_COIN_TYPE }
        )(tx);
    }
        // const usdcCoins = await client.getCoins({
        //     owner: sender,
        //     coinType: USDC_COIN_TYPE,
        // });

        // if (usdcCoins.data.length === 0)
        //     throw new Error("No USDC coins found");

        // const usdcCoin = usdcCoins.data[0]!;
        // inputCoin = tx.objectRef({
        //     objectId: usdcCoin.coinObjectId,
        //     digest: usdcCoin.digest,
        //     version: usdcCoin.version,
        // });


    // ---
    // D. Load shared objects
    // ---

    // === Type tags ===
    const SUI = SUI_COIN_TYPE;
    const USDC = USDC_COIN_TYPE;

    const sqrtPriceLimit = tx.pure.u128(
        params.sqrtPriceLimit ??
            79226673515401279992447579050n
    );

    // ---
    // E. Call flash_swap()
    // ---

    const [flashSwap_r1, flashSwap_r2, flashSwap_r3] = tx.moveCall({
        target: `${MOMENTUM_TRADE_PACKAGE}::trade::flash_swap`,
        typeArguments: [SUI, USDC],
        arguments: [
            tx.object(MOMENTUM_POOL_OBJECT),
            tx.pure.bool(xForY),
            tx.pure.bool(byAmountIn),
            inputCoin,
            sqrtPriceLimit,
            tx.object("0x6"),
            tx.object(MOMENTUM_GLOBAL_CONFIG),
        ],
    });

    // flashSwap returns a struct, so nested indexing is required later

    // ---
    // F. destroy_zero for USDC
    // ---
    tx.moveCall({
        target: `${SUI_FRAMEWORK_PACKAGE}::balance::destroy_zero`,
        typeArguments: [USDC],
        arguments: [flashSwap_r2!],
    });

    // ---
    // G. zero SUI coin
    // ---
    const zeroSui = tx.moveCall({
        target: `${SUI_FRAMEWORK_PACKAGE}::coin::zero`,
        typeArguments: [SUI],
        arguments: [],
    });

    // ---
    // H. receipt debts
    // ---

    const [receiptDebts_r1, receiptDebts_r2] = tx.moveCall({
        target: `${MOMENTUM_TRADE_PACKAGE}::trade::swap_receipt_debts`,
        arguments: [flashSwap_r3!],
    });

    // ---
    // I. split repay amount
    // ---

    const repayFromSplit = tx.moveCall({
        target: `${SUI_FRAMEWORK_PACKAGE}::coin::split`,
        typeArguments: [USDC],
        arguments: [
            inputCoin,
            receiptDebts_r2!,
        ],
    });

    // ---
    // J. Convert to balance
    // ---

    const suiBalance = tx.moveCall({
        target: `${SUI_FRAMEWORK_PACKAGE}::coin::into_balance`,
        typeArguments: [SUI],
        arguments: [zeroSui],
    });

    const usdcBalance = tx.moveCall({
        target: `${SUI_FRAMEWORK_PACKAGE}::coin::into_balance`,
        typeArguments: [USDC],
        arguments: [repayFromSplit],
    });

    // ---
    // K. repay_flash_swap()
    // ---

    tx.moveCall({
        target: `${MOMENTUM_TRADE_PACKAGE}::trade::repay_flash_swap`,
        typeArguments: [SUI, USDC],
        arguments: [
            tx.object(MOMENTUM_POOL_OBJECT),
            flashSwap_r3!,
            suiBalance,
            usdcBalance,
            tx.object(MOMENTUM_GLOBAL_CONFIG),
        ],
    });

    // ---
    // L. Slippage check
    // ---

    tx.moveCall({
        target: `${MOMENTUM_SLIPPAGE_PACKAGE}::slippage_check::assert_slippage`,
        typeArguments: [SUI, USDC],
        arguments: [
            tx.object(MOMENTUM_POOL_OBJECT),
            sqrtPriceLimit,
            tx.pure.bool(xForY),
        ],
    });

    // ---
    // M. Convert to final coin
    // ---

    const finalCoin = tx.moveCall({
        target: `${SUI_FRAMEWORK_PACKAGE}::coin::from_balance`,
        typeArguments: [SUI],
        arguments: [flashSwap_r1!],
    });

    // ---
    // N. Transfer outputs
    // ---

    tx.transferObjects([inputCoin], params.recipient);
    tx.transferObjects([finalCoin], params.recipient);

    return tx;
}
