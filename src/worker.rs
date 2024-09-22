use chrono::Utc;
use std::sync::{Arc, Mutex};

use crate::data::Block;
use crate::data::Dashboard;
use crate::data::Statistics;
use crate::data::Transactions;

use crate::requests;


// Tokio Runtime Worker.
// Collecting all the data.
pub async fn run(dash: Arc<Mutex<Dashboard>>, blocks: Arc<Mutex<Vec<Block>>>,
                 txns: Arc<Mutex<Transactions>>, stats: Arc<Mutex<Statistics>>) -> Result<(), anyhow::Error> {
    let _ = requests::get_status(dash.clone()).await?;
    let _ = requests::get_mempool(dash.clone()).await?;
    let _ = requests::get_connected_peers(dash.clone(), stats.clone()).await?;
    let _ = requests::get_market(dash.clone()).await?;
            requests::get_disk_usage(dash.clone())?;
    let _ = requests::get_mining_stats(dash.clone(), stats.clone()).await?;
    let _ = requests::get_recent_blocks(dash.clone(), blocks.clone()).await?;
    let _ = requests::get_txn_stats(dash.clone(), txns.clone(), stats.clone()).await?;

    let mut stats = stats.lock().unwrap();

    // Push new date and restart timer every 24H
    if stats.timing == 0 || stats.timing >= 86400 {
        stats.date.push(format!("\"{}\"", Utc::now().format("%d-%m-%Y")));
        stats.timing = 0;
    }

    // Increasing timing by 15 seconds (data update period)
    stats.timing = stats.timing + 15;

    Ok(())
}

