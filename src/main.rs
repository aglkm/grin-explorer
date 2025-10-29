#[macro_use] extern crate rocket;
use chrono::Utc;
use either::Either;
use num_format::{Locale, ToFormattedString};
use rocket_dyn_templates::{Template, context};
use rocket::fs::FileServer;
use rocket::{State, tokio};
use rocket::response::Redirect;
use rocket::serde::json::json;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use serde_json::Value;
use tera_thousands::separate_with_commas;

use crate::data::{Block, Dashboard, Kernel, Output, Statistics, Transactions, OUTPUT_SIZE, KERNEL_SIZE};
use crate::exconfig::CONFIG;

mod data;
mod database;
mod exconfig;
mod requests;
mod worker;


// Rendering main (Dashboard) page.
#[get("/")]
fn index(dashboard: &State<Arc<Mutex<Dashboard>>>) -> Template {
    let data = dashboard.lock().unwrap();

    Template::render("index", context! {
        route:     "index",
        node_ver:  &data.node_ver,
        proto_ver: &data.proto_ver,
        cg_api:    CONFIG.coingecko_api.clone(),
    })
}


// Rendering block list (Blocks) page.
#[get("/block_list")]
fn block_list() -> Template {
    Template::render("block_list", context! {
        route:  "block_list",
    })
}


// Rendering block list starting with a specified height.
// [<--] and [-->] buttons at the bottom of the block list (Blocks) page.
#[get("/block_list/<input_height>")]
async fn block_list_by_height(input_height: &str) -> Template {
    let mut blocks = Vec::<Block>::new();
    // Store current latest height
    let mut height = 0;

    let _ = requests::get_block_list_by_height(&input_height, &mut blocks, &mut height).await;

    // Check if user's input doesn't overflow current height
    if blocks.is_empty() == false && blocks[0].height.is_empty() == false {
        let index = blocks[0].height.parse::<u64>().unwrap();

        if index >= height {
            Template::render("block_list", context! {
                route:  "block_list",
            })
        } else {
            Template::render("block_list", context! {
                route:  "block_list_by_height",
                index,
                blocks,
                height,
            })
        }
    } else {
        Template::render("block_list", context! {
            route:  "block_list",
        })
    }
}


// Rendering page for a specified block (by height).
#[get("/block/<height>")]
async fn block_details_by_height(height: &str) -> Template {
    let mut block = Block::new();

    if height.is_empty() == false && height.chars().all(char::is_numeric) == true {
        let _ = requests::get_block_data(&height, &mut block).await;

        if block.height.is_empty() == false {
            return Template::render("block_details", context! {
                route:  "block_details",
                block,
            });
        }
    }

    Template::render("error", context! {
        route:  "error",
    })
}


// Rendering page for a specified block (by hash).
#[get("/hash/<hash>")]
async fn block_header_by_hash(hash: &str) -> Either<Template, Redirect> {
    let mut height = String::new();

    let _ = requests::get_block_header(&hash, &mut height).await;

    if hash.is_empty() == false {
        if height.is_empty() == false {
            return Either::Right(Redirect::to(uri!(block_details_by_height(height.as_str()))));
        }
    }

    return Either::Left(Template::render("error", context! {
        route:  "error",
    }))
}


// Rendering page for a specified kernel.
#[get("/kernel/<excess>")]
async fn kernel(excess: &str) -> Template {
    let mut kernel = Kernel::new();

    let _ = requests::get_kernel(&excess, &mut kernel).await;

    if kernel.excess.is_empty() == false {
        return Template::render("kernel", context! {
            route:  "kernel",
            kernel,
        })
    }

    Template::render("error", context! {
        route:  "error",
    })
}


// Rendering page for a specified output.
#[get("/output/<commit>")]
async fn output(commit: &str) -> Template {
    let mut output = Output::new();

    let _ = requests::get_output(&commit, &mut output).await;

    if output.commit.is_empty() == false {
        return Template::render("output", context! {
            route:  "output",
            output,
        })
    }

    Template::render("error", context! {
        route:  "error",
    })
}


