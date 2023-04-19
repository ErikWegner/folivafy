# administration_api

All URIs are relative to *https://qwirl.de*

Method | HTTP request | Description
------------- | ------------- | -------------
**createCollection**](administration_api.md#createCollection) | **POST** /collections | Create a collection
**getCollections**](administration_api.md#getCollections) | **GET** /collections | List available collections


# **createCollection**
> String createCollection(create_collection_request)
Create a collection

Create a new collection on this server

### Required Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
  **create_collection_request** | [**CreateCollectionRequest**](CreateCollectionRequest.md)| Information about the new collection | 

### Return type

[**String**](string.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: text/plain

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **getCollections**
> models::CollectionsList getCollections()
List available collections

List all available collections on this server

### Required Parameters
This endpoint does not need any parameter.

### Return type

[**models::CollectionsList**](CollectionsList.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

