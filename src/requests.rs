use reqwest::Error;
use serde_json::Value;
use std::sync::{Arc, Mutex};
use num_format::{Locale, ToFormattedString};
use fs_extra::dir::get_size;
use colored::Colorize;
use humantime::format_duration;
use std::time::Duration;
use chrono::{Utc, DateTime};
use config::Config;
use std::collections::HashMap;
use std::fs;
use lazy_static::lazy_static;

use crate::data::Dashboard;
use crate::data::Block;
use crate::data::Transactions;
use crate::data::ExplorerConfig;


// Static explorer config structure
lazy_static! {
    static ref CONFIG: ExplorerConfig = {
        let mut cfg  = ExplorerConfig::new();
        let settings = Config::builder().add_source(config::File::with_name("Explorer"))
                                        .build().unwrap();

        let settings: HashMap<String, String> = settings.try_deserialize().unwrap();

        for (name, value) in settings {
            match name.as_str() {
                "ip"                      => cfg.ip                      = value,
                "port"                    => cfg.port                    = value,
                "proto"                   => cfg.proto                   = value,
                "user"                    => cfg.user                    = value,
                "api_secret_path"         => cfg.api_secret_path         = value,
                "foreign_api_secret_path" => cfg.foreign_api_secret_path = value,
                _ => println!("{} Unknown config setting '{}'.", "[ ERROR   ]".red(), name),
            }
        }

        cfg.api_secret         = fs::read_to_string(format!("{}",
                                     shellexpand::tilde(&cfg.api_secret_path))).unwrap();
        cfg.foreign_api_secret = fs::read_to_string(format!("{}",
                                     shellexpand::tilde(&cfg.foreign_api_secret_path))).unwrap();

        cfg
    };
}


// RPC requests to grin node.
async fn call(method: &str, params: &str, rpc_type: &str) -> Result<Value, Error> {
    let rpc_url;
    let secret;

    if rpc_type == "owner" {
        rpc_url = format!("{}://{}:{}/v2/owner", CONFIG.proto, CONFIG.ip, CONFIG.port);
        secret  = CONFIG.api_secret.clone();
    }
    else {
        rpc_url = format!("{}://{}:{}/v2/foreign", CONFIG.proto, CONFIG.ip, CONFIG.port);
        secret  = CONFIG.foreign_api_secret.clone();
    }

    let client = reqwest::Client::new();
    let result = client.post(rpc_url)
                       .body(format!("{{\"method\": \"{}\", \"params\": {}, \"id\":1}}", method, params))
                       .basic_auth(CONFIG.user.clone(), Some(secret))
                       .header("content-type", "plain/text")
                       .send()
                       .await?;

    let val: Value = serde_json::from_str(&result.text().await.unwrap()).unwrap();

    Ok(val)
}


// Collecting: height, sync, node_ver, proto_ver.
pub async fn get_status(dashboard: Arc<Mutex<Dashboard>>) -> Result<(), Error> {
    let resp = call("get_status", "[]", "owner").await?;

    let mut data = dashboard.lock().unwrap();

    if resp != Value::Null {
        data.height    = resp["result"]["Ok"]["tip"]["height"].to_string();
        data.sync      = resp["result"]["Ok"]["sync_status"].as_str().unwrap().to_string();
        data.node_ver  = resp["result"]["Ok"]["user_agent"].as_str().unwrap().to_string();
        data.proto_ver = resp["result"]["Ok"]["protocol_version"].to_string();
    }

    Ok(())
}


// Collecting: txns, stem.
pub async fn get_mempool(dashboard: Arc<Mutex<Dashboard>>) -> Result<(), Error> {
    let resp1 = call("get_pool_size", "[]", "foreign").await?;
    let resp2 = call("get_stempool_size", "[]", "foreign").await?;
    
    let mut data = dashboard.lock().unwrap();

    if resp1 != Value::Null && resp1 != Value::Null {
        data.txns = resp1["result"]["Ok"].to_string();
        data.stem = resp2["result"]["Ok"].to_string();
    }

    Ok(())
}


