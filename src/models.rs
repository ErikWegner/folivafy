#![allow(clippy::all)]
#![allow(clippy::cfg_not_test)]

use crate::models;

/// Arbitrary event category
#[derive(Debug, Clone, PartialEq, PartialOrd, serde::Serialize, serde::Deserialize)]
pub struct CategoryId(i32);

impl std::convert::From<i32> for CategoryId {
    fn from(x: i32) -> Self {
        CategoryId(x)
    }
}

impl std::convert::From<CategoryId> for i32 {
    fn from(x: CategoryId) -> Self {
        x.0
    }
}

impl std::ops::Deref for CategoryId {
    type Target = i32;
    fn deref(&self) -> &i32 {
        &self.0
    }
}

impl std::ops::DerefMut for CategoryId {
    fn deref_mut(&mut self) -> &mut i32 {
        &mut self.0
    }
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    serde::Serialize,
    serde::Deserialize,
    utoipa::ToSchema,
    validator::Validate,
)]
pub struct Collection {
    /// Path name of the collection
    #[serde(rename = "name")]
    #[validate(length(min = 1, max = 32), regex(path= *RE_COLLECTION_NAME))]
    #[schema(
        examples("shapes", "applications", "reservations"),
        min_length = 1,
        max_length = 32,
    )]
    pub name: String,

    /// Human readable name of the collection
    #[serde(rename = "title")]
    #[validate(length(min = 1, max = 150))]
    #[schema(examples("Shapes", "Job applications", "Car reservations"),
        min_length = 1,
        max_length = 150,
    )]
    pub title: String,

    /// Owner access only. Indicates if documents within the collection are _owner access only_ (value `true`) or all documents in the collection can be read by all users (`false`).
    #[serde(rename = "oao")]
    pub oao: bool,

    /// Indicates if new documents within the collection can be created (value `false`) or the collection is set to read only (`true`).
    #[serde(rename = "locked")]
    #[schema(examples(false, true))]
    pub locked: bool,
}

lazy_static::lazy_static! {
    static ref RE_COLLECTION_NAME: regex::Regex = regex::Regex::new(r"^[a-z][-a-z0-9]*$").unwrap();
}

impl Collection {
    #[allow(clippy::new_without_default)]
    #[allow(dead_code)]
    pub fn new(name: String, title: String, oao: bool, locked: bool) -> Collection {
        Collection {
            name,
            title,
            oao,
            locked,
        }
    }
}

/// Converts the Collection value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::string::ToString for Collection {
    fn to_string(&self) -> String {
        let params: Vec<Option<String>> = vec![
            Some("name".to_string()),
            Some(self.name.to_string()),
            Some("title".to_string()),
            Some(self.title.to_string()),
            Some("oao".to_string()),
            Some(self.oao.to_string()),
            Some("locked".to_string()),
            Some(self.locked.to_string()),
        ];

        params.into_iter().flatten().collect::<Vec<_>>().join(",")
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a Collection value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for Collection {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub name: Vec<String>,
            pub title: Vec<String>,
            pub oao: Vec<bool>,
            pub locked: Vec<bool>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => {
                    return std::result::Result::Err(
                        "Missing value while parsing Collection".to_string(),
                    )
                }
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "name" => intermediate_rep.name.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "title" => intermediate_rep.title.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "oao" => intermediate_rep.oao.push(
                        <bool as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "locked" => intermediate_rep.locked.push(
                        <bool as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    _ => {
                        return std::result::Result::Err(
                            "Unexpected key while parsing Collection".to_string(),
                        )
                    }
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(Collection {
            name: intermediate_rep
                .name
                .into_iter()
                .next()
                .ok_or_else(|| "name missing in Collection".to_string())?,
            title: intermediate_rep
                .title
                .into_iter()
                .next()
                .ok_or_else(|| "title missing in Collection".to_string())?,
            oao: intermediate_rep
                .oao
                .into_iter()
                .next()
                .ok_or_else(|| "oao missing in Collection".to_string())?,
            locked: intermediate_rep
                .locked
                .into_iter()
                .next()
                .ok_or_else(|| "locked missing in Collection".to_string())?,
        })
    }
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    serde::Serialize,
    serde::Deserialize,
    utoipa::ToSchema,
    validator::Validate,
)]
#[schema(
    description = "Item (document) within a collection",
    examples(
        json!({
            "id" :"9f818bff-a1b4-487a-9706-29a5ac1cf898",
            "f": {
                "title": "Rectangle",
                "price": 14
            }    
        })
    ),
)]
pub struct CollectionItem {
    /// Document identifier
    #[serde(rename = "id")]
    #[schema(examples("9f818bff-a1b4-487a-9706-29a5ac1cf898"), format = Uuid)]
    pub id: uuid::Uuid,

    /// Field data
    #[serde(rename = "f")]
    #[schema(
        examples(
            json!({
                "f": {
                    "title": "Rectangle",
                    "price": 14
                }    
            })
        )
    )]
    pub f: serde_json::Value,
}

impl CollectionItem {
    #[allow(clippy::new_without_default)]
    #[allow(dead_code)]
    pub fn new(id: uuid::Uuid, f: serde_json::Value) -> CollectionItem {
        CollectionItem { id, f }
    }
}

