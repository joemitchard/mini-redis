use std::collections::HashMap;
use std::time::Instant;

pub  struct Cache {
    store: HashMap<String, CacheItem>
}

impl Cache {
    pub fn new() -> Self {
        Self {
            store: HashMap::with_capacity(1024),
        }
    }
    
    pub fn set(&mut self, key: &str, value: &str, expires: &Option<usize>) {
        let item = CacheItem::new(value, expires);
        self.store.insert(key.to_string(), item);
    }
    
    pub fn get(&self, key: &str) -> Option<&String> {
        match self.store.get(key) {
            Some(item) if item.is_expired() => None,
            Some(item) => Some(&item.value),
            None => None
        }
    }
}

#[derive(Debug)]
pub struct CacheItem {
    value: String,
    created: Instant,
    expires: Option<usize>
}

impl CacheItem {
    pub fn new(value: &str, expires: &Option<usize>) -> Self {
        CacheItem {
            value: value.to_string(),
            created: Instant::now(),
            expires: *expires
        }
    }

    pub fn is_expired(&self) -> bool {
        match self.expires {
            Some(x) => {
                let expires = x as u128;
                let elapsed = self.created.elapsed().as_millis();
                
                elapsed > expires
            },
            None => false
        }
    }
}

