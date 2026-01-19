use chrono::{Utc, DateTime};
use fs_extra::dir::get_size;
use humantime::format_duration;
use num_format::{Locale, ToFormattedString};
use reqwest::Error;
use serde_json::Value;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;
use std::collections::HashMap;

use crate::data::{Block, Dashboard, Kernel, Output, Statistics, Transactions};
use crate::data::{KERNEL_WEIGHT, INPUT_WEIGHT, OUTPUT_WEIGHT, KERNEL_SIZE, INPUT_SIZE, OUTPUT_SIZE};
use crate::exconfig::CONFIG;


// RPC requests to grin node.
pub async fn call(method: &str, params: &str, id: &str, rpc_type: &str) -> Result<Value, anyhow::Error> {
    let rpc_url;
    let secret;

    if CONFIG.port.is_empty() == false {
        rpc_url = format!("{}://{}:{}/v2/{}", CONFIG.proto, CONFIG.host, CONFIG.port, rpc_type);
    } else {
        rpc_url = format!("{}://{}/v2/{}", CONFIG.proto, CONFIG.host, rpc_type);
    }

    if rpc_type == "owner" {
        secret = CONFIG.api_secret.clone();
    } else {
        secret = CONFIG.foreign_api_secret.clone();
    }

    let client = reqwest::Client::new();
    let result = client.post(rpc_url)
                       .timeout(Duration::from_secs(10))
                       .body(format!("{{\"method\": \"{}\", \"params\": {}, \"id\": {}, \"jsonrpc\": \"2.0\"}}", method, params, id))
                       .basic_auth(CONFIG.user.clone(), Some(secret))
                       .header("content-type", "application/json")
                       .send()
                       .await?;

    match result.error_for_status_ref() {
        Ok(_res) => (),
        Err(err) => { error!("rpc failed, status code: {:?}", err.status().unwrap()); },
    }

    let val: Value = serde_json::from_str(&result.text().await?)?;

    Ok(val)
}


// RPC requests to grin node.
// The same call as above but with no api secrets usage and the option to specify custom endpoint.
pub async fn call_external(method: &str, params: &str, id: &str, rpc_type: &str, endpoint: String) -> Result<Value, anyhow::Error> {
    let rpc_url;

    rpc_url = format!("{}/v2/{}", endpoint, rpc_type);

    let client = reqwest::Client::new();
    let result = client.post(rpc_url)
                       .timeout(Duration::from_secs(10))
                       .body(format!("{{\"method\": \"{}\", \"params\": {}, \"id\": {}, \"jsonrpc\": \"2.0\"}}", method, params, id))
                       .header("content-type", "application/json")
                       .send()
                       .await?;

    match result.error_for_status_ref() {
        Ok(_res) => (),
        Err(err) => { error!("rpc failed, status code: {:?}", err.status().unwrap()); },
    }

    let val: Value = serde_json::from_str(&result.text().await?)?;

    Ok(val)
}


// Collecting: height, sync, node_ver, proto_ver, kernel_mmr_size.
pub async fn get_status(dashboard: Arc<Mutex<Dashboard>>) -> Result<(), anyhow::Error> {
    let resp1 = call("get_status", "[]", "1", "owner").await?;

    if resp1 != Value::Null {
        let params = &format!("[{}, null, null]", resp1["result"]["Ok"]["tip"]["height"])[..];
        let resp2  = call("get_block", params, "1", "foreign").await?;

        let mut data = dashboard.lock().unwrap();

        if resp2 != Value::Null {
            if resp2["result"]["Ok"]["header"]["kernel_mmr_size"] != Value::Null {
                data.kernel_mmr_size = resp2["result"]["Ok"]["header"]["kernel_mmr_size"].to_string();
            }
        }

        if resp1["result"]["Ok"]["chain"] == Value::Null {
            if data.chain.is_empty() {
                warn!("update grin node to version 5.3.3 or later");
                data.chain = "unknown".to_string();
            }
        } else {
            data.chain = resp1["result"]["Ok"]["chain"].as_str().unwrap().to_string();
        }
        data.height    = resp1["result"]["Ok"]["tip"]["height"].to_string();
        data.sync      = resp1["result"]["Ok"]["sync_status"].as_str().unwrap().to_string();
        data.node_ver  = resp1["result"]["Ok"]["user_agent"].as_str().unwrap().to_string();
        data.proto_ver = resp1["result"]["Ok"]["protocol_version"].to_string();
    }

    Ok(())
}


