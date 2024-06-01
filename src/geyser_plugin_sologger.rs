use std::{env, fs::File, io::Read, thread};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::Duration;

use anyhow::Result as AnyResult;
use crossbeam_deque::{Injector, Steal, Worker};
use log::{error, info, trace};
use solana_geyser_plugin_interface::geyser_plugin_interface::{ReplicaAccountInfoV2, ReplicaTransactionInfoV2};
use solana_transaction_status::option_serializer::OptionSerializer;
use sologger_log_context::programs_selector::ProgramsSelector;
use sologger_log_context::sologger_log_context::LogContext;

use {
    solana_geyser_plugin_interface::geyser_plugin_interface::{
        GeyserPlugin, GeyserPluginError,
        ReplicaAccountInfoVersions, ReplicaBlockInfoVersions, ReplicaTransactionInfoVersions,
        SlotStatus,
    },
    std::fmt::{
        Debug, Formatter,
    },
};

use crate::config_loader;
use crate::geyser_plugin_sologger_config::GeyserPluginSologgerConfig;
use crate::inner_transaction::ReplicaTransactionInfo;
use crate::log_processor::{from_rpc_response, log_contexts_from_logs};
use crate::logger_lib::init_logger;
use crate::sologger_config::SologgerConfig;

pub struct PluginContext {
    pub(crate) programs_selector: ProgramsSelector,
    pub(crate) injector: Arc<Injector<Task>>,
    pub sologger_config: SologgerConfig,
    pub running: Arc<AtomicBool>,
    pub handles: Arc<Mutex<Vec<JoinHandle<()>>>>,
}

impl PluginContext {
    pub(crate) fn default() -> PluginContext {
        PluginContext {
            programs_selector: ProgramsSelector::default(),
            injector: Arc::new(Default::default()),
            sologger_config: SologgerConfig::default(),
            running: Arc::new(Default::default()),
            handles: Arc::new(Mutex::new(Vec::new()))
        }
    }

    pub fn join_threads(&self) {
        let handles = {
            let mut handles = self.handles.lock().unwrap();
            std::mem::take(&mut *handles) // empty the Vec inside the Mutex
        };
        for handle in handles {
            handle.join().unwrap();
        }
    }
}

/// This is the main object returned by our dynamic library in entrypoint.rs
pub struct SologgerGeyserPlugin {
    pub context: PluginContext
}

pub struct Task {
    slot: u64,
    transaction_info: Option<ReplicaTransactionInfo>,
    programs_select: ProgramsSelector,
}

