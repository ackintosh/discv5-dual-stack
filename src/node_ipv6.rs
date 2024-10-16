use crate::redis::RedisClient;
use crate::{
    REDIS_KEY_ENR_IPV6, REDIS_KEY_ENR_KEY_IPV6, REDIS_KEY_FINISHED, REDIS_KEY_STARTED, TOTAL_NODES,
};
use discv5::enr::CombinedKey;
use discv5::{Discv5, Enr, ListenConfig};
use std::net::{IpAddr, Ipv6Addr};
use tracing::debug;

pub async fn run(mut redis: RedisClient) {
    let mut enr_key_bytes: Vec<u8> = redis.pop(REDIS_KEY_ENR_KEY_IPV6).await;
    let enr_key = CombinedKey::secp256k1_from_bytes(&mut enr_key_bytes).expect("valid enr key");
    let ip6 = get_addr();
    let udp6 = 9000;
    let enr = Enr::builder()
        .ip6(ip6)
        .udp6(udp6)
        .build(&enr_key)
        .expect("Construct local Enr");
    let mut discv5: Discv5 = Discv5::new(
        enr.clone(),
        enr_key,
        discv5::ConfigBuilder::new(ListenConfig::Ipv6 {
            ip: ip6,
            port: udp6,
        })
        .build(),
    )
    .unwrap();

    discv5.start().await.unwrap();

    redis.push(REDIS_KEY_ENR_IPV6, enr).await;

    redis.signal_and_wait(REDIS_KEY_STARTED, TOTAL_NODES).await;

    redis.signal_and_wait(REDIS_KEY_FINISHED, TOTAL_NODES).await;
    debug!("finished");
}

fn get_addr() -> Ipv6Addr {
    if_addrs::get_if_addrs()
        .unwrap()
        .into_iter()
        .filter_map(|interface| {
            if let IpAddr::V6(ipv6) = interface.ip() {
                Some(ipv6)
            } else {
                None
            }
        })
        .find(|ipv6| !ipv6.is_loopback() && !ipv6.is_multicast())
        .unwrap()
}
