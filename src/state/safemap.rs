use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;

pub struct SafeMap<T, U>(pub Arc<Mutex<HashMap<T, U>>>);

impl<T, U> SafeMap<T, U> {
    pub fn new() -> Self {
        SafeMap(Arc::new(Mutex::new(HashMap::new())))
    }
}

impl<T, U> Clone for SafeMap<T, U> {
    fn clone(&self) -> Self {
        SafeMap(self.0.clone())
    }
}