// Collecting: txns, stem.
pub async fn get_mempool(dashboard: Arc<Mutex<Dashboard>>) -> Result<(), anyhow::Error> {
    let resp1 = call("get_pool_size", "[]", "1", "foreign").await?;
    let resp2 = call("get_stempool_size", "[]", "1", "foreign").await?;
    
    let mut data = dashboard.lock().unwrap();

    if resp1 != Value::Null && resp1 != Value::Null {
        data.txns = resp1["result"]["Ok"].to_string();
        data.stem = resp2["result"]["Ok"].to_string();
    }

    Ok(())
}


// Collecting: inbound, outbound, user_agent.
pub async fn get_connected_peers(dashboard: Arc<Mutex<Dashboard>>, statistics: Arc<Mutex<Statistics>>) -> Result<(), anyhow::Error> {
    let mut peers    = HashMap::new();
    let mut inbound  = 0;
    let mut outbound = 0;

    let resp = call("get_connected_peers", "[]", "1", "owner").await?;
    
    if resp != Value::Null {

        for peer in resp["result"]["Ok"].as_array().unwrap() {
            if peer["direction"] == "Inbound" {
                inbound += 1;
            }
            if peer["direction"] == "Outbound" {
                outbound += 1;
            }
            // Collecting user_agent nodes stats
            *peers.entry(peer["user_agent"].to_string()).or_insert(0) += 1;
        }

    }

    // Collecting peers stats from external endpoints
    for endpoint in CONFIG.external_nodes.clone() {
        match call_external("get_connected_peers", "[]", "1", "owner", endpoint).await {
            Ok(resp) => {
                            if resp != Value::Null {
                                for peer in resp["result"]["Ok"].as_array().unwrap() {
                                    // Collecting user_agent nodes stats
                                    *peers.entry(peer["user_agent"].to_string()).or_insert(0) += 1;
                                }
                            }
                         },
            Err(e)   => warn!("{}", e),
        }
    }

    // Sort HashMap into Vec
    let mut peers_vec: Vec<(&String, &u32)> = peers.iter().collect();
    peers_vec.sort_by(|a, b| b.1.cmp(a.1));

    let mut dash  = dashboard.lock().unwrap();
    let mut stats = statistics.lock().unwrap();

    stats.user_agent.clear();
    stats.count.clear();
    stats.total = 0;

    for v in peers_vec {
        stats.total = stats.total + v.1;
        stats.user_agent.push(v.0.to_string());
        stats.count.push(v.1.to_string());
    }

    dash.inbound  = inbound;
    dash.outbound = outbound;

    Ok(())
}


