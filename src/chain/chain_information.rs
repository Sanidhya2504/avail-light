//! Data structures containing the finalized state of the chain, except for its storage.
//!
//! The types provided in this module contain the state of the chain, other than its storage, that
//! has been finalized.
//!
//! > **Note**: These data structures only provide a way to communicate that finalized state, but
//! >           the existence of a [`ChainInformation`] alone does in no way mean that its content
//! >           is accurate. As an example, one use case of [`ChainInformation`] is to be written
//! >           to disk then later reloaded. It is possible for the user to modify the data on
//! >           disk, in which case the loaded [`ChanInformation`] might be erroneous.
//!
//! These data structures contain all the information that is necessary to verify the
//! authenticity (but not the correctness) of blocks that descend from the finalized block
//! contained in the structure.
//!
//! They do not, however, contain the storage of the finalized block, which is necessary to verify
//! the correctness of new blocks. It possible possible, though, for instance to download the
//! storage of the finalized block from another node. This downloaded storage can be verified
//! to make sure that it matches the content of the [`ChainInformation`].
//!
//! They also do not contain the past history of the chain. It is, however, similarly possible to
//! for instance download the history from other nodes.

use crate::{finality::grandpa, header, verify::babe};

use alloc::vec::Vec;

/// Information about the latest finalized block and state found in its ancestors.
#[derive(Debug, Clone)]
pub struct ChainInformation {
    /// SCALE encoding of the header of the highest known finalized block.
    ///
    /// Once the queue is created, it is as if you had called
    /// [`NonFinalizedTree::set_finalized_block`] with this block.
    pub finalized_block_header: header::Header,

    /// If the number in [`ChainInformation::finalized_block_header`] is superior or equal to 1,
    /// then this field must contain the slot number of the block whose number is 1 and is an
    /// ancestor of the finalized block.
    pub babe_finalized_block1_slot_number: Option<u64>,

    /// Babe epoch information about the epoch the finalized block belongs to.
    ///
    /// Must be `None` if and only if the finalized block is block #0 or belongs to epoch #0.
    pub babe_finalized_block_epoch_information:
        Option<(header::BabeNextEpoch, header::BabeNextConfig)>,

    /// Babe epoch information about the epoch right after the one the finalized block belongs to.
    ///
    /// Must be `None` if and only if the finalized block is block #0.
    pub babe_finalized_next_epoch_transition:
        Option<(header::BabeNextEpoch, header::BabeNextConfig)>,

    /// Grandpa authorities set ID of the block right after finalized block.
    ///
    /// If the finalized block is the genesis, should be 0. Otherwise,
    // TODO: document how to know this
    pub grandpa_after_finalized_block_authorities_set_id: u64,

    /// List of GrandPa authorities that need to finalize the block right after the finalized
    /// block.
    pub grandpa_finalized_triggered_authorities: Vec<header::GrandpaAuthority>,

    /// List of changes in the GrandPa authorities list that have been scheduled by blocks that
    /// are already finalized but not triggered yet. These changes will for sure happen.
    // TODO: I believe this should be an Option
    pub grandpa_finalized_scheduled_changes: Vec<FinalizedScheduledChange>,
}

impl ChainInformation {
    /// Builds the chain information corresponding to the genesis block.
    ///
    /// Must be passed a closure that returns the storage value corresponding to the given key in
    /// the genesis block storage.
    pub fn from_genesis_storage<'a>(
        genesis_storage: impl Iterator<Item = (&'a [u8], &'a [u8])> + Clone,
    ) -> Result<Self, FromGenesisStorageError> {
        let grandpa_genesis_config =
            grandpa::chain_config::GrandpaGenesisConfiguration::from_genesis_storage(|key| {
                genesis_storage
                    .clone()
                    .find(|(k, _)| *k == key)
                    .map(|(_, v)| v.to_owned())
            })
            .unwrap();

        Ok(ChainInformation {
            finalized_block_header: crate::calculate_genesis_block_header(genesis_storage),
            babe_finalized_block1_slot_number: None,
            babe_finalized_block_epoch_information: None,
            babe_finalized_next_epoch_transition: None,
            grandpa_after_finalized_block_authorities_set_id: 0,
            grandpa_finalized_scheduled_changes: Vec::new(),
            grandpa_finalized_triggered_authorities: grandpa_genesis_config.initial_authorities,
        })
    }
}

