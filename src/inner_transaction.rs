use solana_geyser_plugin_interface::geyser_plugin_interface::ReplicaTransactionInfoVersions;
use solana_sdk::clock::UnixTimestamp;
use solana_sdk::signature::Signature;
use solana_sdk::transaction::SanitizedTransaction;
use solana_transaction_status::UiTransactionStatusMeta;

#[derive(Debug, Clone)]
pub struct ReplicaTransactionInfo {
    pub signature: Signature,
    pub is_vote: bool,
    pub transaction: SanitizedTransaction,
    pub meta: UiTransactionStatusMeta,
    pub slot: u64,
    pub block_time: Option<UnixTimestamp>,
    pub index: usize,
}

impl<'a> From<(ReplicaTransactionInfoVersions<'a>, u64)> for ReplicaTransactionInfo {
    fn from((transaction, slot): (ReplicaTransactionInfoVersions<'a>, u64)) -> Self {
        match transaction {
            ReplicaTransactionInfoVersions::V0_0_1(_transaction) => {
                unreachable!("ReplicaTransactionInfoVersions::V0_0_1 is not supported")
            }
            ReplicaTransactionInfoVersions::V0_0_2(transaction) => Self {
                signature: *transaction.signature,
                is_vote: transaction.is_vote,
                transaction: transaction.transaction.clone(),
                meta: transaction.transaction_status_meta.clone().into(),
                slot,
                block_time: None,
                index: transaction.index,
            },
        }
    }
}