pub mod collection;
pub mod collection_document;
pub mod event;

use collection_document::Model as Documents;

pub static DELETED_AT_FIELD: &str = "folivafy_deleted_at";
pub static DELETED_BY_FIELD: &str = "folivafy_deleted_by";

impl Documents {
    pub fn is_deleted(&self) -> bool {
        let field = self.f.get(DELETED_AT_FIELD);
        if let Some(field) = field {
            let s = field.as_str();
            if let Some(s) = s {
                return !s.is_empty();
            }
        }

        false
    }
}
