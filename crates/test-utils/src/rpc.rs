//! RPC API keys utilities.

use foundry_config::NamedChain::{
    self, Arbitrum, Base, BinanceSmartChainTestnet, Celo, Mainnet, Optimism, Polygon, Sepolia,
};
use rand::seq::SliceRandom;
use std::{
    env,
    sync::{
        LazyLock,
        atomic::{AtomicUsize, Ordering},
    },
};

macro_rules! shuffled_list {
    ($name:ident, $e:expr $(,)?) => {
        static $name: LazyLock<ShuffledList<&'static str>> =
            LazyLock::new(|| ShuffledList::new($e));
    };
}

struct ShuffledList<T> {
    list: Vec<T>,
    index: AtomicUsize,
}

impl<T> ShuffledList<T> {
    fn new(mut list: Vec<T>) -> Self {
        assert!(!list.is_empty());
        list.shuffle(&mut rand::rng());
        Self { list, index: AtomicUsize::new(0) }
    }

    fn next(&self) -> &T {
        let index = self.index.fetch_add(1, Ordering::Relaxed);
        &self.list[index % self.list.len()]
    }
}

// List of general purpose DRPC keys to rotate through
static DRPC_KEYS: LazyLock<Vec<String>> = LazyLock::new(|| {
    let mut keys = vec!["AgasqIYODEW_j_J0F91L8oETmhtHCXkR8JAVssvAG40d".to_owned()];
    // Fetch secret from GitHub Actions environment variable
    if let Ok(secret) = std::env::var("DLRP_API_KEY") {
        keys.clear();
        keys.push(secret);
    }

    keys.shuffle(&mut rand::rng());

    keys
});

shuffled_list!(
    HTTP_ARCHIVE_DOMAINS,
    vec![
        //
        "ethereum.reth.rs",
    ]
);
shuffled_list!(
    HTTP_DOMAINS,
    vec![
        //
        "reth-ethereum.ithaca.xyz/rpc",
        // "reth-ethereum-full.ithaca.xyz/rpc",
    ],
);
shuffled_list!(
    WS_ARCHIVE_DOMAINS,
    vec![
        //
        "reth-ethereum.ithaca.xyz/ws",
    ],
);
shuffled_list!(
    WS_DOMAINS,
    vec![
        //
        "reth-ethereum.ithaca.xyz/ws",
        // "reth-ethereum-full.ithaca.xyz/ws",
    ],
);

/// Returns the fallback hardcoded Etherscan API keys.
fn fallback_etherscan_keys() -> Vec<String> {
    vec![
        "MCAUM7WPE9XP5UQMZPCKIBUJHPM1C24FP6".to_string(),
        "JW6RWCG2C5QF8TANH4KC7AYIF1CX7RB5D1".to_string(),
        "ZSMDY6BI2H55MBE3G9CUUQT4XYUDBB6ZSK".to_string(),
        "4FYHTY429IXYMJNS4TITKDMUKW5QRYDX61".to_string(),
        "QYKNT5RHASZ7PGQE68FNQWH99IXVTVVD2I".to_string(),
        "VXMQ117UN58Y4RHWUB8K1UGCEA7UQEWK55".to_string(),
        "C7I2G4JTA5EPYS42Z8IZFEIMQNI5GXIJEV".to_string(),
        "A15KZUMZXXCK1P25Y1VP1WGIVBBHIZDS74".to_string(),
        "3IA6ASNQXN8WKN7PNFX7T72S9YG56X9FPG".to_string(),
        "ZUB97R31KSYX7NYVW6224Q6EYY6U56H591".to_string(),
    ]
}

// List of etherscan keys.
static ETHERSCAN_KEYS: LazyLock<Vec<String>> = LazyLock::new(|| {
    // Fetch from GitHub Actions environment variable (comma-separated) or use fallback
    let mut keys = std::env::var("ETHERSCAN_API_KEYS")
        .ok()
        .map(|env_keys| env_keys.split(',').map(|s| s.trim().to_string()).collect::<Vec<String>>())
        .filter(|keys| !keys.is_empty())
        .unwrap_or_else(fallback_etherscan_keys);

    keys.shuffle(&mut rand::rng());
    keys
});

/// Returns the next index to use.
fn next_idx() -> usize {
    static NEXT_INDEX: AtomicUsize = AtomicUsize::new(0);
    NEXT_INDEX.fetch_add(1, Ordering::SeqCst)
}

/// Returns the next item in the list to use.
fn next<T>(list: &[T]) -> &T {
    &list[next_idx() % list.len()]
}

/// Returns the next _mainnet_ rpc URL in inline
///
/// This will rotate all available rpc endpoints
pub fn next_http_rpc_endpoint() -> String {
    next_rpc_endpoint(NamedChain::Mainnet)
}

