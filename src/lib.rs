use core::future::Future;
use core::hash::Hash;
use dashmap::{DashMap, DashSet};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::marker::{Send, Sync};
use std::pin::Pin;
#[derive(Clone, Debug)]
pub struct DashmapCache {
    inner: DashMap<Vec<u8>, Vec<u8>>,
    tags: DashMap<String, DashSet<Vec<u8>>>,
}

#[derive(Debug)]
pub enum CacheError {
    Decode(rmp_serde::decode::Error),
    Encode(rmp_serde::encode::Error),
}

impl From<rmp_serde::decode::Error> for CacheError {
    fn from(value: rmp_serde::decode::Error) -> Self {
        Self::Decode(value)
    }
}

impl From<rmp_serde::encode::Error> for CacheError {
    fn from(value: rmp_serde::encode::Error) -> Self {
        Self::Encode(value)
    }
}

impl<'a> DashmapCache {
    pub fn new() -> Self {
        let inner = DashMap::new();
        Self {
            inner,
            tags: DashMap::new(),
        }
    }

    fn insert(&self, tags: &Vec<String>, key: Vec<u8>, val: Vec<u8>) -> Option<Vec<u8>> {
        for tag in tags {
            if !self.tags.contains_key(tag) {
                let dash = DashSet::new();
                dash.insert(key.clone());
                self.tags.insert(tag.to_owned(), dash);
            } else {
                self.tags.alter(tag, |_k, ex_tags| {
                    ex_tags.insert(key.clone());
                    ex_tags
                })
            }
        }
        self.inner.insert(key, val)
    }

    /// Atomic operation to replace a cached entry by a new computation value
    pub fn refresh_cache<F, A, V>(
        &self,
        invalidate_keys: &Vec<String>,
        closure: F,
        arg: A,
    ) -> Result<V, CacheError>
    where
        F: Fn(&A) -> V,
        A: Hash + Sync + Send + Eq + Serialize,
        V: Send + Sync + Clone + Serialize + for<'b> Deserialize<'b>,
    {
        let arg_bytes = rmp_serde::to_vec(&arg)?;
        let val = closure(&arg);
        let val_bytes = rmp_serde::to_vec(&val)?;
        self.insert(invalidate_keys, arg_bytes, val_bytes);
        Ok(val)
    }

    /// Computes a signature for arg
    /// If already present in the cache, returns directly associated return value
    /// Otherwise, compute a new return value and fills the cache with it
    /// It is recommended to use a call enum and dispatch in the same closure for the same cache if the input types or values are susceptible to overlap.
    pub fn cached<F, A, V>(
        &self,
        invalidate_keys: &Vec<String>,
        closure: F,
        arg: A,
    ) -> Result<V, CacheError>
    where
        F: Fn(&A) -> V,
        A: Hash + Sync + Send + Eq + Serialize,
        V: Send + Sync + Clone + Serialize + for<'b> Deserialize<'b>,
    {
        let arg_bytes = rmp_serde::to_vec(&arg)?;

        match self.inner.get(&arg_bytes) {
            None => {
                let val = closure(&arg);
                let val_bytes = rmp_serde::to_vec(&val)?;
                self.insert(invalidate_keys, arg_bytes, val_bytes);
                Ok(val)
            }
            Some(val) => {
                let ret_val = rmp_serde::from_slice::<V>(&val)?;
                Ok(ret_val.to_owned())
            }
        }
    }

    /// Async version of cached()
    pub async fn async_cached<F, A, V>(
        &self,
        invalidate_keys: &Vec<String>,
        closure: F,
        arg: A,
    ) -> Result<V, CacheError>
    where
        F: Fn(&A) -> Pin<Box<dyn Future<Output = V>>>,
        A: Hash + Sync + Send + Eq + Serialize,
        V: Send + Sync + Clone + Serialize + for<'b> Deserialize<'b>,
    {
        let arg_bytes = rmp_serde::to_vec(&arg)?;

        match self.inner.get(&arg_bytes) {
            None => {
                let val = closure(&arg).await;
                let val_bytes = rmp_serde::to_vec(&val)?;
                self.insert(invalidate_keys, arg_bytes, val_bytes);
                Ok(val)
            }
            Some(val) => {
                let ret_val = rmp_serde::from_slice::<V>(&val)?;
                Ok(ret_val.to_owned())
            }
        }
    }

    /// Tokio version of cached()
    #[cfg(feature = "tokio")]
    pub async fn tokio_cached<F, A, V>(
        &self,
        invalidate_keys: &Vec<String>,
        closure: F,
        arg: A,
    ) -> Result<V, CacheError>
    where
        F: Fn(&A) -> tokio::task::JoinHandle<V>,
        A: Hash + Sync + Send + Eq + Serialize,
        V: Send + Sync + Clone + Serialize + for<'b> Deserialize<'b>,
    {
        let arg_bytes = rmp_serde::to_vec(&arg)?;

        match self.inner.get(&arg_bytes) {
            None => {
                let val = closure(&arg).await.unwrap();
                let val_bytes = rmp_serde::to_vec(&val)?;
                self.insert(invalidate_keys, arg_bytes, val_bytes);
                Ok(val)
            }
            Some(val) => {
                let ret_val = rmp_serde::from_slice::<V>(&val)?;
                Ok(ret_val.to_owned())
            }
        }
    }

    pub fn invalidate(&self, tag: &str) {
        let hashes = self.tags.get(tag);
        if hashes.is_some() {
            self.tags.remove(tag);
            for hsh in hashes.unwrap().clone() {
                self.inner.remove(&hsh);
            }
        }
    }
}
