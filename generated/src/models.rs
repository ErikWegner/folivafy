#![allow(unused_qualifications)]

use validator::Validate;

use crate::models;
#[cfg(any(feature = "client", feature = "server"))]
use crate::header;

/// Arbitrary event category
#[derive(Debug, Clone, PartialEq, PartialOrd, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
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


#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct Collection {
    /// Path name of the collection
    #[serde(rename = "name")]
    #[validate(
            length(min = 1, max = 32),
           regex = "RE_COLLECTION_NAME",
        )]
    pub name: String,

    /// Human readable name of the collection
    #[serde(rename = "title")]
    #[validate(
            length(min = 1, max = 150),
        )]
    pub title: String,

    /// Owner access only. Indicates if documents within the collection are _owner access only_ (value `true`) or all documents in the collection can be read by all users (`false`). 
    #[serde(rename = "oao")]
    pub oao: bool,

    /// Indicates if new documents within the collection can be created (value `false`) or the collection is set to read only (`true`). 
    #[serde(rename = "locked")]
    pub locked: bool,

}

lazy_static::lazy_static! {
    static ref RE_COLLECTION_NAME: regex::Regex = regex::Regex::new(r"^[a-z][-a-z0-9]*$").unwrap();
}

impl Collection {
    #[allow(clippy::new_without_default)]
    pub fn new(name: String, title: String, oao: bool, locked: bool, ) -> Collection {
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
                None => return std::result::Result::Err("Missing value while parsing Collection".to_string())
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "name" => intermediate_rep.name.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "title" => intermediate_rep.title.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "oao" => intermediate_rep.oao.push(<bool as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "locked" => intermediate_rep.locked.push(<bool as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    _ => return std::result::Result::Err("Unexpected key while parsing Collection".to_string())
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(Collection {
            name: intermediate_rep.name.into_iter().next().ok_or_else(|| "name missing in Collection".to_string())?,
            title: intermediate_rep.title.into_iter().next().ok_or_else(|| "title missing in Collection".to_string())?,
            oao: intermediate_rep.oao.into_iter().next().ok_or_else(|| "oao missing in Collection".to_string())?,
            locked: intermediate_rep.locked.into_iter().next().ok_or_else(|| "locked missing in Collection".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<Collection> and hyper::header::HeaderValue

#[cfg(any(feature = "client", feature = "server"))]
impl std::convert::TryFrom<header::IntoHeaderValue<Collection>> for hyper::header::HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<Collection>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match hyper::header::HeaderValue::from_str(&hdr_value) {
             std::result::Result::Ok(value) => std::result::Result::Ok(value),
             std::result::Result::Err(e) => std::result::Result::Err(
                 format!("Invalid header value for Collection - value: {} is invalid {}",
                     hdr_value, e))
        }
    }
}

#[cfg(any(feature = "client", feature = "server"))]
impl std::convert::TryFrom<hyper::header::HeaderValue> for header::IntoHeaderValue<Collection> {
    type Error = String;

    fn try_from(hdr_value: hyper::header::HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
             std::result::Result::Ok(value) => {
                    match <Collection as std::str::FromStr>::from_str(value) {
                        std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                        std::result::Result::Err(err) => std::result::Result::Err(
                            format!("Unable to convert header value '{}' into Collection - {}",
                                value, err))
                    }
             },
             std::result::Result::Err(e) => std::result::Result::Err(
                 format!("Unable to convert header: {:?} to string: {}",
                     hdr_value, e))
        }
    }
}


#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct CollectionItem {
    /// Document identifier
    #[serde(rename = "id")]
    pub id: uuid::Uuid,

    /// Field data
    #[serde(rename = "f")]
    pub f: serde_json::Value,

}


impl CollectionItem {
    #[allow(clippy::new_without_default)]
    pub fn new(id: uuid::Uuid, f: serde_json::Value, ) -> CollectionItem {
        CollectionItem {
            id,
            f,
        }
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
                None => return std::result::Result::Err("Missing value while parsing CollectionItem".to_string())
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "id" => intermediate_rep.id.push(<uuid::Uuid as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "f" => intermediate_rep.f.push(<serde_json::Value as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    _ => return std::result::Result::Err("Unexpected key while parsing CollectionItem".to_string())
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(CollectionItem {
            id: intermediate_rep.id.into_iter().next().ok_or_else(|| "id missing in CollectionItem".to_string())?,
            f: intermediate_rep.f.into_iter().next().ok_or_else(|| "f missing in CollectionItem".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<CollectionItem> and hyper::header::HeaderValue

#[cfg(any(feature = "client", feature = "server"))]
impl std::convert::TryFrom<header::IntoHeaderValue<CollectionItem>> for hyper::header::HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<CollectionItem>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match hyper::header::HeaderValue::from_str(&hdr_value) {
             std::result::Result::Ok(value) => std::result::Result::Ok(value),
             std::result::Result::Err(e) => std::result::Result::Err(
                 format!("Invalid header value for CollectionItem - value: {} is invalid {}",
                     hdr_value, e))
        }
    }
}

#[cfg(any(feature = "client", feature = "server"))]
impl std::convert::TryFrom<hyper::header::HeaderValue> for header::IntoHeaderValue<CollectionItem> {
    type Error = String;

    fn try_from(hdr_value: hyper::header::HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
             std::result::Result::Ok(value) => {
                    match <CollectionItem as std::str::FromStr>::from_str(value) {
                        std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                        std::result::Result::Err(err) => std::result::Result::Err(
                            format!("Unable to convert header value '{}' into CollectionItem - {}",
                                value, err))
                    }
             },
             std::result::Result::Err(e) => std::result::Result::Err(
                 format!("Unable to convert header: {:?} to string: {}",
                     hdr_value, e))
        }
    }
}


#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct CollectionItemsList {
    #[serde(rename = "limit")]
    #[validate(
            range(min = 1, max = 250),
        )]
    pub limit: u8,

    #[serde(rename = "offset")]
    #[validate(
            range(min = 0),
        )]
    pub offset: u32,

    #[serde(rename = "total")]
    #[validate(
            range(min = 0),
        )]
    pub total: u32,

