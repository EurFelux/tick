use serde::Serialize;

pub fn print<T: Serialize>(value: &T) {
    println!("{}", serde_json::to_string(value).expect("serialization failed"));
}