impl<'a> From<ChainInformationRef<'a>> for ChainInformation {
    fn from(info: ChainInformationRef<'a>) -> ChainInformation {
        ChainInformation {
            finalized_block_header: info.finalized_block_header.into(),
            babe_finalized_block1_slot_number: info.babe_finalized_block1_slot_number,
            babe_finalized_block_epoch_information: info
                .babe_finalized_block_epoch_information
                .map(|(e, c)| (e.clone().into(), c)),
            babe_finalized_next_epoch_transition: info
                .babe_finalized_next_epoch_transition
                .map(|(e, c)| (e.clone().into(), c)),
            grandpa_after_finalized_block_authorities_set_id: info
                .grandpa_after_finalized_block_authorities_set_id,
            grandpa_finalized_triggered_authorities: info
                .grandpa_finalized_triggered_authorities
                .into(),
            grandpa_finalized_scheduled_changes: info.grandpa_finalized_scheduled_changes.into(),
        }
    }
}

/// Error when building the chain information from the genesis storage.
#[derive(Debug, derive_more::Display)]
pub enum FromGenesisStorageError {
    /// Error when retrieving the GrandPa configuration.
    GrandpaConfigLoad(grandpa::chain_config::FromGenesisStorageError),
}

#[derive(Debug, Clone)]
pub struct FinalizedScheduledChange {
    pub trigger_block_height: u64,
    pub new_authorities_list: Vec<header::GrandpaAuthority>,
}

/// Equivalent to a [`ChainInformation`] but referencing an existing structure. Cheap to copy.
#[derive(Debug, Clone)]
pub struct ChainInformationRef<'a> {
    /// See equivalent field in [`ChanInformation`].
    pub finalized_block_header: header::HeaderRef<'a>,

    /// See equivalent field in [`ChanInformation`].
    pub babe_finalized_block1_slot_number: Option<u64>,

    /// See equivalent field in [`ChanInformation`].
    pub babe_finalized_block_epoch_information:
        Option<(header::BabeNextEpochRef<'a>, header::BabeNextConfig)>,

    /// See equivalent field in [`ChanInformation`].
    pub babe_finalized_next_epoch_transition:
        Option<(header::BabeNextEpochRef<'a>, header::BabeNextConfig)>,

    /// See equivalent field in [`ChanInformation`].
    pub grandpa_after_finalized_block_authorities_set_id: u64,

    /// See equivalent field in [`ChanInformation`].
    pub grandpa_finalized_triggered_authorities: &'a [header::GrandpaAuthority],

    /// See equivalent field in [`ChanInformation`].
    // TODO: better type, as a Vec is not in the spirit of this struct; however it's likely that this "scheduled changes" field will disappear altogether
    pub grandpa_finalized_scheduled_changes: Vec<FinalizedScheduledChange>,
}

impl<'a> From<&'a ChainInformation> for ChainInformationRef<'a> {
    fn from(info: &'a ChainInformation) -> ChainInformationRef<'a> {
        ChainInformationRef {
            finalized_block_header: (&info.finalized_block_header).into(),
            babe_finalized_block1_slot_number: info.babe_finalized_block1_slot_number,
            babe_finalized_block_epoch_information: info
                .babe_finalized_block_epoch_information
                .as_ref()
                .map(|(i, c)| (i.into(), *c)),
            babe_finalized_next_epoch_transition: info
                .babe_finalized_next_epoch_transition
                .as_ref()
                .map(|(i, c)| (i.into(), *c)),
            grandpa_after_finalized_block_authorities_set_id: info
                .grandpa_after_finalized_block_authorities_set_id,
            grandpa_finalized_triggered_authorities: &info.grandpa_finalized_triggered_authorities,
            grandpa_finalized_scheduled_changes: info.grandpa_finalized_scheduled_changes.clone(),
        }
    }
}

/// Includes a [`ChainInformation`] plus some chain-wide configuration.
#[derive(Debug, Clone)]
pub struct ChainInformationConfig {
    /// Information about the latest finalized block.
    pub chain_information: ChainInformation,

    /// Configuration for BABE, retrieved from the genesis block.
    pub babe_genesis_config: babe::BabeGenesisConfiguration,
}

impl ChainInformationConfig {
    /// Builds the [`ChainInformationConfig`] corresponding to the genesis block.
    ///
    /// Must be passed a closure that returns the storage value corresponding to the given key in
    /// the genesis block storage.
    pub fn from_genesis_storage<'a>(
        genesis_storage: impl Iterator<Item = (&'a [u8], &'a [u8])> + Clone,
    ) -> Result<Self, FromGenesisStorageError> {
        let babe_genesis_config = babe::BabeGenesisConfiguration::from_genesis_storage(|k| {
            genesis_storage
                .clone()
                .find(|(k2, _)| *k2 == k)
                .map(|(_, v)| v.to_owned())
        })
        .unwrap(); // TODO: no unwrap

        let chain_information = ChainInformation::from_genesis_storage(genesis_storage)?;

        Ok(ChainInformationConfig {
            chain_information,
            babe_genesis_config,
        })
    }
}
