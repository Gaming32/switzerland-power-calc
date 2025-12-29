use derive_more::Display;
use serde::de::{DeserializeOwned, Error};
use serde::{Deserialize, Deserializer};
use serde_json::{Number, Value};

#[derive(Debug, Display, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct TurboStreamed<T>(pub T);

// Based on https://github.com/jacob-ebey/turbo-stream/blob/c974fa4af885aeb145e4517e474b6b4079677685/src/unflatten.ts
impl<'de, T: DeserializeOwned> Deserialize<'de> for TurboStreamed<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let unparsed = Vec::<Value>::deserialize(deserializer)?;

        fn parse_index_num(x: &Value) -> Result<isize, Value> {
            let index = x.as_i64().ok_or_else(|| x.clone())?;
            Ok(index.try_into().map_err(|_| index)?)
        }

        fn parse_index_str(x: &str) -> Result<isize, Value> {
            Ok(x.trim_start_matches('_').parse().map_err(|_| x)?)
        }

        fn parse_value_from_index(unparsed: &Vec<Value>, index: isize) -> Result<Value, Value> {
            if index < 0 {
                return match index {
                    -4 => Ok(Value::Number(Number::from_f64(-0.0).unwrap())),
                    -5 | -7 => Ok(Value::Null), // Map undefined and null to null
                    unsupported => Err(unsupported.into()), // NaN, -inf, +inf, HOLE
                };
            }
            let parsed_value = match unparsed.get(index as usize).ok_or(index)? {
                Value::Array(values) => Value::Array(
                    values
                        .iter()
                        .map(|v| parse_value_from_index(unparsed, parse_index_num(v)?))
                        .collect::<Result<Vec<Value>, Value>>()?,
                ),
                Value::Object(values) => Value::Object(
                    values
                        .iter()
                        .map(|(k, v)| {
                            Ok((
                                match parse_value_from_index(unparsed, parse_index_str(k)?)? {
                                    Value::String(s) => s,
                                    invalid => return Err(invalid),
                                },
                                parse_value_from_index(unparsed, parse_index_num(v)?)?,
                            ))
                        })
                        .collect::<Result<serde_json::Map<String, Value>, Value>>()?,
                ),
                other => other.clone(),
            };
            Ok(parsed_value)
        }

        let value = parse_value_from_index(&unparsed, 0)
            .map_err(|i| Error::custom(format_args!("Invalid index {i}")))?;
        let parsed = serde_json::from_value(value)
            .map_err(|e| Error::custom(format_args!("Failed to parse via JSON: {e}")))?;
        Ok(TurboStreamed(parsed))
    }
}

// type StringRef = Rc<String>;
// type ArrayRef = Rc<RefCell<Vec<ValueRef>>>;
// type ObjectRef = Rc<RefCell<LinkedHashMap<StringRef, ValueRef>>>;
//
// enum Setter {
//     Set(Rc<Cell<ValueRef>>),
//     Push(ArrayRef),
// }
//
// impl Setter {
//     fn set(self, value: ValueRef) {
//         match self {
//             Setter::Set(target) => {
//                 target.replace(value);
//             }
//             Setter::Push(array) => {
//                 array.borrow_mut().push(value);
//             }
//         }
//     }
// }
//
// #[derive(Clone)]
// enum ValueRef {
//     Null,
//     Bool(bool),
//     Number(Number),
//     String(StringRef),
//     Array(ArrayRef),
//     Object(ObjectRef),
// }
//
// impl From<Value> for ValueRef {
//     fn from(value: Value) -> Self {
//         match value {
//             Value::Null => ValueRef::Null,
//             Value::Bool(b) => ValueRef::Bool(b),
//             Value::Number(n) => ValueRef::Number(n),
//             Value::String(s) => ValueRef::String(Rc::new(s)),
//             Value::Array(arr) => ValueRef::Array(Rc::new(RefCell::new(
//                 arr.into_iter().map(Into::into).collect(),
//             ))),
//             Value::Object(obj) => ValueRef::Object(Rc::new(RefCell::new(
//                 obj.into_iter().map(|(k, v)| (k.into(), v.into())).collect(),
//             ))),
//         }
//     }
// }
//
// impl Into<Value> for ValueRef {
//     fn into(self) -> Value {
//         match self {
//             ValueRef::Null => Value::Null,
//             ValueRef::Bool(b) => Value::Bool(b),
//             ValueRef::Number(n) => Value::Number(n),
//             ValueRef::String(s) => Value::String(Rc::unwrap_or_clone(s)),
//             ValueRef::Array(arr) => Value::Array(Rc::try_unwrap(arr).map_or_else(
//                 |arr| arr.borrow().iter().cloned().map(Into::into).collect(),
//                 |arr| arr.into_inner().into_iter().map(Into::into).collect(),
//             )),
//             ValueRef::Object(obj) => Value::Object(Rc::try_unwrap(obj).map_or_else(
//                 |obj| {
//                     obj.borrow()
//                         .iter()
//                         .map(|(k, v)| (Rc::unwrap_or_clone(k.clone()), v.clone().into()))
//                         .collect()
//                 },
//                 |obj| {
//                     obj.into_inner()
//                         .into_iter()
//                         .map(|(k, v)| (Rc::unwrap_or_clone(k), v.into()))
//                         .collect()
//                 },
//             )),
//         }
//     }
// }
