use serde::Deserialize;

// struct representing the authorized caller, deserializable from JWT claims
#[derive(Debug, Deserialize, Clone)]
pub(crate) struct User {
    sub: String,
}

impl User {
    pub(crate) fn isCollectionsAdministrator() -> bool {
        false
    }
}
