Simple example how to use the lib.

Dashmap cache is thread safe and can be shared among threads through Arc<T> in a similar way than Dashmap can.

```rust
mod lib;

use crate::lib::DashmapCache;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug)]
struct Repository {}

#[derive(Clone, Serialize, Deserialize, Hash, PartialEq, Eq, Debug)]
pub struct SomeStruct {
    pub some_field: String,
    pub some_id: u64,
}

#[derive(Clone, Serialize, Deserialize, Hash, PartialEq, Eq, Debug)]
pub struct SomeResult {
    pub some_result_field: String,
    pub some_result_int: u64,
}
impl Repository {
    pub fn c(&self, arg: &SomeStruct) -> SomeResult {
        // Emulates a database call
        println!("call C with {:?}", arg);
        SomeResult {
            some_result_field: arg.some_field.clone(),
            some_result_int: arg.some_id,
        }
    }
}

fn main() {
    let repo = Repository {};
    let dmc: DashmapCache = DashmapCache::new();
    let invalidation_keys = vec!["some-key".to_string(), "some-other".to_string()];
    let const_arg = SomeStruct {
        some_field: "test".to_owned(),
        some_id: 1,
    };
    // Fills cache with a value
    println!(
        "{}",
        dmc.cached(
            &invalidation_keys,
            |some_struct| repo.c(some_struct),
            &const_arg
        )
        .unwrap()
        .some_result_field
    );
    // Identical call gets memoized;
    println!(
        "{}",
        dmc.cached(
            &invalidation_keys,
            |some_struct| repo.c(some_struct),
            &const_arg
        )
        .unwrap()
        .some_result_field
    );
    // Until you invalidate one of the associated key
    dmc.invalidate("some-key");
    println!(
        "{}",
        dmc.cached(
            &invalidation_keys,
            |some_struct| repo.c(some_struct),
            &const_arg
        )
        .unwrap()
        .some_result_field
    );

    /*
    > cargo run

        call C with SomeStruct { some_field: "test", some_id: 1 }
        test
        test
        call C with SomeStruct { some_field: "test", some_id: 1 }
        test
    */
}
```