// Collecting: inbound, outbound.
pub async fn get_connected_peers(dashboard: Arc<Mutex<Dashboard>>) -> Result<(), Error> {
    let resp = call("get_connected_peers", "[]", "owner").await?;

    let mut data = dashboard.lock().unwrap();
    
    if resp != Value::Null {
        let mut inbound  = 0;
        let mut outbound = 0;

        for peer in resp["result"]["Ok"].as_array().unwrap() {
            if peer["direction"] == "Inbound" {
                inbound += 1;
            }
            if peer["direction"] == "Outbound" {
                outbound += 1;
            }
        }
        data.inbound  = inbound;
        data.outbound = outbound;
    }

    Ok(())
}


// Collecting: supply, inflation, price_usd, price_btc, volume_usd, volume_btc, cap_usd, cap_btc.
pub async fn get_market(dashboard: Arc<Mutex<Dashboard>>) -> Result<(), Error> {
    let client = reqwest::Client::new();
    let result = client.get("https://api.coingecko.com/api/v3/simple/price?ids=grin&vs_currencies=usd%2Cbtc&include_24hr_vol=true")
                       .send()
                       .await?;

    let val: Value = serde_json::from_str(&result.text().await.unwrap()).unwrap();
    
    let mut data = dashboard.lock().unwrap();
   
    if data.height.is_empty() == false {
        // Calculating coin supply
        // Adding +1 as block index starts with 0
        let supply = (data.height.parse::<u64>().unwrap() + 1) * 60;

        // 31536000 seconds in a year
        let inflation = (31536000.0 / (supply as f64)) * 100.0;

        data.inflation   = format!("{:.2}", inflation);
        data.supply      = supply.to_formatted_string(&Locale::en);

        // https://john-tromp.medium.com/a-case-for-using-soft-total-supply-1169a188d153
        data.soft_supply = format!("{:.2}",
                           supply.to_string().parse::<f64>().unwrap() / 3150000000.0 * 100.0);
    
        // Check if CoingGecko API returned error
        if let Some(status) = val.get("status") {
            println!("{} {}.", "[ WARNING ]".yellow(),
                     status["error_message"].as_str().unwrap().to_string());
        } else {
            data.price_usd  = format!("{:.3}", val["grin"]["usd"].to_string().parse::<f64>().unwrap());
            data.price_btc  = format!("{:.8}", val["grin"]["btc"].to_string().parse::<f64>().unwrap());
            data.volume_usd = (val["grin"]["usd_24h_vol"].to_string().parse::<f64>().unwrap() as u64)
                              .to_formatted_string(&Locale::en);
            data.volume_btc = format!("{:.2}", val["grin"]["btc_24h_vol"].to_string().parse::<f64>()
                              .unwrap());
            data.cap_usd    = (((supply as f64) * data.price_usd.parse::<f64>().unwrap()) as u64)
                              .to_formatted_string(&Locale::en);
            data.cap_btc    = (((supply as f64) * data.price_btc.parse::<f64>().unwrap()) as u64)
                              .to_formatted_string(&Locale::en);
        }
    }

    Ok(())
}


// Collecting: disk_usage.
pub fn get_disk_usage(dashboard: Arc<Mutex<Dashboard>>) -> Result<(), Error> { 
    let mut data = dashboard.lock().unwrap();
    let grin_dir = format!("{}/.grin", std::env::var("HOME").unwrap());

    data.disk_usage = format!("{:.2}", (get_size(grin_dir).unwrap() as f64) / 1000.0 / 1000.0 / 1000.0);

    Ok(())
}


