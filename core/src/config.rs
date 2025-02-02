use std::{fs::File, io::Read, path::Path};

use anyhow::{Error, Result};
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub filter_keywords: Vec<String>,
}

pub fn load_config<P: AsRef<Path>>(path: P) -> Result<Config, Error> {
    let mut file = File::open(path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    let config: Config = toml::from_str(&contents)?;

    Ok(config)
}
