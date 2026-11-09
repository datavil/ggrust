//! `{"name": "none"}`: keep every row (the same as `sampling="none"`).

use serde_json::Map;

use super::{Sampler, check_options};
use crate::Result;
use crate::spec::Value;

/// Keeps every row.
#[derive(Debug)]
pub struct NoSampler;

impl Sampler for NoSampler {
    fn keep(&self, _row_count: usize) -> Option<Vec<u32>> {
        None
    }
}

pub(super) fn parse(options: &Map<String, Value>, path: &str) -> Result<Box<dyn Sampler>> {
    check_options(options, &[], "sampling_none", path)?;
    Ok(Box::new(NoSampler))
}
