use crate::redis::RedisClient;

mod node_dual_stack;
mod node_ipv4;
mod node_ipv6;
mod redis;

const REDIS_KEY_ENR_KEY_IPV6: &str = "ENR_KEY_IPV6";
const REDIS_KEY_ENR_KEY_IPV4: &str = "ENR_KEY_IPV4";
const REDIS_KEY_ENR_IPV6: &str = "ENR_IPV6";
const REDIS_KEY_ENR_IPV4: &str = "ENR_IPV4";
const REDIS_KEY_STARTED: &str = "STARTED";
const REDIS_KEY_FINISHED: &str = "FINISHED";

const IPV6_NODES: u64 = 2;
const IPV4_NODES: u64 = 16;
const TOTAL_NODES: u64 = 19;

#[tokio::main]
async fn main() {
    // Enable tracing.
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .or_else(|_| tracing_subscriber::EnvFilter::try_new("info"))
        .expect("EnvFilter");
    let _ = tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .try_init();

    let args = std::env::args().collect::<Vec<_>>();
    // if args.len() != 2 {
    //     panic!(
    //         "Usage: {} <actor> \n <actor> possible values: node-a, node-b",
    //         args.first().unwrap()
    //     );
    // }

    let redis = RedisClient::new().await;

    match args.get(1).unwrap().as_str() {
        "node-dual-stack" => node_dual_stack::run(redis).await,
        "node-ipv6" => node_ipv6::run(redis).await,
        "node-ipv4" => node_ipv4::run(redis).await,
        _ => unreachable!(),
    }
}
