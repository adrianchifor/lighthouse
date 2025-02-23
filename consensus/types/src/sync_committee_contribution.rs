use super::{AggregateSignature, EthSpec, SignedRoot};
use crate::slot_data::SlotData;
use crate::{test_utils::TestRandom, BitVector, Hash256, Slot, SyncCommitteeMessage};
use safe_arith::ArithError;
use serde::{Deserialize, Serialize};
use ssz_derive::{Decode, Encode};
use test_random_derive::TestRandom;
use tree_hash_derive::TreeHash;

#[derive(Debug, PartialEq)]
pub enum Error {
    SszTypesError(ssz_types::Error),
    AlreadySigned(usize),
    SubnetCountIsZero(ArithError),
}

/// An aggregation of `SyncCommitteeMessage`s, used in creating a `SignedContributionAndProof`.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    Encode,
    Decode,
    TreeHash,
    TestRandom,
    arbitrary::Arbitrary,
)]
#[serde(bound = "E: EthSpec")]
#[arbitrary(bound = "E: EthSpec")]
pub struct SyncCommitteeContribution<E: EthSpec> {
    pub slot: Slot,
    pub beacon_block_root: Hash256,
    #[serde(with = "serde_utils::quoted_u64")]
    pub subcommittee_index: u64,
    pub aggregation_bits: BitVector<E::SyncSubcommitteeSize>,
    pub signature: AggregateSignature,
}

impl<E: EthSpec> SyncCommitteeContribution<E> {
    /// Create a `SyncCommitteeContribution` from:
    ///
    /// - `message`: A single `SyncCommitteeMessage`.
    /// - `subcommittee_index`: The subcommittee this contribution pertains to out of the broader
    ///     sync committee. This can be determined from the `SyncSubnetId` of the gossip subnet
    ///     this message was seen on.
    /// - `validator_sync_committee_index`: The index of the validator **within** the subcommittee.
    pub fn from_message(
        message: &SyncCommitteeMessage,
        subcommittee_index: u64,
        validator_sync_committee_index: usize,
    ) -> Result<Self, Error> {
        let mut bits = BitVector::new();
        bits.set(validator_sync_committee_index, true)
            .map_err(Error::SszTypesError)?;
        Ok(Self {
            slot: message.slot,
            beacon_block_root: message.beacon_block_root,
            subcommittee_index,
            aggregation_bits: bits,
            signature: AggregateSignature::from(&message.signature),
        })
    }

    /// Aggregate another `SyncCommitteeContribution` into this one.
    ///
    /// The aggregation bitfields must be disjoint, and the data must be the same.
    pub fn aggregate(&mut self, other: &Self) {
        debug_assert_eq!(self.slot, other.slot);
        debug_assert_eq!(self.beacon_block_root, other.beacon_block_root);
        debug_assert_eq!(self.subcommittee_index, other.subcommittee_index);

        self.aggregation_bits = self.aggregation_bits.union(&other.aggregation_bits);
        self.signature.add_assign_aggregate(&other.signature);
    }
}

impl SignedRoot for Hash256 {}

/// This is not in the spec, but useful for determining uniqueness of sync committee contributions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Encode, Decode, TreeHash, TestRandom)]
pub struct SyncContributionData {
    pub slot: Slot,
    pub beacon_block_root: Hash256,
    pub subcommittee_index: u64,
}

impl SyncContributionData {
    pub fn from_contribution<E: EthSpec>(signing_data: &SyncCommitteeContribution<E>) -> Self {
        Self {
            slot: signing_data.slot,
            beacon_block_root: signing_data.beacon_block_root,
            subcommittee_index: signing_data.subcommittee_index,
        }
    }
}

impl<E: EthSpec> SlotData for SyncCommitteeContribution<E> {
    fn get_slot(&self) -> Slot {
        self.slot
    }
}

impl<E: EthSpec> SlotData for &SyncCommitteeContribution<E> {
    fn get_slot(&self) -> Slot {
        self.slot
    }
}

impl SlotData for SyncContributionData {
    fn get_slot(&self) -> Slot {
        self.slot
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::*;

    ssz_and_tree_hash_tests!(SyncCommitteeContribution<MainnetEthSpec>);
}
