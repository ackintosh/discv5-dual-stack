use crate::redis::RedisClient;

mod node_dual_stack;
mod redis;
mod node_ipv6;

const REDIS_KEY_ENR_KEY_IPV6: &str = "ENR_KEY_IPV6";
const REDIS_KEY_ENR_IPV6: &str = "ENR_IPV6";
const REDIS_KEY_FINISHED: &str = "FINISHED";

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
    if args.len() != 2 {
        panic!(
            "Usage: {} <actor> \n <actor> possible values: node-a, node-b",
            args.first().unwrap()
        );
    }

    let redis = RedisClient::new().await;

    match args.get(1).unwrap().as_str() {
        "node-dual-stack" => node_dual_stack::run(redis).await,
        "node-ipv6" => node_ipv6::run(redis).await,
        _ => unreachable!(),
    }
}
