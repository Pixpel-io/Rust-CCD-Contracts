use concordium_std::schema::SchemaType;
use twox_hash::xxh3::hash64;

use crate::state::VestingLimits;


#[test]
fn launchpad_id_smoke() {
    let id_1 = hash64("Pixpel.io DEX".as_bytes());
    let id_2 = hash64("Pixpel.io SWAP".as_bytes());

    // Validating isolation
    assert!(id_1 != id_2);
    // Validating reverse encoding
    assert!(id_1 == hash64("Pixpel.io DEX".as_bytes()));
}