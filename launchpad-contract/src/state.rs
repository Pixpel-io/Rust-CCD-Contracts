use concordium_std::{collections::BTreeMap, *};

use crate::types::*;

#[derive(Serialize, Clone)]
pub struct State {
    pub total_launchpad: LaunchpadID, // length of launchpad
    pub launchpad: BTreeMap<LaunchpadID, Launchpad>,
    pub lockup_details: BTreeMap<LaunchpadID, LockupDetails>,
    pub admin: AccountAddress,
}
