// sorry for shitcode lol

use {
    futures::{sink::SinkExt, stream::StreamExt},
    log::info,
    solana_sdk::{self, signer::Signer},
    std::{env, str::FromStr},
    tokio::{task, time::{interval, Duration}},
    tonic::transport::channel::ClientTlsConfig,
    yellowstone_grpc_client::GeyserGrpcClient,
    yellowstone_grpc_proto::prelude::{
        subscribe_update::UpdateOneof, CommitmentLevel, SubscribeRequest,
        SubscribeRequestFilterSlots, SubscribeRequestPing, SubscribeUpdatePong,
        SubscribeUpdateSlot,
    },
};

#[derive(serde::Deserialize, Clone)]
struct YellowstoneConfig {
    endpoint: String,
    x_token: String,
}

#[derive(serde::Deserialize, Clone)]
struct Config {
    yellowstone: YellowstoneConfig,
    wallet: String,
    recipient: String,
    amount: u64,
    cluster: String,
}

impl Config {
    pub fn new() -> Self {
        let content = std::fs::read_to_string("config.yaml").expect("Failed to read config file");
        serde_yaml::from_str(&content).expect("Invalid config format")
    }
}

fn parse_private_key(private_key: &str) -> solana_sdk::signature::Keypair {
    let keypair = solana_sdk::signature::Keypair::from_base58_string(private_key);
    // keypair.expect("Invalid private key format")
    keypair
}

async fn send_transaction(
    client: &solana_client::rpc_client::RpcClient,
    sender: &solana_sdk::signature::Keypair,
    recipient: &solana_sdk::pubkey::Pubkey,
    amount: u64,
) -> solana_sdk::signature::Signature {
    let recent_blockhash = client
        .get_latest_blockhash()
        .expect("Failed to get blockhash");

    let tx = solana_sdk::transaction::Transaction::new_signed_with_payer(
        &[solana_sdk::system_instruction::transfer(
            &sender.pubkey(),
            recipient,
            amount,
        )],
        Some(&sender.pubkey()),
        &[sender],
        recent_blockhash,
    );

    client
        .send_and_confirm_transaction(&tx)
        .expect("Failed to send transaction")
}

pub fn lamports_to_sol(lamports: u64) -> f64 {
    (lamports as f64) / 1_000_000_000.00
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env::set_var(
        env_logger::DEFAULT_FILTER_ENV,
        env::var_os(env_logger::DEFAULT_FILTER_ENV).unwrap_or_else(|| "info".into()),
    );
    env_logger::init();

    let config = Config::new();

    let rpc_url = match config.cluster.as_str() {
        "mainnet" => "https://api.mainnet-beta.solana.com",
        "testnet" => "https://api.testnet.solana.com",
        "devnet" => "https://api.devnet.solana.com",
        _ => panic!("Unsupported cluster: {}", config.cluster),
    };

    let recipient_pubkey =
        solana_sdk::pubkey::Pubkey::from_str(&config.recipient).expect("Invalid recipient address");

    let mut client = GeyserGrpcClient::build_from_shared(config.yellowstone.endpoint)?
        .x_token(Some(config.yellowstone.x_token.clone()))?
        .tls_config(ClientTlsConfig::new().with_native_roots())?
        .connect()
        .await?;
    let (mut subscribe_tx, mut stream) = client.subscribe().await?;

    futures::try_join!(
        async move {
            subscribe_tx
            .send(SubscribeRequest {
                slots: maplit::hashmap! { "".to_owned() => SubscribeRequestFilterSlots { filter_by_commitment: Some(true) } },
                commitment: Some(CommitmentLevel::Processed as i32),
                ..Default::default()
            })
            .await?;

            let mut timer = interval(Duration::from_secs(3));
            let mut id = 0;
            loop {
                timer.tick().await;
                id += 1;
                subscribe_tx
                    .send(SubscribeRequest {
                        ping: Some(SubscribeRequestPing { id }),
                        ..Default::default()
                    })
                    .await?;
            }
            #[allow(unreachable_code)]
            Ok::<(), anyhow::Error>(())
        },
        async move {
            while let Some(message) = stream.next().await {
                match message?.update_oneof.expect("valid message") {
                    UpdateOneof::Slot(SubscribeUpdateSlot { slot, .. }) => {
                        info!("slot received: {slot}");

                        task::spawn(async move {
                            let config = Config::new();

                            let solana_client = solana_client::rpc_client::RpcClient::new(rpc_url.to_string());
                            let keypair = parse_private_key(&config.wallet);

                            info!(
                                "sending {:.2} SOL to {}",
                                lamports_to_sol(config.amount),
                                recipient_pubkey,
                            );
                            let signature = send_transaction(
                                &solana_client,
                                &keypair,
                                &recipient_pubkey,
                                config.amount,
                            )
                            .await;
                            info!("sended! tx hash: {}", signature.to_string());
                        });

                    }
                    UpdateOneof::Ping(_msg) => {
                        info!("ping received");
                    }
                    UpdateOneof::Pong(SubscribeUpdatePong { id }) => {
                        info!("pong received: id#{id}");
                    }
                    msg => anyhow::bail!("received unexpected message: {msg:?}"),
                }
            }
            Ok::<(), anyhow::Error>(())
        }
    )?;

    Ok(())
}
