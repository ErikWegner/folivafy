use uuid::Uuid;

use super::dto::Grant;

pub(crate) fn default_grants(
    collection_oao: bool,
    collection_uuid: Uuid,
    user: Uuid,
) -> Vec<Grant> {
    if collection_oao {
        vec![Grant::author_grant(user)]
    } else {
        vec![Grant::read_collection(collection_uuid)]
    }
}
