# maintenance_api

All URIs are relative to */api*

Method | HTTP request | Description
------------- | ------------- | -------------
**rebuildGrants**](maintenance_api.md#rebuildGrants) | **POST** /maintenance/{collection}/rebuild-grants | Rebuild grants for a collection


# **rebuildGrants**
> String rebuildGrants(collection)
Rebuild grants for a collection

Iterate over all documents and refresh grants.

### Required Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
  **collection** | **String**| Path name of the collection | 

### Return type

[**String**](string.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: text/plain

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

