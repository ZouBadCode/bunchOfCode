import { Transaction } from "@mysten/sui/transactions";

export const send_sui = (recipient: string) => {
    const tx = new Transaction();
    const [coin] = tx.splitCoins(tx.gas, [100]);
    tx.transferObjects([coin], recipient);
    return tx;
};