use serde::{
    Deserialize,
    Serialize
};

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct GeyserPluginSologgerConfig {
    pub(crate) logstash_config_location: String,
    pub(crate) log_geyser_functions: bool
}