# event_api

All URIs are relative to */api*

Method | HTTP request | Description
------------- | ------------- | -------------
**createEvent**](event_api.md#createEvent) | **POST** /events | Create event for document in collection


# **createEvent**
> String createEvent(create_event_body)
Create event for document in collection

Create an event for the given document in a given collection. The collection must not be locked.  ### Required permissions  To create an event, the user must have one of the following permission:  * `C_COLLECTIONNAME_READER` * `C_COLLECTIONNAME_ALLREADER` 

### Required Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
  **create_event_body** | [**CreateEventBody**](CreateEventBody.md)| Event data | 

### Return type

[**String**](string.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: text/plain

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

