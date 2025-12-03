import {
    Transaction,
} from '@mysten/sui/transactions';

const MMT = "0xd7c99e1546b1fc87a6489afdc08bcece4ae1340cbd8efd2ab152ad71dea0f0f2";
const AtokenType = "0x0576369c9dd28886d5b94e0bd3cc4b5f23fb1728abfde5b61e672ca7d4994ba4::atoken::ATOKEN";
const BtokenType = "0x0576369c9dd28886d5b94e0bd3cc4b5f23fb1728abfde5b61e672ca7d4994ba4::btoken::BTOKEN";

export const momentumAddLiquidity = () => {
    const tx = new Transaction();

    const result1 = tx.moveCall({
        target: `${MMT}::tick_math::get_tick_at_sqrt_price`,
        arguments: [
            tx.pure.u128(17979662081777052694),
        ],
    });

    const result2 = tx.moveCall({
        target: `${MMT}::tick_math::get_tick_at_sqrt_price`,
        arguments: [
            tx.pure.u128(18902287831555877210),
        ],
    });

    const result3 = tx.moveCall({
        target: `${MMT}::i32::from_u32`,
        arguments: [
            tx.pure.u32(60),
        ],
    })

    const result4 = tx.moveCall({
        target: `${MMT}::i32::mod`,
        arguments: [
            result1!,
            result3!,
        ],
    })

    const result5 = tx.moveCall({
        target: `${MMT}::i32::mod`,
        arguments: [
            result2!,
            result3!,
        ],
    })

    const result6 = tx.moveCall({
        target: `${MMT}::i32::sub`,
        arguments: [
            result1!,
            result4!,
        ],
    })

    const result7 = tx.moveCall({
        target: `${MMT}::i32::sub`,
        arguments: [
            result2!,
            result5!,
        ],
    })

    const position = tx.moveCall({
        target: `${MMT}::liquidity::open_position`,
        arguments: [
            tx.object("0xa3593a0e01b6294da826ac24e0d5fdfbd276862fce7e1528136c8fa1f2b9b9c9"),
            result6!,
            result7!,
            tx.object("0x83ea3e3e7384efd6b524ff973e4b627cd84d190c45d3f4fd9f5f4fc6c95fd26b")
        ],
        typeArguments: [AtokenType, BtokenType]
    });

    const [result8, result9] = tx.moveCall({
        target: `${MMT}::liquidity::add_liquidity`,
        arguments: [
            tx.object("0xa3593a0e01b6294da826ac24e0d5fdfbd276862fce7e1528136c8fa1f2b9b9c9"),
            position!,
            tx.object("0x387ef1e803c10c1176d4eb6e364d1b7a42de4ec59c9224619cc6f3a2189d7295"),
            tx.object("0x09fa250545954bd877206cb8e6d887b1be187f0d35a329a75b88a70b8f8bfe47"),
            tx.pure.u64(0),
            tx.pure.u64(0),
            tx.object("0x6"),
            tx.object("0x83ea3e3e7384efd6b524ff973e4b627cd84d190c45d3f4fd9f5f4fc6c95fd26b"),
        ],
        typeArguments: [AtokenType, BtokenType]
    })
    
    tx.transferObjects(
        [result8!],
        tx.pure.address("0x06b86b719563850f1364044e931f437b80954ae6b50a92c9de6530c4b137824c")
    );
    tx.transferObjects(
        [result9!],
        tx.pure.address("0x06b86b719563850f1364044e931f437b80954ae6b50a92c9de6530c4b137824c")
    );
    tx.transferObjects(
        [position!],
        tx.pure.address("0x06b86b719563850f1364044e931f437b80954ae6b50a92c9de6530c4b137824c")
    );
    
    return tx;
}