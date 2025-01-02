use serde::Deserialize;
use validator::Validate;

#[derive(Debug, Clone, Deserialize, Validate, utoipa::IntoParams)]
#[serde(default)]
#[into_params(parameter_in = Query)]
pub(crate) struct Pagination {
    #[validate(range(min = 1, max = 250))]
    #[param(minimum = 1, maximum = 250, example = 50, default = 50)]
    limit: u8,
    #[param(example = 0, default = 0)]
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