/// Returns the next _mainnet_ rpc URL in inline
///
/// This will rotate all available rpc endpoints
pub fn next_ws_rpc_endpoint() -> String {
    next_ws_endpoint(NamedChain::Mainnet)
}

/// Returns the next HTTP RPC URL.
pub fn next_rpc_endpoint(chain: NamedChain) -> String {
    next_url(false, chain)
}

/// Returns the next WS RPC URL.
pub fn next_ws_endpoint(chain: NamedChain) -> String {
    next_url(true, chain)
}

/// Returns a websocket URL that has access to archive state
pub fn next_http_archive_rpc_url() -> String {
    next_archive_url(false)
}

/// Returns an HTTP URL that has access to archive state
pub fn next_ws_archive_rpc_url() -> String {
    next_archive_url(true)
}

/// Returns a URL that has access to archive state.
fn next_archive_url(is_ws: bool) -> String {
    let domain = if is_ws { &WS_ARCHIVE_DOMAINS } else { &HTTP_ARCHIVE_DOMAINS }.next();
    let url = if is_ws { format!("wss://{domain}") } else { format!("https://{domain}") };
    test_debug!("next_archive_url(is_ws={is_ws}) = {}", debug_url(&url));
    url
}

/// Returns the next etherscan api key.
pub fn next_etherscan_api_key() -> String {
    let key = next(&ETHERSCAN_KEYS).clone();
    eprintln!("--- next_etherscan_api_key() = {key} ---");
    key
}

fn next_url(is_ws: bool, chain: NamedChain) -> String {
    let url = next_url_inner(is_ws, chain);
    test_debug!("next_url(is_ws={is_ws}, chain={chain:?}) = {}", debug_url(&url));
    url
}

fn next_url_inner(is_ws: bool, chain: NamedChain) -> String {
    if matches!(chain, Base) {
        return "https://mainnet.base.org".to_string();
    }

    if matches!(chain, Optimism) {
        return "https://mainnet.optimism.io".to_string();
    }

    if matches!(chain, BinanceSmartChainTestnet) {
        return "https://bsc-testnet-rpc.publicnode.com".to_string();
    }

    if matches!(chain, Celo) {
        return "https://celo.drpc.org".to_string();
    }

    if matches!(chain, Arbitrum) {
        let rpc_url = env::var("ARBITRUM_RPC").unwrap_or_default();
        if !rpc_url.is_empty() {
            return rpc_url;
        }
    }

    let reth_works = true;
    let domain = if reth_works && matches!(chain, Mainnet) {
        *(if is_ws { &WS_DOMAINS } else { &HTTP_DOMAINS }).next()
    } else {
        // DRPC for other networks used in tests.
        let idx = next_idx() % DRPC_KEYS.len();
        let key = &DRPC_KEYS[idx];

        let network = match chain {
            Mainnet => "ethereum",
            Polygon => "polygon",
            Arbitrum => "arbitrum",
            Sepolia => "sepolia",
            _ => "",
        };
        &format!("lb.drpc.org/ogrpc?network={network}&dkey={key}")
    };

    if is_ws { format!("wss://{domain}") } else { format!("https://{domain}") }
}

/// Basic redaction for debugging RPC URLs.
fn debug_url(url: &str) -> impl std::fmt::Display + '_ {
    let url = reqwest::Url::parse(url).unwrap();
    format!(
        "{scheme}://{host}{path}",
        scheme = url.scheme(),
        host = url.host_str().unwrap(),
        path = url.path().get(..8).unwrap_or(url.path()),
    )
}

#[cfg(test)]
#[expect(clippy::disallowed_macros)]
mod tests {
    use super::*;
    use alloy_primitives::address;
    use foundry_config::Chain;

    #[tokio::test]
    #[ignore = "run manually"]
    async fn test_etherscan_keys() {
        let address = address!("0xdAC17F958D2ee523a2206206994597C13D831ec7");
        let mut first_abi = None;
        let mut failed = Vec::new();
        for (i, key) in ETHERSCAN_KEYS.iter().enumerate() {
            println!("trying key {i} ({key})");

            let client = foundry_block_explorers::Client::builder()
                .chain(Chain::mainnet())
                .unwrap()
                .with_api_key(key)
                .build()
                .unwrap();

            let mut fail = |e: &str| {
                eprintln!("key {i} ({key}) failed: {e}");
                failed.push(key.as_str());
            };

            let abi = match client.contract_abi(address).await {
                Ok(abi) => abi,
                Err(e) => {
                    fail(&e.to_string());
                    continue;
                }
            };

            if let Some(first_abi) = &first_abi {
                if abi != *first_abi {
                    fail("abi mismatch");
                }
            } else {
                first_abi = Some(abi);
            }
        }
        if !failed.is_empty() {
            panic!("failed keys: {failed:#?}");
        }
    }
}
