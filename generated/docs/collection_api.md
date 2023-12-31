# collection_api

All URIs are relative to */api*

Method | HTTP request | Description
------------- | ------------- | -------------
**getItemById**](collection_api.md#getItemById) | **GET** /collections/{collection}/{documentId} | Get item
**listCollection**](collection_api.md#listCollection) | **GET** /collections/{collection} | List collection items
**listRecoverablesInCollection**](collection_api.md#listRecoverablesInCollection) | **GET** /recoverables/{collection} | List recoverable items within the collection
**storeIntoCollection**](collection_api.md#storeIntoCollection) | **POST** /collections/{collection} | Create new item
**updateItemById**](collection_api.md#updateItemById) | **PUT** /collections/{collection} | Replace item


# **getItemById**
> models::CollectionItemDetails getItemById(collection, document_id)
Get item

Get item data

### Required Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
  **collection** | **String**| Path name of the collection | 
  **document_id** | [****](.md)| Document id as path component | 

### Return type

[**models::CollectionItemDetails**](CollectionItemDetails.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **listCollection**
> models::CollectionItemsList listCollection(collection, optional)
List collection items

Get a list of items within the collection

### Required Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
  **collection** | **String**| Path name of the collection | 
 **optional** | **map[string]interface{}** | optional parameters | nil if no parameters

### Optional Parameters
Optional parameters are passed through a map[string]interface{}.

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **collection** | **String**| Path name of the collection | 
 **exact_title** | **String**| Search for documents with this exact title (upper and lower case are respected)  | 
 **extra_fields** | **String**| A comma separated list of document fields that should be contained in the response  | 
 **limit** | **i32**| Number of items in the response. Defaults to `50`. | 
 **offset** | **i32**| Number of skipped items in the response. Defaults to `0`. | 
 **pfilter** | **String**| Filter some columns  | 
 **sort** | **String**| A comma separated list of document fields that should be used to sort the collection.  * Append a `+` to sort text ascending, * append a `-` to sort text descending. * Append a `f` to sort the native field value ascending (forward), * append a `b` to sort the native field value descending (backwards).  | 

### Return type

[**models::CollectionItemsList**](CollectionItemsList.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **listRecoverablesInCollection**
> models::CollectionItemsList listRecoverablesInCollection(collection, optional)
List recoverable items within the collection

Get a list of recoverable items within the collection. Requires activation of the two-staged-deletion.  ### Required permissions  * `C_COLLECTIONNAME_READER` and `C_COLLECTIONNAME_REMOVER` to recover documents from the first stage. * `C_COLLECTIONNAME_ADMIN` to recover documents from the second stage. 

### Required Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
  **collection** | **String**| Path name of the collection | 
 **optional** | **map[string]interface{}** | optional parameters | nil if no parameters

### Optional Parameters
Optional parameters are passed through a map[string]interface{}.

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **collection** | **String**| Path name of the collection | 
 **exact_title** | **String**| Search for documents with this exact title (upper and lower case are respected)  | 
 **extra_fields** | **String**| A comma separated list of document fields that should be contained in the response  | 
 **limit** | **i32**| Number of items in the response. Defaults to `50`. | 
 **offset** | **i32**| Number of skipped items in the response. Defaults to `0`. | 
 **pfilter** | **String**| Filter some columns  | 
 **sort** | **String**| A comma separated list of document fields that should be used to sort the collection.  * Append a `+` to sort text ascending, * append a `-` to sort text descending. * Append a `f` to sort the native field value ascending (forward), * append a `b` to sort the native field value descending (backwards).  | 

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
> String updateItemById(collection, collection_item)
Replace item

Replace the item data

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

