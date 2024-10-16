use crate::redis::RedisClient;
use crate::{
    IPV4_NODES, IPV6_NODES, REDIS_KEY_ENR_IPV4, REDIS_KEY_ENR_IPV6, REDIS_KEY_ENR_KEY_IPV4,
    REDIS_KEY_ENR_KEY_IPV6, REDIS_KEY_FINISHED, REDIS_KEY_STARTED, TOTAL_NODES,
};
use discv5::enr::CombinedKey;
use discv5::{enr, Discv5, Enr, ListenConfig};
use rand::{RngCore, SeedableRng};
use std::net::{Ipv4Addr, Ipv6Addr};
use tracing::debug;

pub async fn run(mut redis: RedisClient) {
    // Generate 20 key pairs. Distances between the first key pair and all other ones are the
    // same. So in the node with the first key pair, node ids given from the other ones will be
    // inserted into the same bucket.
    //
    // The `122488` seed is a pre-computed one for this function. See `find_seed_same_bucket()`
    // in https://github.com/sigp/discv5/blob/master/src/discv5/test.rs for more details of the
    // pre-computing.
    let mut keypairs = generate_deterministic_keypair(20, 122488);
    let enr_key = keypairs.remove(0);

    // ////////////////////////////////////////////////////////////////////////
    // Publish ENR keys for other nodes.
    // ////////////////////////////////////////////////////////////////////////
    // IPv6
    for _ in 0..IPV6_NODES {
        redis
            .push(REDIS_KEY_ENR_KEY_IPV6, keypairs.remove(0).encode())
            .await;
    }
    // IPv4
    for _ in 0..IPV4_NODES {
        redis
            .push(REDIS_KEY_ENR_KEY_IPV4, keypairs.remove(0).encode())
            .await;
    }

    let ip4 = Ipv4Addr::new(172, 16, 238, 10);
    let udp4 = 9000;
    let ip6 = Ipv6Addr::new(0x2001, 0x3984, 0x3989, 0, 0, 0, 0, 0x0010);
    let udp6 = 9000;
    let enr = Enr::builder()
        .ip4(ip4)
        .udp4(udp4)
        .ip6(ip6)
        .udp6(udp6)
        .build(&enr_key)
        .expect("Construct local Enr");
    let mut discv5: Discv5 = Discv5::new(
        enr,
        enr_key,
        discv5::ConfigBuilder::new(ListenConfig::DualStack {
            ipv4: ip4,
            ipv4_port: udp4,
            ipv6: ip6,
            ipv6_port: udp6,
        })
        // Set the minimum number to IPV6_NODES to trigger the ENR update.
        .enr_peer_update_min(IPV6_NODES as usize)
        .build(),
    )
    .unwrap();

    discv5.start().await.unwrap();

    tokio::time::sleep(std::time::Duration::from_secs(3)).await;

    redis.signal_and_wait(REDIS_KEY_STARTED, TOTAL_NODES).await;

    // ////////////////////////////////////////////////////////////////////////
    // Sending FINDENODE to IPv4 nodes.
    // ////////////////////////////////////////////////////////////////////////
    debug!("Sending FINDENODE to IPv4 nodes.");
    for _ in 0..IPV4_NODES {
        let ipv4_enr: Enr = redis.pop(REDIS_KEY_ENR_IPV4).await;
        discv5
            .find_node_designated_peer(ipv4_enr, vec![0])
            .await
            .unwrap();
    }

    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    let nums = discv5
        .kbuckets()
        .buckets_iter()
        .map(|bucket| bucket.num_connected())
        .collect::<Vec<_>>();
    debug!("connected_nodes: {nums:?}");
    let nums = discv5
        .kbuckets()
        .buckets_iter()
        .map(|bucket| bucket.num_disconnected())
        .collect::<Vec<_>>();
    debug!("disconnected_nodes: {nums:?}");

    // ////////////////////////////////////////////////////////////////////////
    // Sending FINDENODE to IPv6 nodes.
    // ////////////////////////////////////////////////////////////////////////
    debug!("Sending FINDENODE to IPv6 nodes.");
    for _ in 0..IPV6_NODES {
        let node_ipv6_enr: Enr = redis.pop(REDIS_KEY_ENR_IPV6).await;
        let _result = discv5
            .find_node_designated_peer(node_ipv6_enr, vec![253, 254, 255])
            .await
            .unwrap();
    }

    tokio::time::sleep(std::time::Duration::from_secs(10)).await;

    redis.signal_and_wait(REDIS_KEY_FINISHED, TOTAL_NODES).await;
    debug!("finished");
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    redis.remove(REDIS_KEY_ENR_KEY_IPV6).await;
    redis.remove(REDIS_KEY_ENR_KEY_IPV4).await;
    redis.remove(REDIS_KEY_ENR_IPV6).await;
    redis.remove(REDIS_KEY_ENR_IPV4).await;
    redis.remove(REDIS_KEY_STARTED).await;
    redis.remove(REDIS_KEY_FINISHED).await;
}

// This function is copied from https://github.com/sigp/discv5/blob/master/src/discv5/test.rs
// Generate `n` deterministic keypairs from a given seed.
fn generate_deterministic_keypair(n: usize, seed: u64) -> Vec<CombinedKey> {
    let mut keypairs = Vec::new();
    for i in 0..n {
        let sk = {
            let rng = &mut rand_xorshift::XorShiftRng::seed_from_u64(seed + i as u64);
            let mut b = [0; 32];
            loop {
                // until a value is given within the curve order
                rng.fill_bytes(&mut b);
                if let Ok(k) = enr::k256::ecdsa::SigningKey::from_slice(&b) {
                    break k;
                }
            }
        };
        let kp = CombinedKey::from(sk);
        keypairs.push(kp);
    }
    keypairs
}