    #[serde(rename = "items")]
    pub items: Vec<models::CollectionItem>,

}


impl CollectionItemsList {
    #[allow(clippy::new_without_default)]
    pub fn new(items: Vec<models::CollectionItem>, ) -> CollectionItemsList {
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
                None => return std::result::Result::Err("Missing value while parsing CollectionItemsList".to_string())
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "limit" => intermediate_rep.limit.push(<u8 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "offset" => intermediate_rep.offset.push(<u32 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "total" => intermediate_rep.total.push(<u32 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    "items" => return std::result::Result::Err("Parsing a container in this style is not supported in CollectionItemsList".to_string()),
                    _ => return std::result::Result::Err("Unexpected key while parsing CollectionItemsList".to_string())
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(CollectionItemsList {
            limit: intermediate_rep.limit.into_iter().next().ok_or_else(|| "limit missing in CollectionItemsList".to_string())?,
            offset: intermediate_rep.offset.into_iter().next().ok_or_else(|| "offset missing in CollectionItemsList".to_string())?,
            total: intermediate_rep.total.into_iter().next().ok_or_else(|| "total missing in CollectionItemsList".to_string())?,
            items: intermediate_rep.items.into_iter().next().ok_or_else(|| "items missing in CollectionItemsList".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<CollectionItemsList> and hyper::header::HeaderValue

#[cfg(any(feature = "client", feature = "server"))]
impl std::convert::TryFrom<header::IntoHeaderValue<CollectionItemsList>> for hyper::header::HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<CollectionItemsList>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match hyper::header::HeaderValue::from_str(&hdr_value) {
             std::result::Result::Ok(value) => std::result::Result::Ok(value),
             std::result::Result::Err(e) => std::result::Result::Err(
                 format!("Invalid header value for CollectionItemsList - value: {} is invalid {}",
                     hdr_value, e))
        }
    }
}

#[cfg(any(feature = "client", feature = "server"))]
impl std::convert::TryFrom<hyper::header::HeaderValue> for header::IntoHeaderValue<CollectionItemsList> {
    type Error = String;

    fn try_from(hdr_value: hyper::header::HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
             std::result::Result::Ok(value) => {
                    match <CollectionItemsList as std::str::FromStr>::from_str(value) {
                        std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                        std::result::Result::Err(err) => std::result::Result::Err(
                            format!("Unable to convert header value '{}' into CollectionItemsList - {}",
                                value, err))
                    }
             },
             std::result::Result::Err(e) => std::result::Result::Err(
                 format!("Unable to convert header: {:?} to string: {}",
                     hdr_value, e))
        }
    }
}


/// Path name of the collection
#[derive(Debug, Clone, PartialEq, PartialOrd, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
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


#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct CollectionsList {
    #[serde(rename = "limit")]
    #[validate(
            range(min = 1, max = 250),
        )]
    pub limit: u8,

    #[serde(rename = "offset")]
    #[validate(
            range(min = 0),
        )]
    pub offset: u32,

    #[serde(rename = "total")]
    #[validate(
            range(min = 0),
        )]
    pub total: u32,

    #[serde(rename = "items")]
    pub items: Vec<models::Collection>,

}


