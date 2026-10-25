//! `position="identity"`: no adjustment.

use super::Position;
use crate::Result;
use crate::data::LayerData;
use crate::parameters::Parameters;

/// Leaves positions as they are.
pub struct IdentityPosition;

impl Position for IdentityPosition {
    fn name(&self) -> &'static str {
        "identity"
    }

    fn adjust(&self, data: LayerData, _parameters: &Parameters) -> Result<LayerData> {
        Ok(data)
    }
}