/// Implementation of GeyserPlugin trait/interface
/// https://docs.rs/solana-geyser-plugin-interface/latest/solana_geyser_plugin_interface/geyser_plugin_interface/trait.GeyserPlugin.html
impl GeyserPlugin for SologgerGeyserPlugin {
    fn name(&self) -> &'static str {
        "GeyserPluginHook"
    }
    
    /// Lifecycle: the plugin has been loaded by the system
    /// used for doing whatever initialization is required by the plugin.
    /// The _config_file contains the name of the
    /// of the config file. The config must be in JSON format and
    /// include a field "libpath" indicating the full path
    /// name of the shared library implementing this interface.
    fn on_load(&mut self, _config_file: &str) -> solana_geyser_plugin_interface::geyser_plugin_interface::Result<()> {
        let (sologger_config, program_selector) = config_loader::load_config().expect("Error loading sologger config");
        let config = sologger_config.clone();
        self.context.sologger_config = sologger_config;
        self.context.programs_selector = program_selector;

        let _logger = init_logger(&config);

        info!("Programs Selected: {:?}", &self.context.programs_selector);

        // Create an atomic flag for shutdown signal
        let running = Arc::new(AtomicBool::new(true));
        let running_clone = Arc::clone(&running);
        self.context.running = Arc::clone(&running_clone);

        // Create an injector for task management
        self.context.injector = Arc::new(Injector::new());

        // Start worker threads for processing tasks
        let thread_count = config.log_processor_worker_thread_count as usize;
        info!("Starting {:?} log processing worker threads...", &thread_count);
        for _ in 0..thread_count {
            let injector_clone = Arc::clone(&self.context.injector);
            let running_worker = Arc::clone(&running_clone);
            let handle = thread::spawn(move || {
                Self::worker_thread(injector_clone, running_worker);
            });
            self.context.handles.lock().unwrap().push(handle);
        }

        info!("sologger-geyser-plugin loaded");

        Ok(())
    }

    /// Lifecycle: the plugin will be unloaded by the plugin manager
    /// Note: Do any cleanup necessary.
    fn on_unload(&mut self) {
        info!("[on_unload] - Flushing logger");
        let _ = &self.context.running.store(false, Ordering::SeqCst);
        self.context.join_threads();
        log::logger().flush();
    }

    /// Event: an account has been updated at slot
    /// - When `is_startup` is true, it indicates the account is loaded from
    /// snapshots when the validator starts up.
    /// - When `is_startup` is false, the account is updated during transaction processing.
    /// Note: The account is versioned, so you can decide how to handle the different
    /// implementations.
    fn update_account(&self, account: ReplicaAccountInfoVersions, slot: u64, _is_startup: bool) -> solana_geyser_plugin_interface::geyser_plugin_interface::Result<()> {
        match account {
            ReplicaAccountInfoVersions::V0_0_1(_) => {
                return Err(GeyserPluginError::AccountsUpdateError { msg: "ReplicaAccountInfoVersions::V0_0_1 it not supported".to_string() });
            }
            ReplicaAccountInfoVersions::V0_0_2(account) => {
                let _acc = format!(
                    "V0_0_2 pubkey: {}, owner: {}",
                    bs58::encode(account.pubkey).into_string(),
                    bs58::encode(account.owner).into_string(),
                );
                info!(target: "sologger_geyser_plugin::geyser_plugin_sologger::update_account", "[update_account] - account: {:#?}, slot:{:#?}, is_startup:{:#?}", _acc, slot, _is_startup);
            }
            ReplicaAccountInfoVersions::V0_0_3(account) => {
                let _acc = format!(
                    "Updated V0_0_3 account pubkey: {}, owner: {}",
                    bs58::encode(account.pubkey).into_string(),
                    bs58::encode(account.owner).into_string(),
                );
                info!(target: "sologger_geyser_plugin::geyser_plugin_sologger::update_account", "[update_account] - account: {:#?}, slot:{:#?}, is_startup:{:#?}", _acc, slot, _is_startup);
            }
        }
        Ok(())
    }

    // Lifecycle: called when all accounts have been notified when the validator
    // restores the AccountsDb from snapshots at startup.
    fn notify_end_of_startup(&self) -> solana_geyser_plugin_interface::geyser_plugin_interface::Result<()> {
        info!("[notify_end_of_startup]");
        Ok(())
    }

    // Event: a slot status is updated.
    fn update_slot_status(&self, _slot: u64, _parent: Option<u64>, _status: SlotStatus) -> solana_geyser_plugin_interface::geyser_plugin_interface::Result<()> {
        info!(target: "sologger_geyser_plugin::geyser_plugin_sologger::update_slot_status", "[update_slot_status], slot:{:#?}, parent:{:#?}, status:{:#?}", _slot, _parent, _status);
        Ok(())
    }

    /// Event: a transaction is updated at a slot.
    #[allow(unused_variables)]
    fn notify_transaction(&self, transaction: ReplicaTransactionInfoVersions, slot: u64) -> solana_geyser_plugin_interface::geyser_plugin_interface::Result<()> {
        let replication_transaction_info = ReplicaTransactionInfo::from((transaction, slot));
        let task = Task {
            slot,
            transaction_info: Some(replication_transaction_info),
            programs_select: self.context.programs_selector.clone(),
        };
        self.context.injector.push(task);

        Ok(())
    }

    fn notify_block_metadata(&self, blockinfo: ReplicaBlockInfoVersions) -> solana_geyser_plugin_interface::geyser_plugin_interface::Result<()> {
        match blockinfo {
            ReplicaBlockInfoVersions::V0_0_1(_blockinfo) => {
                info!(target: "sologger_geyser_plugin::geyser_plugin_sologger::notify_block_metadata", "[notify_block_metadata], block_info:{:#?}", _blockinfo);
            }
            ReplicaBlockInfoVersions::V0_0_2(_blockinfo) => {
                info!(target: "sologger_geyser_plugin::geyser_plugin_sologger::notify_block_metadata", "[notify_block_metadata], block_info:{:#?}", _blockinfo);
            }
            ReplicaBlockInfoVersions::V0_0_3(_blockinfo) => {
                info!(target: "sologger_geyser_plugin::geyser_plugin_sologger::notify_block_metadata", "[notify_block_metadata], block_info:{:#?}", _blockinfo);
            }
        }
        Ok(())
    }

    fn account_data_notifications_enabled(&self) -> bool {
        trace!("[account_data_notifications_enabled] - are account data notifications enabled: true");
        self.context.sologger_config.account_data_notifications_enabled
    }

    fn transaction_notifications_enabled(&self) -> bool {
        trace!("[transaction_notifications_enabled] - are transaction notifications enabled: true");
        self.context.sologger_config.transaction_notifications_enabled
    }
}

