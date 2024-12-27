// sorry for shitcode lol

mod utils {
    use regex::Regex;

    pub fn is_solana_wallet(address: &str) -> bool {
        let solana_wallet_regex = Regex::new(r"[1-9A-HJ-NP-Za-km-z]{32,44}").unwrap();
        solana_wallet_regex.is_match(address)
    }

    pub fn lamports_to_sol(lamports: u64) -> f64 {
        (lamports as f64) / 1_000_000_000.00
    }
}

mod config {
    use crate::solana;
    use serde::{Deserialize, Serialize};
    use serde_yaml;
    use std::fs::File;
    use std::io::Read;

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct Config {
        pub wallets: Vec<solana::Wallet>,
        pub cluster: solana::Cluster,
    }

    impl Config {
        pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
            let mut file = File::open("config.yaml")?;
            let mut contents = String::new();
            file.read_to_string(&mut contents)?;

            let config_yaml: serde_yaml::Value = serde_yaml::from_str(&contents)?;

            let wallets = config_yaml
                .get("wallets")
                .and_then(|v| v.as_sequence())
                .ok_or("Missing or invalid 'wallets' field")?
                .iter()
                .filter_map(|v| v.as_str())
                .filter_map(|address| {
                    let wallet = solana::Wallet::new(address.to_string());
                    if wallet.clone().is_valid() {
                        Some(wallet)
                    } else {
                        None
                    }
                })
                .collect::<Vec<solana::Wallet>>();

            let cluster = config_yaml
                .get("cluster")
                .and_then(|v| v.as_str())
                .map_or_else(
                    || solana::Cluster::from_str("mainnet"),
                    |c| solana::Cluster::from_str(c),
                );

            Ok(Self { wallets, cluster })
        }
    }
}

mod solana {
    use crate::utils;
    use serde::{Deserialize, Serialize};
    use solana_client;
    use solana_program;
    use std::{collections::HashMap, str::FromStr};

    pub const SOL_CA: &str = "So11111111111111111111111111111111111111112";

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub enum Cluster {
        Mainnet,
        Devnet,
        Testnet,
    }

    impl Cluster {
        pub fn from_str(cluster: &str) -> Self {
            match cluster {
                "mainnet" => Self::Mainnet,
                "devnet" => Self::Devnet,
                "testnet" => Self::Testnet,
                _ => Self::Mainnet,
            }
        }

        pub fn rpc_url(self) -> &'static str {
            match self {
                Self::Mainnet => "https://api.mainnet-beta.solana.com",
                Self::Devnet => "https://api.devnet.solana.com",
                Self::Testnet => "https://api.testnet.solana.com",
            }
        }
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct Wallet {
        pub public_key: String,
    }

    impl Wallet {
        pub fn new(public_key: String) -> Self {
            Self { public_key }
        }

        pub fn is_valid(self) -> bool {
            utils::is_solana_wallet(self.public_key.as_str())
        }

        pub fn to_pubkey(self) -> solana_program::pubkey::Pubkey {
            solana_program::pubkey::Pubkey::from_str(self.public_key.as_str()).unwrap()
        }
    }

    pub struct Solana {
        client: solana_client::nonblocking::rpc_client::RpcClient,
    }

    impl Solana {
        pub fn new(cluster: Cluster) -> Self {
            let client: solana_client::nonblocking::rpc_client::RpcClient =
                solana_client::nonblocking::rpc_client::RpcClient::new(
                    cluster.rpc_url().to_string(),
                );

            Self { client }
        }

        pub async fn get_balance(
            &self,
            wallet: Wallet,
        ) -> solana_client::client_error::Result<u64> {
            self.client.get_balance(&wallet.to_pubkey()).await
        }