// Collecting: supply, inflation, price_usd, price_btc, volume_usd, volume_btc, cap_usd, cap_btc.
pub async fn get_market(dashboard: Arc<Mutex<Dashboard>>) -> Result<(), anyhow::Error> {
    let client;
    let result;
    let mut val = Value::Null;

    static COINGECKO_COUNT: AtomicU32 = AtomicU32::new(0);

    let count = COINGECKO_COUNT.fetch_add(1, Ordering::Relaxed);

    // Call CG API only once every 10 calls (15sec * 10)
    if CONFIG.coingecko_api == "enabled" && count % 10 == 0 {
        client = reqwest::Client::new();
        result = client.get("https://api.coingecko.com/api/v3/simple/price?ids=grin&vs_currencies=usd%2Cbtc&include_24hr_vol=true").send().await?;
        val    = serde_json::from_str(&result.text().await?)?;
    }

    let mut data = dashboard.lock().unwrap();
  
    if data.height.is_empty() == false {
        // Calculating coin supply
        // Adding +1 as block index starts with 0
        let supply = (data.height.parse::<u64>().unwrap() + 1) * 60;

        // 31536000 seconds in a year
        let inflation = (31536000.0 / (supply as f64)) * 100.0;

        data.inflation  = format!("{:.2}", inflation);
        data.supply_raw = supply.to_string();
        data.supply     = supply.to_formatted_string(&Locale::en);

        // https://john-tromp.medium.com/a-case-for-using-soft-total-supply-1169a188d153
        data.soft_supply = format!("{:.2}", supply.to_string().parse::<f64>().unwrap() / 3150000000.0 * 100.0);
    
        if CONFIG.coingecko_api == "enabled" && val != Value::Null {
            // Check if CoingGecko API returned error
            if let Some(status) = val.get("status") {
                warn!("{}", status["error_message"].to_string());
            } else {
                data.price_usd  = format!("{:.3}", val["grin"]["usd"].to_string().parse::<f64>().unwrap());
                data.price_btc  = format!("{:.8}", val["grin"]["btc"].to_string().parse::<f64>().unwrap());
                data.volume_usd = (val["grin"]["usd_24h_vol"].to_string().parse::<f64>().unwrap() as u64)
                                  .to_formatted_string(&Locale::en);
                data.volume_btc = format!("{:.2}", val["grin"]["btc_24h_vol"].to_string().parse::<f64>().unwrap());
                data.cap_usd    = (((supply as f64) * data.price_usd.parse::<f64>().unwrap()) as u64)
                                  .to_formatted_string(&Locale::en);
                data.cap_btc    = (((supply as f64) * data.price_btc.parse::<f64>().unwrap()) as u64)
                                  .to_formatted_string(&Locale::en);
            }
        }
    }

    Ok(())
}


// Collecting: disk_usage.
pub fn get_disk_usage(dashboard: Arc<Mutex<Dashboard>>) -> Result<(), Error> { 
    let mut data = dashboard.lock().unwrap();
    let chain_dir;

    if data.chain == "main" {
        chain_dir = format!("{}/main/chain_data", CONFIG.grin_dir);
    } else if data.chain == "test" {
        chain_dir = format!("{}/test/chain_data", CONFIG.grin_dir);
    } else {
        // Chain parameter in get_status() rpc is added in 5.3.3 node.
        // Default to main chain in case of node version less than 5.3.3.
        chain_dir = format!("{}/main/chain_data", CONFIG.grin_dir);
    }

    match get_size(chain_dir.clone()) {
        Ok(chain_size) => data.disk_usage = format!("{:.2}", (chain_size as f64) / 1000.0 / 1000.0 / 1000.0),
        Err(e)         => {
            if CONFIG.host == "127.0.0.1" || CONFIG.host == "0.0.0.0" {
                error!("{}: \"{}\"", e, chain_dir);
            } else {
                // Ignore error for external node connection
            }
        },
    }

    Ok(())
}


