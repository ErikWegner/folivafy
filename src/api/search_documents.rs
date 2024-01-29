use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
enum OperationWithValue {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct SearchFilterFieldOpValue {
    #[serde(rename = "f")]
    field: String,
    #[serde(rename = "o")]
    operation: OperationWithValue,
    #[serde(rename = "v")]
    value: Value,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Operation {
    Null,
    NotNull,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct SearchFilterFieldOp {
    #[serde(rename = "f")]
    field: String,
    #[serde(rename = "o")]
    operation: Operation,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
enum SearchGroup {
    #[serde(rename = "and")]
    AndGroup(Vec<SearchFilter>),
    #[serde(rename = "or")]
    OrGroup(Vec<SearchFilter>),
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
enum SearchFilter {
    FieldOpValue(SearchFilterFieldOpValue),
    FieldOp(SearchFilterFieldOp),
    SearchGroup(SearchGroup),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works_for_fieldop() {
        // Arrange
        let p = SearchFilterFieldOp {
            field: "my_name".to_string(),
            operation: Operation::NotNull,
        };

        // Act
        let s = serde_json::to_string(&p).unwrap();

        // Assert
        assert_eq!(s, r#"{"f":"my_name","o":"notnull"}"#);
    }

    #[test]
    fn it_works_for_fieldopvalue() {
        // Arrange
        let p = SearchFilterFieldOpValue {
            field: "my_name".to_string(),
            operation: OperationWithValue::Ne,
            value: Value::String("my_value".to_string()),
        };

        // Act
        let s = serde_json::to_string(&p).unwrap();

        // Assert
        assert_eq!(s, r#"{"f":"my_name","o":"Ne","v":"my_value"}"#);
    }

    #[test]
    fn it_works_for_searchgroup() {
        // Arrange
        let p = SearchGroup::OrGroup(vec![
            SearchFilter::FieldOpValue(SearchFilterFieldOpValue {
                field: "my_name".to_string(),
                operation: OperationWithValue::Eq,
                value: Value::String("my_value".to_string()),
            }),
            SearchFilter::FieldOp(SearchFilterFieldOp {
                field: "other".to_string(),
                operation: Operation::NotNull,
            }),
            SearchFilter::SearchGroup(SearchGroup::AndGroup(vec![
                SearchFilter::FieldOpValue(SearchFilterFieldOpValue {
                    field: "my_name3".to_string(),
                    operation: OperationWithValue::Eq,
                    value: Value::String("my_value3".to_string()),
                }),
                SearchFilter::FieldOp(SearchFilterFieldOp {
                    field: "other4".to_string(),
                    operation: Operation::Null,
                }),
            ])),
            SearchFilter::SearchGroup(SearchGroup::OrGroup(vec![
                SearchFilter::FieldOpValue(SearchFilterFieldOpValue {
                    field: "my_name5".to_string(),
                    operation: OperationWithValue::Eq,
                    value: Value::String("my_value5".to_string()),
                }),
                SearchFilter::FieldOp(SearchFilterFieldOp {
                    field: "other6".to_string(),
                    operation: Operation::Null,
                }),
            ])),
        ]);

        // Act
        let s = serde_json::to_string(&p).unwrap();

        // Assert
        assert_eq!(
            s,
            r#"{"or":[{"f":"my_name","o":"Eq","v":"my_value"},{"f":"other","o":"notnull"},{"and":[{"f":"my_name3","o":"Eq","v":"my_value3"},{"f":"other4","o":"null"}]},{"or":[{"f":"my_name5","o":"Eq","v":"my_value5"},{"f":"other6","o":"null"}]}]}"#
        );
    }

    #[test]
    fn it_can_deserialize_searchgroup() {
        // Arrange
        let s = r#"{"or":[{"f":"my_name","o":"Eq","v":"my_value"},{"f":"other","o":"notnull"},{"and":[{"f":"my_name3","o":"Eq","v":"my_value3"},{"f":"other4","o":"null"}]},{"or":[{"f":"my_name5","o":"Eq","v":"my_value5"},{"f":"other6","o":"null"}]}]}"#;
        // Act
        let p: SearchGroup = serde_json::from_str(s).unwrap();

        // Assert
        assert_eq!(
            p,
            SearchGroup::OrGroup(vec![
                SearchFilter::FieldOpValue(SearchFilterFieldOpValue {
                    field: "my_name".to_string(),
                    operation: OperationWithValue::Eq,
                    value: Value::String("my_value".to_string()),
                }),
                SearchFilter::FieldOp(SearchFilterFieldOp {
                    field: "other".to_string(),
                    operation: Operation::NotNull,
                }),
                SearchFilter::SearchGroup(SearchGroup::AndGroup(vec![
                    SearchFilter::FieldOpValue(SearchFilterFieldOpValue {
                        field: "my_name3".to_string(),
                        operation: OperationWithValue::Eq,
                        value: Value::String("my_value3".to_string()),
                    }),
                    SearchFilter::FieldOp(SearchFilterFieldOp {
                        field: "other4".to_string(),
                        operation: Operation::Null,
                    }),
                ])),
                SearchFilter::SearchGroup(SearchGroup::OrGroup(vec![
                    SearchFilter::FieldOpValue(SearchFilterFieldOpValue {
                        field: "my_name5".to_string(),
                        operation: OperationWithValue::Eq,
                        value: Value::String("my_value5".to_string()),
                    }),
                    SearchFilter::FieldOp(SearchFilterFieldOp {
                        field: "other6".to_string(),
                        operation: Operation::Null,
                    }),
                ])),
            ])
        );
    }
}
