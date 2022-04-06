use std::str::FromStr;
use web3::contract::{Contract, Options};
use web3::types::{Address, U256};

#[tokio::main]
async fn main() -> web3::Result<()> {
    let transport = web3::transports::Http::new("https://bsc-dataseed.binance.org/")?;
    let web3 = web3::Web3::new(transport);

    web3.eth();
    let address = Address::from_str("0x1ec94be5c72cf0e0524d6ecb6e7bd0ba1700bf70").unwrap();
    let token_contract =
        Contract::from_json(web3.eth(), address, include_bytes!("contract-abi.json")).unwrap();

    let token_name: String = token_contract
        .query("name", (), None, Options::default(), None)
        .await
        .unwrap();

    let total_supply: U256 = token_contract
        .query("totalSupply", (), None, Options::default(), None)
        .await
        .unwrap();

    let tokenid: U256 = U256::from(9000);

    let owner: Address = token_contract
        .query("ownerOf", tokenid, None, Options::default(), None)
        .await
        .expect("Token inexistente");

    let token_uri: String = token_contract
        .query("tokenURI", tokenid, None, Options::default(), None)
        .await
        .unwrap();

    println!("Token name: {}, total supply: {}", token_name, total_supply);
    println!("NFT of id {} is owned by: {}", tokenid, owner);
    println!("Token URI: {}", token_uri);

    Ok(())
}