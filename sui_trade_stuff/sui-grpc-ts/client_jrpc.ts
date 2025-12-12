import { SuiClient } from "@mysten/sui/client";

const suiclient =  new SuiClient({
    url: "https://fullnode.testnet.sui.io:443",
    network: "mainnet",
})

async function main(suiclient: SuiClient, address: string) {
    console.time("Query execution time");
    suiclient.getAllBalances({ owner: address }).then((balances) => {
        console.timeEnd("Query execution time");
        console.log(balances);
    });
}

main(suiclient, "0x0ecb503cecc7b10091914bb7b1f19fdc7eecef176bded2322ccb1f031f86ed09");