use uuid::Uuid;

use super::{db::CollectionDocumentVisibility, dto::Grant};

pub(crate) fn default_document_grants(
    collection_oao: bool,
    collection_uuid: Uuid,
    user_id: Uuid,
) -> Vec<Grant> {
    if collection_oao {
        vec![
            Grant::author_grant(user_id),
            Grant::read_all_collection(collection_uuid),
        ]
    } else {
        vec![Grant::read_collection(collection_uuid)]
    }
}

pub(crate) struct DefaultUserGrantsParameters {
    visibility: CollectionDocumentVisibility,
    collection_uuid: Uuid,
    user_id: Uuid,
    user_is_all_reader: bool,
}

pub(crate) fn default_user_grants(params: DefaultUserGrantsParameters) -> Vec<Grant> {
    match params.visibility {
        CollectionDocumentVisibility::PrivateAndUserCanAccessAllDocuments => {
            vec![Grant::read_all_collection(params.collection_uuid)]
        }
        CollectionDocumentVisibility::PrivateAndUserIs(user_id) => {
            vec![Grant::author_grant(user_id)]
        }
        CollectionDocumentVisibility::PublicAndUserIsReader => {
            vec![Grant::read_collection(params.collection_uuid)]
        }
    }
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use crate::api::{db::CollectionDocumentVisibility, grants::DefaultUserGrantsParameters};

    use super::{default_document_grants, default_user_grants};

    #[test]
    fn it_has_required_default_document_grants_for_public_collection() {
        // Arrange
        let oao = false;
        let collection_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        // Act
        let grants = default_document_grants(oao, collection_id, user_id);

        // Assert
        assert_eq!(1, grants.len(), "Provides 1 grant");
        assert!(
            grants
                .iter()
                .any(|g| g.realm() == "read-collection" && g.grant_id() == collection_id),
            "Grants {:?} has no read-collection (basic user read access) for {collection_id}",
            grants
        );
    }

    #[test]
    fn it_has_required_default_document_grants_for_oao_collection() {
        // Arrange
        let oao = true;
        let collection_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        // Act
        let grants = default_document_grants(oao, collection_id, user_id);

        // Assert
        assert_eq!(2, grants.len(), "Provides 2 grants");
        assert!(
            grants
                .iter()
                .any(|g| g.realm() == "author" && g.grant_id() == user_id),
            "Grants {:?} has no author grant for {user_id}",
            grants
        );
        assert!(
            grants
                .iter()
                .any(|g| g.realm() == "read-all-collection" && g.grant_id() == collection_id),
            "Grants {:?} has no read-all-collection (access to entire collection) for {collection_id}",
            grants
        );
    }

    #[test]
    fn it_provides_user_grants_for_public_collection() {
        // Arrange
        let visibility = CollectionDocumentVisibility::PublicAndUserIsReader;
        let collection_uuid = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let user_is_all_reader = false;

        // Act
        let grants = default_user_grants(DefaultUserGrantsParameters {
            visibility,
            collection_uuid,
            user_id,
            user_is_all_reader,
        });

        // Assert
        assert_eq!(1, grants.len(), "Provides 1 grants");
        assert!(
            grants
                .iter()
                .any(|g| g.realm() == "read-collection" && g.grant_id() == collection_uuid),
            "Grants {:?} has no read-collection (basic user read access) for {collection_uuid}",
            grants
        );
    }

    #[test]
    fn it_provides_user_grants_for_public_collection_with_all_read_permission() {
        // Arrange
        let visibility = CollectionDocumentVisibility::PublicAndUserIsReader;
        let collection_uuid = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let user_is_all_reader = true;

        // Act
        let grants = default_user_grants(DefaultUserGrantsParameters {
            visibility,
            collection_uuid,
            user_id,
            user_is_all_reader,
        });

        // Assert
        assert_eq!(1, grants.len(), "Provides 1 grants");
        assert!(
            grants
                .iter()
                .any(|g| g.realm() == "read-collection" && g.grant_id() == collection_uuid),
            "Grants {:?} has no read-collection (basic user read access) for {collection_uuid}",
            grants
        );
    }

    #[test]
    fn it_provides_user_grants_for_basic_reader_for_oao_collection() {
        // Arrange
        let user_id = Uuid::new_v4();
        let visibility = CollectionDocumentVisibility::PrivateAndUserIs(user_id);
        let collection_uuid = Uuid::new_v4();
        let user_is_all_reader = false;

        // Act
        let grants = default_user_grants(DefaultUserGrantsParameters {
            visibility,
            collection_uuid,
            user_id,
            user_is_all_reader,
        });

        // Assert
        assert_eq!(1, grants.len(), "Provides 1 grant");
        assert!(
            grants
                .iter()
                .any(|g| g.realm() == "author" && g.grant_id() == user_id),
            "Grants {:?} has no author grant for {user_id}",
            grants
        );
    }

    #[test]
    fn it_provides_user_grants_with_read_all_permission_for_oao_collection() {
        // Arrange
        let user_id = Uuid::new_v4();
        let visibility = CollectionDocumentVisibility::PrivateAndUserCanAccessAllDocuments;
        let collection_uuid = Uuid::new_v4();
        let user_is_all_reader = true;

        // Act
        let grants = default_user_grants(DefaultUserGrantsParameters {
            visibility,
            collection_uuid,
            user_id,
            user_is_all_reader,
        });

        // Assert
        assert_eq!(1, grants.len(), "Provides 1 grant");
        assert!(
            grants
                .iter()
                .any(|g| g.realm() == "read-all-collection" && g.grant_id() == collection_uuid),
            "Grants {:?} has no read-all-collection (access to entire collection) for {collection_uuid}",
            grants
        );
    }
}
