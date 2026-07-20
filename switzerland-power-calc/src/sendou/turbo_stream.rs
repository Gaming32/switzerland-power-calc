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
        let value = decode_turbo_stream(&unparsed)
            .map_err(|i| Error::custom(format_args!("Invalid index {i}")))?;
        let parsed = serde_json::from_value(value)
            .map_err(|e| Error::custom(format_args!("Failed to parse via JSON: {e}")))?;
        Ok(TurboStreamed(parsed))
    }
}

fn decode_turbo_stream(stream: &[Value]) -> Result<Value, Value> {
    fn parse_index_num(x: &Value) -> Result<isize, Value> {
        let index = x.as_i64().ok_or_else(|| x.clone())?;
        Ok(index.try_into().map_err(|_| index)?)
    }

    fn parse_index_str(x: &str) -> Result<isize, Value> {
        Ok(x.trim_start_matches('_').parse().map_err(|_| x)?)
    }

    fn parse_value_from_index(unparsed: &[Value], index: isize) -> Result<Value, Value> {
        if index < 0 {
            return match index {
                -4 => Ok(Value::Number(Number::from_f64(-0.0).unwrap())),
                -5 | -7 => Ok(Value::Null), // Map undefined and null to null
                unsupported => Err(unsupported.into()), // NaN, -inf, +inf, HOLE
            };
        }
        let parsed_value = match unparsed.get(index as usize).ok_or(index)? {
            Value::Array(values) if let Some(Value::String(ty)) = values.first() => {
                match ty.as_str() {
                    "M" => Value::Object(
                        values[1..]
                            .as_chunks()
                            .0
                            .iter()
                            .map(|[k, v]| {
                                Ok((
                                    parse_value_from_index(unparsed, parse_index_num(k)?)?
                                        .to_string(),
                                    parse_value_from_index(unparsed, parse_index_num(v)?)?,
                                ))
                            })
                            .collect::<Result<serde_json::Map<String, Value>, Value>>()?,
                    ),
                    unknown => return Err(unknown.into()),
                }
            }
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

    parse_value_from_index(stream, 0)
}

#[cfg(test)]
mod test {
    use super::decode_turbo_stream;
    use serde_json::json;

    #[test]
    fn test_simple_turbo_stream() {
        assert_eq!(decode_turbo_stream(&[json!("Hello")]), Ok(json!("Hello")),);
        assert_eq!(
            decode_turbo_stream(&[
                json!({"_1": 2, "_3": 4, "_5": 6}),
                json!("Hello"),
                json!("World"),
                json!("Banana"),
                json!(5),
                json!("Array"),
                json!([7, 8, 9, 4]),
                json!("Hi"),
                json!("Bye"),
                json!(10),
            ]),
            Ok(json!({
                "Hello": "World",
                "Banana": 5,
                "Array": ["Hi", "Bye", 10, 5]
            })),
        );
    }

    // Maps get decoded such that their keys are JSON values, as JSON cannot have arbitrary key types
    #[test]
    fn decode_maps() {
        assert_eq!(
            decode_turbo_stream(&[
                json!(["M", 1, 2, 3, 2, 4, 5]),
                json!("Key"),
                json!("Value"),
                json!("Key2"),
                json!(5),
                json!(10),
            ]),
            Ok(json!({
                "\"Key\"": "Value",
                "\"Key2\"": "Value",
                "5": 10,
            }))
        );
    }
}
