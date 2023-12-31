//! Main library entry point for openapi implementation.

#![allow(unused_imports)]

use async_trait::async_trait;
use futures::{future, Stream, StreamExt, TryFutureExt, TryStreamExt};
use hyper::server::conn::Http;
use hyper::service::Service;
use log::info;
use std::future::Future;
use std::marker::PhantomData;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use swagger::{Has, XSpanIdString};
use swagger::auth::MakeAllowAllAuthenticator;
use swagger::EmptyContext;
use tokio::net::TcpListener;

#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "ios")))]
use openssl::ssl::{Ssl, SslAcceptor, SslAcceptorBuilder, SslFiletype, SslMethod};

use openapi::models;

/// Builds an SSL implementation for Simple HTTPS from some hard-coded file names
pub async fn create(addr: &str, https: bool) {
    let addr = addr.parse().expect("Failed to parse bind address");

    let server = Server::new();

    let service = MakeService::new(server);

    let service = MakeAllowAllAuthenticator::new(service, "cosmo");

    #[allow(unused_mut)]
    let mut service =
        openapi::server::context::MakeAddContext::<_, EmptyContext>::new(
            service
        );

    if https {
        #[cfg(any(target_os = "macos", target_os = "windows", target_os = "ios"))]
        {
            unimplemented!("SSL is not implemented for the examples on MacOS, Windows or iOS");
        }

        #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "ios")))]
        {
            let mut ssl = SslAcceptor::mozilla_intermediate_v5(SslMethod::tls()).expect("Failed to create SSL Acceptor");

            // Server authentication
            ssl.set_private_key_file("examples/server-key.pem", SslFiletype::PEM).expect("Failed to set private key");
            ssl.set_certificate_chain_file("examples/server-chain.pem").expect("Failed to set certificate chain");
            ssl.check_private_key().expect("Failed to check private key");

            let tls_acceptor = ssl.build();
            let tcp_listener = TcpListener::bind(&addr).await.unwrap();

            loop {
                if let Ok((tcp, _)) = tcp_listener.accept().await {
                    let ssl = Ssl::new(tls_acceptor.context()).unwrap();
                    let addr = tcp.peer_addr().expect("Unable to get remote address");
                    let service = service.call(addr);

                    tokio::spawn(async move {
                        let tls = tokio_openssl::SslStream::new(ssl, tcp).map_err(|_| ())?;
                        let service = service.await.map_err(|_| ())?;

                        Http::new()
                            .serve_connection(tls, service)
                            .await
                            .map_err(|_| ())
                    });
                }
            }
        }
    } else {
        // Using HTTP
        hyper::server::Server::bind(&addr).serve(service).await.unwrap()
    }
}

#[derive(Copy, Clone)]
pub struct Server<C> {
    marker: PhantomData<C>,
}

impl<C> Server<C> {
    pub fn new() -> Self {
        Server{marker: PhantomData}
    }
}


use openapi::{
    Api,
    CreateCollectionResponse,
    GetCollectionsResponse,
    GetItemByIdResponse,
    ListCollectionResponse,
    ListRecoverablesInCollectionResponse,
    StoreIntoCollectionResponse,
    UpdateItemByIdResponse,
    CreateEventResponse,
    RebuildGrantsResponse,
};
use openapi::server::MakeService;
use std::error::Error;
use swagger::ApiError;

#[async_trait]
impl<C> Api<C> for Server<C> where C: Has<XSpanIdString> + Send + Sync
{
    /// Create a collection
    async fn create_collection(
        &self,
        create_collection_request: models::CreateCollectionRequest,
        context: &C) -> Result<CreateCollectionResponse, ApiError>
    {
        info!("create_collection({:?}) - X-Span-ID: {:?}", create_collection_request, context.get().0.clone());
        Err(ApiError("Generic failure".into()))
    }

    /// List available collections
    async fn get_collections(
        &self,
        context: &C) -> Result<GetCollectionsResponse, ApiError>
    {
        info!("get_collections() - X-Span-ID: {:?}", context.get().0.clone());
        Err(ApiError("Generic failure".into()))
    }

    /// Get item
    async fn get_item_by_id(
        &self,
        collection: String,
        document_id: uuid::Uuid,
        context: &C) -> Result<GetItemByIdResponse, ApiError>
    {
        info!("get_item_by_id(\"{}\", {:?}) - X-Span-ID: {:?}", collection, document_id, context.get().0.clone());
        Err(ApiError("Generic failure".into()))
    }

    /// List collection items
    async fn list_collection(
        &self,
        collection: String,
        exact_title: Option<String>,
        extra_fields: Option<String>,
        limit: Option<i32>,
        offset: Option<i32>,
        pfilter: Option<String>,
        sort: Option<String>,
        context: &C) -> Result<ListCollectionResponse, ApiError>
    {
        info!("list_collection(\"{}\", {:?}, {:?}, {:?}, {:?}, {:?}, {:?}) - X-Span-ID: {:?}", collection, exact_title, extra_fields, limit, offset, pfilter, sort, context.get().0.clone());
        Err(ApiError("Generic failure".into()))
    }

    /// List recoverable items within the collection
    async fn list_recoverables_in_collection(
        &self,
        collection: String,
        exact_title: Option<String>,
        extra_fields: Option<String>,
        limit: Option<i32>,
        offset: Option<i32>,
        pfilter: Option<String>,
        sort: Option<String>,
        context: &C) -> Result<ListRecoverablesInCollectionResponse, ApiError>
    {
        info!("list_recoverables_in_collection(\"{}\", {:?}, {:?}, {:?}, {:?}, {:?}, {:?}) - X-Span-ID: {:?}", collection, exact_title, extra_fields, limit, offset, pfilter, sort, context.get().0.clone());
        Err(ApiError("Generic failure".into()))
    }

    /// Create new item
    async fn store_into_collection(
        &self,
        collection: String,
        collection_item: models::CollectionItem,
        context: &C) -> Result<StoreIntoCollectionResponse, ApiError>
    {
        info!("store_into_collection(\"{}\", {:?}) - X-Span-ID: {:?}", collection, collection_item, context.get().0.clone());
        Err(ApiError("Generic failure".into()))
    }

    /// Replace item
    async fn update_item_by_id(
        &self,
        collection: String,
        collection_item: models::CollectionItem,
        context: &C) -> Result<UpdateItemByIdResponse, ApiError>
    {
        info!("update_item_by_id(\"{}\", {:?}) - X-Span-ID: {:?}", collection, collection_item, context.get().0.clone());
        Err(ApiError("Generic failure".into()))
    }

    /// Create event for document in collection
    async fn create_event(
        &self,
        create_event_body: models::CreateEventBody,
        context: &C) -> Result<CreateEventResponse, ApiError>
    {
        info!("create_event({:?}) - X-Span-ID: {:?}", create_event_body, context.get().0.clone());
        Err(ApiError("Generic failure".into()))
    }

    /// Rebuild grants for a collection
    async fn rebuild_grants(
        &self,
        collection: String,
        context: &C) -> Result<RebuildGrantsResponse, ApiError>
    {
        info!("rebuild_grants(\"{}\") - X-Span-ID: {:?}", collection, context.get().0.clone());
        Err(ApiError("Generic failure".into()))
    }

}
