use serde::{Serialize, Deserialize};


// Dashboard data 
#[derive(Debug)]
pub struct Dashboard {
    // status
    pub height:          String,
    pub sync:            String,
    pub node_ver:        String,
    pub proto_ver:       String,
    // connections
    pub inbound:         u16,
    pub outbound:        u16,
    //price & market
    pub supply:          String,
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
    pub age:             String,
    // hashrate
    pub hashrate:        String,
    pub difficulty:      String,
    // mining
    pub production_cost: String,
    pub reward_ratio:    String,
    pub breakeven_cost:  String,
    // mempool
    pub txns:            String,
    pub stem:            String,
}

impl Dashboard {
    pub fn new() -> Dashboard {
        Dashboard {
            height:          String::new(),
            sync:            String::new(),
            node_ver:        String::new(),
            proto_ver:       String::new(),
            inbound:         0,
            outbound:        0,
            supply:          String::new(),
            soft_supply:     String::new(),
            inflation:       String::new(),
            price_usd:       String::new(),
            price_btc:       String::new(),
            volume_usd:      String::new(),
            volume_btc:      String::new(),
            cap_usd:         String::new(),
            cap_btc:         String::new(),
            disk_usage:      String::new(),
            age:             String::new(),
            hashrate:        String::new(),
            difficulty:      String::new(),
            production_cost: String::new(),
            reward_ratio:    String::new(),
            breakeven_cost:  String::new(),
            txns:            String::new(),
            stem:            String::new(),
        }
    }
}


// Block data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub hash:     String,
    pub height:   String,
    pub time:     String,
    pub version:  String,
    pub weight:   f64,
    pub fees:     f64,
    pub kernels:  Vec<(String, String, String)>,
    pub inputs:   Vec<String>,
    pub outputs:  Vec<(String,String)>,
    pub ker_len:  u64,
    pub in_len:   u64,
    pub out_len:  u64,
    pub raw_data: String,
}

impl Block {
    pub fn new() -> Block {
        Block {
            hash:     String::new(),
            height:   String::new(),
            time:     String::new(),
            version:  String::new(),
            weight:   0.0,
            fees:     0.0,
            kernels:  Vec::new(),
            inputs:   Vec::new(),
            outputs:  Vec::new(),
            ker_len:  0,
            in_len:   0,
            out_len:  0,
            raw_data: String::new(),
        }
    }
}


// Kernel data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Kernel {
    pub height:   String,
    pub excess:   String,
    pub ker_type: String,
    pub fee:      String,
    pub status:   String,
    pub raw_data: String,
}

impl Kernel {
    pub fn new() -> Kernel {
        Kernel {
            height:   String::new(),
            excess:   String::new(),
            ker_type: String::new(),
            fee:      String::new(),
            status:   String::new(),
            raw_data: String::new(),
        }
    }
}


// Transactions data
#[derive(Debug)]
pub struct Transactions {
    pub period_1h:  String,
    pub period_24h: String,
    pub fees_1h:    String,
    pub fees_24h:   String,
}

impl Transactions {
    pub fn new() -> Transactions {
        Transactions {
            period_1h:  String::new(),
            period_24h: String::new(),
            fees_1h:    String::new(),
            fees_24h:   String::new(),
        }
    }
}


// Explorer configuration
#[derive(Debug)]
pub struct ExplorerConfig {
    pub ip:                      String,
    pub port:                    String,
    pub proto:                   String,
    pub user:                    String,
    pub api_secret_path:         String,
    pub foreign_api_secret_path: String,
    pub grin_dir:                String,
    pub api_secret:              String,
    pub foreign_api_secret:      String,
    pub coingecko_api:           String,
}

impl ExplorerConfig {
    pub fn new() -> ExplorerConfig {
        ExplorerConfig {
            ip:                      String::new(),
            port:                    String::new(),
            proto:                   String::new(),
            user:                    String::new(),
            api_secret_path:         String::new(),
            foreign_api_secret_path: String::new(),
            grin_dir:                String::new(),
            api_secret:              String::new(),
            foreign_api_secret:      String::new(),
            coingecko_api:           String::new(),
        }
    }
}

