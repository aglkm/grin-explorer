use rusqlite::{Connection, Result};

pub fn open_db_connection(db_name: &str) -> Result<Connection> {
    let conn = Connection::open(db_name)?;

    Ok(conn)
}

pub fn create_statistics_table(conn: &Connection) -> Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS statistics (
            id       INTEGER PRIMARY KEY,
            date     TEXT NOT NULL UNIQUE,
            hashrate TEXT NOT NULL,
            txns     TEXT NOT NULL,
            fees     TEXT NOT NULL,
            utxos    TEXT NOT NULL,
            kernels  TEXT NOT NULL
        )",
        (), // empty list of parameters.
    )?;

    Ok(())
}

pub fn read_row(conn: &Connection, row_name: &str) -> Result<Vec<String>> {
    let sql = format!("SELECT {} FROM statistics ORDER BY id", row_name);
    let mut stmt = conn.prepare(&sql)?;
        
    let data_iter = stmt
        .query_map([], |row| {
            row.get(0)
        }).unwrap();

    // Collect all the results into a vector of strings
    let data: Vec<String> = data_iter.collect::<Result<Vec<_>, _>>().unwrap();

    Ok(data)
}

