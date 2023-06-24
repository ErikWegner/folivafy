#![allow(missing_docs, trivial_casts, unused_variables, unused_mut, unused_imports, unused_extern_crates, non_camel_case_types)]
#![allow(unused_imports, unused_attributes)]
#![allow(clippy::derive_partial_eq_without_eq, clippy::disallowed_names)]

use async_trait::async_trait;
use futures::Stream;
use std::error::Error;
use std::task::{Poll, Context};
use swagger::{ApiError, ContextWrapper};
use serde::{Serialize, Deserialize};

type ServiceError = Box<dyn Error + Send + Sync + 'static>;

pub const BASE_PATH: &str = "/api";
pub const API_VERSION: &str = "1.0.0";

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
pub enum CreateCollectionResponse {
    /// successful operation
    SuccessfulOperation
    (String)
    ,
    /// Creating the collection failed
    CreatingTheCollectionFailed
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum GetCollectionsResponse {
    /// Successful operation
    SuccessfulOperation
    (models::CollectionsList)
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
pub enum GetItemByIdResponse {
    /// successful operation
    SuccessfulOperation
    (models::CollectionItem)
    ,
    /// Item not found
    ItemNotFound
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
pub enum ListCollectionResponse {
    /// successful operation
    SuccessfulOperation
    (models::CollectionItemsList)
    ,
    /// Collection not found
    CollectionNotFound
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
pub enum StoreIntoCollectionResponse {
    /// successful operation
    SuccessfulOperation
    (String)
    ,
    /// Creating the collection failed
    CreatingTheCollectionFailed
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
pub enum UpdateItemByIdResponse {
    /// successful operation
    SuccessfulOperation
    (String)
    ,
    /// Updating failed
    UpdatingFailed
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
pub enum CreateEventResponse {
    /// successful operation
    SuccessfulOperation
    (String)
    ,
    /// Creating the collection failed
    CreatingTheCollectionFailed
}

/// API
#[async_trait]
#[allow(clippy::too_many_arguments, clippy::ptr_arg)]
pub trait Api<C: Send + Sync> {
    fn poll_ready(&self, _cx: &mut Context) -> Poll<Result<(), Box<dyn Error + Send + Sync + 'static>>> {
        Poll::Ready(Ok(()))
    }

    /// Create a collection
    async fn create_collection(
        &self,
        create_collection_request: models::CreateCollectionRequest,
        context: &C) -> Result<CreateCollectionResponse, ApiError>;

    /// List available collections
    async fn get_collections(
        &self,
        context: &C) -> Result<GetCollectionsResponse, ApiError>;

    /// Get item
    async fn get_item_by_id(
        &self,
        collection: String,
        document_id: uuid::Uuid,
        context: &C) -> Result<GetItemByIdResponse, ApiError>;

    /// List collection items
    async fn list_collection(
        &self,
        collection: String,
        extra_fields: Option<String>,
        exact_title: Option<String>,
        context: &C) -> Result<ListCollectionResponse, ApiError>;

    /// Create new item
    async fn store_into_collection(
        &self,
        collection: String,
        collection_item: models::CollectionItem,
        context: &C) -> Result<StoreIntoCollectionResponse, ApiError>;

    /// Replace item
    async fn update_item_by_id(
        &self,
        collection: String,
        collection_item: models::CollectionItem,
        context: &C) -> Result<UpdateItemByIdResponse, ApiError>;

    async fn create_event(
        &self,
        create_event_body: models::CreateEventBody,
        context: &C) -> Result<CreateEventResponse, ApiError>;

}

/// API where `Context` isn't passed on every API call
#[async_trait]
#[allow(clippy::too_many_arguments, clippy::ptr_arg)]
pub trait ApiNoContext<C: Send + Sync> {

    fn poll_ready(&self, _cx: &mut Context) -> Poll<Result<(), Box<dyn Error + Send + Sync + 'static>>>;

    fn context(&self) -> &C;

    /// Create a collection
    async fn create_collection(
        &self,
        create_collection_request: models::CreateCollectionRequest,
        ) -> Result<CreateCollectionResponse, ApiError>;

    /// List available collections
    async fn get_collections(
        &self,
        ) -> Result<GetCollectionsResponse, ApiError>;

    /// Get item
    async fn get_item_by_id(
        &self,
        collection: String,
        document_id: uuid::Uuid,
        ) -> Result<GetItemByIdResponse, ApiError>;

    /// List collection items
    async fn list_collection(
        &self,
        collection: String,
        extra_fields: Option<String>,
        exact_title: Option<String>,
        ) -> Result<ListCollectionResponse, ApiError>;

    /// Create new item
    async fn store_into_collection(
        &self,
        collection: String,
        collection_item: models::CollectionItem,
        ) -> Result<StoreIntoCollectionResponse, ApiError>;

    /// Replace item
    async fn update_item_by_id(
        &self,
        collection: String,
        collection_item: models::CollectionItem,
        ) -> Result<UpdateItemByIdResponse, ApiError>;

    async fn create_event(
        &self,
        create_event_body: models::CreateEventBody,
        ) -> Result<CreateEventResponse, ApiError>;

}

/// Trait to extend an API to make it easy to bind it to a context.
pub trait ContextWrapperExt<C: Send + Sync> where Self: Sized
{
    /// Binds this API to a context.
    fn with_context(self, context: C) -> ContextWrapper<Self, C>;
}

impl<T: Api<C> + Send + Sync, C: Clone + Send + Sync> ContextWrapperExt<C> for T {
    fn with_context(self: T, context: C) -> ContextWrapper<T, C> {
         ContextWrapper::<T, C>::new(self, context)
    }
}

#[async_trait]
impl<T: Api<C> + Send + Sync, C: Clone + Send + Sync> ApiNoContext<C> for ContextWrapper<T, C> {
    fn poll_ready(&self, cx: &mut Context) -> Poll<Result<(), ServiceError>> {
        self.api().poll_ready(cx)
    }

    fn context(&self) -> &C {
        ContextWrapper::context(self)
    }

    /// Create a collection
    async fn create_collection(
        &self,
        create_collection_request: models::CreateCollectionRequest,
        ) -> Result<CreateCollectionResponse, ApiError>
    {
        let context = self.context().clone();
        self.api().create_collection(create_collection_request, &context).await
    }

    /// List available collections
    async fn get_collections(
        &self,
        ) -> Result<GetCollectionsResponse, ApiError>
    {
        let context = self.context().clone();
        self.api().get_collections(&context).await
    }

    /// Get item
    async fn get_item_by_id(
        &self,
        collection: String,
        document_id: uuid::Uuid,
        ) -> Result<GetItemByIdResponse, ApiError>
    {
        let context = self.context().clone();
        self.api().get_item_by_id(collection, document_id, &context).await
    }

    /// List collection items
    async fn list_collection(
        &self,
        collection: String,
        extra_fields: Option<String>,
        exact_title: Option<String>,
        ) -> Result<ListCollectionResponse, ApiError>
    {
        let context = self.context().clone();
        self.api().list_collection(collection, extra_fields, exact_title, &context).await
    }

    /// Create new item
    async fn store_into_collection(
        &self,
        collection: String,
        collection_item: models::CollectionItem,
        ) -> Result<StoreIntoCollectionResponse, ApiError>
    {
        let context = self.context().clone();
        self.api().store_into_collection(collection, collection_item, &context).await
    }

    /// Replace item
    async fn update_item_by_id(
        &self,
        collection: String,
        collection_item: models::CollectionItem,
        ) -> Result<UpdateItemByIdResponse, ApiError>
    {
        let context = self.context().clone();
        self.api().update_item_by_id(collection, collection_item, &context).await
    }

    async fn create_event(
        &self,
        create_event_body: models::CreateEventBody,
        ) -> Result<CreateEventResponse, ApiError>
    {
        let context = self.context().clone();
        self.api().create_event(create_event_body, &context).await
    }

}


#[cfg(feature = "client")]
pub mod client;

// Re-export Client as a top-level name
#[cfg(feature = "client")]
pub use client::Client;

#[cfg(feature = "server")]
pub mod server;

// Re-export router() as a top-level name
#[cfg(feature = "server")]
pub use self::server::Service;

#[cfg(feature = "server")]
pub mod context;

pub mod models;

#[cfg(any(feature = "client", feature = "server"))]
pub(crate) mod header;