impl SologgerGeyserPlugin {
    fn worker_thread(injector: Arc<Injector<Task>>, running: Arc<AtomicBool>) {
        // Create a worker deque for this thread
        let worker = Worker::new_fifo();
        let stealer = worker.stealer();
        info!("Worker thread started");

        while running.load(Ordering::SeqCst) {
            if let Some(task) = worker.pop() {
                Self::process_task(task);
            } else if let Steal::Success(task) = injector.steal() {
                Self::process_task(task);
            } else if let Steal::Success(task) = stealer.steal() {
                Self::process_task(task);
            } else {
                // Sleep for a short duration to avoid busy-waiting
                thread::sleep(Duration::from_millis(10));
            }
        }
        info!("Worker thread shutting down");
    }

    fn process_task(task: Task) {
        // Process the task (example: log the slot number)
        if let Some(transaction_info) = task.transaction_info {
            trace!("Processing transaction info at slot: {}", task.slot);
            // Process the transaction info here
            match transaction_info.meta.log_messages {
                OptionSerializer::Some(ref log_messages) => {
                    if log_messages.is_empty() {
                        return;
                    }
                    let log_context_result = from_rpc_response(&transaction_info, task.programs_select, task.slot);
                    match log_context_result {
                        Ok(log_contexts) => {
                            log_contexts_from_logs(&log_contexts).expect("Error logging log contexts");
                        }
                        Err(_) => { error!("Error occurred logging the log contexts") }
                    }
                }
                OptionSerializer::None => {}
                OptionSerializer::Skip => {}
            }
        }
    }
}

/// Also required by GeyserPlugin trait
impl Debug for SologgerGeyserPlugin {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "GeyserPluginHook")
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use solana_transaction_status::TransactionStatusMeta;

    use {
        solana_sdk::{
            hash::Hash,
            signature::{Keypair, Signature, Signer},
            system_transaction,
            transaction::{
                SanitizedTransaction, SimpleAddressLoader, Transaction, VersionedTransaction,
            },
        },
        super::*,
    };

    fn build_test_transaction_legacy() -> Transaction {
        let keypair1 = Keypair::new();
        let pubkey1 = keypair1.pubkey();
        let zero = Hash::default();
        system_transaction::transfer(&keypair1, &pubkey1, 42, zero)
    }

    #[test]
    fn notify_transaction_test() {
        solana_logger::setup_with_default("info");

        let mut logs: Vec<String> = vec![];
        logs.push("Program 9RX7oz3WN5VRTqekBBHBvEJFVMNRnrCmVy7S6B6S5oU7 invoke [1]".to_string());
        logs.push("Program log: Instruction: Initialize".to_string());
        logs.push("Program 11111111111111111111111111111111 invoke [2]".to_string());
        logs.push("Program 11111111111111111111111111111111 success".to_string());
        logs.push("Program log: Initialized new event. Current value".to_string());
        logs.push("Program 9RX7oz3WN5VRTqekBBHBvEJFVMNRnrCmVy7S6B6S5oU7 consumed 59783 of 200000 compute units".to_string());
        logs.push("Program 9RX7oz3WN5VRTqekBBHBvEJFVMNRnrCmVy7S6B6S5oU7 success".to_string());

        let signature = Signature::from([1u8; 64]);

        let message_hash = Hash::new_unique();
        let transaction = build_test_transaction_legacy();

        let transaction = VersionedTransaction::from(transaction);

        let transaction = SanitizedTransaction::try_create(
            transaction,
            message_hash,
            Some(true),
            SimpleAddressLoader::Disabled,
        )
            .unwrap();

        let transaction_status_meta = TransactionStatusMeta {
            status: Ok(()),
            fee: 0,
            pre_balances: vec![],
            post_balances: vec![],
            inner_instructions: None,
            log_messages: Option::from(logs),
            pre_token_balances: None,
            post_token_balances: None,
            rewards: None,
            loaded_addresses: Default::default(),
            return_data: None,
            compute_units_consumed: None,
        };

        let transaction_info_v2 = ReplicaTransactionInfoV2 {
            signature: &Default::default(),
            is_vote: false,
            transaction: &(transaction),
            transaction_status_meta: &transaction_status_meta,
            index: 0,
        };

        let programs_selector = ProgramsSelector::new(&["9RX7oz3WN5VRTqekBBHBvEJFVMNRnrCmVy7S6B6S5oU7".to_string()]);
        let mut geyser_logstash_plugin = SologgerGeyserPlugin {
            context: PluginContext {
                programs_selector,
                injector: Arc::new(Default::default()),
                sologger_config: Default::default(),
                running: Arc::new(Default::default()),
                handles: Arc::new(Mutex::new(vec![])),
            },
        };
        
        ReplicaTransactionInfoVersions::V0_0_2(&transaction_info_v2);
        let _ = SologgerGeyserPlugin::notify_transaction(&mut geyser_logstash_plugin, ReplicaTransactionInfoVersions::V0_0_2(&transaction_info_v2), 1u64);
    }
}
