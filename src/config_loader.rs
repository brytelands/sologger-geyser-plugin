use std::env;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use log::trace;
use anyhow::Result;
use sologger_log_context::programs_selector::ProgramsSelector;
use crate::sologger_config::SologgerConfig;

const DEFAULT_CONFIG_PATH: &str = "/config/local/sologger-config.json";
const DEFAULT_DIR: &str = "/";

pub(crate) fn load_config() -> Result<(SologgerConfig, ProgramsSelector)> {
    let default_config = get_default_config();
    let sologger_config_path = env::var("SOLOGGER_APP_CONFIG_LOC").unwrap_or(default_config.to_string());

    trace!("sologger_config_path: {}", sologger_config_path);
    let mut file = File::open(Path::new(sologger_config_path.as_str()))?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .expect("Failed to read contents of sologger-config.json");

    let result: serde_json::Value = serde_json::from_str(&contents).unwrap();
    trace!("SologgerConfig: {}", result.to_string());
    let programs_selector = create_programs_selector_from_config(&result);
    let sologger_config = serde_json::from_str(&contents).map_err(|_err| ConfigError::Loading)?;

    Ok((sologger_config, programs_selector))
}

fn get_default_config() -> String {
    let parent_dir = get_parent_dir();
    format!("{}{}", parent_dir, DEFAULT_CONFIG_PATH)
}

fn get_parent_dir() -> String {
    match env::current_dir() {
        Ok(path_buf) => match path_buf.parent() {
            Some(path) => path.display().to_string(),
            None => String::from(DEFAULT_DIR),
        },
        Err(_) => String::from(DEFAULT_DIR),
    }
}

fn create_programs_selector_from_config(config: &serde_json::Value) -> ProgramsSelector {
    let programs_selector = &config["programsSelector"];

    if programs_selector.is_null() {
        ProgramsSelector::default()
    } else {
        let programs = &programs_selector["programs"];
        let programs: Vec<String> = if programs.is_array() {
            programs
                .as_array()
                .unwrap()
                .iter()
                .map(|val| val.as_str().unwrap().to_string())
                .collect()
        } else {
            Vec::default()
        };

        ProgramsSelector::new(&programs)
    }
}

#[derive(Debug)]
enum ConfigError {
    Loading,
}

impl std::error::Error for ConfigError {}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        use ConfigError::*;
        match self {
            Loading => write!(f, "Loading"),
        }
    }
}