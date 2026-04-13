use serde::Serialize;

pub fn print<T: Serialize>(value: &T) {
    println!(
        "{}",
        serde_json::to_string(value).expect("serialization failed")
    );
}

/// Serialize value to JSON, then filter to only keep the specified keys.
/// Works for both objects and arrays of objects.
pub fn print_filtered<T: Serialize>(value: &T, fields: &[&str]) {
    let v = serde_json::to_value(value).expect("serialization failed");
    let filtered = filter_value(v, fields);
    println!(
        "{}",
        serde_json::to_string(&filtered).expect("serialization failed")
    );
}

fn filter_value(v: serde_json::Value, fields: &[&str]) -> serde_json::Value {
    match v {
        serde_json::Value::Object(map) => {
            let filtered: serde_json::Map<String, serde_json::Value> = map
                .into_iter()
                .filter(|(k, _)| fields.contains(&k.as_str()))
                .collect();
            serde_json::Value::Object(filtered)
        }
        serde_json::Value::Array(arr) => {
            let filtered: Vec<serde_json::Value> = arr
                .into_iter()
                .map(|item| filter_value(item, fields))
                .collect();
            serde_json::Value::Array(filtered)
        }
        other => other,
    }
}
