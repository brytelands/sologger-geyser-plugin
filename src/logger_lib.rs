#![cfg_attr(debug_assertions, allow(dead_code, unused_imports))]
use std::path::Path;
use log4rs::init_file;
use log::{debug, error};

use crate::sologger_config::SologgerConfig;

pub fn init_logger(sologger_config: &SologgerConfig) {
    #[cfg(feature = "enable_logstash")]
    init_logger_logstash(sologger_config);
    #[cfg(feature = "enable_otel")]
    init_log4rs(&sologger_config.log4rs_config_location).expect("Error initializing log4rs for enable_otel feature");
    #[cfg(feature = "enable_otel")]
    init_logger_otel(sologger_config);
}

#[cfg(feature = "enable_logstash")]
pub fn init_logger_logstash(sologger_config: &SologgerConfig) {
    if !Path::new(&sologger_config.log4rs_config_location).exists() {
        panic!("Log4rs config file not found");
    };
    sologger_log_transport::logstash_lib::init_logstash_logger(
        &sologger_config.log4rs_config_location,
    )
    .expect("Logger not initialized");
}

#[cfg(feature = "enable_otel")]
pub fn init_logger_otel(sologger_config: &SologgerConfig) {

    let rt =  tokio::runtime::Runtime::new().unwrap();
    if !Path::new(&sologger_config.opentelemetry_config_location).exists() {
        panic!("OTel config file not found {}", &sologger_config.opentelemetry_config_location);
    };

    let config = sologger_config.clone();
    rt.spawn(async move {
        println!("Starting OTel in Tokio RT");

        let config = sologger_log_transport::opentelemetry_lib::get_otel_config(&config.opentelemetry_config_location);
        let _ = sologger_log_transport::opentelemetry_lib::init_logs_opentelemetry(&config);
    });
}

pub fn init_log4rs(log4rs_config_location: &String) -> anyhow::Result<()> {
    match {
        init_file(
            log4rs_config_location,
            log4rs::config::Deserializers::default(),
        )
    } {
        Ok(_) => {
            debug!("Logger initialized with logstash successfully")
        }
        Err(err) => {
            error!("init_logstash_logger not initialized! {}", err.to_string())
        }
    };
    Ok(())
}


#[cfg(test)]
mod tests {
    use crate::logger_lib::init_logger;
    use crate::sologger_config::SologgerConfig;
    use serde_json::json;

    #[test]
    pub fn init_logger_test() {
        //TODO fix for config location
        let config = json!(
            {
                "log4rsConfigLocation": "./config/log4rs-config.yml",
                "rpcUrl": "wss://api.mainnet-beta.solana.com",
                "programsSelector" : {
                    "programs" : ["*"]
                },
                "accountDataNotificationsEnabled": false,
                "transactionNotificationsEnabled": true,
                "logProcessorWorkerThreadCount": 1

            }
        );

        let sologger_config = serde_json::from_value::<SologgerConfig>(config).unwrap();
        init_logger(&sologger_config);
    }
}