// Handling search request.
// Using Option<&str> to match '/search' query without query params.
// https://github.com/rwf2/Rocket/issues/608
#[get("/search?<query>")]
pub async fn search(query: Option<&str>) -> Either<Template, Redirect> {
    // Unwrap Option and forward to Search page if no parameters
    let query = match query {
        Some(value) => value,
        None => return Either::Left(Template::render("search", context! {
                           route:  "search",
                       })),
    };

    // Trim and lowercase the query
    let query = query.trim().to_lowercase();

    // Check for valid chars
    if query.chars().all(|x| (x >= 'a' && x <= 'f') || (x >= '0' && x <= '9')) == true {

        // Block number
        if query.chars().all(char::is_numeric) == true {
            return Either::Right(Redirect::to(uri!(block_details_by_height(query))));

        // Block hash
        } else if query.len() == 64 {
            return Either::Right(Redirect::to(uri!(block_header_by_hash(query))));
            
        // Kernel or Unspent Output
        } else if query.len() == 66 {
            // First search for Kernel.
            // If found, redirect to Kernel page, otherwise search for Unspent Output.
            // As we can't distinguish between Kernel and Output, this will produce redundant
            // get_kernel and get_output calls, but will allow for better UI (no need to ask user to
            // input the type of the search request).
            let mut kernel = Kernel::new();
            let mut output = Output::new();

            let _ = requests::get_kernel(&query, &mut kernel).await;

            if kernel.excess.is_empty() == false {
                // Here we are redirecting to kernel page and call get_kernel again there.
                // Kernel page is a separate route and we want it to be accessed directly and
                // via search functionality.
                return Either::Right(Redirect::to(uri!(kernel(query))));
            } else {
                // If Kernel not found, then search for Unspent Output
                let _ = requests::get_output(&query, &mut output).await;

                if output.commit.is_empty() == false {
                    return Either::Right(Redirect::to(uri!(output(query))));
                }
            }
        }
    }
    
    Either::Left(Template::render("error", context! {
        route:  "error",
    }))
}


// Rendering Statistics page.
#[get("/stats")]
fn stats(statistics: &State<Arc<Mutex<Statistics>>>) -> Template {
    let data = statistics.lock().unwrap();

    // Get the length of our data vectors (all vectors are the same size)
    let len = data.date.len();

    // Construct chart periods
    let mut month      = 0;
    let mut six_months = 0;
    let mut year       = 0;

    // Usize type can't be negative, so check the lenght of the vector
    if len > 30 {
        month = len - 30;
    }
    if len > (30 * 6) {
        six_months = len - (30 * 6);
    }
    if len > 365 {
        year = len - 365;
    }

    let mut m_date     = data.date.clone();
    let mut m_hashrate = data.hashrate.clone();
    let mut m_txns     = data.txns.clone();
    let mut m_fees     = data.fees.clone();
    let mut m_utxos    = data.utxos.clone();
    let mut m_kernels  = data.kernels.clone();

    // Get stats for a month period
    if month > 0 {  
        m_date     = data.date.get(month..).unwrap().to_vec();
        m_hashrate = data.hashrate.get(month..).unwrap().to_vec();
        m_txns     = data.txns.get(month..).unwrap().to_vec();
        m_fees     = data.fees.get(month..).unwrap().to_vec();
        m_utxos    = data.utxos.get(month..).unwrap().to_vec();
        m_kernels  = data.kernels.get(month..).unwrap().to_vec();
    }

    let mut sm_date     = data.date.clone();
    let mut sm_hashrate = data.hashrate.clone();
    let mut sm_txns     = data.txns.clone();
    let mut sm_fees     = data.fees.clone();
    let mut sm_utxos    = data.utxos.clone();
    let mut sm_kernels  = data.kernels.clone();

    // Get stats for six months period
    if six_months > 0 {  
        sm_date     = data.date.get(six_months..).unwrap().to_vec();
        sm_hashrate = data.hashrate.get(six_months..).unwrap().to_vec();
        sm_txns     = data.txns.get(six_months..).unwrap().to_vec();
        sm_fees     = data.fees.get(six_months..).unwrap().to_vec();
        sm_utxos    = data.utxos.get(six_months..).unwrap().to_vec();
        sm_kernels  = data.kernels.get(six_months..).unwrap().to_vec();
    }
        
    let mut y_date     = data.date.clone();
    let mut y_hashrate = data.hashrate.clone();
    let mut y_txns     = data.txns.clone();
    let mut y_fees     = data.fees.clone();
    let mut y_utxos    = data.utxos.clone();
    let mut y_kernels  = data.kernels.clone();
        
    // Get stats for a year period
    if year > 0 {  
        y_date     = data.date.get(year..).unwrap().to_vec();
        y_hashrate = data.hashrate.get(year..).unwrap().to_vec();
        y_txns     = data.txns.get(year..).unwrap().to_vec();
        y_fees     = data.fees.get(year..).unwrap().to_vec();
        y_utxos    = data.utxos.get(year..).unwrap().to_vec();
        y_kernels  = data.kernels.get(year..).unwrap().to_vec();
    }

    Template::render("stats", context! {
        route:      "stats",
        user_agent: data.user_agent.clone(),
        count:      data.count.clone(),
        total:      data.total,
        date:       data.date.clone(),
        hashrate:   data.hashrate.clone(),
        txns:       data.txns.clone(),
        fees:       data.fees.clone(),
        utxos:      data.utxos.clone(),
        kernels:    data.kernels.clone(),
        m_date,
        m_hashrate,
        m_txns,
        m_fees,
        m_utxos,
        m_kernels,
        sm_date,
        sm_hashrate,
        sm_txns,
        sm_fees,
        sm_utxos,
        sm_kernels,
        y_date,
        y_hashrate,
        y_txns,
        y_fees,
        y_utxos,
        y_kernels,
        output_size: OUTPUT_SIZE,
        kernel_size: KERNEL_SIZE,
    })
}


