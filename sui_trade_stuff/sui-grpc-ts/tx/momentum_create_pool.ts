import {
    Transaction,
} from '@mysten/sui/transactions';

const MMT = "0xd7c99e1546b1fc87a6489afdc08bcece4ae1340cbd8efd2ab152ad71dea0f0f2";
const AtokenType = "0x0576369c9dd28886d5b94e0bd3cc4b5f23fb1728abfde5b61e672ca7d4994ba4::atoken::ATOKEN";
const BtokenType = "0x0576369c9dd28886d5b94e0bd3cc4b5f23fb1728abfde5b61e672ca7d4994ba4::btoken::BTOKEN";

export const createPool = () => {
    const tx = new Transaction();

    const [pool] = tx.moveCall({
        target: `${MMT}::create_pool::new`,
        typeArguments: [AtokenType, BtokenType],
        arguments: [
            tx.object("0x3c4385bf373c7997a953ee548f45188d9f1ca4284ec835467688d8ee276e1af7"),
            tx.pure.u64(3000),
            tx.object("0x83ea3e3e7384efd6b524ff973e4b627cd84d190c45d3f4fd9f5f4fc6c95fd26b"),
        ],
    });

    tx.moveCall({
        target: `${MMT}::pool::initialize`,
        typeArguments: [AtokenType, BtokenType],
        arguments: [
            tx.object(pool!),
            tx.pure.u128(18446744073709551616),
            tx.object("0x6")
        ],
    })

    tx.moveCall({
        target: `${MMT}::pool::transfer`,
        typeArguments: [AtokenType, BtokenType],
        arguments: [
            tx.object(pool!),
        ],
    })
    return tx;
}