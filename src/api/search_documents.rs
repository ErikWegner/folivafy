use serde::{Deserialize, Serialize};
use serde_json::Value;
use typed_builder::TypedBuilder;

use super::db::FieldFilter;

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub(crate) enum OperationWithValue {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    StartsWith,
    ContainsText,
    In,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, TypedBuilder)]
pub(crate) struct SearchFilterFieldOpValue {
    #[serde(rename = "f")]
    field: String,
    #[serde(rename = "o")]
    operation: OperationWithValue,
    #[serde(rename = "v")]
    value: Value,
}

impl SearchFilterFieldOpValue {
    pub(crate) fn field(&self) -> &str {
        self.field.as_ref()
    }

    pub(crate) fn operation(&self) -> OperationWithValue {
        self.operation
    }

    pub(crate) fn value(&self) -> &Value {
        &self.value
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Operation {
    Null,
    NotNull,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, TypedBuilder)]
pub(crate) struct SearchFilterFieldOp {
    #[serde(rename = "f")]
    field: String,
    #[serde(rename = "o")]
    operation: Operation,
}

impl SearchFilterFieldOp {
    pub(crate) fn field(&self) -> &str {
        self.field.as_ref()
    }

    pub(crate) fn operation(&self) -> Operation {
        self.operation
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub(crate) enum SearchGroup {
    #[serde(rename = "and")]
    AndGroup(Vec<SearchFilter>),
    #[serde(rename = "or")]
    OrGroup(Vec<SearchFilter>),
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub(crate) enum SearchFilter {
    FieldOpValue(SearchFilterFieldOpValue),
    FieldOp(SearchFilterFieldOp),
    Group(SearchGroup),
}

impl From<&FieldFilter> for SearchFilter {
    fn from(value: &FieldFilter) -> Self {
        match value {
            FieldFilter::ExactFieldMatch { field_name, value } => {
                SearchFilter::FieldOpValue(SearchFilterFieldOpValue {
                    field: field_name.clone(),
                    operation: OperationWithValue::Eq,
                    value: Value::String(value.clone()),
                })
            }
            FieldFilter::FieldStartsWith { field_name, value } => {
                SearchFilter::FieldOpValue(SearchFilterFieldOpValue {
                    field: field_name.clone(),
                    operation: OperationWithValue::StartsWith,
                    value: Value::String(value.clone()),
                })
            }
            FieldFilter::FieldContains { field_name, value } => {
                SearchFilter::FieldOpValue(SearchFilterFieldOpValue {
                    field: field_name.clone(),
                    operation: OperationWithValue::ContainsText,
                    value: Value::String(value.clone()),
                })
            }
            FieldFilter::FieldValueInMatch { field_name, values } => {
                SearchFilter::FieldOpValue(SearchFilterFieldOpValue {
                    field: field_name.clone(),
                    operation: OperationWithValue::In,
                    value: Value::Array(values.iter().cloned().map(Value::String).collect()),
                })
            }
            FieldFilter::FieldIsNull { field_name } => SearchFilter::FieldOp(SearchFilterFieldOp {
                field: field_name.clone(),
                operation: Operation::Null,
            }),
            FieldFilter::FieldIsNotNull { field_name } => {
                SearchFilter::FieldOp(SearchFilterFieldOp {
                    field: field_name.clone(),
                    operation: Operation::NotNull,
                })
            }
            FieldFilter::DateFieldLessThan { field_name, value } => {
                SearchFilter::FieldOpValue(SearchFilterFieldOpValue {
                    field: field_name.clone(),
                    operation: OperationWithValue::Lt,
                    value: Value::String(value.format("%Y-%m-%d").to_string()),
                })
            }
        }
    }
}

impl From<Vec<FieldFilter>> for SearchFilter {
    fn from(value: Vec<FieldFilter>) -> Self {
        SearchFilter::Group(SearchGroup::AndGroup(
            value.into_iter().map(|v| (&v).into()).collect(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

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
        assert_eq!(s, r#"{"f":"my_name","o":"ne","v":"my_value"}"#);
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
            SearchFilter::Group(SearchGroup::AndGroup(vec![
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
            SearchFilter::Group(SearchGroup::OrGroup(vec![
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
            r#"{"or":[{"f":"my_name","o":"eq","v":"my_value"},{"f":"other","o":"notnull"},{"and":[{"f":"my_name3","o":"eq","v":"my_value3"},{"f":"other4","o":"null"}]},{"or":[{"f":"my_name5","o":"eq","v":"my_value5"},{"f":"other6","o":"null"}]}]}"#
        );
    }

    #[test]
    fn it_can_deserialize_searchgroup() {
        // Arrange
        let s = r#"{"or":[{"f":"my_name","o":"eq","v":"my_value"},{"f":"other","o":"notnull"},{"and":[{"f":"my_name3","o":"eq","v":"my_value3"},{"f":"other4","o":"null"}]},{"or":[{"f":"my_name5","o":"eq","v":"my_value5"},{"f":"other6","o":"null"}]}]}"#;
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
                SearchFilter::Group(SearchGroup::AndGroup(vec![
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
                SearchFilter::Group(SearchGroup::OrGroup(vec![
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

    #[test]
    fn it_convers_in_clause() {
        // Arrange
        let i = vec![FieldFilter::FieldValueInMatch {
            field_name: "f4".to_string(),
            values: vec!["191".to_string(), "291".to_string()],
        }];

        // Act
        let r: SearchFilter = i.into();

        // Assert
        assert_eq!(
            r,
            SearchFilter::Group(SearchGroup::AndGroup(vec![SearchFilter::FieldOpValue(
                SearchFilterFieldOpValue {
                    field: "f4".to_string(),
                    operation: OperationWithValue::In,
                    value: serde_json::json!(vec!["191", "291"])
                }
            )]))
        )
    }
}