impl CollectionsList {
    #[allow(clippy::new_without_default)]
    pub fn new(items: Vec<models::Collection>, ) -> CollectionsList {
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
                None => return std::result::Result::Err("Missing value while parsing CollectionsList".to_string())
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "limit" => intermediate_rep.limit.push(<u8 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "offset" => intermediate_rep.offset.push(<u32 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "total" => intermediate_rep.total.push(<u32 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    "items" => return std::result::Result::Err("Parsing a container in this style is not supported in CollectionsList".to_string()),
                    _ => return std::result::Result::Err("Unexpected key while parsing CollectionsList".to_string())
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(CollectionsList {
            limit: intermediate_rep.limit.into_iter().next().ok_or_else(|| "limit missing in CollectionsList".to_string())?,
            offset: intermediate_rep.offset.into_iter().next().ok_or_else(|| "offset missing in CollectionsList".to_string())?,
            total: intermediate_rep.total.into_iter().next().ok_or_else(|| "total missing in CollectionsList".to_string())?,
            items: intermediate_rep.items.into_iter().next().ok_or_else(|| "items missing in CollectionsList".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<CollectionsList> and hyper::header::HeaderValue

#[cfg(any(feature = "client", feature = "server"))]
impl std::convert::TryFrom<header::IntoHeaderValue<CollectionsList>> for hyper::header::HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<CollectionsList>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match hyper::header::HeaderValue::from_str(&hdr_value) {
             std::result::Result::Ok(value) => std::result::Result::Ok(value),
             std::result::Result::Err(e) => std::result::Result::Err(
                 format!("Invalid header value for CollectionsList - value: {} is invalid {}",
                     hdr_value, e))
        }
    }
}

#[cfg(any(feature = "client", feature = "server"))]
impl std::convert::TryFrom<hyper::header::HeaderValue> for header::IntoHeaderValue<CollectionsList> {
    type Error = String;

    fn try_from(hdr_value: hyper::header::HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
             std::result::Result::Ok(value) => {
                    match <CollectionsList as std::str::FromStr>::from_str(value) {
                        std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                        std::result::Result::Err(err) => std::result::Result::Err(
                            format!("Unable to convert header value '{}' into CollectionsList - {}",
                                value, err))
                    }
             },
             std::result::Result::Err(e) => std::result::Result::Err(
                 format!("Unable to convert header: {:?} to string: {}",
                     hdr_value, e))
        }
    }
}


#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct CreateCollectionRequest {
    /// Path name of the collection
    #[serde(rename = "name")]
    #[validate(
            length(min = 1, max = 32),
           regex = "RE_CREATECOLLECTIONREQUEST_NAME",
        )]
    pub name: String,

    /// Human readable name of the collection
    #[serde(rename = "title")]
    #[validate(
            length(min = 1, max = 150),
        )]
    pub title: String,

    /// Owner access only?
    #[serde(rename = "oao")]
    pub oao: bool,

}

lazy_static::lazy_static! {
    static ref RE_CREATECOLLECTIONREQUEST_NAME: regex::Regex = regex::Regex::new(r"^[a-z][-a-z0-9]*$").unwrap();
}