// Rendering Emission page.
#[get("/emission")]
fn emission(dashboard: &State<Arc<Mutex<Dashboard>>>) -> Template {
    let data = dashboard.lock().unwrap();

    let mut usd = 0.0;
    let mut btc = 0.0;

    if data.price_usd.is_empty() == false && data.price_btc.is_empty() == false {
        usd = data.price_usd.parse::<f64>().unwrap();
        btc = data.price_btc.parse::<f64>().unwrap();
    }

    Template::render("emission", context! {
        route:      "emission",
        cg_api:     CONFIG.coingecko_api.clone(),
        usd_minute: format!("{:.2}", usd * 60.0),
        usd_hour:   ((usd * 3600.0) as u64).to_formatted_string(&Locale::en),
        usd_day:    ((usd * 86400.0) as u64).to_formatted_string(&Locale::en),
        usd_week:   ((usd * 604800.0) as u64).to_formatted_string(&Locale::en),
        usd_month:  ((usd * 2592000.0) as u64).to_formatted_string(&Locale::en),
        usd_year:   ((usd * 31557600.0) as u64).to_formatted_string(&Locale::en),
        btc_minute: format!("{:.8}", btc * 60.0),
        btc_hour:   format!("{:.8}", btc * 3600.0),
        btc_day:    format!("{:.8}", btc * 86400.0),
        btc_week:   format!("{:.8}", btc * 604800.0),
        btc_month:  format!("{:.8}", btc * 2592000.0),
        btc_year:   format!("{:.8}", btc * 31557600.0),
    })
}


// Rendering Donate page.
#[get("/donate")]
fn donate() -> Template {
    Template::render("donate", context! {
        route:      "donate",
        public_api: CONFIG.public_api.clone(),
    })
}

// Rendering API Overview page.
#[get("/api_overview")]
fn api_overview() -> Template {
    Template::render("api_overview", context! {
        route:      "api_overview",
        public_api: CONFIG.public_api.clone(),
    })
}


// Owner API.
// Whitelisted methods: get_connected_peers, get_peers, get_status.
#[post("/v2/owner", data="<data>")]
async fn api_owner(data: &str) -> Value {
    if CONFIG.public_api == "enabled" {
        let result = serde_json::from_str(data);

        let v: Value = match result {
            Ok(value) => value,
            Err(_err) => return json!({"error":"bad syntax"}),
        };

        let method = match v["method"].as_str() {
            Some(value) => value,
            _ => return json!({"error":"bad syntax"}),
        };
    
        if method == "get_connected_peers" || method == "get_peers" || method == "get_status" {
            let resp = requests::call(method, v["params"].to_string().as_str(), v["id"].to_string().as_str(), "owner").await;

            let result = match resp {
                Ok(value) => value,
                Err(_err) => return json!({"error":"rpc call failed"}),
            };

            return json!(result);
        }

        json!({"error":"not allowed"})
    } else {
        json!({"error":"not allowed"})
    }
}


