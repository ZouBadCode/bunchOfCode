import {
    Transaction,
    coinWithBalance,
    type TransactionObjectArgument,
    type TransactionResult,
} from '@mysten/sui/transactions';

export const MOMENTUM_TRADE_PACKAGE = "0xd7c99e1546b1fc87a6489afdc08bcece4ae1340cbd8efd2ab152ad71dea0f0f2";
const MOMENTUM_SLIPPAGE_PACKAGE = "0xfd6a45c396a90811fd93efaf585cc95c29aecd079c87822893f1e97e3fee8c50";
const AtokenType = "0x0576369c9dd28886d5b94e0bd3cc4b5f23fb1728abfde5b61e672ca7d4994ba4::atoken::ATOKEN";
const BtokenType = "0x0576369c9dd28886d5b94e0bd3cc4b5f23fb1728abfde5b61e672ca7d4994ba4::btoken::BTOKEN";

export const swap = (Token: string, amount: number, direction: boolean, pool: string, sender: string) => {
    const tx = new Transaction();

    const [splitCoin] = tx.splitCoins(
        tx.object(Token),
        [amount],
    )

    const [flashSwap_r1, flashSwap_r2, flashSwap_r3] = tx.moveCall({
        target: `${MOMENTUM_TRADE_PACKAGE}::trade::flash_swap`,
        typeArguments: [AtokenType, BtokenType],
        arguments: [
            tx.object(pool),
            tx.pure.bool(direction),
            tx.pure.bool(true),
            tx.pure.u64(amount),
            tx.pure.u128(direction ? 4295048016n : 79226673515401279992447579055n),
            tx.object("0x6"),
            tx.object("0x83ea3e3e7384efd6b524ff973e4b627cd84d190c45d3f4fd9f5f4fc6c95fd26b")
        ],
    });

    // Destroy the zero balance (input side after swap)
    tx.moveCall({
        target: "0x2::balance::destroy_zero",
        typeArguments: [direction ? AtokenType : BtokenType],
        arguments: [direction ? flashSwap_r1! : flashSwap_r2!],
    })

    // Convert output balance to coin
    const outputCoin = tx.moveCall({
        target: "0x2::coin::from_balance",
        typeArguments: [direction ? BtokenType : AtokenType],
        arguments: [direction ? flashSwap_r2! : flashSwap_r1!],
    });

    const [receiptDebts_r1, receiptDebts_r2] = tx.moveCall({
        target: `${MOMENTUM_TRADE_PACKAGE}::trade::swap_receipt_debts`,
        arguments: [flashSwap_r3!],
    });

    const repayFromSplit = tx.moveCall({
        target: "0x2::coin::split",
        typeArguments: [direction ? AtokenType : BtokenType],
        arguments: [splitCoin, direction ? receiptDebts_r1! : receiptDebts_r2!],
    });

    const balance1 = tx.moveCall({
        target: "0x2::coin::into_balance",
        typeArguments: [direction ? AtokenType : BtokenType],
        arguments: [repayFromSplit!],
    })

    const zeroCoin = tx.moveCall({
        target: "0x2::coin::zero",
        typeArguments: [direction ? BtokenType : AtokenType],
    })

    const balance2 = tx.moveCall({
        target: "0x2::coin::into_balance",
        typeArguments: [direction ? BtokenType : AtokenType],
        arguments: [zeroCoin!],
    })

    tx.moveCall({
        target: `${MOMENTUM_TRADE_PACKAGE}::trade::repay_flash_swap`,
        typeArguments: [AtokenType, BtokenType],
        arguments: [
            tx.object(pool),
            flashSwap_r3!,
            balance1!,
            balance2!,
            tx.object("0x83ea3e3e7384efd6b524ff973e4b627cd84d190c45d3f4fd9f5f4fc6c95fd26b")
        ],
    });

    tx.moveCall({
        target: `${MOMENTUM_SLIPPAGE_PACKAGE}::slippage_check::assert_slippage`,
        typeArguments: [AtokenType, BtokenType],
        arguments: [
            tx.object(pool),
            tx.pure.u128(direction ? 0n : 18446744073709551615n),
            tx.pure.bool(direction)
        ],
    });

    // Transfer output coin and remaining input coin to sender
    tx.transferObjects([outputCoin!, splitCoin], sender);

    return tx;
}