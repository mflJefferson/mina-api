use std::fmt::{Display, Formatter};
use std::str::FromStr;
use actix_web::{App, HttpResponse, HttpServer, web, get, error, ResponseError};
use actix_web::http::StatusCode;
use actix_web::web::Json;
use serde_json::{json, Value};
use serde::{Serialize};
use web3::contract::{Contract, Options};
use web3::types::{Address, U256};
use crate::CustomResponseErrors::{ConnectionProblems, InvalidToken};

#[derive(Debug)]
enum CustomResponseErrors {
    InvalidToken(web3::contract::Error),
    ConnectionProblems(String)
}

impl CustomResponseErrors {
    pub fn name(&self) -> String {
        match self {
            Self::InvalidToken(e) => e.to_string(),
            Self::ConnectionProblems(error) => error.to_string()
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

        HttpResponse::build(self.status_code())
            .json(error_response)
    }
}

#[derive(Serialize)]
struct ErrorResponse {
    code: u16,
    error: String,
    message: String,
}

#[get("/binance/token/{id}")]
async fn owner_of(id: web::Path<i64>) -> Result<Json<Value>, CustomResponseErrors> {
    let t = web3::transports::Http::new("https://bsc-dataseed.binance.org/");

    let transport = match t {
        Ok(transport) => transport,
        Err(_error) => return Err(ConnectionProblems(String::from("Connection problems to the blockchain")))
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
        Err(e) => return Err(InvalidToken(e))
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

#[get("/")]
async fn index() -> Json<Value> {
    web::Json(json!({
        "Hello": "World!",
    }))
}

#[actix_web::main] // or #[tokio::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| App::new()
        .service(owner_of)
        .service(index)
    )
        .bind(("127.0.0.1", 8080))?
        .run()
        .await
}