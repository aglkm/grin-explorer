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
    // hashrate
    pub hashrate:        String,
    pub hashrate_kgs:    String,
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
            hashrate:        String::new(),
            hashrate_kgs:    String::new(),
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

impl Block {
    pub fn new() -> Block {
        Block {
            hash:     String::new(),
            height:   String::new(),
            time:     String::new(),
            version:  String::new(),
            weight:   0.0,
            size:     String::new(),
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
    pub external_nodes:          Vec<String>,
}

impl ExplorerConfig {
    pub fn new() -> ExplorerConfig {
        ExplorerConfig {
            host:                    String::new(),
            port:                    String::new(),
            proto:                   String::new(),
            user:                    String::new(),
            api_secret_path:         String::new(),
            foreign_api_secret_path: String::new(),
            grin_dir:                String::new(),
            api_secret:              String::new(),
            foreign_api_secret:      String::new(),
            coingecko_api:           String::new(),
            public_api:              String::new(),
            external_nodes:          Vec::new(),
        }
    }
}


// Output data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Output {
    pub height:   String,
    pub commit:   String,
    pub out_type: String,
    pub status:   String,
    pub raw_data: String,
}

impl Output {
    pub fn new() -> Output {
        Output {
            height:   String::new(),
            commit:   String::new(),
            out_type: String::new(),
            status:   String::new(),
            raw_data: String::new(),
        }
    }
}


// Statistics data
#[derive(Debug, Serialize)]
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
}

impl Statistics {
    pub fn new() -> Statistics {
        Statistics {
            date:       Vec::new(),
            user_agent: Vec::new(),
            count:      Vec::new(),
            total:      0,
            hashrate:   Vec::new(),
            txns:       Vec::new(),
            fees:       Vec::new(),
        }
    }
}

