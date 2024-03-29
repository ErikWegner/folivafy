#![allow(missing_docs, unused_variables, trivial_casts)]


#[allow(unused_imports)]
use futures::{future, Stream, stream};
#[allow(unused_imports)]
use openapi::{Api, ApiNoContext, Client, ContextWrapperExt, models,
                      CreateCollectionResponse,
                      GetCollectionsResponse,
                      GetItemByIdResponse,
                      ListCollectionResponse,
                      ListRecoverablesInCollectionResponse,
                      SearchCollectionResponse,
                      StoreIntoCollectionResponse,
                      UpdateItemByIdResponse,
                      CreateEventResponse,
                      RebuildGrantsResponse,
                     };
use clap::{App, Arg};

#[allow(unused_imports)]
use log::info;

// swagger::Has may be unused if there are no examples
#[allow(unused_imports)]
use swagger::{AuthData, ContextBuilder, EmptyContext, Has, Push, XSpanIdString};

type ClientContext = swagger::make_context_ty!(ContextBuilder, EmptyContext, Option<AuthData>, XSpanIdString);

// rt may be unused if there are no examples
#[allow(unused_mut)]
fn main() {
    env_logger::init();

    let matches = App::new("client")
        .arg(Arg::with_name("operation")
            .help("Sets the operation to run")
            .possible_values(&[
                "GetCollections",
                "GetItemById",
                "ListCollection",
                "ListRecoverablesInCollection",
                "RebuildGrants",
            ])
            .required(true)
            .index(1))
        .arg(Arg::with_name("https")
            .long("https")
            .help("Whether to use HTTPS or not"))
        .arg(Arg::with_name("host")
            .long("host")
            .takes_value(true)
            .default_value("localhost")
            .help("Hostname to contact"))
        .arg(Arg::with_name("port")
            .long("port")
            .takes_value(true)
            .default_value("8080")
            .help("Port to contact"))
        .get_matches();

    let is_https = matches.is_present("https");
    let base_url = format!("{}://{}:{}",
                           if is_https { "https" } else { "http" },
                           matches.value_of("host").unwrap(),
                           matches.value_of("port").unwrap());

    let context: ClientContext =
        swagger::make_context!(ContextBuilder, EmptyContext, None as Option<AuthData>, XSpanIdString::default());

    let mut client : Box<dyn ApiNoContext<ClientContext>> = if matches.is_present("https") {
        // Using Simple HTTPS
        let client = Box::new(Client::try_new_https(&base_url)
            .expect("Failed to create HTTPS client"));
        Box::new(client.with_context(context))
    } else {
        // Using HTTP
        let client = Box::new(Client::try_new_http(
            &base_url)
            .expect("Failed to create HTTP client"));
        Box::new(client.with_context(context))
    };

    let mut rt = tokio::runtime::Runtime::new().unwrap();

    match matches.value_of("operation") {
        /* Disabled because there's no example.
        Some("CreateCollection") => {
            let result = rt.block_on(client.create_collection(
                  ???
            ));
            info!("{:?} (X-Span-ID: {:?})", result, (client.context() as &dyn Has<XSpanIdString>).get().clone());
        },
        */
        Some("GetCollections") => {
            let result = rt.block_on(client.get_collections(
            ));
            info!("{:?} (X-Span-ID: {:?})", result, (client.context() as &dyn Has<XSpanIdString>).get().clone());
        },
        Some("GetItemById") => {
            let result = rt.block_on(client.get_item_by_id(
                  "collection_example".to_string(),
                  serde_json::from_str::<uuid::Uuid>(r#"38400000-8cf0-11bd-b23e-10b96e4ef00d"#).expect("Failed to parse JSON example")
            ));
            info!("{:?} (X-Span-ID: {:?})", result, (client.context() as &dyn Has<XSpanIdString>).get().clone());
        },
        Some("ListCollection") => {
            let result = rt.block_on(client.list_collection(
                  "collection_example".to_string(),
                  Some("Rectangle".to_string()),
                  Some("price,length".to_string()),
                  Some(25),
                  Some(0),
                  Some("f1='v12'".to_string()),
                  Some("price+,length-".to_string())
            ));
            info!("{:?} (X-Span-ID: {:?})", result, (client.context() as &dyn Has<XSpanIdString>).get().clone());
        },
        Some("ListRecoverablesInCollection") => {
            let result = rt.block_on(client.list_recoverables_in_collection(
                  "collection_example".to_string(),
                  Some("Rectangle".to_string()),
                  Some("price,length".to_string()),
                  Some(25),
                  Some(0),
                  Some("f1='v12'".to_string()),
                  Some("price+,length-".to_string())
            ));
            info!("{:?} (X-Span-ID: {:?})", result, (client.context() as &dyn Has<XSpanIdString>).get().clone());
        },
        /* Disabled because there's no example.
        Some("SearchCollection") => {
            let result = rt.block_on(client.search_collection(
                  "collection_example".to_string(),
                  ???,
                  Some("price,length".to_string()),
                  Some(25),
                  Some(0),
                  Some("price+,length-".to_string())
            ));
            info!("{:?} (X-Span-ID: {:?})", result, (client.context() as &dyn Has<XSpanIdString>).get().clone());
        },
        */
        /* Disabled because there's no example.
        Some("StoreIntoCollection") => {
            let result = rt.block_on(client.store_into_collection(
                  "collection_example".to_string(),
                  ???
            ));
            info!("{:?} (X-Span-ID: {:?})", result, (client.context() as &dyn Has<XSpanIdString>).get().clone());
        },
        */
        /* Disabled because there's no example.
        Some("UpdateItemById") => {
            let result = rt.block_on(client.update_item_by_id(
                  "collection_example".to_string(),
                  ???
            ));
            info!("{:?} (X-Span-ID: {:?})", result, (client.context() as &dyn Has<XSpanIdString>).get().clone());
        },
        */
        /* Disabled because there's no example.
        Some("CreateEvent") => {
            let result = rt.block_on(client.create_event(
                  ???
            ));
            info!("{:?} (X-Span-ID: {:?})", result, (client.context() as &dyn Has<XSpanIdString>).get().clone());
        },
        */
        Some("RebuildGrants") => {
            let result = rt.block_on(client.rebuild_grants(
                  "collection_example".to_string()
            ));
            info!("{:?} (X-Span-ID: {:?})", result, (client.context() as &dyn Has<XSpanIdString>).get().clone());
        },
        _ => {
            panic!("Invalid operation provided")
        }
    }
}
