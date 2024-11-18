use crate::types::*;
use concordium_std::collections::BTreeMap;

pub struct VestingHelper;

impl VestingHelper {
    pub(crate) fn re_arrange_release_data(
        release_data: BTreeMap<u8, ReleaseData>,
    ) -> BTreeMap<u8, ReleaseData> {
        release_data
    }
}