// Collecting: hashrate, difficulty, production cost, breakeven cost.
pub async fn get_mining_stats(dashboard: Arc<Mutex<Dashboard>>) -> Result<(), Error> {
    let difficulty_window = 1440;
    let height            = get_current_height(dashboard.clone());

    if height.is_empty() == false {
        let params1 = &format!("[{}, null, null]", height)[..];
        let params2 = &format!("[{}, null, null]", height.parse::<u64>().unwrap()
                               - difficulty_window)[..];
        let resp1   = call("get_block", params1, "foreign").await?;
        let resp2   = call("get_block", params2, "foreign").await?;
    
        let mut data = dashboard.lock().unwrap();

        if resp1 != Value::Null && resp2 != Value::Null {
            // Calculate network difficulty
            let net_diff = (resp1["result"]["Ok"]["header"]["total_difficulty"]
                           .to_string().parse::<u64>().unwrap()
                           - resp2["result"]["Ok"]["header"]["total_difficulty"]
                           .to_string().parse::<u64>().unwrap()) /
                           difficulty_window;

            // https://forum.grin.mw/t/on-dual-pow-graph-rates-gps-and-difficulty/2144/52
            // https://forum.grin.mw/t/difference-c31-and-c32-c33/7018/7
            let hashrate    = (net_diff as f64) * 42.0 / 60.0 / 16384.0;
            data.hashrate   = format!("{:.2}", hashrate / 1000.0);
            data.difficulty = net_diff.to_string();

            // Calculating G1-mini production per hour
            let coins_per_hour = 1.2 / hashrate * 60.0 * 60.0;

            // Calculating production cost of 1 grin
            // Assuming $0.07 per kW/h
            data.production_cost = format!("{:.3}", 120.0 / 1000.0 * 0.07 * (1.0 / coins_per_hour));

            data.reward_ratio    = format!("{:.2}", data.price_usd.parse::<f64>().unwrap()
                                                    / data.production_cost.parse::<f64>().unwrap());
            data.breakeven_cost = format!("{:.2}", data.price_usd.parse::<f64>().unwrap()
                                   / (120.0 / 1000.0 * (1.0 / coins_per_hour)));
        }
    }

    Ok(())
}


// Collecting block data for recent blocks (block_list page).
pub async fn get_block_list_data(height: &String, block: &mut Block)
                                  -> Result<(), Error> {
    // Max block weight is 40000
    // One unit of weight is 32 bytes
    let kernel_weight = 3.0;
    let input_weight  = 1.0;
    let output_weight = 21.0;

    if height.is_empty() == false {
        let params   = &format!("[{}, null, null]", height)[..];
        let resp     = call("get_block", params, "foreign").await.unwrap();

        if resp["result"]["Ok"].is_null() == false {
            block.height  = resp["result"]["Ok"]["header"]["height"].to_string();

            let dt: DateTime<Utc> = resp["result"]["Ok"]["header"]["timestamp"]
                                    .as_str().unwrap().to_string().parse().unwrap();
        
            // Utc --> human time
            let duration = Duration::new((Utc::now().timestamp() - dt.timestamp()) as u64, 0);

            if duration.as_secs() > 2592000 {
                let string = format_duration(duration).to_string();
                let (a, _b) = string.split_once(" ").unwrap();

                block.time = format!("{} ago", a);
            } else {
                block.time = format_duration(duration).to_string();
            }

            for kernel in resp["result"]["Ok"]["kernels"].as_array().unwrap() {
                let fee = kernel["fee"].to_string().parse::<f64>().unwrap();

                block.fees    += fee;
                block.weight  += kernel_weight;
                block.ker_len = block.ker_len + 1;
            }

            for _input in resp["result"]["Ok"]["inputs"].as_array().unwrap() {
                block.weight += input_weight;
                block.in_len = block.in_len + 1;
            }

            for _output in resp["result"]["Ok"]["outputs"].as_array().unwrap() {
                block.weight  += output_weight;
                block.out_len = block.out_len + 1;
            }
        } else {
            return Ok(());
        }
    }

    block.weight  = format!("{:.2}", block.weight / 40000.0 * 100.0).parse::<f64>().unwrap();

    Ok(())
}


