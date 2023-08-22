use serde::Deserialize;
use validator::Validate;

#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(default)]
pub(crate) struct Pagination {
    #[validate(range(min = 1, max = 250))]
    limit: u8,
    offset: u32,
}

impl Pagination {
    pub(crate) fn new(limit: u8, offset: u32) -> Self {
        Self { limit, offset }
    }

    pub(crate) fn limit(&self) -> u8 {
        self.limit
    }

    pub(crate) fn offset(&self) -> u32 {
        self.offset
    }
}

impl Default for Pagination {
    fn default() -> Self {
        Self {
            limit: 50,
            offset: 0,
        }
    }
}