        pub async fn get_token_accounts(
            &self,
            wallet: Wallet,
        ) -> solana_client::client_error::Result<HashMap<String, u64>> {
            let response: serde_json::Value = self
                .client
                .send(
                    solana_client::rpc_request::RpcRequest::GetTokenAccountsByOwner,
                    serde_json::json!([
                        &wallet.to_pubkey().to_string(),
                        { "programId": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA" }, // SPL Token Program ID
                        { "encoding": "jsonParsed" }
                    ]),
                )
                .await?;

            let accounts = response["value"]
                .as_array()
                .ok_or("Failed to parse token accounts response")
                .unwrap();

            let mut token_balances: HashMap<String, u64> = HashMap::new();

            for account in accounts {
                if let Some(account_data) = account.get("account") {
                    if let Some(mint) = account_data
                        .get("data")
                        .and_then(|data| data.get("parsed"))
                        .and_then(|parsed| parsed.get("info"))
                        .and_then(|info| info.get("mint"))
                        .and_then(|mint| mint.as_str())
                    {
                        if let Some(balance) = account_data
                            .get("data")
                            .and_then(|data| data.get("parsed"))
                            .and_then(|parsed| parsed.get("info"))
                            .and_then(|info| info.get("tokenAmount"))
                            .and_then(|token_amount| token_amount.get("uiAmount"))
                            .and_then(|ui_amount| ui_amount.as_f64())
                        {
                            token_balances.insert(mint.to_string(), balance as u64);
                        }
                    }
                }
            }

            Ok(token_balances)
        }
    }
}

mod jupiter {
    use reqwest;
    use serde::{Deserialize, Serialize};
    use serde_json;

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct TokenPrice {
        pub contract_address: String,
        pub price: f64,
    }

    #[derive(Debug, Clone)]
    pub struct JupiterClient {
        client: reqwest::Client,
        url: String,
    }

    impl JupiterClient {
        pub fn new() -> Self {
            Self {
                client: reqwest::Client::new(),
                url: "https://api.jup.ag/".into(),
            }
        }

        pub async fn price(
            &self,
            tokens: Vec<String>,
        ) -> Result<Vec<TokenPrice>, Box<dyn std::error::Error>> {
            let mut token_prices: Vec<Vec<TokenPrice>> = Vec::new();

            let tokens_chunks = tokens
                .chunks(100)
                .map(|chunk| chunk.to_vec())
                .collect::<Vec<Vec<String>>>();

            for tokens_chunk in tokens_chunks {
                let url = format!(
                    "{}/price/v2?ids={}",
                    self.url,
                    tokens_chunk.join(","),
                );
                let response: serde_json::Value =
                    self.client.get(&url).send().await?.json().await?;
                let data = response
                    .get("data")
                    .ok_or("Missing 'data' field in response")?
                    .as_object()
                    .ok_or("'data' field is not an object")?;
                let prices = data
                    .iter()
                    .filter_map(|(contract_address, details)| {
                        let price = details.get("price")?.as_str()?.parse::<f64>().ok()?;
                        Some(TokenPrice {
                            contract_address: contract_address.clone(),
                            price,
                        })
                    })
                    .collect::<Vec<TokenPrice>>();

                token_prices.push(prices);
            }

            Ok(token_prices.concat())
        }
    }
}

use tokio;

#[tokio::main]
async fn main() {
    let config = config::Config::new().unwrap();
    let solana_client = solana::Solana::new(config.cluster);
    let jupiper_client = jupiter::JupiterClient::new();

    for wallet in config.wallets {
        let mut wallet_total_balance: f64 = 0.00;

        let wallet_sol = solana_client.get_balance(wallet.clone()).await.unwrap();
        let wallet_tokens = solana_client
            .get_token_accounts(wallet.clone())
            .await
            .unwrap();
        let sol_price = jupiper_client
            .price(
                vec![solana::SOL_CA.to_string()],
            )
            .await
            .unwrap()[0]
            .price;
        let wallet_tokens_prices = jupiper_client
            .price(
                Vec::from_iter(wallet_tokens.keys().cloned()),
            )
            .await
            .unwrap();

        wallet_total_balance += utils::lamports_to_sol(wallet_sol) * sol_price;

        for (wallet_token, wallet_token_amount) in wallet_tokens {
            let index = wallet_tokens_prices
                .iter()
                .position(|t| t.contract_address == wallet_token);

            if index.is_none() {
                continue;
            }

            wallet_total_balance +=
                (wallet_token_amount as f64) * { wallet_tokens_prices[index.unwrap()].price };
        }

        println!(
            "{} {:.2}$",
            wallet.public_key,
            wallet_total_balance
        );
    }
}