// Collecting block data.
pub async fn get_block_data(height: &str, block: &mut Block)
             -> Result<(), Error> {
    // Max block weight is 40000
    // One unit of weight is 32 bytes
    let kernel_weight = 3.0;
    let input_weight  = 1.0;
    let output_weight = 21.0;

    if height.is_empty() == false {
        let params = &format!("[{}, null, null]", height)[..];

        let resp = call("get_block", params, "foreign").await?;

        if resp["result"]["Ok"].is_null() == false {
            block.hash    = resp["result"]["Ok"]["header"]["hash"].as_str().unwrap().to_string();
            block.height  = resp["result"]["Ok"]["header"]["height"].to_string();

            let dt: DateTime<Utc> = resp["result"]["Ok"]["header"]["timestamp"]
                                    .as_str().unwrap().to_string().parse().unwrap();
        
            block.time    = dt.to_string();
            block.version = resp["result"]["Ok"]["header"]["version"].to_string();

            for kernel in resp["result"]["Ok"]["kernels"].as_array().unwrap() {
                let fee = kernel["fee"].to_string().parse::<f64>().unwrap();
                block.kernels.push((kernel["excess"].as_str().unwrap().to_string(),
                                    kernel["features"].as_str().unwrap().to_string(),
                                    (fee / 1000000000.0).to_string()));
                block.fees += fee;
                block.weight += kernel_weight;
            }

            for input in resp["result"]["Ok"]["inputs"].as_array().unwrap() {
                block.inputs.push(input.as_str().unwrap().to_string());
                block.weight += input_weight;
            }

            for output in resp["result"]["Ok"]["outputs"].as_array().unwrap() {
                block.outputs.push((output["commit"].as_str().unwrap().to_string(),
                                   output["output_type"].as_str().unwrap().to_string()));
                block.weight += output_weight;
            }

            block.weight   = format!("{:.2}", block.weight / 40000.0 * 100.0).parse::<f64>().unwrap();
            block.ker_len  = block.kernels.iter().count() as u64;
            block.in_len   = block.inputs.iter().count() as u64;
            block.out_len  = block.outputs.iter().count() as u64;
            block.raw_data = serde_json::to_string_pretty(&resp).unwrap();
        }
    }

    Ok(())
}


// Get block height by hash.
pub async fn get_block_header(hash: &str, height: &mut String)
             -> Result<(), Error> {
    let params = &format!("[null, \"{}\", null]", hash)[..];

    let resp = call("get_header", params, "foreign").await.unwrap();
    
    if resp["result"]["Ok"].is_null() == false {
        *height = resp["result"]["Ok"]["height"].to_string();
    }

    Ok(())
}


// Get kernel.
pub async fn get_kernel(kernel: &str, height: &mut String)
             -> Result<(), Error> {
    let params = &format!("[\"{}\", null, null]", kernel)[..];

    let resp = call("get_kernel", params, "foreign").await.unwrap();
    
    if resp["result"]["Ok"].is_null() == false {
        *height = resp["result"]["Ok"]["height"].to_string();
    }

    Ok(())
}


// Collecting block kernels for transactions stats.
pub async fn get_block_kernels(height: &String, blocks: &mut Vec<Block>)
             -> Result<(), Error> {
    if height.is_empty() == false {
        let params = &format!("[{}, {}, 720, false]", height.parse::<u64>().unwrap() - 720,
                              height)[..];
        let resp   = call("get_blocks", params, "foreign").await.unwrap();

        for resp_block in resp["result"]["Ok"]["blocks"].as_array().unwrap() {
            let mut block = Block::new();
            
            for kernel in resp_block["kernels"].as_array().unwrap() {
                block.kernels.push((kernel["excess"].to_string(),
                                    kernel["features"].as_str().unwrap().to_string(),
                                    kernel["fee"].to_string()));
            }
            
            blocks.push(block);
        }
    }

    Ok(())
}


