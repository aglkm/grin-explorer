use serde::{Serialize, Deserialize};

    
// Weights
pub const KERNEL_WEIGHT: f64 = 3.0;
pub const INPUT_WEIGHT:  f64 = 1.0;
pub const OUTPUT_WEIGHT: f64 = 21.0;

    
// Sizes in bytes
pub const KERNEL_SIZE: u64 = 1 + 8 + 8 + 33 + 64;
pub const INPUT_SIZE:  u64 = 1 + 33;
pub const OUTPUT_SIZE: u64 = 674 + 33 + 1;


// Dashboard data 
#[derive(Debug, Default)]
pub struct Dashboard {
    // status
    pub chain:           String,
    pub height:          String,
    pub sync:            String,
    pub node_ver:        String,
    pub proto_ver:       String,
    // connections
    pub inbound:         u16,
    pub outbound:        u16,
    // price & market
    pub supply:          String,
    pub supply_raw:      String,
    pub soft_supply:     String,
    pub inflation:       String,
    pub price_usd:       String,
    pub price_btc:       String,
    pub volume_usd:      String,
    pub volume_btc:      String,
    pub cap_usd:         String,
    pub cap_btc:         String,
    // blockchain
    pub disk_usage:      String,
    // hashrate
    pub hashrate:        String,
    pub hashrate_kgs:    String,
    pub difficulty:      String,
    // Breakeven electricity cost
    pub breakeven_g1:    String,
    pub breakeven_g1mini: String,
    // Hourly production
    pub prod_g1:         f64,
    pub prod_g1mini:     f64,
    // mempool
    pub txns:            String,
    pub stem:            String,
    // utxo
    pub utxo_count:      String,
    // kernel
    pub kernel_mmr_size: String,
}


// Block data
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Block {
    pub hash:     String,
    pub height:   String,
    pub time:     String,
    pub version:  String,
    pub weight:   f64,
    pub size:     String,
    pub fees:     f64,
    pub kernels:  Vec<(String, String, String)>,
    pub inputs:   Vec<String>,
    pub outputs:  Vec<(String,String)>,
    pub ker_len:  u64,
    pub in_len:   u64,
    pub out_len:  u64,
    pub raw_data: String,
}


// Kernel data
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Kernel {
    pub height:   String,
    pub excess:   String,
    pub ker_type: String,
    pub fee:      String,
    pub status:   String,
    pub raw_data: String,
}


// Transactions data
#[derive(Debug, Default)]
pub struct Transactions {
    pub period_1h:  String,
    pub period_24h: String,
    pub fees_1h:    String,
    pub fees_24h:   String,
}


// Explorer configuration
#[derive(Debug, Default)]
pub struct ExplorerConfig {
    pub host:                    String,
    pub port:                    String,
    pub proto:                   String,
    pub user:                    String,
    pub api_secret_path:         String,
    pub foreign_api_secret_path: String,
    pub grin_dir:                String,
    pub api_secret:              String,
    pub foreign_api_secret:      String,
    pub coingecko_api:           String,
    pub public_api:              String,
    pub stats_source:            Vec<String>,
    pub public_nodes:            Vec<String>,
    pub database:                String,
}


// Output data
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Output {
    pub height:   String,
    pub commit:   String,
    pub out_type: String,
    pub status:   String,
    pub raw_data: String,
}


// Statistics data
#[derive(Debug, Default, Serialize)]
pub struct Statistics {
    pub date:       Vec<String>,
    // Node versions
    pub user_agent: Vec<String>,
    pub count:      Vec<String>,
    pub total:      u32,
    // Hashrate
    pub hashrate:   Vec<String>,
    // Transactions & fees
    pub txns:       Vec<String>,
    pub fees:       Vec<String>,
    // UTXOs
    pub utxos: Vec<String>,
    // Kernels
    pub kernels:    Vec<String>,
}


// Public node data
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PublicNode {
    pub name:    String,
    pub version: String,
    pub height:  String,
    pub hash:    String,
}


// Connected node data
#[derive(PartialEq, Eq, Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConnectedNode {
    pub address:     String,
    pub user_agent:  String,
    pub bits:        u32,
    pub is_archival: bool,
    pub location:    String,
    pub flag:        String,
    pub isp:         String,
}


// Network data
#[derive(Debug, Default, Serialize)]
pub struct NetStats {
    pub pub_nodes:   Vec<PublicNode>,
    pub reach_nodes: Vec<ConnectedNode>,
    pub conn_nodes:  Vec<ConnectedNode>,
}

