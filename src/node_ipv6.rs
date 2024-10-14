use crate::redis::RedisClient;
use crate::{REDIS_KEY_ENR_IPV6, REDIS_KEY_ENR_KEY_IPV6, REDIS_KEY_FINISHED};
use discv5::enr::CombinedKey;
use discv5::{Discv5, Enr, ListenConfig};
use std::net::Ipv6Addr;

pub async fn run(mut redis: RedisClient) {
    let mut enr_key_bytes: Vec<u8> = redis.pop(REDIS_KEY_ENR_KEY_IPV6).await;
    let enr_key = CombinedKey::secp256k1_from_bytes(&mut enr_key_bytes).expect("valid enr key");
    let ip6 = Ipv6Addr::new(0x2001, 0x3984, 0x3989, 0x2000, 0, 0, 0, 0);
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

    redis.signal_and_wait(REDIS_KEY_FINISHED, 2).await;
    println!("finished");
}
