[![GitHub](https://img.shields.io/badge/github-tlaferriere/cmp_by_derive-8da0cb?labelColor=555555&logo=github)](https://github.com/tlaferriere/cmp_by_derive)
[![Crates.io](https://img.shields.io/crates/v/cmp_by_derive)](https://crates.io/crates/cmp_by_derive)
[![docs.rs](https://img.shields.io/docsrs/cmp_by_derive)](https://docs.rs/cmp_by_derive)
[![Continuous integration](https://github.com/tlaferriere/cmp_by_derive/actions/workflows/rust.yml/badge.svg)](https://github.com/tlaferriere/cmp_by_derive/actions/workflows/rust.yml)
# cmp_by_derive

This crate provides the `CmpBy` and `HashBy` derive macros.
- `CmpBy` derives the traits `Ord`, `PartialOrd`, `Eq` and `PartialEq` on types that can't automatically derive those traits because they contain unorderable fields such as `f32` by selecting fields to use in the comparison.
- `CmpBy` and `HashBy` can also implement their traits by calling arbitrary methods


## Usage

Fields that should be used for sorting are marked with the attribute `#[cmp_by]`.
Other fields will be ignored.

This saves a lot of boilerplate, as you can see with the `SomethingElse` struct.

```rust
use std::cmp::Ordering;
use cmp_by_derive::CmpBy;

#[derive(CmpBy)]
struct Something {
    #[cmp_by]
    a: u16,
    #[cmp_by]
    b: u16,
    c: f32,
}

struct SomethingElse {
    a: u16,
    b: u16,
    c: f32,
}

impl Eq for SomethingElse {}

impl PartialEq<Self> for SomethingElse {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl PartialOrd<Self> for SomethingElse {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SomethingElse {
    fn cmp(&self, other: &Self) -> Ordering {
        self.a.cmp(&other.a).then_with(|| { self.b.cmp(&other.b) })
    }
}



assert_eq!(Something { a: 2, b: 0, c: 0.2 }.cmp(&Something { a: 1, b: 1, c: 1.3 }),
           SomethingElse { a: 2, b: 0, c: 0.2 }.cmp(&SomethingElse { a: 1, b: 1, c: 1.3 }));
assert_eq!(Something { a: 1, b: 0, c: 3.3 }.cmp(&Something { a: 1, b: 1, c: 2.3 }),
           SomethingElse { a: 1, b: 0, c: 3.3 }.cmp(&SomethingElse { a: 1, b: 1, c: 2.3 }));
```

You can use `HashBy` the same way you would use `CmpBy`:

```rust
use cmp_by_derive::HashBy;
use cmp_by_derive::CmpBy;
use std::collections::hash_set::HashSet;

#[derive(HashBy, CmpBy)]
struct Something {
    #[cmp_by]
    #[hash_by]
    a: u16,
    #[cmp_by]
    #[hash_by]
    b: u16,
    c: f32,
}

let mut set = HashSet::new();
let something = Something { a: 2, b: 0, c: 0.2 };
assert!(set.insert(something));
```
