use futures::{future, future::BoxFuture, Stream, stream, future::FutureExt, stream::TryStreamExt};
use hyper::{Request, Response, StatusCode, Body, HeaderMap};
use hyper::header::{HeaderName, HeaderValue, CONTENT_TYPE};
use log::warn;
#[allow(unused_imports)]
use std::convert::{TryFrom, TryInto};
use std::error::Error;
use std::future::Future;
use std::marker::PhantomData;
use std::task::{Context, Poll};
use swagger::{ApiError, BodyExt, Has, RequestParser, XSpanIdString};
pub use swagger::auth::Authorization;
use swagger::auth::Scopes;
use url::form_urlencoded;

#[allow(unused_imports)]
use crate::models;
use crate::header;

pub use crate::context;

type ServiceFuture = BoxFuture<'static, Result<Response<Body>, crate::ServiceError>>;

use crate::{Api,
     CreateCollectionResponse,
     GetCollectionsResponse,
     GetItemByIdResponse,
     ListCollectionResponse,
     ListRecoverablesInCollectionResponse,
     SearchCollectionResponse,
     StoreIntoCollectionResponse,
     UpdateItemByIdResponse,
     CreateEventResponse,
     RebuildGrantsResponse
};

mod paths {
    use lazy_static::lazy_static;

    lazy_static! {
        pub static ref GLOBAL_REGEX_SET: regex::RegexSet = regex::RegexSet::new(vec![
            r"^/api/collections$",
            r"^/api/collections/(?P<collection>[^/?#]*)$",
            r"^/api/collections/(?P<collection>[^/?#]*)/searches$",
            r"^/api/collections/(?P<collection>[^/?#]*)/(?P<documentId>[^/?#]*)$",
            r"^/api/events$",
            r"^/api/maintenance/(?P<collection>[^/?#]*)/rebuild-grants$",
            r"^/api/recoverables/(?P<collection>[^/?#]*)$"
        ])
        .expect("Unable to create global regex set");
    }
    pub(crate) static ID_COLLECTIONS: usize = 0;
    pub(crate) static ID_COLLECTIONS_COLLECTION: usize = 1;
    lazy_static! {
        pub static ref REGEX_COLLECTIONS_COLLECTION: regex::Regex =
            #[allow(clippy::invalid_regex)]
            regex::Regex::new(r"^/api/collections/(?P<collection>[^/?#]*)$")
                .expect("Unable to create regex for COLLECTIONS_COLLECTION");
    }
    pub(crate) static ID_COLLECTIONS_COLLECTION_SEARCHES: usize = 2;
    lazy_static! {
        pub static ref REGEX_COLLECTIONS_COLLECTION_SEARCHES: regex::Regex =
            #[allow(clippy::invalid_regex)]
            regex::Regex::new(r"^/api/collections/(?P<collection>[^/?#]*)/searches$")
                .expect("Unable to create regex for COLLECTIONS_COLLECTION_SEARCHES");
    }
    pub(crate) static ID_COLLECTIONS_COLLECTION_DOCUMENTID: usize = 3;
    lazy_static! {
        pub static ref REGEX_COLLECTIONS_COLLECTION_DOCUMENTID: regex::Regex =
            #[allow(clippy::invalid_regex)]
            regex::Regex::new(r"^/api/collections/(?P<collection>[^/?#]*)/(?P<documentId>[^/?#]*)$")
                .expect("Unable to create regex for COLLECTIONS_COLLECTION_DOCUMENTID");
    }
    pub(crate) static ID_EVENTS: usize = 4;
    pub(crate) static ID_MAINTENANCE_COLLECTION_REBUILD_GRANTS: usize = 5;
    lazy_static! {
        pub static ref REGEX_MAINTENANCE_COLLECTION_REBUILD_GRANTS: regex::Regex =
            #[allow(clippy::invalid_regex)]
            regex::Regex::new(r"^/api/maintenance/(?P<collection>[^/?#]*)/rebuild-grants$")
                .expect("Unable to create regex for MAINTENANCE_COLLECTION_REBUILD_GRANTS");
    }
    pub(crate) static ID_RECOVERABLES_COLLECTION: usize = 6;
    lazy_static! {
        pub static ref REGEX_RECOVERABLES_COLLECTION: regex::Regex =
            #[allow(clippy::invalid_regex)]
            regex::Regex::new(r"^/api/recoverables/(?P<collection>[^/?#]*)$")
                .expect("Unable to create regex for RECOVERABLES_COLLECTION");
    }
}

pub struct MakeService<T, C> where
    T: Api<C> + Clone + Send + 'static,
    C: Has<XSpanIdString>  + Send + Sync + 'static
{
    api_impl: T,
    marker: PhantomData<C>,
}

impl<T, C> MakeService<T, C> where
    T: Api<C> + Clone + Send + 'static,
    C: Has<XSpanIdString>  + Send + Sync + 'static
{
    pub fn new(api_impl: T) -> Self {
        MakeService {
            api_impl,
            marker: PhantomData
        }
    }
}

impl<T, C, Target> hyper::service::Service<Target> for MakeService<T, C> where
    T: Api<C> + Clone + Send + 'static,
    C: Has<XSpanIdString>  + Send + Sync + 'static
{
    type Response = Service<T, C>;
    type Error = crate::ServiceError;
    type Future = future::Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, target: Target) -> Self::Future {
        futures::future::ok(Service::new(
            self.api_impl.clone(),
        ))
    }
}

fn method_not_allowed() -> Result<Response<Body>, crate::ServiceError> {
    Ok(
        Response::builder().status(StatusCode::METHOD_NOT_ALLOWED)
            .body(Body::empty())
            .expect("Unable to create Method Not Allowed response")
    )
}

pub struct Service<T, C> where
    T: Api<C> + Clone + Send + 'static,
    C: Has<XSpanIdString>  + Send + Sync + 'static
{
    api_impl: T,
    marker: PhantomData<C>,
}

impl<T, C> Service<T, C> where
    T: Api<C> + Clone + Send + 'static,
    C: Has<XSpanIdString>  + Send + Sync + 'static
{
    pub fn new(api_impl: T) -> Self {
        Service {
            api_impl,
            marker: PhantomData
        }
    }
}

impl<T, C> Clone for Service<T, C> where
    T: Api<C> + Clone + Send + 'static,
    C: Has<XSpanIdString>  + Send + Sync + 'static
{
    fn clone(&self) -> Self {
        Service {
            api_impl: self.api_impl.clone(),
            marker: self.marker,
        }
    }
}

