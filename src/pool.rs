use crate::log_record::LogRecord;
use once_cell::sync::Lazy;
use std::sync::Mutex;

/// A simple thread-safe object pool
pub(crate) struct Pool<T: Default> {
    items: Mutex<Vec<T>>,
    max_size: usize,
}

impl<T: Default> Pool<T> {
    pub fn new(max_size: usize) -> Self {
        Self {
            items: Mutex::new(Vec::with_capacity(max_size)),
            max_size,
        }
    }

    pub fn get(&self) -> T {
        if let Ok(mut items) = self.items.lock() {
            if let Some(item) = items.pop() {
                return item;
            }
        }
        T::default()
    }

    pub fn put(&self, item: T) {
        if let Ok(mut items) = self.items.lock() {
            if items.len() < self.max_size {
                items.push(item);
            }
        }
    }
}

/// Global pool for LogRecord to reduce allocations
pub(crate) static LOG_RECORD_POOL: Lazy<Pool<LogRecord>> = Lazy::new(|| Pool::new(1024));

/// Global pool for String buffers to reduce allocations
pub(crate) static STRING_POOL: Lazy<Pool<String>> = Lazy::new(|| Pool::new(1024));
