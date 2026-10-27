//! `stat="identity"`: data passes through unchanged.

use super::{Stat, StatContext};
use crate::Result;
use crate::data::LayerData;
use crate::parameters::Parameters;

/// Leaves the data as is.
pub struct IdentityStat;

impl Stat for IdentityStat {
    fn name(&self) -> &'static str {
        "identity"
    }

    fn compute(
        &self,
        data: LayerData,
        _parameters: &Parameters,
        _context: &StatContext,
    ) -> Result<LayerData> {
        Ok(data)
    }
}
