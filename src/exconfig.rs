use config::Config;
use std::fs;
use lazy_static::lazy_static;

use crate::data::ExplorerConfig;


// Static explorer config structure
lazy_static! {
    pub static ref CONFIG: ExplorerConfig = {
        let mut cfg = ExplorerConfig::new();
        let toml    = Config::builder().add_source(config::File::with_name("Explorer")).build().unwrap();

        // Mandatory settings
        cfg.host          = toml.get_string("host").unwrap();
        cfg.proto         = toml.get_string("proto").unwrap();
        cfg.coingecko_api = toml.get_string("coingecko_api").unwrap();
        cfg.public_api    = toml.get_string("public_api").unwrap();
        
        // Optional settings
        match toml.get_string("port") {
            Ok(v)   => cfg.port = v,
            Err(_e) => {},
        }
        
        match toml.get_string("user") {
            Ok(v)   => cfg.user = v,
            Err(_e) => {},
        }
        
        match toml.get_string("api_secret_path") {
            Ok(v)   => cfg.api_secret_path = v,
            Err(_e) => {},
        }
        
        match toml.get_string("foreign_api_secret_path") {
            Ok(v)   => cfg.foreign_api_secret_path = v,
            Err(_e) => {},
        }
        
        match toml.get_string("grin_dir") {
            Ok(v)   => cfg.grin_dir = v,
            Err(_e) => {},
        }
       
        match toml.get_array("external_nodes") {
            Ok(nodes)   => {
                               for endpoint in nodes.clone() {
                                   cfg.external_nodes.push(endpoint.into_string().unwrap());
                               }
                           },
            Err(_e) => {},
        }
        
        match toml.get_string("database") {
            Ok(v)   => cfg.database = v,
            Err(_e) => {},
        }

        if cfg.api_secret_path.is_empty() == false {
            cfg.api_secret = fs::read_to_string(format!("{}", shellexpand::tilde(&cfg.api_secret_path))).unwrap();
        }

        if cfg.foreign_api_secret_path.is_empty() == false {
            cfg.foreign_api_secret = fs::read_to_string(format!("{}", shellexpand::tilde(&cfg.foreign_api_secret_path))).unwrap();
        }

        if cfg.grin_dir.is_empty() == false {
            cfg.grin_dir = format!("{}", shellexpand::tilde(&cfg.grin_dir));
        }
        
        cfg
    };
}