// Foreign API.
// All methods are whitelisted.
#[post("/v2/foreign", data="<data>")]
async fn api_foreign(data: &str) -> Value {
    if CONFIG.public_api == "enabled" {
        let result = serde_json::from_str(data);

        let v: Value = match result {
            Ok(value) => value,
            Err(_err) => return json!({"error":"bad syntax"}),
        };

        let method = match v["method"].as_str() {
            Some(value) => value,
            _ => return json!({"error":"bad syntax"}),
        };

        let resp = requests::call(method, v["params"].to_string().as_str(), v["id"].to_string().as_str(), "foreign").await;

        let result = match resp {
            Ok(value) => value,
            Err(_err) => return json!({"error":"rpc call failed"}),
        };

        return json!(result);
    } else {
        json!({"error":"not allowed"})
    }
}


// Start of HTMX routes.
#[get("/rpc/peers/inbound")]
fn peers_inbound(dashboard: &State<Arc<Mutex<Dashboard>>>) -> String {
    let data = dashboard.lock().unwrap();

    data.inbound.to_string()
}


#[get("/rpc/peers/outbound")]
fn peers_outbound(dashboard: &State<Arc<Mutex<Dashboard>>>) -> String {
    let data = dashboard.lock().unwrap();

    data.outbound.to_string()
}


