use crate::redis::RedisClient;
use discv5::enr::CombinedKey;
use discv5::{enr, Discv5, Enr, ListenConfig};
use rand::{RngCore, SeedableRng};
use std::net::{Ipv4Addr, Ipv6Addr};
use crate::{REDIS_KEY_ENR_IPV6, REDIS_KEY_ENR_KEY_IPV6, REDIS_KEY_FINISHED};

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

    let ip4 = Ipv4Addr::new(172, 16, 238, 10);
    let udp4 = 9000;
    let ip6 = Ipv6Addr::new(0x2001, 0x3984, 0x3989, 0x1000, 0, 0, 0, 0);
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
        .build(),
    )
    .unwrap();

    loop {
        let rand_enr = Enr::builder()
            .ip4(generate_rand_ipv4())
            .udp4(9000)
            .build(&keypairs.remove(0))
            .expect("Construct local Enr");
        let result = discv5.add_enr(rand_enr);
        if matches!(result, Err("Table full")) {
            break;
        }
    }

    redis.push(REDIS_KEY_ENR_KEY_IPV6, keypairs.remove(0).encode()).await;

    discv5.start().await.unwrap();

    let node_ipv6_enr = redis.pop(REDIS_KEY_ENR_IPV6).await;

    let result = discv5.find_node_designated_peer(node_ipv6_enr, vec![253, 254, 255]).await;
    println!("result: {result:?}");

    redis.signal_and_wait(REDIS_KEY_FINISHED, 2).await;
    println!("finished");
}

fn generate_rand_ipv4() -> Ipv4Addr {
    let a: u8 = rand::random();
    let b: u8 = rand::random();
    let c: u8 = rand::random();
    let d: u8 = rand::random();
    Ipv4Addr::new(a, b, c, d)
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
