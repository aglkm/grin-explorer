# Grincoin.org (GRIN) Blockchain Explorer
Blockchain explorer for Grin cryptocurrency.

## What is Grin?
Grin is the very first, simple and fair MimbleWimble blockchain implementation.

- Scalable, privacy-preserving blockchain.
- Fair and verifiable coin distribution.
- Not controlled by any company, foundation or individual.

## Prerequisites

- OS packages:
     + `sudo apt update`
     + `sudo apt install rustup build-essential pkg-config libssl-dev tor`
- Rust: https://www.rust-lang.org/tools/install.
     + curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
- SQLite: `sudo apt install sqlite3 libsqlite3-dev`
- Grin node: https://github.com/mimblewimble/grin. You need to enable archival mode, so the explorer can see all the blocks, otherwise only the recent blocks can be explored.
     + mkdir grin/main
     + cd grin/main
     + wget "URL grin tar gz"
     + tar -xvf grin-node...gz
     + ./grin server config
     + Edit grin-server.toml => archive_mode = true
     + tmux
     + ./grin 

## Installation

1. Clone repository: `git clone https://github.com/aglkm/grin-explorer.git`
2. Build explorer:
   ```
   cd grin-explorer
   cargo build --release
   ```
   Edit Explorer.toml => grin_dir = "~/grin" (remove the dot)
4. Run executable: `RUST_LOG=rocket=warn,grin_explorer ./target/release/grin-explorer`

   You will see the following output:

   ```
   [2024-09-30T13:30:02Z INFO  grin_explorer] starting up.
   [2024-09-30T13:30:02Z WARN  rocket::launch] 🚀 Rocket has launched from http://127.0.0.1:8000
   [2024-09-30T13:30:03Z INFO  grin_explorer] worker::data ready.
   [2024-09-30T13:30:10Z INFO  grin_explorer] worker::stats ready.
   ```

5. Open explorer in your browser: http://127.0.0.1:8000