impl<T, C> hyper::service::Service<(Request<Body>, C)> for Service<T, C> where
    T: Api<C> + Clone + Send + Sync + 'static,
    C: Has<XSpanIdString>  + Send + Sync + 'static
{
    type Response = Response<Body>;
    type Error = crate::ServiceError;
    type Future = ServiceFuture;

    fn poll_ready(&mut self, cx: &mut Context) -> Poll<Result<(), Self::Error>> {
        self.api_impl.poll_ready(cx)
    }

    fn call(&mut self, req: (Request<Body>, C)) -> Self::Future { async fn run<T, C>(mut api_impl: T, req: (Request<Body>, C)) -> Result<Response<Body>, crate::ServiceError> where
        T: Api<C> + Clone + Send + 'static,
        C: Has<XSpanIdString>  + Send + Sync + 'static
    {
        let (request, context) = req;
        let (parts, body) = request.into_parts();
        let (method, uri, headers) = (parts.method, parts.uri, parts.headers);
        let path = paths::GLOBAL_REGEX_SET.matches(uri.path());

        match method {

            // CreateCollection - POST /collections
            hyper::Method::POST if path.matched(paths::ID_COLLECTIONS) => {
                // Body parameters (note that non-required body parameters will ignore garbage
                // values, rather than causing a 400 response). Produce warning header and logs for
                // any unused fields.
                let result = body.into_raw().await;
                match result {
                            Ok(body) => {
                                let mut unused_elements = Vec::new();
                                let param_create_collection_request: Option<models::CreateCollectionRequest> = if !body.is_empty() {
                                    let deserializer = &mut serde_json::Deserializer::from_slice(&body);
                                    match serde_ignored::deserialize(deserializer, |path| {
                                            warn!("Ignoring unknown field in body: {}", path);
                                            unused_elements.push(path.to_string());
                                    }) {
                                        Ok(param_create_collection_request) => param_create_collection_request,
                                        Err(e) => return Ok(Response::builder()
                                                        .status(StatusCode::BAD_REQUEST)
                                                        .body(Body::from(format!("Couldn't parse body parameter CreateCollectionRequest - doesn't match schema: {}", e)))
                                                        .expect("Unable to create Bad Request response for invalid body parameter CreateCollectionRequest due to schema")),
                                    }
                                } else {
                                    None
                                };
                                let param_create_collection_request = match param_create_collection_request {
                                    Some(param_create_collection_request) => param_create_collection_request,
                                    None => return Ok(Response::builder()
                                                        .status(StatusCode::BAD_REQUEST)
                                                        .body(Body::from("Missing required body parameter CreateCollectionRequest"))
                                                        .expect("Unable to create Bad Request response for missing body parameter CreateCollectionRequest")),
                                };

                                let result = api_impl.create_collection(
                                            param_create_collection_request,
                                        &context
                                    ).await;
                                let mut response = Response::new(Body::empty());
                                response.headers_mut().insert(
                                            HeaderName::from_static("x-span-id"),
                                            HeaderValue::from_str((&context as &dyn Has<XSpanIdString>).get().0.clone().as_str())
                                                .expect("Unable to create X-Span-ID header value"));

                                        if !unused_elements.is_empty() {
                                            response.headers_mut().insert(
                                                HeaderName::from_static("warning"),
                                                HeaderValue::from_str(format!("Ignoring unknown fields in body: {:?}", unused_elements).as_str())
                                                    .expect("Unable to create Warning header value"));
                                        }

                                        match result {
                                            Ok(rsp) => match rsp {
                                                CreateCollectionResponse::SuccessfulOperation
                                                    (body)
                                                => {
                                                    *response.status_mut() = StatusCode::from_u16(201).expect("Unable to turn 201 into a StatusCode");
                                                    response.headers_mut().insert(
                                                        CONTENT_TYPE,
                                                        HeaderValue::from_str("text/plain")
                                                            .expect("Unable to create Content-Type header for CREATE_COLLECTION_SUCCESSFUL_OPERATION"));
                                                    let body_content = body;
                                                    *response.body_mut() = Body::from(body_content);
                                                },
                                                CreateCollectionResponse::CreatingTheCollectionFailed
                                                => {
                                                    *response.status_mut() = StatusCode::from_u16(400).expect("Unable to turn 400 into a StatusCode");
                                                },
                                            },
                                            Err(_) => {
                                                // Application code returned an error. This should not happen, as the implementation should
                                                // return a valid response.
                                                *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                                                *response.body_mut() = Body::from("An internal error occurred");
                                            },
                                        }

                                        Ok(response)
                            },
                            Err(e) => Ok(Response::builder()
                                                .status(StatusCode::BAD_REQUEST)
                                                .body(Body::from(format!("Couldn't read body parameter CreateCollectionRequest: {}", e)))
                                                .expect("Unable to create Bad Request response due to unable to read body parameter CreateCollectionRequest")),
                        }
            },

            // GetCollections - GET /collections
            hyper::Method::GET if path.matched(paths::ID_COLLECTIONS) => {
                                let result = api_impl.get_collections(
                                        &context
                                    ).await;
                                let mut response = Response::new(Body::empty());
                                response.headers_mut().insert(
                                            HeaderName::from_static("x-span-id"),
                                            HeaderValue::from_str((&context as &dyn Has<XSpanIdString>).get().0.clone().as_str())
                                                .expect("Unable to create X-Span-ID header value"));

                                        match result {
                                            Ok(rsp) => match rsp {
                                                GetCollectionsResponse::SuccessfulOperation
                                                    (body)
                                                => {
                                                    *response.status_mut() = StatusCode::from_u16(200).expect("Unable to turn 200 into a StatusCode");
                                                    response.headers_mut().insert(
                                                        CONTENT_TYPE,
                                                        HeaderValue::from_str("application/json")
                                                            .expect("Unable to create Content-Type header for GET_COLLECTIONS_SUCCESSFUL_OPERATION"));
                                                    let body_content = serde_json::to_string(&body).expect("impossible to fail to serialize");
                                                    *response.body_mut() = Body::from(body_content);
                                                },
                                            },
                                            Err(_) => {
                                                // Application code returned an error. This should not happen, as the implementation should
                                                // return a valid response.
                                                *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                                                *response.body_mut() = Body::from("An internal error occurred");
                                            },
                                        }

                                        Ok(response)
            },

            // GetItemById - GET /collections/{collection}/{documentId}
            hyper::Method::GET if path.matched(paths::ID_COLLECTIONS_COLLECTION_DOCUMENTID) => {
                // Path parameters
                let path: &str = uri.path();
                let path_params =
                    paths::REGEX_COLLECTIONS_COLLECTION_DOCUMENTID
                    .captures(path)
                    .unwrap_or_else(||
                        panic!("Path {} matched RE COLLECTIONS_COLLECTION_DOCUMENTID in set but failed match against \"{}\"", path, paths::REGEX_COLLECTIONS_COLLECTION_DOCUMENTID.as_str())
                    );

                let param_collection = match percent_encoding::percent_decode(path_params["collection"].as_bytes()).decode_utf8() {
                    Ok(param_collection) => match param_collection.parse::<String>() {
                        Ok(param_collection) => param_collection,
                        Err(e) => return Ok(Response::builder()
                                        .status(StatusCode::BAD_REQUEST)
                                        .body(Body::from(format!("Couldn't parse path parameter collection: {}", e)))
                                        .expect("Unable to create Bad Request response for invalid path parameter")),
                    },
                    Err(_) => return Ok(Response::builder()
                                        .status(StatusCode::BAD_REQUEST)
                                        .body(Body::from(format!("Couldn't percent-decode path parameter as UTF-8: {}", &path_params["collection"])))
                                        .expect("Unable to create Bad Request response for invalid percent decode"))
                };

                let param_document_id = match percent_encoding::percent_decode(path_params["documentId"].as_bytes()).decode_utf8() {
                    Ok(param_document_id) => match param_document_id.parse::<uuid::Uuid>() {
                        Ok(param_document_id) => param_document_id,
                        Err(e) => return Ok(Response::builder()
                                        .status(StatusCode::BAD_REQUEST)
                                        .body(Body::from(format!("Couldn't parse path parameter documentId: {}", e)))
                                        .expect("Unable to create Bad Request response for invalid path parameter")),
                    },
                    Err(_) => return Ok(Response::builder()
                                        .status(StatusCode::BAD_REQUEST)
                                        .body(Body::from(format!("Couldn't percent-decode path parameter as UTF-8: {}", &path_params["documentId"])))
                                        .expect("Unable to create Bad Request response for invalid percent decode"))
                };

                                let result = api_impl.get_item_by_id(
                                            param_collection,
                                            param_document_id,
                                        &context
                                    ).await;
                                let mut response = Response::new(Body::empty());
                                response.headers_mut().insert(
                                            HeaderName::from_static("x-span-id"),
                                            HeaderValue::from_str((&context as &dyn Has<XSpanIdString>).get().0.clone().as_str())
                                                .expect("Unable to create X-Span-ID header value"));

                                        match result {
                                            Ok(rsp) => match rsp {
                                                GetItemByIdResponse::SuccessfulOperation
                                                    (body)
                                                => {
                                                    *response.status_mut() = StatusCode::from_u16(200).expect("Unable to turn 200 into a StatusCode");
                                                    response.headers_mut().insert(
                                                        CONTENT_TYPE,
                                                        HeaderValue::from_str("application/json")
                                                            .expect("Unable to create Content-Type header for GET_ITEM_BY_ID_SUCCESSFUL_OPERATION"));
                                                    let body_content = serde_json::to_string(&body).expect("impossible to fail to serialize");
                                                    *response.body_mut() = Body::from(body_content);
                                                },
                                                GetItemByIdResponse::ItemNotFound
                                                => {
                                                    *response.status_mut() = StatusCode::from_u16(404).expect("Unable to turn 404 into a StatusCode");
                                                },
                                            },
                                            Err(_) => {
                                                // Application code returned an error. This should not happen, as the implementation should
                                                // return a valid response.
                                                *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                                                *response.body_mut() = Body::from("An internal error occurred");
                                            },
                                        }

                                        Ok(response)
            },

            // ListCollection - GET /collections/{collection}
            hyper::Method::GET if path.matched(paths::ID_COLLECTIONS_COLLECTION) => {
                // Path parameters
                let path: &str = uri.path();
                let path_params =
                    paths::REGEX_COLLECTIONS_COLLECTION
                    .captures(path)
                    .unwrap_or_else(||
                        panic!("Path {} matched RE COLLECTIONS_COLLECTION in set but failed match against \"{}\"", path, paths::REGEX_COLLECTIONS_COLLECTION.as_str())
                    );

                let param_collection = match percent_encoding::percent_decode(path_params["collection"].as_bytes()).decode_utf8() {
                    Ok(param_collection) => match param_collection.parse::<String>() {
                        Ok(param_collection) => param_collection,
                        Err(e) => return Ok(Response::builder()
                                        .status(StatusCode::BAD_REQUEST)
                                        .body(Body::from(format!("Couldn't parse path parameter collection: {}", e)))
                                        .expect("Unable to create Bad Request response for invalid path parameter")),
                    },
                    Err(_) => return Ok(Response::builder()
                                        .status(StatusCode::BAD_REQUEST)
                                        .body(Body::from(format!("Couldn't percent-decode path parameter as UTF-8: {}", &path_params["collection"])))
                                        .expect("Unable to create Bad Request response for invalid percent decode"))
                };

                // Query parameters (note that non-required or collection query parameters will ignore garbage values, rather than causing a 400 response)
                let query_params = form_urlencoded::parse(uri.query().unwrap_or_default().as_bytes()).collect::<Vec<_>>();
                let param_exact_title = query_params.iter().filter(|e| e.0 == "exactTitle").map(|e| e.1.clone())
                    .next();
                let param_exact_title = match param_exact_title {
                    Some(param_exact_title) => {
                        let param_exact_title =
                            <String as std::str::FromStr>::from_str
                                (&param_exact_title);
                        match param_exact_title {
                            Ok(param_exact_title) => Some(param_exact_title),
                            Err(e) => return Ok(Response::builder()
                                .status(StatusCode::BAD_REQUEST)
                                .body(Body::from(format!("Couldn't parse query parameter exactTitle - doesn't match schema: {}", e)))
                                .expect("Unable to create Bad Request response for invalid query parameter exactTitle")),
                        }
                    },
                    None => None,
                };
                let param_extra_fields = query_params.iter().filter(|e| e.0 == "extraFields").map(|e| e.1.clone())
                    .next();
                let param_extra_fields = match param_extra_fields {
                    Some(param_extra_fields) => {
                        let param_extra_fields =
                            <String as std::str::FromStr>::from_str
                                (&param_extra_fields);
                        match param_extra_fields {
                            Ok(param_extra_fields) => Some(param_extra_fields),
                            Err(e) => return Ok(Response::builder()
                                .status(StatusCode::BAD_REQUEST)
                                .body(Body::from(format!("Couldn't parse query parameter extraFields - doesn't match schema: {}", e)))
                                .expect("Unable to create Bad Request response for invalid query parameter extraFields")),
                        }
                    },
                    None => None,
                };
                let param_limit = query_params.iter().filter(|e| e.0 == "limit").map(|e| e.1.clone())
                    .next();
                let param_limit = match param_limit {
                    Some(param_limit) => {
                        let param_limit =
                            <i32 as std::str::FromStr>::from_str
                                (&param_limit);
                        match param_limit {
                            Ok(param_limit) => Some(param_limit),
                            Err(e) => return Ok(Response::builder()
                                .status(StatusCode::BAD_REQUEST)
                                .body(Body::from(format!("Couldn't parse query parameter limit - doesn't match schema: {}", e)))
                                .expect("Unable to create Bad Request response for invalid query parameter limit")),
                        }
                    },
                    None => None,
                };
                let param_offset = query_params.iter().filter(|e| e.0 == "offset").map(|e| e.1.clone())
                    .next();
                let param_offset = match param_offset {
                    Some(param_offset) => {
                        let param_offset =
                            <i32 as std::str::FromStr>::from_str
                                (&param_offset);
                        match param_offset {
                            Ok(param_offset) => Some(param_offset),
                            Err(e) => return Ok(Response::builder()
                                .status(StatusCode::BAD_REQUEST)
                                .body(Body::from(format!("Couldn't parse query parameter offset - doesn't match schema: {}", e)))
                                .expect("Unable to create Bad Request response for invalid query parameter offset")),
                        }
                    },
                    None => None,
                };
                let param_pfilter = query_params.iter().filter(|e| e.0 == "pfilter").map(|e| e.1.clone())
                    .next();
                let param_pfilter = match param_pfilter {
                    Some(param_pfilter) => {
                        let param_pfilter =
                            <String as std::str::FromStr>::from_str
                                (&param_pfilter);
                        match param_pfilter {
                            Ok(param_pfilter) => Some(param_pfilter),
                            Err(e) => return Ok(Response::builder()
                                .status(StatusCode::BAD_REQUEST)
                                .body(Body::from(format!("Couldn't parse query parameter pfilter - doesn't match schema: {}", e)))
                                .expect("Unable to create Bad Request response for invalid query parameter pfilter")),
                        }
                    },
                    None => None,
                };
                let param_sort = query_params.iter().filter(|e| e.0 == "sort").map(|e| e.1.clone())
                    .next();
                let param_sort = match param_sort {
                    Some(param_sort) => {
                        let param_sort =
                            <String as std::str::FromStr>::from_str
                                (&param_sort);
                        match param_sort {
                            Ok(param_sort) => Some(param_sort),
                            Err(e) => return Ok(Response::builder()
                                .status(StatusCode::BAD_REQUEST)
                                .body(Body::from(format!("Couldn't parse query parameter sort - doesn't match schema: {}", e)))
                                .expect("Unable to create Bad Request response for invalid query parameter sort")),
                        }
                    },
                    None => None,
                };

                                let result = api_impl.list_collection(
                                            param_collection,
                                            param_exact_title,
                                            param_extra_fields,
                                            param_limit,
                                            param_offset,
                                            param_pfilter,
                                            param_sort,
                                        &context
                                    ).await;
                                let mut response = Response::new(Body::empty());
                                response.headers_mut().insert(
                                            HeaderName::from_static("x-span-id"),
                                            HeaderValue::from_str((&context as &dyn Has<XSpanIdString>).get().0.clone().as_str())
                                                .expect("Unable to create X-Span-ID header value"));

                                        match result {
                                            Ok(rsp) => match rsp {
                                                ListCollectionResponse::SuccessfulOperation
                                                    (body)
                                                => {
                                                    *response.status_mut() = StatusCode::from_u16(200).expect("Unable to turn 200 into a StatusCode");
                                                    response.headers_mut().insert(
                                                        CONTENT_TYPE,
                                                        HeaderValue::from_str("application/json")
                                                            .expect("Unable to create Content-Type header for LIST_COLLECTION_SUCCESSFUL_OPERATION"));
                                                    let body_content = serde_json::to_string(&body).expect("impossible to fail to serialize");
                                                    *response.body_mut() = Body::from(body_content);
                                                },
                                                ListCollectionResponse::CollectionNotFound
                                                => {
                                                    *response.status_mut() = StatusCode::from_u16(404).expect("Unable to turn 404 into a StatusCode");
                                                },
                                            },
                                            Err(_) => {
                                                // Application code returned an error. This should not happen, as the implementation should
                                                // return a valid response.
                                                *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                                                *response.body_mut() = Body::from("An internal error occurred");
                                            },
                                        }

                                        Ok(response)
            },

            // ListRecoverablesInCollection - GET /recoverables/{collection}
            hyper::Method::GET if path.matched(paths::ID_RECOVERABLES_COLLECTION) => {
                // Path parameters
                let path: &str = uri.path();
                let path_params =
                    paths::REGEX_RECOVERABLES_COLLECTION
                    .captures(path)
                    .unwrap_or_else(||
                        panic!("Path {} matched RE RECOVERABLES_COLLECTION in set but failed match against \"{}\"", path, paths::REGEX_RECOVERABLES_COLLECTION.as_str())
                    );

                let param_collection = match percent_encoding::percent_decode(path_params["collection"].as_bytes()).decode_utf8() {
                    Ok(param_collection) => match param_collection.parse::<String>() {
                        Ok(param_collection) => param_collection,
                        Err(e) => return Ok(Response::builder()
                                        .status(StatusCode::BAD_REQUEST)
                                        .body(Body::from(format!("Couldn't parse path parameter collection: {}", e)))
                                        .expect("Unable to create Bad Request response for invalid path parameter")),
                    },
                    Err(_) => return Ok(Response::builder()
                                        .status(StatusCode::BAD_REQUEST)
                                        .body(Body::from(format!("Couldn't percent-decode path parameter as UTF-8: {}", &path_params["collection"])))
                                        .expect("Unable to create Bad Request response for invalid percent decode"))
                };

                // Query parameters (note that non-required or collection query parameters will ignore garbage values, rather than causing a 400 response)
                let query_params = form_urlencoded::parse(uri.query().unwrap_or_default().as_bytes()).collect::<Vec<_>>();
                let param_exact_title = query_params.iter().filter(|e| e.0 == "exactTitle").map(|e| e.1.clone())
                    .next();
                let param_exact_title = match param_exact_title {
                    Some(param_exact_title) => {
                        let param_exact_title =
                            <String as std::str::FromStr>::from_str
                                (&param_exact_title);
                        match param_exact_title {
                            Ok(param_exact_title) => Some(param_exact_title),
                            Err(e) => return Ok(Response::builder()
                                .status(StatusCode::BAD_REQUEST)
                                .body(Body::from(format!("Couldn't parse query parameter exactTitle - doesn't match schema: {}", e)))
                                .expect("Unable to create Bad Request response for invalid query parameter exactTitle")),
                        }
                    },
                    None => None,
                };
                let param_extra_fields = query_params.iter().filter(|e| e.0 == "extraFields").map(|e| e.1.clone())
                    .next();
                let param_extra_fields = match param_extra_fields {
                    Some(param_extra_fields) => {
                        let param_extra_fields =
                            <String as std::str::FromStr>::from_str
                                (&param_extra_fields);
                        match param_extra_fields {
                            Ok(param_extra_fields) => Some(param_extra_fields),
                            Err(e) => return Ok(Response::builder()
                                .status(StatusCode::BAD_REQUEST)
                                .body(Body::from(format!("Couldn't parse query parameter extraFields - doesn't match schema: {}", e)))
                                .expect("Unable to create Bad Request response for invalid query parameter extraFields")),
                        }
                    },
                    None => None,
                };
                let param_limit = query_params.iter().filter(|e| e.0 == "limit").map(|e| e.1.clone())
                    .next();
                let param_limit = match param_limit {
                    Some(param_limit) => {
                        let param_limit =
                            <i32 as std::str::FromStr>::from_str
                                (&param_limit);
                        match param_limit {
                            Ok(param_limit) => Some(param_limit),
                            Err(e) => return Ok(Response::builder()
                                .status(StatusCode::BAD_REQUEST)
                                .body(Body::from(format!("Couldn't parse query parameter limit - doesn't match schema: {}", e)))
                                .expect("Unable to create Bad Request response for invalid query parameter limit")),
                        }
                    },
                    None => None,
                };
                let param_offset = query_params.iter().filter(|e| e.0 == "offset").map(|e| e.1.clone())
                    .next();
                let param_offset = match param_offset {
                    Some(param_offset) => {
                        let param_offset =
                            <i32 as std::str::FromStr>::from_str
                                (&param_offset);
                        match param_offset {
                            Ok(param_offset) => Some(param_offset),
                            Err(e) => return Ok(Response::builder()
                                .status(StatusCode::BAD_REQUEST)
                                .body(Body::from(format!("Couldn't parse query parameter offset - doesn't match schema: {}", e)))
                                .expect("Unable to create Bad Request response for invalid query parameter offset")),
                        }
                    },
                    None => None,
                };
                let param_pfilter = query_params.iter().filter(|e| e.0 == "pfilter").map(|e| e.1.clone())
                    .next();
                let param_pfilter = match param_pfilter {
                    Some(param_pfilter) => {
                        let param_pfilter =
                            <String as std::str::FromStr>::from_str
                                (&param_pfilter);
                        match param_pfilter {
                            Ok(param_pfilter) => Some(param_pfilter),
                            Err(e) => return Ok(Response::builder()
                                .status(StatusCode::BAD_REQUEST)
                                .body(Body::from(format!("Couldn't parse query parameter pfilter - doesn't match schema: {}", e)))
                                .expect("Unable to create Bad Request response for invalid query parameter pfilter")),
                        }
                    },
                    None => None,
                };
                let param_sort = query_params.iter().filter(|e| e.0 == "sort").map(|e| e.1.clone())
                    .next();
                let param_sort = match param_sort {
                    Some(param_sort) => {
                        let param_sort =
                            <String as std::str::FromStr>::from_str
                                (&param_sort);
                        match param_sort {
                            Ok(param_sort) => Some(param_sort),
                            Err(e) => return Ok(Response::builder()
                                .status(StatusCode::BAD_REQUEST)
                                .body(Body::from(format!("Couldn't parse query parameter sort - doesn't match schema: {}", e)))
                                .expect("Unable to create Bad Request response for invalid query parameter sort")),
                        }
                    },
                    None => None,
                };

                                let result = api_impl.list_recoverables_in_collection(
                                            param_collection,
                                            param_exact_title,
                                            param_extra_fields,
                                            param_limit,
                                            param_offset,
                                            param_pfilter,
                                            param_sort,
                                        &context
                                    ).await;
                                let mut response = Response::new(Body::empty());
                                response.headers_mut().insert(
                                            HeaderName::from_static("x-span-id"),
                                            HeaderValue::from_str((&context as &dyn Has<XSpanIdString>).get().0.clone().as_str())
                                                .expect("Unable to create X-Span-ID header value"));

                                        match result {
                                            Ok(rsp) => match rsp {
                                                ListRecoverablesInCollectionResponse::SuccessfulOperation
                                                    (body)
                                                => {
                                                    *response.status_mut() = StatusCode::from_u16(200).expect("Unable to turn 200 into a StatusCode");
                                                    response.headers_mut().insert(
                                                        CONTENT_TYPE,
                                                        HeaderValue::from_str("application/json")
                                                            .expect("Unable to create Content-Type header for LIST_RECOVERABLES_IN_COLLECTION_SUCCESSFUL_OPERATION"));
                                                    let body_content = serde_json::to_string(&body).expect("impossible to fail to serialize");
                                                    *response.body_mut() = Body::from(body_content);
                                                },
                                                ListRecoverablesInCollectionResponse::CollectionNotFound
                                                => {
                                                    *response.status_mut() = StatusCode::from_u16(404).expect("Unable to turn 404 into a StatusCode");
                                                },
                                            },
                                            Err(_) => {
                                                // Application code returned an error. This should not happen, as the implementation should
                                                // return a valid response.
                                                *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                                                *response.body_mut() = Body::from("An internal error occurred");
                                            },
                                        }

                                        Ok(response)
            },

            // SearchCollection - POST /collections/{collection}/searches
            hyper::Method::POST if path.matched(paths::ID_COLLECTIONS_COLLECTION_SEARCHES) => {
                // Path parameters
                let path: &str = uri.path();
                let path_params =
                    paths::REGEX_COLLECTIONS_COLLECTION_SEARCHES
                    .captures(path)
                    .unwrap_or_else(||
                        panic!("Path {} matched RE COLLECTIONS_COLLECTION_SEARCHES in set but failed match against \"{}\"", path, paths::REGEX_COLLECTIONS_COLLECTION_SEARCHES.as_str())
                    );

                let param_collection = match percent_encoding::percent_decode(path_params["collection"].as_bytes()).decode_utf8() {
                    Ok(param_collection) => match param_collection.parse::<String>() {
                        Ok(param_collection) => param_collection,
                        Err(e) => return Ok(Response::builder()
                                        .status(StatusCode::BAD_REQUEST)
                                        .body(Body::from(format!("Couldn't parse path parameter collection: {}", e)))
                                        .expect("Unable to create Bad Request response for invalid path parameter")),
                    },
                    Err(_) => return Ok(Response::builder()
                                        .status(StatusCode::BAD_REQUEST)
                                        .body(Body::from(format!("Couldn't percent-decode path parameter as UTF-8: {}", &path_params["collection"])))
                                        .expect("Unable to create Bad Request response for invalid percent decode"))
                };

                // Query parameters (note that non-required or collection query parameters will ignore garbage values, rather than causing a 400 response)
                let query_params = form_urlencoded::parse(uri.query().unwrap_or_default().as_bytes()).collect::<Vec<_>>();
                let param_extra_fields = query_params.iter().filter(|e| e.0 == "extraFields").map(|e| e.1.clone())
                    .next();
                let param_extra_fields = match param_extra_fields {
                    Some(param_extra_fields) => {
                        let param_extra_fields =
                            <String as std::str::FromStr>::from_str
                                (&param_extra_fields);
                        match param_extra_fields {
                            Ok(param_extra_fields) => Some(param_extra_fields),
                            Err(e) => return Ok(Response::builder()
                                .status(StatusCode::BAD_REQUEST)
                                .body(Body::from(format!("Couldn't parse query parameter extraFields - doesn't match schema: {}", e)))
                                .expect("Unable to create Bad Request response for invalid query parameter extraFields")),
                        }
                    },
                    None => None,
                };
                let param_limit = query_params.iter().filter(|e| e.0 == "limit").map(|e| e.1.clone())
                    .next();
                let param_limit = match param_limit {
                    Some(param_limit) => {
                        let param_limit =
                            <i32 as std::str::FromStr>::from_str
                                (&param_limit);
                        match param_limit {
                            Ok(param_limit) => Some(param_limit),
                            Err(e) => return Ok(Response::builder()
                                .status(StatusCode::BAD_REQUEST)
                                .body(Body::from(format!("Couldn't parse query parameter limit - doesn't match schema: {}", e)))
                                .expect("Unable to create Bad Request response for invalid query parameter limit")),
                        }
                    },
                    None => None,
                };
                let param_offset = query_params.iter().filter(|e| e.0 == "offset").map(|e| e.1.clone())
                    .next();
                let param_offset = match param_offset {
                    Some(param_offset) => {
                        let param_offset =
                            <i32 as std::str::FromStr>::from_str
                                (&param_offset);
                        match param_offset {
                            Ok(param_offset) => Some(param_offset),
                            Err(e) => return Ok(Response::builder()
                                .status(StatusCode::BAD_REQUEST)
                                .body(Body::from(format!("Couldn't parse query parameter offset - doesn't match schema: {}", e)))
                                .expect("Unable to create Bad Request response for invalid query parameter offset")),
                        }
                    },
                    None => None,
                };
                let param_sort = query_params.iter().filter(|e| e.0 == "sort").map(|e| e.1.clone())
                    .next();
                let param_sort = match param_sort {
                    Some(param_sort) => {
                        let param_sort =
                            <String as std::str::FromStr>::from_str
                                (&param_sort);
                        match param_sort {
                            Ok(param_sort) => Some(param_sort),
                            Err(e) => return Ok(Response::builder()
                                .status(StatusCode::BAD_REQUEST)
                                .body(Body::from(format!("Couldn't parse query parameter sort - doesn't match schema: {}", e)))
                                .expect("Unable to create Bad Request response for invalid query parameter sort")),
                        }
                    },
                    None => None,
                };

                // Body parameters (note that non-required body parameters will ignore garbage
                // values, rather than causing a 400 response). Produce warning header and logs for
                // any unused fields.
                let result = body.into_raw().await;
                match result {
                            Ok(body) => {
                                let mut unused_elements = Vec::new();
                                let param_search_collection_body: Option<models::SearchCollectionBody> = if !body.is_empty() {
                                    let deserializer = &mut serde_json::Deserializer::from_slice(&body);
                                    match serde_ignored::deserialize(deserializer, |path| {
                                            warn!("Ignoring unknown field in body: {}", path);
                                            unused_elements.push(path.to_string());
                                    }) {
                                        Ok(param_search_collection_body) => param_search_collection_body,
                                        Err(e) => return Ok(Response::builder()
                                                        .status(StatusCode::BAD_REQUEST)
                                                        .body(Body::from(format!("Couldn't parse body parameter SearchCollectionBody - doesn't match schema: {}", e)))
                                                        .expect("Unable to create Bad Request response for invalid body parameter SearchCollectionBody due to schema")),
                                    }
                                } else {
                                    None
                                };
                                let param_search_collection_body = match param_search_collection_body {
                                    Some(param_search_collection_body) => param_search_collection_body,
                                    None => return Ok(Response::builder()
                                                        .status(StatusCode::BAD_REQUEST)
                                                        .body(Body::from("Missing required body parameter SearchCollectionBody"))
                                                        .expect("Unable to create Bad Request response for missing body parameter SearchCollectionBody")),
                                };

                                let result = api_impl.search_collection(
                                            param_collection,
                                            param_search_collection_body,
                                            param_extra_fields,
                                            param_limit,
                                            param_offset,
                                            param_sort,
                                        &context
                                    ).await;
                                let mut response = Response::new(Body::empty());
                                response.headers_mut().insert(
                                            HeaderName::from_static("x-span-id"),
                                            HeaderValue::from_str((&context as &dyn Has<XSpanIdString>).get().0.clone().as_str())
                                                .expect("Unable to create X-Span-ID header value"));

                                        if !unused_elements.is_empty() {
                                            response.headers_mut().insert(
                                                HeaderName::from_static("warning"),
                                                HeaderValue::from_str(format!("Ignoring unknown fields in body: {:?}", unused_elements).as_str())
                                                    .expect("Unable to create Warning header value"));
                                        }

                                        match result {
                                            Ok(rsp) => match rsp {
                                                SearchCollectionResponse::SuccessfulOperation
                                                    (body)
                                                => {
                                                    *response.status_mut() = StatusCode::from_u16(200).expect("Unable to turn 200 into a StatusCode");
                                                    response.headers_mut().insert(
                                                        CONTENT_TYPE,
                                                        HeaderValue::from_str("application/json")
                                                            .expect("Unable to create Content-Type header for SEARCH_COLLECTION_SUCCESSFUL_OPERATION"));
                                                    let body_content = serde_json::to_string(&body).expect("impossible to fail to serialize");
                                                    *response.body_mut() = Body::from(body_content);
                                                },
                                                SearchCollectionResponse::CollectionNotFound
                                                => {
                                                    *response.status_mut() = StatusCode::from_u16(404).expect("Unable to turn 404 into a StatusCode");
                                                },
                                            },
                                            Err(_) => {
                                                // Application code returned an error. This should not happen, as the implementation should
                                                // return a valid response.
                                                *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                                                *response.body_mut() = Body::from("An internal error occurred");
                                            },
                                        }

                                        Ok(response)
                            },
                            Err(e) => Ok(Response::builder()
                                                .status(StatusCode::BAD_REQUEST)
                                                .body(Body::from(format!("Couldn't read body parameter SearchCollectionBody: {}", e)))
                                                .expect("Unable to create Bad Request response due to unable to read body parameter SearchCollectionBody")),
                        }
            },

            // StoreIntoCollection - POST /collections/{collection}
            hyper::Method::POST if path.matched(paths::ID_COLLECTIONS_COLLECTION) => {
                // Path parameters
                let path: &str = uri.path();
                let path_params =
                    paths::REGEX_COLLECTIONS_COLLECTION
                    .captures(path)
                    .unwrap_or_else(||
                        panic!("Path {} matched RE COLLECTIONS_COLLECTION in set but failed match against \"{}\"", path, paths::REGEX_COLLECTIONS_COLLECTION.as_str())
                    );

                let param_collection = match percent_encoding::percent_decode(path_params["collection"].as_bytes()).decode_utf8() {
                    Ok(param_collection) => match param_collection.parse::<String>() {
                        Ok(param_collection) => param_collection,
                        Err(e) => return Ok(Response::builder()
                                        .status(StatusCode::BAD_REQUEST)
                                        .body(Body::from(format!("Couldn't parse path parameter collection: {}", e)))
                                        .expect("Unable to create Bad Request response for invalid path parameter")),
                    },
                    Err(_) => return Ok(Response::builder()
                                        .status(StatusCode::BAD_REQUEST)
                                        .body(Body::from(format!("Couldn't percent-decode path parameter as UTF-8: {}", &path_params["collection"])))
                                        .expect("Unable to create Bad Request response for invalid percent decode"))
                };

                // Body parameters (note that non-required body parameters will ignore garbage
                // values, rather than causing a 400 response). Produce warning header and logs for
                // any unused fields.
                let result = body.into_raw().await;
                match result {
                            Ok(body) => {
                                let mut unused_elements = Vec::new();
                                let param_collection_item: Option<models::CollectionItem> = if !body.is_empty() {
                                    let deserializer = &mut serde_json::Deserializer::from_slice(&body);
                                    match serde_ignored::deserialize(deserializer, |path| {
                                            warn!("Ignoring unknown field in body: {}", path);
                                            unused_elements.push(path.to_string());
                                    }) {
                                        Ok(param_collection_item) => param_collection_item,
                                        Err(e) => return Ok(Response::builder()
                                                        .status(StatusCode::BAD_REQUEST)
                                                        .body(Body::from(format!("Couldn't parse body parameter CollectionItem - doesn't match schema: {}", e)))
                                                        .expect("Unable to create Bad Request response for invalid body parameter CollectionItem due to schema")),
                                    }
                                } else {
                                    None
                                };
                                let param_collection_item = match param_collection_item {
                                    Some(param_collection_item) => param_collection_item,
                                    None => return Ok(Response::builder()
                                                        .status(StatusCode::BAD_REQUEST)
                                                        .body(Body::from("Missing required body parameter CollectionItem"))
                                                        .expect("Unable to create Bad Request response for missing body parameter CollectionItem")),
                                };

                                let result = api_impl.store_into_collection(
                                            param_collection,
                                            param_collection_item,
                                        &context
                                    ).await;
                                let mut response = Response::new(Body::empty());
                                response.headers_mut().insert(
                                            HeaderName::from_static("x-span-id"),
                                            HeaderValue::from_str((&context as &dyn Has<XSpanIdString>).get().0.clone().as_str())
                                                .expect("Unable to create X-Span-ID header value"));

                                        if !unused_elements.is_empty() {
                                            response.headers_mut().insert(
                                                HeaderName::from_static("warning"),
                                                HeaderValue::from_str(format!("Ignoring unknown fields in body: {:?}", unused_elements).as_str())
                                                    .expect("Unable to create Warning header value"));
                                        }

                                        match result {
                                            Ok(rsp) => match rsp {
                                                StoreIntoCollectionResponse::SuccessfulOperation
                                                    (body)
                                                => {
                                                    *response.status_mut() = StatusCode::from_u16(201).expect("Unable to turn 201 into a StatusCode");
                                                    response.headers_mut().insert(
                                                        CONTENT_TYPE,
                                                        HeaderValue::from_str("text/plain")
                                                            .expect("Unable to create Content-Type header for STORE_INTO_COLLECTION_SUCCESSFUL_OPERATION"));
                                                    let body_content = body;
                                                    *response.body_mut() = Body::from(body_content);
                                                },
                                                StoreIntoCollectionResponse::CreatingTheCollectionFailed
                                                => {
                                                    *response.status_mut() = StatusCode::from_u16(400).expect("Unable to turn 400 into a StatusCode");
                                                },
                                            },
                                            Err(_) => {
                                                // Application code returned an error. This should not happen, as the implementation should
                                                // return a valid response.
                                                *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                                                *response.body_mut() = Body::from("An internal error occurred");
                                            },
                                        }

                                        Ok(response)
                            },
                            Err(e) => Ok(Response::builder()
                                                .status(StatusCode::BAD_REQUEST)
                                                .body(Body::from(format!("Couldn't read body parameter CollectionItem: {}", e)))
                                                .expect("Unable to create Bad Request response due to unable to read body parameter CollectionItem")),
                        }
            },

            // UpdateItemById - PUT /collections/{collection}
            hyper::Method::PUT if path.matched(paths::ID_COLLECTIONS_COLLECTION) => {
                // Path parameters
                let path: &str = uri.path();
                let path_params =
                    paths::REGEX_COLLECTIONS_COLLECTION
                    .captures(path)
                    .unwrap_or_else(||
                        panic!("Path {} matched RE COLLECTIONS_COLLECTION in set but failed match against \"{}\"", path, paths::REGEX_COLLECTIONS_COLLECTION.as_str())
                    );

                let param_collection = match percent_encoding::percent_decode(path_params["collection"].as_bytes()).decode_utf8() {
                    Ok(param_collection) => match param_collection.parse::<String>() {
                        Ok(param_collection) => param_collection,
                        Err(e) => return Ok(Response::builder()
                                        .status(StatusCode::BAD_REQUEST)
                                        .body(Body::from(format!("Couldn't parse path parameter collection: {}", e)))
                                        .expect("Unable to create Bad Request response for invalid path parameter")),
                    },
                    Err(_) => return Ok(Response::builder()
                                        .status(StatusCode::BAD_REQUEST)
                                        .body(Body::from(format!("Couldn't percent-decode path parameter as UTF-8: {}", &path_params["collection"])))
                                        .expect("Unable to create Bad Request response for invalid percent decode"))
                };

                // Body parameters (note that non-required body parameters will ignore garbage
                // values, rather than causing a 400 response). Produce warning header and logs for
                // any unused fields.
                let result = body.into_raw().await;
                match result {
                            Ok(body) => {
                                let mut unused_elements = Vec::new();
                                let param_collection_item: Option<models::CollectionItem> = if !body.is_empty() {
                                    let deserializer = &mut serde_json::Deserializer::from_slice(&body);
                                    match serde_ignored::deserialize(deserializer, |path| {
                                            warn!("Ignoring unknown field in body: {}", path);
                                            unused_elements.push(path.to_string());
                                    }) {
                                        Ok(param_collection_item) => param_collection_item,
                                        Err(e) => return Ok(Response::builder()
                                                        .status(StatusCode::BAD_REQUEST)
                                                        .body(Body::from(format!("Couldn't parse body parameter CollectionItem - doesn't match schema: {}", e)))
                                                        .expect("Unable to create Bad Request response for invalid body parameter CollectionItem due to schema")),
                                    }
                                } else {
                                    None
                                };
                                let param_collection_item = match param_collection_item {
                                    Some(param_collection_item) => param_collection_item,
                                    None => return Ok(Response::builder()
                                                        .status(StatusCode::BAD_REQUEST)
                                                        .body(Body::from("Missing required body parameter CollectionItem"))
                                                        .expect("Unable to create Bad Request response for missing body parameter CollectionItem")),
                                };

                                let result = api_impl.update_item_by_id(
                                            param_collection,
                                            param_collection_item,
                                        &context
                                    ).await;
                                let mut response = Response::new(Body::empty());
                                response.headers_mut().insert(
                                            HeaderName::from_static("x-span-id"),
                                            HeaderValue::from_str((&context as &dyn Has<XSpanIdString>).get().0.clone().as_str())
                                                .expect("Unable to create X-Span-ID header value"));

                                        if !unused_elements.is_empty() {
                                            response.headers_mut().insert(
                                                HeaderName::from_static("warning"),
                                                HeaderValue::from_str(format!("Ignoring unknown fields in body: {:?}", unused_elements).as_str())
                                                    .expect("Unable to create Warning header value"));
                                        }

                                        match result {
                                            Ok(rsp) => match rsp {
                                                UpdateItemByIdResponse::SuccessfulOperation
                                                    (body)
                                                => {
                                                    *response.status_mut() = StatusCode::from_u16(201).expect("Unable to turn 201 into a StatusCode");
                                                    response.headers_mut().insert(
                                                        CONTENT_TYPE,
                                                        HeaderValue::from_str("text/plain")
                                                            .expect("Unable to create Content-Type header for UPDATE_ITEM_BY_ID_SUCCESSFUL_OPERATION"));
                                                    let body_content = body;
                                                    *response.body_mut() = Body::from(body_content);
                                                },
                                                UpdateItemByIdResponse::UpdatingFailed
                                                => {
                                                    *response.status_mut() = StatusCode::from_u16(400).expect("Unable to turn 400 into a StatusCode");
                                                },
                                            },
                                            Err(_) => {
                                                // Application code returned an error. This should not happen, as the implementation should
                                                // return a valid response.
                                                *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                                                *response.body_mut() = Body::from("An internal error occurred");
                                            },
                                        }

                                        Ok(response)
                            },
                            Err(e) => Ok(Response::builder()
                                                .status(StatusCode::BAD_REQUEST)
                                                .body(Body::from(format!("Couldn't read body parameter CollectionItem: {}", e)))
                                                .expect("Unable to create Bad Request response due to unable to read body parameter CollectionItem")),
                        }
            },

            // CreateEvent - POST /events
            hyper::Method::POST if path.matched(paths::ID_EVENTS) => {
                // Body parameters (note that non-required body parameters will ignore garbage
                // values, rather than causing a 400 response). Produce warning header and logs for
                // any unused fields.
                let result = body.into_raw().await;
                match result {
                            Ok(body) => {
                                let mut unused_elements = Vec::new();
                                let param_create_event_body: Option<models::CreateEventBody> = if !body.is_empty() {
                                    let deserializer = &mut serde_json::Deserializer::from_slice(&body);
                                    match serde_ignored::deserialize(deserializer, |path| {
                                            warn!("Ignoring unknown field in body: {}", path);
                                            unused_elements.push(path.to_string());
                                    }) {
                                        Ok(param_create_event_body) => param_create_event_body,
                                        Err(e) => return Ok(Response::builder()
                                                        .status(StatusCode::BAD_REQUEST)
                                                        .body(Body::from(format!("Couldn't parse body parameter CreateEventBody - doesn't match schema: {}", e)))
                                                        .expect("Unable to create Bad Request response for invalid body parameter CreateEventBody due to schema")),
                                    }
                                } else {
                                    None
                                };
                                let param_create_event_body = match param_create_event_body {
                                    Some(param_create_event_body) => param_create_event_body,
                                    None => return Ok(Response::builder()
                                                        .status(StatusCode::BAD_REQUEST)
                                                        .body(Body::from("Missing required body parameter CreateEventBody"))
                                                        .expect("Unable to create Bad Request response for missing body parameter CreateEventBody")),
                                };

                                let result = api_impl.create_event(
                                            param_create_event_body,
                                        &context
                                    ).await;
                                let mut response = Response::new(Body::empty());
                                response.headers_mut().insert(
                                            HeaderName::from_static("x-span-id"),
                                            HeaderValue::from_str((&context as &dyn Has<XSpanIdString>).get().0.clone().as_str())
                                                .expect("Unable to create X-Span-ID header value"));

                                        if !unused_elements.is_empty() {
                                            response.headers_mut().insert(
                                                HeaderName::from_static("warning"),
                                                HeaderValue::from_str(format!("Ignoring unknown fields in body: {:?}", unused_elements).as_str())
                                                    .expect("Unable to create Warning header value"));
                                        }

                                        match result {
                                            Ok(rsp) => match rsp {
                                                CreateEventResponse::SuccessfulOperation
                                                    (body)
                                                => {
                                                    *response.status_mut() = StatusCode::from_u16(201).expect("Unable to turn 201 into a StatusCode");
                                                    response.headers_mut().insert(
                                                        CONTENT_TYPE,
                                                        HeaderValue::from_str("text/plain")
                                                            .expect("Unable to create Content-Type header for CREATE_EVENT_SUCCESSFUL_OPERATION"));
                                                    let body_content = body;
                                                    *response.body_mut() = Body::from(body_content);
                                                },
                                                CreateEventResponse::CreatingTheCollectionFailed
                                                => {
                                                    *response.status_mut() = StatusCode::from_u16(400).expect("Unable to turn 400 into a StatusCode");
                                                },
                                            },
                                            Err(_) => {
                                                // Application code returned an error. This should not happen, as the implementation should
                                                // return a valid response.
                                                *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                                                *response.body_mut() = Body::from("An internal error occurred");
                                            },
                                        }

                                        Ok(response)
                            },
                            Err(e) => Ok(Response::builder()
                                                .status(StatusCode::BAD_REQUEST)
                                                .body(Body::from(format!("Couldn't read body parameter CreateEventBody: {}", e)))
                                                .expect("Unable to create Bad Request response due to unable to read body parameter CreateEventBody")),
                        }
            },

            // RebuildGrants - POST /maintenance/{collection}/rebuild-grants
            hyper::Method::POST if path.matched(paths::ID_MAINTENANCE_COLLECTION_REBUILD_GRANTS) => {
                // Path parameters
                let path: &str = uri.path();
                let path_params =
                    paths::REGEX_MAINTENANCE_COLLECTION_REBUILD_GRANTS
                    .captures(path)
                    .unwrap_or_else(||
                        panic!("Path {} matched RE MAINTENANCE_COLLECTION_REBUILD_GRANTS in set but failed match against \"{}\"", path, paths::REGEX_MAINTENANCE_COLLECTION_REBUILD_GRANTS.as_str())
                    );

                let param_collection = match percent_encoding::percent_decode(path_params["collection"].as_bytes()).decode_utf8() {
                    Ok(param_collection) => match param_collection.parse::<String>() {
                        Ok(param_collection) => param_collection,
                        Err(e) => return Ok(Response::builder()
                                        .status(StatusCode::BAD_REQUEST)
                                        .body(Body::from(format!("Couldn't parse path parameter collection: {}", e)))
                                        .expect("Unable to create Bad Request response for invalid path parameter")),
                    },
                    Err(_) => return Ok(Response::builder()
                                        .status(StatusCode::BAD_REQUEST)
                                        .body(Body::from(format!("Couldn't percent-decode path parameter as UTF-8: {}", &path_params["collection"])))
                                        .expect("Unable to create Bad Request response for invalid percent decode"))
                };

                                let result = api_impl.rebuild_grants(
                                            param_collection,
                                        &context
                                    ).await;
                                let mut response = Response::new(Body::empty());
                                response.headers_mut().insert(
                                            HeaderName::from_static("x-span-id"),
                                            HeaderValue::from_str((&context as &dyn Has<XSpanIdString>).get().0.clone().as_str())
                                                .expect("Unable to create X-Span-ID header value"));

                                        match result {
                                            Ok(rsp) => match rsp {
                                                RebuildGrantsResponse::Success
                                                    (body)
                                                => {
                                                    *response.status_mut() = StatusCode::from_u16(201).expect("Unable to turn 201 into a StatusCode");
                                                    response.headers_mut().insert(
                                                        CONTENT_TYPE,
                                                        HeaderValue::from_str("text/plain")
                                                            .expect("Unable to create Content-Type header for REBUILD_GRANTS_SUCCESS"));
                                                    let body_content = body;
                                                    *response.body_mut() = Body::from(body_content);
                                                },
                                            },
                                            Err(_) => {
                                                // Application code returned an error. This should not happen, as the implementation should
                                                // return a valid response.
                                                *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                                                *response.body_mut() = Body::from("An internal error occurred");
                                            },
                                        }

                                        Ok(response)
            },

            _ if path.matched(paths::ID_COLLECTIONS) => method_not_allowed(),
            _ if path.matched(paths::ID_COLLECTIONS_COLLECTION) => method_not_allowed(),
            _ if path.matched(paths::ID_COLLECTIONS_COLLECTION_SEARCHES) => method_not_allowed(),
            _ if path.matched(paths::ID_COLLECTIONS_COLLECTION_DOCUMENTID) => method_not_allowed(),
            _ if path.matched(paths::ID_EVENTS) => method_not_allowed(),
            _ if path.matched(paths::ID_MAINTENANCE_COLLECTION_REBUILD_GRANTS) => method_not_allowed(),
            _ if path.matched(paths::ID_RECOVERABLES_COLLECTION) => method_not_allowed(),
            _ => Ok(Response::builder().status(StatusCode::NOT_FOUND)
                    .body(Body::empty())
                    .expect("Unable to create Not Found response"))
        }
    } Box::pin(run(self.api_impl.clone(), req)) }
}

