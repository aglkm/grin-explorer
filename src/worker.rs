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
    let _ = requests::get_mining_stats(dash.clone()).await?;
    let _ = requests::get_recent_blocks(dash.clone(), blocks.clone()).await?;
    let _ = requests::get_txn_stats(dash.clone(), txns.clone()).await?;

    let mut stats = stats.lock().unwrap();
    let dash      = dash.lock().unwrap();
    let txns      = txns.lock().unwrap();

    // Every day push new stats into the vectors
    if let Some(v) = stats.date.last() {
        // If new day, push the stats
        if *v != format!("\"{}\"", Utc::now().format("%d-%m-%Y")) {
            if stats.date.len() == 30 {
                stats.date.remove(0);
                stats.hashrate.remove(0);
                stats.txns.remove(0);
                stats.fees.remove(0);
            }
            stats.date.push(format!("\"{}\"", Utc::now().format("%d-%m-%Y")));
            stats.hashrate.push(dash.hashrate_kgs.clone());
            stats.txns.push(txns.period_24h.clone());
            stats.fees.push(txns.fees_24h.clone());
        }
    } else {
        // Vector is empty, pushing current stats
        stats.date.push(format!("\"{}\"", Utc::now().format("%d-%m-%Y")));
        stats.hashrate.push(dash.hashrate_kgs.clone());
        stats.txns.push(txns.period_24h.clone());
        stats.fees.push(txns.fees_24h.clone());
    }

    Ok(())
}

