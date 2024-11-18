use crate::types::*;
use concordium_std::{collections::BTreeMap, *};

#[derive(Serialize, SchemaType)]
pub struct VestingView {
    pub total_launchpad: LaunchpadID, // length of launchpad
    pub launchpad: BTreeMap<LaunchpadID, Launchpad>,
    pub lockup_details: BTreeMap<LaunchpadID, LockupDetails>,
}