// Collecting: hashrate, difficulty, production cost, breakeven cost.
pub async fn get_mining_stats(dashboard: Arc<Mutex<Dashboard>>) -> Result<(), anyhow::Error> {
    let difficulty_window = 1440;
    let height            = get_current_height(dashboard.clone());

    if height.is_empty() == false && height.parse::<u64>().unwrap() > 1440 {
        let params1 = &format!("[{}, null, null]", height)[..];
        let params2 = &format!("[{}, null, null]", height.parse::<u64>().unwrap() - difficulty_window)[..];
        let resp1   = call("get_block", params1, "1", "foreign").await?;
        let resp2   = call("get_block", params2, "1", "foreign").await?;
    
        let mut data = dashboard.lock().unwrap();

        if resp1 != Value::Null && resp2 != Value::Null &&
           resp1["result"]["Ok"].is_null() == false &&
           resp2["result"]["Ok"].is_null() == false {
            // Calculate network difficulty
            let net_diff = (resp1["result"]["Ok"]["header"]["total_difficulty"]
                           .to_string().parse::<u64>().unwrap()
                           - resp2["result"]["Ok"]["header"]["total_difficulty"]
                           .to_string().parse::<u64>().unwrap()) /
                           difficulty_window;

            // https://forum.grin.mw/t/on-dual-pow-graph-rates-gps-and-difficulty/2144/52
            // https://forum.grin.mw/t/difference-c31-and-c32-c33/7018/7
            let hashrate = (net_diff as f64) * 42.0 / 60.0 / 16384.0;

            // kG/s
            if hashrate > 1000.0 {
                data.hashrate = format!("{:.2} kG/s", hashrate / 1000.0);
            // G/s
            } else {
                data.hashrate = format!("{:.2} G/s", hashrate);
            }

            // Save hashrate as kG/s for chart stats
            data.hashrate_kgs = format!("{:.2}", hashrate / 1000.0);

            data.difficulty = net_diff.to_string();

            if CONFIG.coingecko_api == "enabled" {
                // Calculating G1-mini production per hour
                let coins_per_hour = 1.2 / hashrate * 60.0 * 60.0;

                // Calculating production cost of 1 grin
                // Assuming $0.07 per kW/h
                data.production_cost = format!("{:.3}", 120.0 / 1000.0 * 0.07 * (1.0 / coins_per_hour));

                if data.price_usd.is_empty() == false {
                    data.reward_ratio   = format!("{:.2}", data.price_usd.parse::<f64>().unwrap()
                                                        / data.production_cost.parse::<f64>().unwrap());
                    data.breakeven_cost = format!("{:.2}", data.price_usd.parse::<f64>().unwrap()
                                                        / (120.0 / 1000.0 * (1.0 / coins_per_hour)));
                }
            }
        } else {
            error!("get_mining_stats() failed");
            error!("RPC response 1: {:?}", resp1);
            error!("RPC response 2: {:?}", resp2);
        }
    }

    Ok(())
}


// Collecting block data for recent blocks (block_list page).
pub async fn get_block_list_data(height: &String, block: &mut Block)
                                  -> Result<(), anyhow::Error> {
    if height.is_empty() == false {
        let params = &format!("[{}, null, null]", height)[..];
        let resp   = call("get_block", params, "1", "foreign").await?;

        if resp["result"]["Ok"].is_null() == false {
            block.height = resp["result"]["Ok"]["header"]["height"].to_string();

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
                block.weight  += KERNEL_WEIGHT;
                block.ker_len = block.ker_len + 1;
            }

            for _input in resp["result"]["Ok"]["inputs"].as_array().unwrap() {
                block.weight += INPUT_WEIGHT;
                block.in_len = block.in_len + 1;
            }

            for _output in resp["result"]["Ok"]["outputs"].as_array().unwrap() {
                block.weight  += OUTPUT_WEIGHT;
                block.out_len = block.out_len + 1;
            }
        } else {
            return Ok(());
        }
    }

    block.weight = format!("{:.2}", block.weight / 40000.0 * 100.0).parse::<f64>().unwrap();

    let block_size = ((block.ker_len * KERNEL_SIZE) + (block.in_len * INPUT_SIZE) + (block.out_len * OUTPUT_SIZE)) as f64;

    if block_size > 1000000.0 {
        block.size = format!("{:.2} MB", block_size / 1000.0 / 1000.0);
    } else if block_size > 1000.0 {
        block.size = format!("{:.2} KB", block_size / 1000.0);
    } else {
        block.size = format!("{} B", block_size);
    }

    Ok(())
}


// Collecting block data.
pub async fn get_block_data(height: &str, block: &mut Block)
             -> Result<(), anyhow::Error> {
    if height.is_empty() == false {
        let params = &format!("[{}, null, null]", height)[..];

        let resp = call("get_block", params, "1", "foreign").await?;

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
                block.weight += KERNEL_WEIGHT;
            }

            for input in resp["result"]["Ok"]["inputs"].as_array().unwrap() {
                block.inputs.push(input.as_str().unwrap().to_string());
                block.weight += INPUT_WEIGHT;
            }

            for output in resp["result"]["Ok"]["outputs"].as_array().unwrap() {
                block.outputs.push((output["commit"].as_str().unwrap().to_string(),
                                   output["output_type"].as_str().unwrap().to_string()));
                block.weight += OUTPUT_WEIGHT;
            }

            block.weight   = format!("{:.2}", block.weight / 40000.0 * 100.0).parse::<f64>().unwrap();
            block.ker_len  = block.kernels.iter().count() as u64;
            block.in_len   = block.inputs.iter().count() as u64;
            block.out_len  = block.outputs.iter().count() as u64;
            block.raw_data = serde_json::to_string_pretty(&resp).unwrap();

            let block_size = ((block.ker_len * KERNEL_SIZE) + (block.in_len * INPUT_SIZE) + (block.out_len * OUTPUT_SIZE)) as f64;

            if block_size > 1000000.0 {
                block.size = format!("{:.2} MB", block_size / 1000.0 / 1000.0);
            } else if block_size > 1000.0 {
                block.size = format!("{:.2} KB", block_size / 1000.0);
            } else {
                block.size = format!("{} B", block_size);
            }
        }
    }

    Ok(())
}


