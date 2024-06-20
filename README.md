# Grincoin.org (GRIN) Blockchain Explorer
Blockchain explorer for Grin cryptocurrency.

## What is Grin?
Grin is the very first, simple and fair MimbleWimble blockchain implementation.

- Scalable, privacy-preserving blockchain.
- Fair and verifiable coin distribution.
- Not controlled by any company, foundation or individual.

## Prerequisites

- Rust: https://www.rust-lang.org/tools/install.
- Grin node: https://github.com/mimblewimble/grin. You need to enable archival mode, so the explorer can see all the blocks, otherwise only the recent blocks can be explored.


## Installation

1. Clone repository: `git clone https://github.com/aglkm/grin-explorer.git`
2. Build explorer:
   ```
   cd grin-explorer
   cargo build --release
   ```
4. Run executable: `RUST_LOG=rocket=warn,grin_explorer ./target/release/grin-explorer`

   You will see the following output:

   ```
   [2024-06-19T13:12:34Z INFO  grin_explorer] starting up.
   [2024-06-19T13:12:34Z WARN  rocket::launch] 🚀 Rocket has launched from http://127.0.0.1:8000
   [2024-06-19T13:12:34Z INFO  grin_explorer] ready.
   ```

5. Open explorer in your browser: http://127.0.0.1:8000
