use crate::types::*;
use concordium_std::{collections::BTreeMap, *};

#[derive(Serialize, SchemaType)]
pub struct VestingView {
    pub total_launchpad: LaunchPadID, // length of launchpad
    pub launchpad: BTreeMap<LaunchPadID, Launchpad>,
    pub lockup_details: BTreeMap<LaunchPadID, LockupDetails>,
}