#[get("/rpc/sync/status")]
fn sync_status(dashboard: &State<Arc<Mutex<Dashboard>>>) -> String {
    let data = dashboard.lock().unwrap();

    if data.sync == "no_sync" {
        "Synced".to_string()
    } else {
        format!("Syncing
                 <div class='spinner-grow spinner-grow-sm' role='status'>
                 <span class='visually-hidden'>Syncing...</span></div>")
    }
}


#[get("/rpc/market/supply")]
fn market_supply(dashboard: &State<Arc<Mutex<Dashboard>>>) -> String {
    let data = dashboard.lock().unwrap();

    format!("ツ {}", data.supply)
}


#[get("/rpc/market/supply_raw")]
fn supply_raw(dashboard: &State<Arc<Mutex<Dashboard>>>) -> String {
    let data = dashboard.lock().unwrap();

    data.supply_raw.clone()
}


#[get("/rpc/market/soft_supply")]
fn soft_supply(dashboard: &State<Arc<Mutex<Dashboard>>>) -> String {
    let data = dashboard.lock().unwrap();

    if data.supply.is_empty() == false {
        // 9 digits plus 2 commas, e.g. 168,038,400
        if data.supply.len() == 11 {
            return format!("{} % ({}M/3150M)", data.soft_supply, &data.supply[..3]);
        // 10 digits plus 2 commas
        } else if data.supply.len() == 12 {
            return format!("{} % ({}M/3150M)", data.soft_supply, &data.supply[..4]);
        }
    }
    
    "3150M".to_string()
}


#[get("/rpc/inflation/rate")]
fn inflation_rate(dashboard: &State<Arc<Mutex<Dashboard>>>) -> String {
    let data = dashboard.lock().unwrap();

    format!("{} %", data.inflation)
}


#[get("/rpc/market/volume_usd")]
fn volume_usd(dashboard: &State<Arc<Mutex<Dashboard>>>) -> String {
    let data = dashboard.lock().unwrap();

    format!("$ {}", data.volume_usd)
}


#[get("/rpc/market/volume_btc")]
fn volume_btc(dashboard: &State<Arc<Mutex<Dashboard>>>) -> String {
    let data = dashboard.lock().unwrap();

    format!("₿ {}", data.volume_btc)
}


#[get("/rpc/price/usd")]
fn price_usd(dashboard: &State<Arc<Mutex<Dashboard>>>) -> String {
    let data = dashboard.lock().unwrap();

    format!("$ {}", data.price_usd)
}


#[get("/rpc/price/btc")]
fn price_btc(dashboard: &State<Arc<Mutex<Dashboard>>>) -> String {
    let data       = dashboard.lock().unwrap();
    let trim: &[_] = &['0', '.'];

    format!("{} sats", data.price_btc.trim_start_matches(trim))
}


#[get("/rpc/market/cap_usd")]
fn mcap_usd(dashboard: &State<Arc<Mutex<Dashboard>>>) -> String {
    let data = dashboard.lock().unwrap();

    format!("$ {}", data.cap_usd)
}


#[get("/rpc/market/cap_btc")]
fn mcap_btc(dashboard: &State<Arc<Mutex<Dashboard>>>) -> String {
    let data = dashboard.lock().unwrap();

    format!("₿ {}", data.cap_btc)
}


#[get("/rpc/block/latest")]
fn latest_height(dashboard: &State<Arc<Mutex<Dashboard>>>) -> String {
    let data = dashboard.lock().unwrap();

    data.height.clone()
}


#[get("/rpc/block/time_since_last")]
fn last_block_age(blocks: &State<Arc<Mutex<Vec<Block>>>>) -> String {
    let data = blocks.lock().unwrap();

    if data.is_empty() == false {
        return data[0].time.clone();
    }
    
    "".to_string()
}


#[get("/rpc/disk/usage")]
fn disk_usage(dashboard: &State<Arc<Mutex<Dashboard>>>) -> String {
    let data = dashboard.lock().unwrap();

    if data.disk_usage.is_empty() == false {
        return format!("{} GB", data.disk_usage);
    } else {
        return format!("<i class=\"bi bi-x-lg\"></i>");
    }
}


#[get("/rpc/network/hashrate")]
fn network_hashrate(dashboard: &State<Arc<Mutex<Dashboard>>>) -> String {
    let data = dashboard.lock().unwrap();

    data.hashrate.clone()
}


#[get("/rpc/mining/production_cost")]
fn production_cost(dashboard: &State<Arc<Mutex<Dashboard>>>) -> String {
    let data = dashboard.lock().unwrap();

    format!("$ {}", data.production_cost)
}


#[get("/rpc/mining/reward_ratio")]
fn reward_ratio(dashboard: &State<Arc<Mutex<Dashboard>>>) -> String {
    let data = dashboard.lock().unwrap();

    if data.reward_ratio.is_empty() == false {
        let ratio = data.reward_ratio.parse::<f64>().unwrap();

        if ratio <= 1.0 {
            return format!("x{} <i class='bi bi-hand-thumbs-down'></i>", data.reward_ratio);
        } else if ratio < 2.0 {
            return format!("x{} <i class='bi bi-hand-thumbs-up'></i>", data.reward_ratio);
        } else if ratio < 3.0 {
            return format!("x{} <i class='bi bi-emoji-sunglasses'></i>", data.reward_ratio);
        } else if ratio >= 3.0 {
            return format!("x{} <i class='bi bi-rocket-takeoff'></i>", data.reward_ratio);
        }
    }

    data.reward_ratio.clone()
}


#[get("/rpc/mining/breakeven_cost")]
fn breakeven_cost(dashboard: &State<Arc<Mutex<Dashboard>>>) -> String {
    let data = dashboard.lock().unwrap();

    format!("$ {} (kW/h)", data.breakeven_cost)
}


#[get("/rpc/network/difficulty")]
fn network_difficulty(dashboard: &State<Arc<Mutex<Dashboard>>>) -> String {
    let data = dashboard.lock().unwrap();

    data.difficulty.to_string()
}


#[get("/rpc/mempool/txns")]
fn mempool_txns(dashboard: &State<Arc<Mutex<Dashboard>>>) -> String {
    let data = dashboard.lock().unwrap();

    data.txns.to_string()
}


#[get("/rpc/mempool/stem")]
fn mempool_stem(dashboard: &State<Arc<Mutex<Dashboard>>>) -> String {
    let data = dashboard.lock().unwrap();

    data.stem.to_string()
}


#[get("/rpc/txns/count_1h")]
fn txns_count_1h(transactions: &State<Arc<Mutex<Transactions>>>) -> String {
    let data = transactions.lock().unwrap();

    format!("{}, ツ {}", data.period_1h, data.fees_1h)
}


#[get("/rpc/txns/count_24h")]
fn txns_count_24h(transactions: &State<Arc<Mutex<Transactions>>>) -> String {
    let data = transactions.lock().unwrap();

    format!("{}, ツ {}", data.period_24h, data.fees_24h)
}


#[get("/rpc/block/link?<count>")]
fn block_link(count: usize, blocks: &State<Arc<Mutex<Vec<Block>>>>) -> String {
    let data = blocks.lock().unwrap();

    if data.is_empty() == false && count < 10 {
        return format!("<a href=/block/{} class='text-decoration-none'>{}</a>",
                       data[count].height, data[count].height);
    }

    "".to_string()
}


#[get("/rpc/block/link_color?<count>")]
fn block_link_color(count: usize, blocks: &State<Arc<Mutex<Vec<Block>>>>) -> String {
    let data = blocks.lock().unwrap();

    if data.is_empty() == false && count < 10 {
        return format!("<a href=/block/{} class='text-decoration-none darkorange-text'>{}</a>",
                       data[count].height, data[count].height);
    }
    
    "".to_string()
}


#[get("/rpc/block/time?<count>")]
fn block_time(count: usize, blocks: &State<Arc<Mutex<Vec<Block>>>>) -> String {
    let data = blocks.lock().unwrap();

    if data.is_empty() == false && count < 10 {
        return data[count].time.clone();
    }

    "".to_string()
}


#[get("/rpc/block/kernels?<count>")]
fn block_txns(count: usize, blocks: &State<Arc<Mutex<Vec<Block>>>>) -> String {
    let data = blocks.lock().unwrap();

    if data.is_empty() == false && count < 10 {
        return data[count].ker_len.to_string();
    }

    "".to_string()
}


#[get("/rpc/block/inputs?<count>")]
fn block_inputs(count: usize, blocks: &State<Arc<Mutex<Vec<Block>>>>) -> String {
    let data = blocks.lock().unwrap();

    if data.is_empty() == false && count < 10 {
        return data[count].in_len.to_string();
    }

    "".to_string()
}


#[get("/rpc/block/outputs?<count>")]
fn block_outputs(count: usize, blocks: &State<Arc<Mutex<Vec<Block>>>>) -> String {
    let data = blocks.lock().unwrap();

    if data.is_empty() == false && count < 10 {
        return data[count].out_len.to_string();
    }

    "".to_string()
}


#[get("/rpc/block/fees?<count>")]
fn block_fees(count: usize, blocks: &State<Arc<Mutex<Vec<Block>>>>) -> String {
    let data = blocks.lock().unwrap();

    if data.is_empty() == false && count < 10 {
        return format!("ツ {}", data[count].fees / 1000000000.0);
    }

    "".to_string()
}


#[get("/rpc/block/size?<count>")]
fn block_size(count: usize, blocks: &State<Arc<Mutex<Vec<Block>>>>) -> String {
    let data = blocks.lock().unwrap();

    if data.is_empty() == false && count < 10 {
        return data[count].size.clone();
    }

    "".to_string()
}


#[get("/rpc/block/weight?<count>")]
fn block_weight(count: usize, blocks: &State<Arc<Mutex<Vec<Block>>>>) -> String {
    let data = blocks.lock().unwrap();

    if data.is_empty() == false && count < 10 {
        return format!("{} %", data[count].weight);
    }

    "".to_string()
}


#[get("/rpc/block_list/index")]
fn block_list_index(dashboard: &State<Arc<Mutex<Dashboard>>>) -> String {
    let data = dashboard.lock().unwrap();

    if data.height.is_empty() == false && data.height.parse::<u64>().unwrap() > 10 {
        return format!("<a class='text-decoration-none' href='/block_list/{}'>
                        <div class='col-sm'><h2><i class='bi bi-arrow-right-square'></i></h2></div>
                        </a>", data.height.parse::<u64>().unwrap() - 10);
    }

    "".to_string()
}


#[get("/rpc/blockchain/unspent_outputs")]
fn unspent_outputs(dashboard: &State<Arc<Mutex<Dashboard>>>) -> String {
    let data = dashboard.lock().unwrap();

    if data.utxo_count.is_empty() == false {
        let utxo_count    = data.utxo_count.parse::<u64>().unwrap();
        let mut utxo_size = utxo_count as f64 * OUTPUT_SIZE as f64 / 1000.0 / 1000.0;
        let mut unit      = "MB";
        
        if utxo_size > 1000.0 {
            unit = "GB";
            utxo_size = utxo_size / 1000.0;
        }
        
        return format!("{} ({:.2} {})", utxo_count.to_formatted_string(&Locale::en), utxo_size, unit);
    }

    "".to_string()
}


#[get("/rpc/blockchain/kernels")]
fn kernels(dashboard: &State<Arc<Mutex<Dashboard>>>) -> String {
    let data = dashboard.lock().unwrap();

    if data.kernel_mmr_size.is_empty() == false {
        let kernel_count    = data.kernel_mmr_size.parse::<u64>().unwrap() / 2;
        let mut kernel_size = kernel_count as f64 * KERNEL_SIZE as f64 / 1000.0 / 1000.0;
        let mut unit        = "MB";

        if kernel_size > 1000.0 {
            unit = "GB";
            kernel_size = kernel_size / 1000.0;
        }
        
        return format!("{} ({:.2} {})", kernel_count.to_formatted_string(&Locale::en), kernel_size, unit);
    }
    
    "".to_string()
}

// End of HTMX routes.


// Main
#[rocket::main]
async fn main() {
    env_logger::init();

    info!("starting up.");
    
    let dash         = Arc::new(Mutex::new(Dashboard::new()));
    let dash_clone   = dash.clone();
    let blocks       = Arc::new(Mutex::new(Vec::<Block>::new()));
    let blocks_clone = blocks.clone();
    let txns         = Arc::new(Mutex::new(Transactions::new()));
    let txns_clone   = txns.clone();
    let stats        = Arc::new(Mutex::new(Statistics::new()));
    let stats_clone  = stats.clone();

    let mut ready_data  = false;
    let mut ready_stats = false;
    let mut ready_db    = false;
    let mut date        = "".to_string();
    
    // Initializing db and table
    if CONFIG.database.is_empty() == false {
        info!("initializing db.");
        let conn = database::open_db_connection(&CONFIG.database).expect("failed to open database");
        database::create_statistics_table(&conn).expect("failed to create statistics table");

        let mut s = stats.lock().unwrap();
        let mut d = dash.lock().unwrap();

        // Reading the database
        s.date     = database::read_row(&conn, "date").unwrap();
        s.hashrate = database::read_row(&conn, "hashrate").unwrap();
        s.txns     = database::read_row(&conn, "txns").unwrap();
        s.fees     = database::read_row(&conn, "fees").unwrap();
        s.utxos    = database::read_row(&conn, "utxos").unwrap();
        s.kernels  = database::read_row(&conn, "kernels").unwrap();

        // Read utxos right here, because we have it in worker::stats thread launched next day only
        if s.utxos.is_empty() == false {
            d.utxo_count = s.utxos.get(s.utxos.len() - 1).unwrap().to_string();
        }

        // Get the latest date
        if s.date.is_empty() == false {
            date = s.date.get(s.date.len() - 1).unwrap().to_string();
        }
    }

    // Collecting main data
    tokio::spawn(async move {
        loop {
            let result = worker::data(dash_clone.clone(), blocks_clone.clone(),
                                      txns_clone.clone(), stats_clone.clone()).await;
            
            match result {
                Ok(_v)  => {
                    if ready_data == false {
                        ready_data = true;
                        info!("worker::data ready.");
                    }
                },
                Err(e) => {
                    ready_data = false;
                    error!("{}", e);
                },
            }

            let date_now = format!("\"{}\"", Utc::now().format("%d-%m-%Y"));

            if date != date_now {
                date = date_now;
                let result = worker::stats(dash_clone.clone(), txns_clone.clone(),
                                           stats_clone.clone()).await;
            
                match result {
                    Ok(_v)  => {
                        if ready_stats == false {
                            ready_stats = true;
                            ready_db    = true;
                            info!("worker::stats ready.");
                        }
                    },
                    Err(e) => {
                        ready_stats = false;
                        error!("{}", e);
                    },
                }
            // Got stats from DB, indicate ready state
            } else if ready_db == false && CONFIG.database.is_empty() == false {
                info!("worker::stats ready.");
                ready_db = true;
            }

            tokio::time::sleep(Duration::from_secs(15)).await;
        }
    });
    
    // Starting Rocket engine.
    let _ = rocket::build()
            .manage(dash)
            .manage(blocks)
            .manage(txns)
            .manage(stats)
            .mount("/", routes![index, peers_inbound, peers_outbound, sync_status, market_supply,
                                inflation_rate, volume_usd, volume_btc, price_usd, price_btc,
                                mcap_usd, mcap_btc,latest_height, disk_usage, network_hashrate,
                                network_difficulty, mempool_txns, mempool_stem, txns_count_1h,
                                txns_count_24h, block_list, block_link, block_link_color,
                                block_time, block_txns, block_inputs, block_outputs, block_fees,
                                block_size, block_weight, block_details_by_height, block_header_by_hash,
                                soft_supply, production_cost, reward_ratio, breakeven_cost,
                                last_block_age, block_list_by_height, block_list_index, search, kernel,
                                output, api_owner, api_foreign, stats, unspent_outputs, kernels,
                                emission, api_overview, donate, supply_raw])
            .mount("/static", FileServer::from("static"))
            .attach(Template::custom(|engines| {engines.tera.register_filter("separate_with_commas", separate_with_commas)}))
            .launch()
            .await;
}