// Get block height by hash.
pub async fn get_block_header(hash: &str, height: &mut String)
             -> Result<(), anyhow::Error> {
    let params = &format!("[null, \"{}\", null]", hash)[..];

    let resp = call("get_header", params, "1", "foreign").await?;
    
    if resp["result"]["Ok"].is_null() == false {
        *height = resp["result"]["Ok"]["height"].to_string();
    }

    Ok(())
}


// Get output.
pub async fn get_output(commit: &str, output: &mut Output) -> Result<(), anyhow::Error> {
    // First check whether output is broadcasted but not confirmed yet (in mempool)
    let mut resp = call("get_unconfirmed_transactions", "[]", "1", "foreign").await?;

    if resp["result"]["Ok"].is_null() == false {
        for tx in resp["result"]["Ok"].as_array().unwrap() {
            for out in tx["tx"]["body"]["outputs"].as_array().unwrap() {
                if out["commit"].as_str().unwrap() == commit {
                    // Only Plain outputs in the mempool
                    output.out_type = "Plain".to_string();
                    output.commit   = out["commit"].as_str().unwrap().to_string();
                    output.status   = "Unconfirmed".to_string();
                    // Found it, no need to continue
                    return Ok(());
                }
            }
        }
    }

    let params = &format!("[[\"{}\"], null, null, true, true]", commit)[..];

    resp = call("get_outputs", params, "1", "foreign").await?;

    if resp["result"]["Ok"][0].is_null() == false {
        output.height   = resp["result"]["Ok"][0]["block_height"].to_string();
        output.commit   = resp["result"]["Ok"][0]["commit"].as_str().unwrap().to_string();
        output.out_type = resp["result"]["Ok"][0]["output_type"].as_str().unwrap().to_string();
        output.raw_data = serde_json::to_string_pretty(&resp).unwrap();

        let resp_status = call("get_status", "[]", "1", "owner").await?;

        if resp_status != Value::Null {
            let curr_height = resp_status["result"]["Ok"]["tip"]["height"].to_string();
            let num_conf    = curr_height.parse::<u64>().unwrap() - output.height.parse::<u64>().unwrap() + 1;

            output.status = format!("{} Confirmations", num_conf.to_string());
        }
    }

    Ok(())
}