/// Converts the CollectionItem value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::string::ToString for CollectionItem {
    fn to_string(&self) -> String {
        let params: Vec<Option<String>> = vec![
            // Skipping id in query parameter serialization

            // Skipping f in query parameter serialization

        ];

        params.into_iter().flatten().collect::<Vec<_>>().join(",")
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a CollectionItem value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for CollectionItem {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub id: Vec<uuid::Uuid>,
            pub f: Vec<serde_json::Value>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => {
                    return std::result::Result::Err(
                        "Missing value while parsing CollectionItem".to_string(),
                    )
                }
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "id" => intermediate_rep.id.push(
                        <uuid::Uuid as std::str::FromStr>::from_str(val)
                            .map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "f" => intermediate_rep.f.push(
                        <serde_json::Value as std::str::FromStr>::from_str(val)
                            .map_err(|x| x.to_string())?,
                    ),
                    _ => {
                        return std::result::Result::Err(
                            "Unexpected key while parsing CollectionItem".to_string(),
                        )
                    }
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(CollectionItem {
            id: intermediate_rep
                .id
                .into_iter()
                .next()
                .ok_or_else(|| "id missing in CollectionItem".to_string())?,
            f: intermediate_rep
                .f
                .into_iter()
                .next()
                .ok_or_else(|| "f missing in CollectionItem".to_string())?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
pub struct CollectionItemDetails {
    /// Document identifier
    #[serde(rename = "id")]
    pub id: uuid::Uuid,

    /// Field data
    #[serde(rename = "f")]
    pub f: serde_json::Value,

    #[serde(rename = "e")]
    pub e: Vec<models::CollectionItemEvent>,
}

impl CollectionItemDetails {
    #[allow(clippy::new_without_default)]
    #[allow(dead_code)]
    pub fn new(
        id: uuid::Uuid,
        f: serde_json::Value,
        e: Vec<models::CollectionItemEvent>,
    ) -> CollectionItemDetails {
        CollectionItemDetails { id, f, e }
    }
}

/// Converts the CollectionItemDetails value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::string::ToString for CollectionItemDetails {
    fn to_string(&self) -> String {
        let params: Vec<Option<String>> = vec![
            // Skipping id in query parameter serialization

            // Skipping f in query parameter serialization

            // Skipping e in query parameter serialization

        ];

        params.into_iter().flatten().collect::<Vec<_>>().join(",")
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a CollectionItemDetails value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for CollectionItemDetails {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub id: Vec<uuid::Uuid>,
            pub f: Vec<serde_json::Value>,
            pub e: Vec<Vec<models::CollectionItemEvent>>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => {
                    return std::result::Result::Err(
                        "Missing value while parsing CollectionItemDetails".to_string(),
                    )
                }
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "id" => intermediate_rep.id.push(<uuid::Uuid as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "f" => intermediate_rep.f.push(<serde_json::Value as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    "e" => return std::result::Result::Err("Parsing a container in this style is not supported in CollectionItemDetails".to_string()),
                    _ => return std::result::Result::Err("Unexpected key while parsing CollectionItemDetails".to_string())
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(CollectionItemDetails {
            id: intermediate_rep
                .id
                .into_iter()
                .next()
                .ok_or_else(|| "id missing in CollectionItemDetails".to_string())?,
            f: intermediate_rep
                .f
                .into_iter()
                .next()
                .ok_or_else(|| "f missing in CollectionItemDetails".to_string())?,
            e: intermediate_rep
                .e
                .into_iter()
                .next()
                .ok_or_else(|| "e missing in CollectionItemDetails".to_string())?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
pub struct CollectionItemEvent {
    #[serde(rename = "id")]
    #[validate(range(min = 0))]
    pub id: u32,

    #[serde(rename = "ts")]
    pub ts: chrono::DateTime<chrono::Utc>,

    /// Arbitrary event category
    #[serde(rename = "category")]
    pub category: i32,

    /// Field data
    #[serde(rename = "e")]
    pub e: serde_json::Value,
}

impl CollectionItemEvent {
    #[allow(clippy::new_without_default)]
    #[allow(dead_code)]
    pub fn new(
        id: u32,
        ts: chrono::DateTime<chrono::Utc>,
        category: i32,
        e: serde_json::Value,
    ) -> CollectionItemEvent {
        CollectionItemEvent {
            id,
            ts,
            category,
            e,
        }
    }
}

/// Converts the CollectionItemEvent value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::string::ToString for CollectionItemEvent {
    fn to_string(&self) -> String {
        let params: Vec<Option<String>> = vec![
            Some("id".to_string()),
            Some(self.id.to_string()),
            // Skipping ts in query parameter serialization
            Some("category".to_string()),
            Some(self.category.to_string()),
            // Skipping e in query parameter serialization
        ];

        params.into_iter().flatten().collect::<Vec<_>>().join(",")
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a CollectionItemEvent value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for CollectionItemEvent {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub id: Vec<u32>,
            pub ts: Vec<chrono::DateTime<chrono::Utc>>,
            pub category: Vec<i32>,
            pub e: Vec<serde_json::Value>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => {
                    return std::result::Result::Err(
                        "Missing value while parsing CollectionItemEvent".to_string(),
                    )
                }
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "id" => intermediate_rep.id.push(
                        <u32 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "ts" => intermediate_rep.ts.push(
                        <chrono::DateTime<chrono::Utc> as std::str::FromStr>::from_str(val)
                            .map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "category" => intermediate_rep.category.push(
                        <i32 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "e" => intermediate_rep.e.push(
                        <serde_json::Value as std::str::FromStr>::from_str(val)
                            .map_err(|x| x.to_string())?,
                    ),
                    _ => {
                        return std::result::Result::Err(
                            "Unexpected key while parsing CollectionItemEvent".to_string(),
                        )
                    }
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(CollectionItemEvent {
            id: intermediate_rep
                .id
                .into_iter()
                .next()
                .ok_or_else(|| "id missing in CollectionItemEvent".to_string())?,
            ts: intermediate_rep
                .ts
                .into_iter()
                .next()
                .ok_or_else(|| "ts missing in CollectionItemEvent".to_string())?,
            category: intermediate_rep
                .category
                .into_iter()
                .next()
                .ok_or_else(|| "category missing in CollectionItemEvent".to_string())?,
            e: intermediate_rep
                .e
                .into_iter()
                .next()
                .ok_or_else(|| "e missing in CollectionItemEvent".to_string())?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
pub struct CollectionItemsList {
    #[serde(rename = "limit")]
    #[validate(range(min = 1, max = 250))]
    pub limit: u8,

    #[serde(rename = "offset")]
    #[validate(range(min = 0))]
    pub offset: u32,

    #[serde(rename = "total")]
    #[validate(range(min = 0))]
    pub total: u32,

    #[serde(rename = "items")]
    pub items: Vec<models::CollectionItem>,
}

impl CollectionItemsList {
    #[allow(clippy::new_without_default)]
    #[allow(dead_code)]
    pub fn new(items: Vec<models::CollectionItem>) -> CollectionItemsList {
        CollectionItemsList {
            limit: 50,
            offset: 0,
            total: 0,
            items,
        }
    }
}

/// Converts the CollectionItemsList value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::string::ToString for CollectionItemsList {
    fn to_string(&self) -> String {
        let params: Vec<Option<String>> = vec![
            Some("limit".to_string()),
            Some(self.limit.to_string()),
            Some("offset".to_string()),
            Some(self.offset.to_string()),
            Some("total".to_string()),
            Some(self.total.to_string()),
            // Skipping items in query parameter serialization
        ];

        params.into_iter().flatten().collect::<Vec<_>>().join(",")
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a CollectionItemsList value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for CollectionItemsList {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub limit: Vec<u8>,
            pub offset: Vec<u32>,
            pub total: Vec<u32>,
            pub items: Vec<Vec<models::CollectionItem>>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => {
                    return std::result::Result::Err(
                        "Missing value while parsing CollectionItemsList".to_string(),
                    )
                }
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "limit" => intermediate_rep
                        .limit
                        .push(<u8 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "offset" => intermediate_rep.offset.push(
                        <u32 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "total" => intermediate_rep.total.push(
                        <u32 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    "items" => return std::result::Result::Err(
                        "Parsing a container in this style is not supported in CollectionItemsList"
                            .to_string(),
                    ),
                    _ => {
                        return std::result::Result::Err(
                            "Unexpected key while parsing CollectionItemsList".to_string(),
                        )
                    }
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(CollectionItemsList {
            limit: intermediate_rep
                .limit
                .into_iter()
                .next()
                .ok_or_else(|| "limit missing in CollectionItemsList".to_string())?,
            offset: intermediate_rep
                .offset
                .into_iter()
                .next()
                .ok_or_else(|| "offset missing in CollectionItemsList".to_string())?,
            total: intermediate_rep
                .total
                .into_iter()
                .next()
                .ok_or_else(|| "total missing in CollectionItemsList".to_string())?,
            items: intermediate_rep
                .items
                .into_iter()
                .next()
                .ok_or_else(|| "items missing in CollectionItemsList".to_string())?,
        })
    }
}

/// Path name of the collection
#[derive(Debug, Clone, PartialEq, PartialOrd, serde::Serialize, serde::Deserialize)]
pub struct CollectionName(String);

impl std::convert::From<String> for CollectionName {
    fn from(x: String) -> Self {
        CollectionName(x)
    }
}

impl std::string::ToString for CollectionName {
    fn to_string(&self) -> String {
        self.0.to_string()
    }
}

impl std::str::FromStr for CollectionName {
    type Err = std::string::ParseError;
    fn from_str(x: &str) -> std::result::Result<Self, Self::Err> {
        std::result::Result::Ok(CollectionName(x.to_string()))
    }
}

impl std::convert::From<CollectionName> for String {
    fn from(x: CollectionName) -> Self {
        x.0
    }
}

impl std::ops::Deref for CollectionName {
    type Target = String;
    fn deref(&self) -> &String {
        &self.0
    }
}

impl std::ops::DerefMut for CollectionName {
    fn deref_mut(&mut self) -> &mut String {
        &mut self.0
    }
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    serde::Deserialize,
    serde::Serialize,
    utoipa::ToSchema,
    validator::Validate,
)]
pub struct CollectionsList {
    #[serde(rename = "limit")]
    #[validate(range(min = 1, max = 250))]
    #[schema(examples(100), minimum = 1, maximum = 250)]
    pub limit: u8,

    #[serde(rename = "offset")]
    #[validate(range(min = 0))]
    #[schema(examples(100))]
    pub offset: u32,

    #[serde(rename = "total")]
    #[validate(range(min = 0))]
    pub total: u32,

    #[serde(rename = "items")]
    pub items: Vec<models::Collection>,
}

impl CollectionsList {
    #[allow(clippy::new_without_default)]
    #[allow(dead_code)]
    pub fn new(items: Vec<models::Collection>) -> CollectionsList {
        CollectionsList {
            limit: 50,
            offset: 0,
            total: 0,
            items,
        }
    }
}

/// Converts the CollectionsList value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::string::ToString for CollectionsList {
    fn to_string(&self) -> String {
        let params: Vec<Option<String>> = vec![
            Some("limit".to_string()),
            Some(self.limit.to_string()),
            Some("offset".to_string()),
            Some(self.offset.to_string()),
            Some("total".to_string()),
            Some(self.total.to_string()),
            // Skipping items in query parameter serialization
        ];

        params.into_iter().flatten().collect::<Vec<_>>().join(",")
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a CollectionsList value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for CollectionsList {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub limit: Vec<u8>,
            pub offset: Vec<u32>,
            pub total: Vec<u32>,
            pub items: Vec<Vec<models::Collection>>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => {
                    return std::result::Result::Err(
                        "Missing value while parsing CollectionsList".to_string(),
                    )
                }
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "limit" => intermediate_rep
                        .limit
                        .push(<u8 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "offset" => intermediate_rep.offset.push(
                        <u32 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "total" => intermediate_rep.total.push(
                        <u32 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    "items" => {
                        return std::result::Result::Err(
                            "Parsing a container in this style is not supported in CollectionsList"
                                .to_string(),
                        )
                    }
                    _ => {
                        return std::result::Result::Err(
                            "Unexpected key while parsing CollectionsList".to_string(),
                        )
                    }
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(CollectionsList {
            limit: intermediate_rep
                .limit
                .into_iter()
                .next()
                .ok_or_else(|| "limit missing in CollectionsList".to_string())?,
            offset: intermediate_rep
                .offset
                .into_iter()
                .next()
                .ok_or_else(|| "offset missing in CollectionsList".to_string())?,
            total: intermediate_rep
                .total
                .into_iter()
                .next()
                .ok_or_else(|| "total missing in CollectionsList".to_string())?,
            items: intermediate_rep
                .items
                .into_iter()
                .next()
                .ok_or_else(|| "items missing in CollectionsList".to_string())?,
        })
    }
}
#[derive(
    Debug,
    Clone,
    PartialEq,
    serde::Serialize,
    serde::Deserialize,
    validator::Validate,
    utoipa::ToSchema,
)]
#[schema(
    description = "Information about the new collection",
    examples(
        json!({
            "name": "room-reservations",
            "title": "Room reservations",
            "oao": false
        })
    ),
)]
pub struct CreateCollectionRequest {
    /// Path name of the collection
    #[serde(rename = "name")]
    #[validate(length(min = 1, max = 32), regex(path= *RE_CREATECOLLECTIONREQUEST_NAME))]
    #[schema(
        min_length = 1,
        max_length = 32,
        pattern = r"^[a-z][-a-z0-9]*$",
        examples("shapes"),
    )]
    pub name: String,

    /// Human readable name of the collection
    #[serde(rename = "title")]
    #[validate(length(min = 1, max = 150))]
    #[schema(min_length = 1, max_length = 150, examples("Two-dimensional shapes"),)]
    pub title: String,

    /// Owner access only?
    #[serde(rename = "oao")]
    pub oao: bool,
}

const COLLECTIONREQUEST_NAME_PATTERN: &str = r"^[a-z][-a-z0-9]*$";
lazy_static::lazy_static! {
    static ref RE_CREATECOLLECTIONREQUEST_NAME: regex::Regex = regex::Regex::new(COLLECTIONREQUEST_NAME_PATTERN).unwrap();
}

impl CreateCollectionRequest {
    #[allow(clippy::new_without_default)]
    #[allow(dead_code)]
    pub fn new(name: String, title: String, oao: bool) -> CreateCollectionRequest {
        CreateCollectionRequest { name, title, oao }
    }
}

/// Converts the CreateCollectionRequest value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::string::ToString for CreateCollectionRequest {
    fn to_string(&self) -> String {
        let params: Vec<Option<String>> = vec![
            Some("name".to_string()),
            Some(self.name.to_string()),
            Some("title".to_string()),
            Some(self.title.to_string()),
            Some("oao".to_string()),
            Some(self.oao.to_string()),
        ];

        params.into_iter().flatten().collect::<Vec<_>>().join(",")
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a CreateCollectionRequest value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for CreateCollectionRequest {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub name: Vec<String>,
            pub title: Vec<String>,
            pub oao: Vec<bool>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => {
                    return std::result::Result::Err(
                        "Missing value while parsing CreateCollectionRequest".to_string(),
                    )
                }
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "name" => intermediate_rep.name.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "title" => intermediate_rep.title.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "oao" => intermediate_rep.oao.push(
                        <bool as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    _ => {
                        return std::result::Result::Err(
                            "Unexpected key while parsing CreateCollectionRequest".to_string(),
                        )
                    }
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(CreateCollectionRequest {
            name: intermediate_rep
                .name
                .into_iter()
                .next()
                .ok_or_else(|| "name missing in CreateCollectionRequest".to_string())?,
            title: intermediate_rep
                .title
                .into_iter()
                .next()
                .ok_or_else(|| "title missing in CreateCollectionRequest".to_string())?,
            oao: intermediate_rep
                .oao
                .into_iter()
                .next()
                .ok_or_else(|| "oao missing in CreateCollectionRequest".to_string())?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
pub struct CreateEventBody {
    /// Arbitrary event category
    #[serde(rename = "category")]
    pub category: i32,

    /// Path name of the collection
    #[serde(rename = "collection")]
    #[validate(length(min = 1, max = 32), regex(path= *RE_CREATEEVENTBODY_COLLECTION))]
    pub collection: String,

    /// Document identifier
    #[serde(rename = "document")]
    pub document: uuid::Uuid,

    /// Field data
    #[serde(rename = "e")]
    pub e: serde_json::Value,
}

lazy_static::lazy_static! {
    static ref RE_CREATEEVENTBODY_COLLECTION: regex::Regex = regex::Regex::new(r"^[a-z][-a-z0-9]*$").unwrap();
}

impl CreateEventBody {
    #[allow(clippy::new_without_default)]
    #[allow(dead_code)]
    pub fn new(
        category: i32,
        collection: String,
        document: uuid::Uuid,
        e: serde_json::Value,
    ) -> CreateEventBody {
        CreateEventBody {
            category,
            collection,
            document,
            e,
        }
    }
}

/// Converts the CreateEventBody value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::string::ToString for CreateEventBody {
    fn to_string(&self) -> String {
        let params: Vec<Option<String>> = vec![
            Some("category".to_string()),
            Some(self.category.to_string()),
            Some("collection".to_string()),
            Some(self.collection.to_string()),
            // Skipping document in query parameter serialization

            // Skipping e in query parameter serialization
        ];

        params.into_iter().flatten().collect::<Vec<_>>().join(",")
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a CreateEventBody value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for CreateEventBody {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub category: Vec<i32>,
            pub collection: Vec<String>,
            pub document: Vec<uuid::Uuid>,
            pub e: Vec<serde_json::Value>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => {
                    return std::result::Result::Err(
                        "Missing value while parsing CreateEventBody".to_string(),
                    )
                }
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "category" => intermediate_rep.category.push(
                        <i32 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "collection" => intermediate_rep.collection.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "document" => intermediate_rep.document.push(
                        <uuid::Uuid as std::str::FromStr>::from_str(val)
                            .map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "e" => intermediate_rep.e.push(
                        <serde_json::Value as std::str::FromStr>::from_str(val)
                            .map_err(|x| x.to_string())?,
                    ),
                    _ => {
                        return std::result::Result::Err(
                            "Unexpected key while parsing CreateEventBody".to_string(),
                        )
                    }
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(CreateEventBody {
            category: intermediate_rep
                .category
                .into_iter()
                .next()
                .ok_or_else(|| "category missing in CreateEventBody".to_string())?,
            collection: intermediate_rep
                .collection
                .into_iter()
                .next()
                .ok_or_else(|| "collection missing in CreateEventBody".to_string())?,
            document: intermediate_rep
                .document
                .into_iter()
                .next()
                .ok_or_else(|| "document missing in CreateEventBody".to_string())?,
            e: intermediate_rep
                .e
                .into_iter()
                .next()
                .ok_or_else(|| "e missing in CreateEventBody".to_string())?,
        })
    }
}

/// Document identifier
#[derive(Debug, Clone, PartialEq, PartialOrd, serde::Serialize, serde::Deserialize)]
pub struct DocumentId(uuid::Uuid);

impl std::convert::From<uuid::Uuid> for DocumentId {
    fn from(x: uuid::Uuid) -> Self {
        DocumentId(x)
    }
}

impl std::convert::From<DocumentId> for uuid::Uuid {
    fn from(x: DocumentId) -> Self {
        x.0
    }
}

impl std::ops::Deref for DocumentId {
    type Target = uuid::Uuid;
    fn deref(&self) -> &uuid::Uuid {
        &self.0
    }
}

impl std::ops::DerefMut for DocumentId {
    fn deref_mut(&mut self) -> &mut uuid::Uuid {
        &mut self.0
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
pub struct SearchCollectionBody {
    #[serde(rename = "filter")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<models::SearchFilter>,
}

impl SearchCollectionBody {
    #[allow(clippy::new_without_default)]
    #[allow(dead_code)]
    pub fn new() -> SearchCollectionBody {
        SearchCollectionBody { filter: None }
    }
}

/// Converts the SearchCollectionBody value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::string::ToString for SearchCollectionBody {
    fn to_string(&self) -> String {
        let params: Vec<Option<String>> = vec![
            // Skipping filter in query parameter serialization

        ];

        params.into_iter().flatten().collect::<Vec<_>>().join(",")
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a SearchCollectionBody value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for SearchCollectionBody {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub filter: Vec<models::SearchFilter>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => {
                    return std::result::Result::Err(
                        "Missing value while parsing SearchCollectionBody".to_string(),
                    )
                }
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "filter" => intermediate_rep.filter.push(
                        <models::SearchFilter as std::str::FromStr>::from_str(val)
                            .map_err(|x| x.to_string())?,
                    ),
                    _ => {
                        return std::result::Result::Err(
                            "Unexpected key while parsing SearchCollectionBody".to_string(),
                        )
                    }
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(SearchCollectionBody {
            filter: intermediate_rep.filter.into_iter().next(),
        })
    }
}

/// A search filter
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
pub struct SearchFilter {
    /// Field name
    #[serde(rename = "f")]
    pub f: String,

    /// Operator
    // Note: inline enums are not fully supported by openapi-generator
    #[serde(rename = "o")]
    pub o: String,

    #[serde(rename = "v")]
    pub v: models::SearchFilterFieldOpValueV,

    /// A list of search filters
    #[serde(rename = "and")]
    pub and: Vec<models::SearchFilter>,

    /// A list of search filters
    #[serde(rename = "or")]
    pub or: Vec<models::SearchFilter>,
}

impl SearchFilter {
    #[allow(clippy::new_without_default)]
    #[allow(dead_code)]
    pub fn new(
        f: String,
        o: String,
        v: models::SearchFilterFieldOpValueV,
        and: Vec<models::SearchFilter>,
        or: Vec<models::SearchFilter>,
    ) -> SearchFilter {
        SearchFilter { f, o, v, and, or }
    }
}

/// Converts the SearchFilter value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::string::ToString for SearchFilter {
    fn to_string(&self) -> String {
        let params: Vec<Option<String>> = vec![
            Some("f".to_string()),
            Some(self.f.to_string()),
            Some("o".to_string()),
            Some(self.o.to_string()),
            // Skipping v in query parameter serialization

            // Skipping and in query parameter serialization

            // Skipping or in query parameter serialization
        ];

        params.into_iter().flatten().collect::<Vec<_>>().join(",")
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a SearchFilter value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for SearchFilter {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub f: Vec<String>,
            pub o: Vec<String>,
            pub v: Vec<models::SearchFilterFieldOpValueV>,
            pub and: Vec<Vec<models::SearchFilter>>,
            pub or: Vec<Vec<models::SearchFilter>>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => {
                    return std::result::Result::Err(
                        "Missing value while parsing SearchFilter".to_string(),
                    )
                }
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "f" => intermediate_rep.f.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "o" => intermediate_rep.o.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "v" => intermediate_rep.v.push(
                        <models::SearchFilterFieldOpValueV as std::str::FromStr>::from_str(val)
                            .map_err(|x| x.to_string())?,
                    ),
                    "and" => {
                        return std::result::Result::Err(
                            "Parsing a container in this style is not supported in SearchFilter"
                                .to_string(),
                        )
                    }
                    "or" => {
                        return std::result::Result::Err(
                            "Parsing a container in this style is not supported in SearchFilter"
                                .to_string(),
                        )
                    }
                    _ => {
                        return std::result::Result::Err(
                            "Unexpected key while parsing SearchFilter".to_string(),
                        )
                    }
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(SearchFilter {
            f: intermediate_rep
                .f
                .into_iter()
                .next()
                .ok_or_else(|| "f missing in SearchFilter".to_string())?,
            o: intermediate_rep
                .o
                .into_iter()
                .next()
                .ok_or_else(|| "o missing in SearchFilter".to_string())?,
            v: intermediate_rep
                .v
                .into_iter()
                .next()
                .ok_or_else(|| "v missing in SearchFilter".to_string())?,
            and: intermediate_rep
                .and
                .into_iter()
                .next()
                .ok_or_else(|| "and missing in SearchFilter".to_string())?,
            or: intermediate_rep
                .or
                .into_iter()
                .next()
                .ok_or_else(|| "or missing in SearchFilter".to_string())?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
pub struct SearchFilterAndGroup {
    /// A list of search filters
    #[serde(rename = "and")]
    pub and: Vec<models::SearchFilter>,
}

impl SearchFilterAndGroup {
    #[allow(clippy::new_without_default)]
    #[allow(dead_code)]
    pub fn new(and: Vec<models::SearchFilter>) -> SearchFilterAndGroup {
        SearchFilterAndGroup { and }
    }
}

/// Converts the SearchFilterAndGroup value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::string::ToString for SearchFilterAndGroup {
    fn to_string(&self) -> String {
        let params: Vec<Option<String>> = vec![
            // Skipping and in query parameter serialization

        ];

        params.into_iter().flatten().collect::<Vec<_>>().join(",")
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a SearchFilterAndGroup value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for SearchFilterAndGroup {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub and: Vec<Vec<models::SearchFilter>>,
        }

        let intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let _val = match string_iter.next() {
                Some(x) => x,
                None => {
                    return std::result::Result::Err(
                        "Missing value while parsing SearchFilterAndGroup".to_string(),
                    )
                }
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    "and" => return std::result::Result::Err("Parsing a container in this style is not supported in SearchFilterAndGroup".to_string()),
                    _ => return std::result::Result::Err("Unexpected key while parsing SearchFilterAndGroup".to_string())
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(SearchFilterAndGroup {
            and: intermediate_rep
                .and
                .into_iter()
                .next()
                .ok_or_else(|| "and missing in SearchFilterAndGroup".to_string())?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
pub struct SearchFilterFieldOp {
    /// Field name
    #[serde(rename = "f")]
    pub f: String,

    /// Operator
    // Note: inline enums are not fully supported by openapi-generator
    #[serde(rename = "o")]
    pub o: String,
}

impl SearchFilterFieldOp {
    #[allow(clippy::new_without_default)]
    #[allow(dead_code)]
    pub fn new(f: String, o: String) -> SearchFilterFieldOp {
        SearchFilterFieldOp { f, o }
    }
}

/// Converts the SearchFilterFieldOp value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::string::ToString for SearchFilterFieldOp {
    fn to_string(&self) -> String {
        let params: Vec<Option<String>> = vec![
            Some("f".to_string()),
            Some(self.f.to_string()),
            Some("o".to_string()),
            Some(self.o.to_string()),
        ];

        params.into_iter().flatten().collect::<Vec<_>>().join(",")
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a SearchFilterFieldOp value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for SearchFilterFieldOp {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub f: Vec<String>,
            pub o: Vec<String>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => {
                    return std::result::Result::Err(
                        "Missing value while parsing SearchFilterFieldOp".to_string(),
                    )
                }
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "f" => intermediate_rep.f.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "o" => intermediate_rep.o.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    _ => {
                        return std::result::Result::Err(
                            "Unexpected key while parsing SearchFilterFieldOp".to_string(),
                        )
                    }
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(SearchFilterFieldOp {
            f: intermediate_rep
                .f
                .into_iter()
                .next()
                .ok_or_else(|| "f missing in SearchFilterFieldOp".to_string())?,
            o: intermediate_rep
                .o
                .into_iter()
                .next()
                .ok_or_else(|| "o missing in SearchFilterFieldOp".to_string())?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
pub struct SearchFilterFieldOpValue {
    /// Field name
    #[serde(rename = "f")]
    pub f: String,

    /// Operator
    // Note: inline enums are not fully supported by openapi-generator
    #[serde(rename = "o")]
    pub o: String,

    #[serde(rename = "v")]
    pub v: models::SearchFilterFieldOpValueV,
}

impl SearchFilterFieldOpValue {
    #[allow(clippy::new_without_default)]
    #[allow(dead_code)]
    pub fn new(
        f: String,
        o: String,
        v: models::SearchFilterFieldOpValueV,
    ) -> SearchFilterFieldOpValue {
        SearchFilterFieldOpValue { f, o, v }
    }
}

/// Converts the SearchFilterFieldOpValue value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::string::ToString for SearchFilterFieldOpValue {
    fn to_string(&self) -> String {
        let params: Vec<Option<String>> = vec![
            Some("f".to_string()),
            Some(self.f.to_string()),
            Some("o".to_string()),
            Some(self.o.to_string()),
            // Skipping v in query parameter serialization
        ];

        params.into_iter().flatten().collect::<Vec<_>>().join(",")
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a SearchFilterFieldOpValue value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for SearchFilterFieldOpValue {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub f: Vec<String>,
            pub o: Vec<String>,
            pub v: Vec<models::SearchFilterFieldOpValueV>,
        }

        let mut intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let val = match string_iter.next() {
                Some(x) => x,
                None => {
                    return std::result::Result::Err(
                        "Missing value while parsing SearchFilterFieldOpValue".to_string(),
                    )
                }
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "f" => intermediate_rep.f.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "o" => intermediate_rep.o.push(
                        <String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?,
                    ),
                    #[allow(clippy::redundant_clone)]
                    "v" => intermediate_rep.v.push(
                        <models::SearchFilterFieldOpValueV as std::str::FromStr>::from_str(val)
                            .map_err(|x| x.to_string())?,
                    ),
                    _ => {
                        return std::result::Result::Err(
                            "Unexpected key while parsing SearchFilterFieldOpValue".to_string(),
                        )
                    }
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(SearchFilterFieldOpValue {
            f: intermediate_rep
                .f
                .into_iter()
                .next()
                .ok_or_else(|| "f missing in SearchFilterFieldOpValue".to_string())?,
            o: intermediate_rep
                .o
                .into_iter()
                .next()
                .ok_or_else(|| "o missing in SearchFilterFieldOpValue".to_string())?,
            v: intermediate_rep
                .v
                .into_iter()
                .next()
                .ok_or_else(|| "v missing in SearchFilterFieldOpValue".to_string())?,
        })
    }
}

/// Value
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
pub struct SearchFilterFieldOpValueV {}

impl SearchFilterFieldOpValueV {
    #[allow(clippy::new_without_default)]
    #[allow(dead_code)]
    pub fn new() -> SearchFilterFieldOpValueV {
        SearchFilterFieldOpValueV {}
    }
}

/// Converts the SearchFilterFieldOpValueV value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::string::ToString for SearchFilterFieldOpValueV {
    fn to_string(&self) -> String {
        let params: Vec<Option<String>> = vec![];

        params.into_iter().flatten().collect::<Vec<_>>().join(",")
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a SearchFilterFieldOpValueV value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for SearchFilterFieldOpValueV {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {}

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let _val = match string_iter.next() {
                Some(x) => x,
                None => {
                    return std::result::Result::Err(
                        "Missing value while parsing SearchFilterFieldOpValueV".to_string(),
                    )
                }
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    _ => {
                        return std::result::Result::Err(
                            "Unexpected key while parsing SearchFilterFieldOpValueV".to_string(),
                        )
                    }
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(SearchFilterFieldOpValueV {})
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
pub struct SearchFilterOrGroup {
    /// A list of search filters
    #[serde(rename = "or")]
    pub or: Vec<models::SearchFilter>,
}

impl SearchFilterOrGroup {
    #[allow(clippy::new_without_default)]
    #[allow(dead_code)]
    pub fn new(or: Vec<models::SearchFilter>) -> SearchFilterOrGroup {
        SearchFilterOrGroup { or }
    }
}

/// Converts the SearchFilterOrGroup value to the Query Parameters representation (style=form, explode=false)
/// specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde serializer
impl std::string::ToString for SearchFilterOrGroup {
    fn to_string(&self) -> String {
        let params: Vec<Option<String>> = vec![
            // Skipping or in query parameter serialization

        ];

        params.into_iter().flatten().collect::<Vec<_>>().join(",")
    }
}

/// Converts Query Parameters representation (style=form, explode=false) to a SearchFilterOrGroup value
/// as specified in https://swagger.io/docs/specification/serialization/
/// Should be implemented in a serde deserializer
impl std::str::FromStr for SearchFilterOrGroup {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        /// An intermediate representation of the struct to use for parsing.
        #[derive(Default)]
        #[allow(dead_code)]
        struct IntermediateRep {
            pub or: Vec<Vec<models::SearchFilter>>,
        }

        let intermediate_rep = IntermediateRep::default();

        // Parse into intermediate representation
        let mut string_iter = s.split(',');
        let mut key_result = string_iter.next();

        while key_result.is_some() {
            let _val = match string_iter.next() {
                Some(x) => x,
                None => {
                    return std::result::Result::Err(
                        "Missing value while parsing SearchFilterOrGroup".to_string(),
                    )
                }
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    "or" => return std::result::Result::Err(
                        "Parsing a container in this style is not supported in SearchFilterOrGroup"
                            .to_string(),
                    ),
                    _ => {
                        return std::result::Result::Err(
                            "Unexpected key while parsing SearchFilterOrGroup".to_string(),
                        )
                    }
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(SearchFilterOrGroup {
            or: intermediate_rep
                .or
                .into_iter()
                .next()
                .ok_or_else(|| "or missing in SearchFilterOrGroup".to_string())?,
        })
    }
}

/// A boolean value
#[derive(Debug, Clone, PartialEq, PartialOrd, serde::Serialize, serde::Deserialize)]
pub struct ValueBoolean(bool);

impl std::convert::From<bool> for ValueBoolean {
    fn from(x: bool) -> Self {
        ValueBoolean(x)
    }
}

impl std::convert::From<ValueBoolean> for bool {
    fn from(x: ValueBoolean) -> Self {
        x.0
    }
}

impl std::ops::Deref for ValueBoolean {
    type Target = bool;
    fn deref(&self) -> &bool {
        &self.0
    }
}

impl std::ops::DerefMut for ValueBoolean {
    fn deref_mut(&mut self) -> &mut bool {
        &mut self.0
    }
}

/// A number value
#[derive(Debug, Clone, PartialEq, PartialOrd, serde::Serialize, serde::Deserialize)]
pub struct ValueNumber(f64);

impl std::convert::From<f64> for ValueNumber {
    fn from(x: f64) -> Self {
        ValueNumber(x)
    }
}

impl std::convert::From<ValueNumber> for f64 {
    fn from(x: ValueNumber) -> Self {
        x.0
    }
}

impl std::ops::Deref for ValueNumber {
    type Target = f64;
    fn deref(&self) -> &f64 {
        &self.0
    }
}

impl std::ops::DerefMut for ValueNumber {
    fn deref_mut(&mut self) -> &mut f64 {
        &mut self.0
    }
}

/// A string value
#[derive(Debug, Clone, PartialEq, PartialOrd, serde::Serialize, serde::Deserialize)]
pub struct ValueString(String);

impl std::convert::From<String> for ValueString {
    fn from(x: String) -> Self {
        ValueString(x)
    }
}

impl std::string::ToString for ValueString {
    fn to_string(&self) -> String {
        self.0.to_string()
    }
}

impl std::str::FromStr for ValueString {
    type Err = std::string::ParseError;
    fn from_str(x: &str) -> std::result::Result<Self, Self::Err> {
        std::result::Result::Ok(ValueString(x.to_string()))
    }
}

impl std::convert::From<ValueString> for String {
    fn from(x: ValueString) -> Self {
        x.0
    }
}

impl std::ops::Deref for ValueString {
    type Target = String;
    fn deref(&self) -> &String {
        &self.0
    }
}

impl std::ops::DerefMut for ValueString {
    fn deref_mut(&mut self) -> &mut String {
        &mut self.0
    }
}
