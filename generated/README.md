# Rust API for openapi

Collection handling for validated forms

## Overview

This client/server was generated by the [openapi-generator]
(https://openapi-generator.tech) project.  By using the
[OpenAPI-Spec](https://github.com/OAI/OpenAPI-Specification) from a remote
server, you can easily generate a server stub.

To see how to make this your own, look here:

[README]((https://openapi-generator.tech))

- API version: 2.3.0
- Build date: 2024-01-29T20:18:45.946028828Z[Etc/UTC]



This autogenerated project defines an API crate `openapi` which contains:
* An `Api` trait defining the API in Rust.
* Data types representing the underlying data model.
* A `Client` type which implements `Api` and issues HTTP requests for each operation.
* A router which accepts HTTP requests and invokes the appropriate `Api` method for each operation.

It also contains an example server and client which make use of `openapi`:

* The example server starts up a web server using the `openapi`
    router, and supplies a trivial implementation of `Api` which returns failure
    for every operation.
* The example client provides a CLI which lets you invoke
    any single operation on the `openapi` client by passing appropriate
    arguments on the command line.

You can use the example server and client as a basis for your own code.
See below for [more detail on implementing a server](#writing-a-server).

## Examples

Run examples with:

```
cargo run --example <example-name>
```

To pass in arguments to the examples, put them after `--`, for example:

```
cargo run --example client -- --help
```

### Running the example server
To run the server, follow these simple steps:

```
cargo run --example server
```

### Running the example client
To run a client, follow one of the following simple steps:

```
cargo run --example client GetCollections
cargo run --example client GetItemById
cargo run --example client ListCollection
cargo run --example client ListRecoverablesInCollection
cargo run --example client RebuildGrants
```

### HTTPS
The examples can be run in HTTPS mode by passing in the flag `--https`, for example:

```
cargo run --example server -- --https
```

This will use the keys/certificates from the examples directory. Note that the
server chain is signed with `CN=localhost`.

## Using the generated library

The generated library has a few optional features that can be activated through Cargo.

* `server`
    * This defaults to enabled and creates the basic skeleton of a server implementation based on hyper
    * To create the server stack you'll need to provide an implementation of the API trait to provide the server function.
* `client`
    * This defaults to enabled and creates the basic skeleton of a client implementation based on hyper
    * The constructed client implements the API trait by making remote API call.
* `conversions`
    * This defaults to disabled and creates extra derives on models to allow "transmogrification" between objects of structurally similar types.

See https://doc.rust-lang.org/cargo/reference/manifest.html#the-features-section for how to use features in your `Cargo.toml`.

## Documentation for API Endpoints

All URIs are relative to */api*

Method | HTTP request | Description
------------- | ------------- | -------------
[**createCollection**](docs/administration_api.md#createCollection) | **POST** /collections | Create a collection
[**getCollections**](docs/administration_api.md#getCollections) | **GET** /collections | List available collections
[**getItemById**](docs/collection_api.md#getItemById) | **GET** /collections/{collection}/{documentId} | Get item
[**listCollection**](docs/collection_api.md#listCollection) | **GET** /collections/{collection} | List collection items
[**listRecoverablesInCollection**](docs/collection_api.md#listRecoverablesInCollection) | **GET** /recoverables/{collection} | List recoverable items within the collection
[**searchCollection**](docs/collection_api.md#searchCollection) | **POST** /collections/{collection}/searches | List collection items
[**storeIntoCollection**](docs/collection_api.md#storeIntoCollection) | **POST** /collections/{collection} | Create new item
[**updateItemById**](docs/collection_api.md#updateItemById) | **PUT** /collections/{collection} | Replace item
[**createEvent**](docs/event_api.md#createEvent) | **POST** /events | Create event for document in collection
[**rebuildGrants**](docs/maintenance_api.md#rebuildGrants) | **POST** /maintenance/{collection}/rebuild-grants | Rebuild grants for a collection


## Documentation For Models

 - [CategoryId](docs/CategoryId.md)
 - [Collection](docs/Collection.md)
 - [CollectionItem](docs/CollectionItem.md)
 - [CollectionItemDetails](docs/CollectionItemDetails.md)
 - [CollectionItemEvent](docs/CollectionItemEvent.md)
 - [CollectionItemsList](docs/CollectionItemsList.md)
 - [CollectionName](docs/CollectionName.md)
 - [CollectionsList](docs/CollectionsList.md)
 - [CreateCollectionRequest](docs/CreateCollectionRequest.md)
 - [CreateEventBody](docs/CreateEventBody.md)
 - [DocumentId](docs/DocumentId.md)
 - [SearchCollectionBody](docs/SearchCollectionBody.md)
 - [SearchFilter](docs/SearchFilter.md)
 - [SearchFilterAndGroup](docs/SearchFilterAndGroup.md)
 - [SearchFilterFieldOp](docs/SearchFilterFieldOp.md)
 - [SearchFilterFieldOpValue](docs/SearchFilterFieldOpValue.md)
 - [SearchFilterFieldOpValueV](docs/SearchFilterFieldOpValueV.md)
 - [SearchFilterOrGroup](docs/SearchFilterOrGroup.md)
 - [ValueBoolean](docs/ValueBoolean.md)
 - [ValueNumber](docs/ValueNumber.md)
 - [ValueString](docs/ValueString.md)


## Documentation For Authorization
Endpoints do not require authorization.


## Author