impl CreateCollectionRequest {
    #[allow(clippy::new_without_default)]
    pub fn new(name: String, title: String, oao: bool, ) -> CreateCollectionRequest {
        CreateCollectionRequest {
            name,
            title,
            oao,
        }
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
                None => return std::result::Result::Err("Missing value while parsing CreateCollectionRequest".to_string())
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "name" => intermediate_rep.name.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "title" => intermediate_rep.title.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "oao" => intermediate_rep.oao.push(<bool as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    _ => return std::result::Result::Err("Unexpected key while parsing CreateCollectionRequest".to_string())
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(CreateCollectionRequest {
            name: intermediate_rep.name.into_iter().next().ok_or_else(|| "name missing in CreateCollectionRequest".to_string())?,
            title: intermediate_rep.title.into_iter().next().ok_or_else(|| "title missing in CreateCollectionRequest".to_string())?,
            oao: intermediate_rep.oao.into_iter().next().ok_or_else(|| "oao missing in CreateCollectionRequest".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<CreateCollectionRequest> and hyper::header::HeaderValue

#[cfg(any(feature = "client", feature = "server"))]
impl std::convert::TryFrom<header::IntoHeaderValue<CreateCollectionRequest>> for hyper::header::HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<CreateCollectionRequest>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match hyper::header::HeaderValue::from_str(&hdr_value) {
             std::result::Result::Ok(value) => std::result::Result::Ok(value),
             std::result::Result::Err(e) => std::result::Result::Err(
                 format!("Invalid header value for CreateCollectionRequest - value: {} is invalid {}",
                     hdr_value, e))
        }
    }
}

#[cfg(any(feature = "client", feature = "server"))]
impl std::convert::TryFrom<hyper::header::HeaderValue> for header::IntoHeaderValue<CreateCollectionRequest> {
    type Error = String;

    fn try_from(hdr_value: hyper::header::HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
             std::result::Result::Ok(value) => {
                    match <CreateCollectionRequest as std::str::FromStr>::from_str(value) {
                        std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                        std::result::Result::Err(err) => std::result::Result::Err(
                            format!("Unable to convert header value '{}' into CreateCollectionRequest - {}",
                                value, err))
                    }
             },
             std::result::Result::Err(e) => std::result::Result::Err(
                 format!("Unable to convert header: {:?} to string: {}",
                     hdr_value, e))
        }
    }
}


#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
pub struct CreateEventBody {
    /// Arbitrary event category
    #[serde(rename = "category")]
    pub category: i32,

    /// Path name of the collection
    #[serde(rename = "collection")]
    #[validate(
            length(min = 1, max = 32),
           regex = "RE_CREATEEVENTBODY_COLLECTION",
        )]
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
    pub fn new(category: i32, collection: String, document: uuid::Uuid, e: serde_json::Value, ) -> CreateEventBody {
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
                None => return std::result::Result::Err("Missing value while parsing CreateEventBody".to_string())
            };

            if let Some(key) = key_result {
                #[allow(clippy::match_single_binding)]
                match key {
                    #[allow(clippy::redundant_clone)]
                    "category" => intermediate_rep.category.push(<i32 as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "collection" => intermediate_rep.collection.push(<String as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "document" => intermediate_rep.document.push(<uuid::Uuid as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    #[allow(clippy::redundant_clone)]
                    "e" => intermediate_rep.e.push(<serde_json::Value as std::str::FromStr>::from_str(val).map_err(|x| x.to_string())?),
                    _ => return std::result::Result::Err("Unexpected key while parsing CreateEventBody".to_string())
                }
            }

            // Get the next key
            key_result = string_iter.next();
        }

        // Use the intermediate representation to return the struct
        std::result::Result::Ok(CreateEventBody {
            category: intermediate_rep.category.into_iter().next().ok_or_else(|| "category missing in CreateEventBody".to_string())?,
            collection: intermediate_rep.collection.into_iter().next().ok_or_else(|| "collection missing in CreateEventBody".to_string())?,
            document: intermediate_rep.document.into_iter().next().ok_or_else(|| "document missing in CreateEventBody".to_string())?,
            e: intermediate_rep.e.into_iter().next().ok_or_else(|| "e missing in CreateEventBody".to_string())?,
        })
    }
}

// Methods for converting between header::IntoHeaderValue<CreateEventBody> and hyper::header::HeaderValue

#[cfg(any(feature = "client", feature = "server"))]
impl std::convert::TryFrom<header::IntoHeaderValue<CreateEventBody>> for hyper::header::HeaderValue {
    type Error = String;

    fn try_from(hdr_value: header::IntoHeaderValue<CreateEventBody>) -> std::result::Result<Self, Self::Error> {
        let hdr_value = hdr_value.to_string();
        match hyper::header::HeaderValue::from_str(&hdr_value) {
             std::result::Result::Ok(value) => std::result::Result::Ok(value),
             std::result::Result::Err(e) => std::result::Result::Err(
                 format!("Invalid header value for CreateEventBody - value: {} is invalid {}",
                     hdr_value, e))
        }
    }
}

#[cfg(any(feature = "client", feature = "server"))]
impl std::convert::TryFrom<hyper::header::HeaderValue> for header::IntoHeaderValue<CreateEventBody> {
    type Error = String;

    fn try_from(hdr_value: hyper::header::HeaderValue) -> std::result::Result<Self, Self::Error> {
        match hdr_value.to_str() {
             std::result::Result::Ok(value) => {
                    match <CreateEventBody as std::str::FromStr>::from_str(value) {
                        std::result::Result::Ok(value) => std::result::Result::Ok(header::IntoHeaderValue(value)),
                        std::result::Result::Err(err) => std::result::Result::Err(
                            format!("Unable to convert header value '{}' into CreateEventBody - {}",
                                value, err))
                    }
             },
             std::result::Result::Err(e) => std::result::Result::Err(
                 format!("Unable to convert header: {:?} to string: {}",
                     hdr_value, e))
        }
    }
}


/// Document identifier
#[derive(Debug, Clone, PartialEq, PartialOrd, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "conversion", derive(frunk::LabelledGeneric))]
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

