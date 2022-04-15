use crate::CustomResponseErrors::{ConnectionProblems, InvalidToken};
use actix_web::http::StatusCode;
use actix_web::web::{Json};
use actix_web::{error, get, web, App, HttpResponse, HttpServer, ResponseError};
use serde_json::{json, Value};
use serde::{Serialize, Deserialize};
use web3::contract::{Contract, Options};
use web3::types::{Address, U256};
use std::fmt::{Display, Formatter};
use std::str::FromStr;

#[derive(Debug)]
enum CustomResponseErrors {
    InvalidToken(web3::contract::Error),
    ConnectionProblems(String),
}

impl CustomResponseErrors {
    pub fn name(&self) -> String {
        match self {
            Self::InvalidToken(e) => e.to_string(),
            Self::ConnectionProblems(error) => error.to_string(),
        }
    }
}

impl Display for CustomResponseErrors {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.status_code())
    }
}

impl error::ResponseError for CustomResponseErrors {
    fn status_code(&self) -> StatusCode {
        match *self {
            CustomResponseErrors::InvalidToken(_) => StatusCode::NOT_FOUND,
            CustomResponseErrors::ConnectionProblems(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        let status_code = self.status_code();
        let error_response = ErrorResponse {
            code: status_code.as_u16(),
            message: self.to_string(),
            error: self.name(),
        };

        HttpResponse::build(self.status_code()).json(error_response)
    }
}

#[derive(Serialize)]
struct ErrorResponse {
    code: u16,
    error: String,
    message: String,
}

#[derive(Serialize)]
struct Token {
    token_id: U256,
    owner_address: Address,
}

#[derive(Serialize)]
struct TokensResponse {
    tokens: Vec<Token>,
    total_supply: U256
}

#[derive(Deserialize)]
struct Pagination {
    page: u32,
    limit: u32
}

impl Default for Pagination {
    fn default() -> Self {
        Pagination {
            page: 1,
            limit: 10
        }
    }
}

#[derive(Serialize)]
struct AccountResponse {
    account: Address,
    balance: U256,
}

#[get("/binance/token/{id}")]
async fn owner_of(id: web::Path<i64>) -> Result<Json<Value>, CustomResponseErrors> {
    let t = web3::transports::Http::new("https://bsc-dataseed.binance.org/");

    let transport = match t {
        Ok(transport) => transport,
        Err(_error) => {
            return Err(ConnectionProblems(String::from(
                "Connection problems to the blockchain",
            )))
        }
    };

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

    let token_id: U256 = U256::from(id.into_inner());

    let o: Result<Address, web3::contract::Error> = token_contract
        .query("ownerOf", token_id, None, Options::default(), None)
        .await;

    let owner = match o {
        Ok(address) => address,
        Err(e) => return Err(InvalidToken(e)),
    };

    let token_uri: String = token_contract
        .query("tokenURI", token_id, None, Options::default(), None)
        .await
        .unwrap();

    Ok(web::Json(json!({
        "owner": owner,
        "uri" : token_uri,
        "token_name" : token_name,
        "total_supply" : total_supply
    })))
}

#[get("/binance/tokens")]
async fn tokens(pagination: web::Query<Pagination>) -> Result<Json<TokensResponse>, CustomResponseErrors> {
    let t = web3::transports::Http::new("https://bsc-dataseed.binance.org/");

    let first_result = (pagination.page - 1) * pagination.limit + 1;
    let last_result = first_result + pagination.limit;
    let transport = match t {
        Ok(transport) => transport,
        Err(_error) => return Err(ConnectionProblems(String::from("Connection problems to the blockchain")))
    };

    let web3 = web3::Web3::new(transport);

    web3.eth();
    let address = Address::from_str("0x1ec94be5c72cf0e0524d6ecb6e7bd0ba1700bf70").unwrap();
    let token_contract =
        Contract::from_json(web3.eth(), address, include_bytes!("contract-abi.json")).unwrap();

    let total_supply: U256 = token_contract
        .query("totalSupply", (), None, Options::default(), None)
        .await
        .unwrap();

    let mut tokens: Vec<Token> = Vec::new();

    for n in first_result..last_result {
        let token_id: U256 = U256::from(n);
        let o: Result<Address, web3::contract::Error> = token_contract
            .query("ownerOf", token_id, None, Options::default(), None)
            .await;

        let owner = match o {
            Ok(address) => address,
            Err(e) => return Err(InvalidToken(e))
        };

        let token = Token {
            token_id,
            owner_address: owner
        };

        tokens.push(token);
    }

    let tokens_response = TokensResponse {
        tokens,
        total_supply
    };

    Ok(web::Json(tokens_response))
}

#[get("/api/local")]
async fn local_accounts() -> Result<Json<Vec<AccountResponse>>, CustomResponseErrors> {
    let t = web3::transports::Http::new("http://127.0.0.1:7545");
    let transport = match t {
        Ok(transport) => transport,
        Err(_error) => {
            return Err(ConnectionProblems(String::from(
                "Connection problems to the blockchain",
            )))
        }
    };

    let web3 = web3::Web3::new(transport);

    println!("Calling accounts.");
    let accs = web3.eth().accounts().await;
    let accounts = match accs {
        Ok(accounts) => accounts,
        Err(_error) => {
            return Err(ConnectionProblems(String::from(
                "Connection problems to the blockchain",
            )))
        }
    };
    // println!("Accounts: {:?}", accounts);
    // accounts.push("00a329c0648769a73afac7f9381e08fb43dbea72".parse().unwrap());

    let mut vec: Vec<AccountResponse> = Vec::new();
    for account in accounts {
        let balance = web3.eth().balance(account, None).await.unwrap();
        vec.push(AccountResponse {
            account,
            balance,
        });
    }

    Ok(web::Json(vec))
}

#[get("/")]
async fn index() -> Json<Value> {
    web::Json(json!({
        "Hello": "World!",
    }))
}

#[actix_web::main] // or #[tokio::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .service(owner_of)
            .service(index)
            .service(local_accounts)
            .service(tokens)
    })
        .bind(("127.0.0.1", 8080))?
        .run()
        .await
}
