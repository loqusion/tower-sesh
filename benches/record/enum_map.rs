use std::collections::HashMap;

pub struct Record {
    data: HashMap<String, Value>,
}

impl Record {
    #[inline]
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    pub fn insert<T>(&mut self, k: &str, v: T) -> Option<T> {
        todo!()
    }
}

enum Value {
    Null,
    Bool(bool),
    String(String),
    Array(Vec<Value>),
}
