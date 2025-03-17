use std::collections::BTreeMap;

use concordium_cis2::TokenAmountU64 as TokenAmount;
use concordium_std::{schema, AccountAddress, Amount, Serialize, Timestamp};

use crate::state::{TimePeriod, VestingLimits};

/// Tagged Launch-pad events to be serialized for the event logging.
/// 
/// Each event represent a major state change in contract or launch-pad
#[derive(Serialize)]
pub enum Event {
    /// Event to be logged when a new launch pad is just created
    CREATED(CreateLaunchPadEvent),
    /// Event to be logged when a new launch pad is approved for
    /// presale
    APPROVED(ApproveEvent),
    /// Event to be logged when a new launch pad is rejected for
    /// presale
    REJECTED(RejectEvent),
    /// Event to be logged when a launch pad is ready for vesting
    /// after token allocation
    VESTINGSTARTED(VestEvent),
    /// Event to be logged when a launch pad finishes vesting and
    /// enters the cliff period
    CLIFFSTARTED(CliffEvent)
}

// Implementing a custom schemaType for the `Event` struct.
// This custom implementation flattens the fields to avoid one
// level of nesting. Deriving the schemaType would result in e.g.:
// {"AddItemEvent": [{...fields}] }. In contrast, this custom schemaType
// implementation results in e.g.: {"AddItemEvent": {...fields} }
impl schema::SchemaType for Event {
    fn get_type() -> schema::Type {
        let mut event_map = BTreeMap::new();
        let events = vec![
            (
                "CreateLaunchPadEvent".to_string(),
                schema::Fields::Named(vec![
                    (String::from("launchpad_id"), u16::get_type()),
                    (String::from("launchpad_name"), String::get_type()),
                    (String::from("owner"), AccountAddress::get_type()),
                    (String::from("allocated_tokens"), TokenAmount::get_type()),
                    (String::from("base_price"), Amount::get_type()),
                ]),
            ),
            (
                "ApproveEvent".to_string(),
                schema::Fields::Named(vec![
                    (String::from("launchpad_id"), u16::get_type()),
                    (String::from("launchpad_name"), String::get_type()),
                ]),
            ),
            (
                "RejectEvent".to_string(),
                schema::Fields::Named(vec![
                    (String::from("launchpad_id"), u16::get_type()),
                    (String::from("launchpad_name"), String::get_type()),
                ]),
            ),
            (
                "CliffEvent".to_string(),
                schema::Fields::Named(vec![
                    (String::from("launchpad_id"), u16::get_type()),
                    (String::from("launchpad_name"), String::get_type()),
                    (String::from("from"), Timestamp::get_type()),
                    (String::from("to"), Timestamp::get_type()),
                ]),
            ),
            (
                "VestEvent".to_string(),
                schema::Fields::Named(vec![
                    (String::from("launchpad_id"), u16::get_type()),
                    (String::from("launchpad_name"), String::get_type()),
                    (String::from("vesting_time"), TimePeriod::get_type()),
                    (String::from("vesting_limits"), VestingLimits::get_type()),
                ]),
            ),
        ];

        for (key, value) in events.iter().enumerate() {
            event_map.insert(key as u8, value.clone());
        }

        schema::Type::TaggedEnum(event_map)
    }
}

#[derive(Serialize)]
pub struct CreateLaunchPadEvent {
    pub launchpad_name: String,
    pub owner: AccountAddress,
    pub allocated_tokens: TokenAmount,
    pub base_price: Amount
}

#[derive(Serialize)]
pub struct ApproveEvent {
    pub launchpad_name: String,
}

#[derive(Serialize)]
pub struct RejectEvent {
    pub launchpad_name: String,
}

#[derive(Serialize)]
pub struct CliffEvent {
    pub launchpad_id: u64,
    pub launchpad_name: String,
    pub from: Timestamp,
    pub to: Timestamp
}

#[derive(Serialize)]
pub struct VestEvent {
    pub launchpad_name: String,
    pub vesting_time: TimePeriod,
    pub vesting_limits: VestingLimits
}