// Collecting: period_1h, period_24h, fees_1h, fees_24h.
pub async fn get_txn_stats(dashboard: Arc<Mutex<Dashboard>>,
                           transactions: Arc<Mutex<Transactions>>) -> Result<(), Error> {
    let mut blocks = Vec::<Block>::new();
    let height     = get_current_height(dashboard.clone());

    if height.is_empty() == false {
        // get_blocks grin rpc has limit of maximum of 1000 blocks request
        // https://github.com/mimblewimble/grin/blob/master/api/src/handlers/blocks_api.rs#L27
        // So, collecting kernels 2 times by 720 blocks to get a day of blocks
        let _ = get_block_kernels(&((height.parse::<u64>().unwrap() - 720).to_string()), &mut blocks)
                                 .await;
        let _ = get_block_kernels(&height, &mut blocks).await;

        if blocks.is_empty() == false {
            let mut ker_count_1h  = 0;
            let mut ker_count_24h = 0;
            let mut fees_1h       = 0.0;
            let mut fees_24h      = 0.0;
            let mut index         = 0;

            for block in blocks {
                // Latest 60 blocks
                if index >= 1380 {
                    for kernel in block.kernels.clone() {
                        if kernel.1 != "Coinbase" {
                            ker_count_1h = ker_count_1h + 1;
                            fees_1h = fees_1h + kernel.2.parse::<f64>().unwrap();
                        }
                    }
                }

                for kernel in block.kernels {
                    if kernel.1 != "Coinbase" {
                        ker_count_24h = ker_count_24h + 1;
                        fees_24h = fees_24h + kernel.2.to_string().parse::<f64>().unwrap();
                    }
                }

                index = index + 1;
            }

            let mut txns = transactions.lock().unwrap();

            txns.period_1h  = ker_count_1h.to_string();
            txns.period_24h = ker_count_24h.to_string();
            txns.fees_1h    = format!("{:.2}", fees_1h / 1000000000.0);
            txns.fees_24h   = format!("{:.2}", fees_24h / 1000000000.0);
        }
    }

    Ok(())
}


// Return current block height
pub fn get_current_height(dashboard: Arc<Mutex<Dashboard>>) -> String {
    let data = dashboard.lock().unwrap();

    data.height.clone()
}


// Collecting recent blocks data.
pub async fn get_recent_blocks(dashboard: Arc<Mutex<Dashboard>>,
                         blocks: Arc<Mutex<Vec<Block>>>) -> Result<(), Error> {
    let mut i      = 0;
    let height_str = get_current_height(dashboard.clone());

    if height_str.is_empty() == false {
        let height         = height_str.parse::<u64>().unwrap();
        let mut blocks_vec = Vec::<Block>::new();

        while i < 10 {
            let mut block = Block::new();
            let height_index = height - i;

            let _ = get_block_list_data(&height_index.to_string(), &mut block).await;

            blocks_vec.push(block);
            i = i + 1;
        }

        let mut blcks = blocks.lock().unwrap();
        blcks.clear();
        *blcks = blocks_vec;

    }

    Ok(())
}


// Collecting a specified list of blocks.
pub async fn get_block_list_by_height(height: &str, blocks: &mut Vec<Block>,
                                      latest_height: &mut u64) -> Result<(), Error> {
    let mut i      = 0;
    let height = height.to_string();

    let resp = call("get_status", "[]", "owner").await.unwrap();

    if resp != Value::Null {
        *latest_height = resp["result"]["Ok"]["tip"]["height"].to_string().parse::<u64>().unwrap();

        if height.is_empty() == false && height.chars().all(char::is_numeric) == true {
            let mut height = height.parse::<u64>().unwrap();

            if height < 10 {
                height = 9;
            }

            while i < 10 {
                let mut block = Block::new();

                let _ = get_block_list_data(&(height - i).to_string(), &mut block).await;

                blocks.push(block);
                i = i + 1;
            }
        }
    }

    Ok(())
}

