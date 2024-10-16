use crate::redis::RedisClient;
use crate::{
    REDIS_KEY_ENR_IPV4, REDIS_KEY_ENR_KEY_IPV4, REDIS_KEY_FINISHED, REDIS_KEY_STARTED, TOTAL_NODES,
};
use discv5::enr::CombinedKey;
use discv5::{Discv5, Enr, ListenConfig};
use std::net::{IpAddr, Ipv4Addr};
use tracing::debug;

pub async fn run(mut redis: RedisClient) {
    let ip4 = get_addr();
    let mut enr_key_bytes: Vec<u8> = redis.pop(REDIS_KEY_ENR_KEY_IPV4).await;
    let enr_key = CombinedKey::secp256k1_from_bytes(&mut enr_key_bytes).expect("valid enr key");
    let udp4 = 9000;
    let enr = Enr::builder()
        .ip4(ip4.clone())
        .udp4(udp4)
        .build(&enr_key)
        .expect("Construct local Enr");

    let mut discv5: Discv5 = Discv5::new(
        enr.clone(),
        enr_key,
        discv5::ConfigBuilder::new(ListenConfig::Ipv4 {
            ip: ip4,
            port: udp4,
        })
        .build(),
    )
    .unwrap();

    discv5.start().await.unwrap();

    redis.push(REDIS_KEY_ENR_IPV4, enr.clone()).await;

    redis.signal_and_wait(REDIS_KEY_STARTED, TOTAL_NODES).await;

    redis.signal_and_wait(REDIS_KEY_FINISHED, TOTAL_NODES).await;
    debug!("finished");
}

fn get_addr() -> Ipv4Addr {
    if_addrs::get_if_addrs()
        .unwrap()
        .into_iter()
        .filter_map(|interface| {
            if let IpAddr::V4(ipv4) = interface.ip() {
                Some(ipv4)
            } else {
                None
            }
        })
        .find(|ipv4| !ipv4.is_loopback() && !ipv4.is_multicast())
        .unwrap()
}
