# collection_api

All URIs are relative to *https://qwirl.de*

Method | HTTP request | Description
------------- | ------------- | -------------
**getItemById**](collection_api.md#getItemById) | **GET** /collections/{collection}/{documentId} | Get item
**listCollection**](collection_api.md#listCollection) | **GET** /collections/{collection} | List collection items
**storeIntoCollection**](collection_api.md#storeIntoCollection) | **POST** /collections/{collection} | Create new item
**updateItemById**](collection_api.md#updateItemById) | **PUT** /collections/{collection}/{documentId} | Replace item


# **getItemById**
> models::CollectionItem getItemById(collection, document_id)
Get item

Get item data

### Required Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
  **collection** | **String**| Path name of the collection | 
  **document_id** | [****](.md)| Document id as path component | 

### Return type

[**models::CollectionItem**](CollectionItem.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **listCollection**
> models::CollectionItemsList listCollection(collection)
List collection items

Get a list of items within the collection

### Required Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
  **collection** | **String**| Path name of the collection | 

### Return type

[**models::CollectionItemsList**](CollectionItemsList.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **storeIntoCollection**
> String storeIntoCollection(collection, collection_item)
Create new item

Create a new item in this collection

### Required Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
  **collection** | **String**| Path name of the collection | 
  **collection_item** | [**CollectionItem**](CollectionItem.md)| Item payload | 

### Return type

[**String**](string.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: text/plain

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **updateItemById**
> String updateItemById(collection, document_id, collection_item)
Replace item

Replace the item data

### Required Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
  **collection** | **String**| Path name of the collection | 
  **document_id** | [****](.md)| Document id as path component | 
  **collection_item** | [**CollectionItem**](CollectionItem.md)| Item payload | 

### Return type

[**String**](string.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: text/plain

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