// Get kernel.
pub async fn get_kernel(excess: &str, kernel: &mut Kernel) -> Result<(), anyhow::Error> {
    // First check whether kernel is broadcasted but not confirmed yet (in mempool)
    let mut resp = call("get_unconfirmed_transactions", "[]", "1", "foreign").await?;
    
    if resp["result"]["Ok"].is_null() == false {
        for tx in resp["result"]["Ok"].as_array().unwrap() {
            for ker in tx["tx"]["body"]["kernels"].as_array().unwrap() {
                if ker["excess"].as_str().unwrap() == excess {
                    // Only Plain kernels in the mempool
                    kernel.ker_type = "Plain".to_string();
                    kernel.excess   = ker["excess"].as_str().unwrap().to_string();
                    kernel.status   = "Unconfirmed".to_string();
                    kernel.fee      = format!("ツ {}",
                                      ker["features"]["Plain"]["fee"]
                                      .to_string().parse::<f64>().unwrap() / 1000000000.0);
                    // Found it, no need to continue
                    return Ok(());
                }
            }
        }
    }
    
    let params = &format!("[\"{}\", null, null]", excess)[..];

    resp = call("get_kernel", params, "1", "foreign").await?;
    
    if resp["result"]["Ok"].is_null() == false {
        kernel.height = resp["result"]["Ok"]["height"].to_string();
        kernel.excess = resp["result"]["Ok"]["tx_kernel"]["excess"].as_str().unwrap().to_string();
        if resp["result"]["Ok"]["tx_kernel"]["features"]["Plain"].is_null() == false {
            kernel.ker_type = "Plain".to_string();
            kernel.fee      = format!("ツ {}",
                                      resp["result"]["Ok"]["tx_kernel"]["features"]["Plain"]["fee"]
                                      .to_string().parse::<f64>().unwrap() / 1000000000.0);
        } else {
            kernel.ker_type = resp["result"]["Ok"]["tx_kernel"]["features"].as_str().unwrap().to_string();
        }

        kernel.raw_data = serde_json::to_string_pretty(&resp).unwrap();
        
        let resp_status = call("get_status", "[]", "1", "owner").await?;

        if resp_status != Value::Null {
            let curr_height = resp_status["result"]["Ok"]["tip"]["height"].to_string();
            let num_conf    = curr_height.parse::<u64>().unwrap() - kernel.height.parse::<u64>().unwrap() + 1;

            kernel.status = format!("{} Confirmations", num_conf.to_string());
        }
    }

    Ok(())
}


// Collecting block kernels for transactions stats.
pub async fn get_block_kernels(height: &String, blocks: &mut Vec<Block>)
             -> Result<(), anyhow::Error> {
    if height.is_empty() == false {
        let params = &format!("[{}, {}, 720, false]", height.parse::<u64>().unwrap() - 720,
                              height)[..];
        let resp   = call("get_blocks", params, "1", "foreign").await?;

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
                           transactions: Arc<Mutex<Transactions>>)-> Result<(), Error> {
    let mut blocks = Vec::<Block>::new();
    let height     = get_current_height(dashboard.clone());

    if height.is_empty() == false && height.parse::<u64>().unwrap() > 1440 {
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

    if height_str.is_empty() == false && height_str.parse::<u64>().unwrap() > 0 {
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
                                      latest_height: &mut u64) -> Result<(), anyhow::Error> {
    let mut i      = 0;
    let height = height.to_string();

    let resp = call("get_status", "[]", "1", "owner").await?;

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

// Collecting unspent outputs.
pub async fn get_unspent_outputs(dashboard: Arc<Mutex<Dashboard>>) -> Result<(), anyhow::Error> {
    let mut highest_mmr_index = 0;
    let mut current_mmr_index = 0;
    let mut utxo_count        = 0;

    // Get the highest MMR index
    let resp = call("get_unspent_outputs", "[1, null, 10000, false]", "1", "foreign").await?;

    if resp != Value::Null {
        highest_mmr_index = resp["result"]["Ok"]["highest_index"].to_string().parse::<u64>().unwrap();
        current_mmr_index = resp["result"]["Ok"]["outputs"].as_array().unwrap().last().unwrap()["mmr_index"].to_string().parse::<u64>().unwrap();
    }

    // Get all unspent outputs
    while current_mmr_index < highest_mmr_index {
        current_mmr_index = current_mmr_index + 1;
        let params = &format!("[{}, {}, 10000, false]", current_mmr_index, highest_mmr_index)[..];

        let resp = call("get_unspent_outputs", params, "1", "foreign").await?;
    
        if resp != Value::Null {
            if resp["result"]["Ok"]["outputs"] != Value::Null {
                if let Some(v) = resp["result"]["Ok"]["outputs"].as_array().unwrap().last() {
                    current_mmr_index = v["mmr_index"].to_string().parse::<u64>().unwrap();
                    utxo_count = utxo_count + resp["result"]["Ok"]["outputs"].as_array().unwrap().len();
                } else {
                    // Break the loop if we got no outputs from the node request
                    break;
                }
            }
        }
    }

    let mut data = dashboard.lock().unwrap();

    data.utxo_count = utxo_count.to_string();

    Ok(())
}

