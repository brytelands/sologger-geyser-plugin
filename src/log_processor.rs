use anyhow::Result;
use log::{error, info};
use solana_geyser_plugin_interface::geyser_plugin_interface::ReplicaTransactionInfoV2;
use solana_transaction_status::option_serializer::OptionSerializer;
use sologger_log_context::programs_selector::ProgramsSelector;
use sologger_log_context::sologger_log_context::LogContext;

use crate::inner_transaction::ReplicaTransactionInfo;

pub fn from_rpc_response(
    transaction: &ReplicaTransactionInfo,
    program_selector: ProgramsSelector,
    slot: u64
) -> Result<Vec<LogContext>> {
    let log_contexts = match &transaction.meta.log_messages {
        OptionSerializer::Some(logs) => {
            let transaction_error = transaction.meta.status.clone().err()
                .map_or_else(|| "".to_string(), |err| err.to_string());
            let sig = transaction.signature.to_string();
            LogContext::parse_logs(
                &logs,
                transaction_error,
                &program_selector,
                slot,
                sig,
            )
        }
        OptionSerializer::None | OptionSerializer::Skip => Vec::new(),
    };
    Ok(log_contexts)
}


pub fn log_contexts_from_logs(log_contexts: &Vec<LogContext>) -> Result<()> {
    for log_context in log_contexts {
        if log_context.has_errors() {
            error!(target: "sologger_geyser_plugin::log_processor::error", "{}", &log_context.to_json());
        } else {
            info!(target: "sologger_geyser_plugin::log_processor::info", "{}", &log_context.to_json());
        }
    }
    Ok(())
}