/// Request parser for `Api`.
pub struct ApiRequestParser;
impl<T> RequestParser<T> for ApiRequestParser {
    fn parse_operation_id(request: &Request<T>) -> Option<&'static str> {
        let path = paths::GLOBAL_REGEX_SET.matches(request.uri().path());
        match *request.method() {
            // CreateCollection - POST /collections
            hyper::Method::POST if path.matched(paths::ID_COLLECTIONS) => Some("CreateCollection"),
            // GetCollections - GET /collections
            hyper::Method::GET if path.matched(paths::ID_COLLECTIONS) => Some("GetCollections"),
            // GetItemById - GET /collections/{collection}/{documentId}
            hyper::Method::GET if path.matched(paths::ID_COLLECTIONS_COLLECTION_DOCUMENTID) => Some("GetItemById"),
            // ListCollection - GET /collections/{collection}
            hyper::Method::GET if path.matched(paths::ID_COLLECTIONS_COLLECTION) => Some("ListCollection"),
            // ListRecoverablesInCollection - GET /recoverables/{collection}
            hyper::Method::GET if path.matched(paths::ID_RECOVERABLES_COLLECTION) => Some("ListRecoverablesInCollection"),
            // SearchCollection - POST /collections/{collection}/searches
            hyper::Method::POST if path.matched(paths::ID_COLLECTIONS_COLLECTION_SEARCHES) => Some("SearchCollection"),
            // StoreIntoCollection - POST /collections/{collection}
            hyper::Method::POST if path.matched(paths::ID_COLLECTIONS_COLLECTION) => Some("StoreIntoCollection"),
            // UpdateItemById - PUT /collections/{collection}
            hyper::Method::PUT if path.matched(paths::ID_COLLECTIONS_COLLECTION) => Some("UpdateItemById"),
            // CreateEvent - POST /events
            hyper::Method::POST if path.matched(paths::ID_EVENTS) => Some("CreateEvent"),
            // RebuildGrants - POST /maintenance/{collection}/rebuild-grants
            hyper::Method::POST if path.matched(paths::ID_MAINTENANCE_COLLECTION_REBUILD_GRANTS) => Some("RebuildGrants"),
            _ => None,
        }
    }
}
