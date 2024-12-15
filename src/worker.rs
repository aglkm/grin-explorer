use chrono::Utc;
use std::sync::{Arc, Mutex};

use crate::data::Block;
use crate::data::Dashboard;
use crate::data::Statistics;
use crate::data::Transactions;
use crate::database;
use crate::exconfig::CONFIG;
use crate::requests;


// Collecting main data.
pub async fn data(dash: Arc<Mutex<Dashboard>>, blocks: Arc<Mutex<Vec<Block>>>,
                  txns: Arc<Mutex<Transactions>>, stats: Arc<Mutex<Statistics>>) -> Result<(), anyhow::Error> {
    let _ = requests::get_status(dash.clone()).await?;
    let _ = requests::get_mempool(dash.clone()).await?;
    let _ = requests::get_connected_peers(dash.clone(), stats.clone()).await?;
    let _ = requests::get_market(dash.clone()).await?;
    let _ = requests::get_disk_usage(dash.clone())?;
    let _ = requests::get_mining_stats(dash.clone()).await?;
    let _ = requests::get_recent_blocks(dash.clone(), blocks.clone()).await?;
    let _ = requests::get_txn_stats(dash.clone(), txns.clone()).await?;

    Ok(())
}

// Collecting statistics.
pub async fn stats(dash: Arc<Mutex<Dashboard>>, txns: Arc<Mutex<Transactions>>, stats: Arc<Mutex<Statistics>>) -> Result<(), anyhow::Error> {

    let _ = requests::get_unspent_outputs(dash.clone()).await?;

    let mut stats = stats.lock().unwrap();
    let dash      = dash.lock().unwrap();
    let txns      = txns.lock().unwrap();

    stats.date.push(format!("\"{}\"", Utc::now().format("%d-%m-%Y")));
    stats.hashrate.push(dash.hashrate_kgs.clone());
    stats.txns.push(txns.period_24h.clone());
    stats.fees.push(txns.fees_24h.clone());
    stats.utxos.push(dash.utxo_count.clone());

    let mut kernel_count = 0;

    if dash.kernel_mmr_size.is_empty() == false {
        kernel_count = dash.kernel_mmr_size.parse::<u64>().unwrap() / 2;
        stats.kernels.push(kernel_count.to_string());
    }

    if CONFIG.database.is_empty() == false {
        // Open the database
        let conn = database::open_db_connection(&CONFIG.database).expect("failed to open database");

        //Insert new data into the database
        conn.execute(
            "INSERT OR IGNORE INTO statistics (date, hashrate, txns, fees, utxos, kernels) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            (&format!("\"{}\"", Utc::now().format("%d-%m-%Y")), &dash.hashrate_kgs.clone(), &txns.period_24h.clone(), &txns.fees_24h.clone(), &dash.utxo_count.clone(), &kernel_count.to_string()),
        )?;
    }

    Ok(())
}

