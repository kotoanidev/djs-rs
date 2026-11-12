# djs-collection

A feature-rich map type modelled on discord.js Collection.

> **Status:** pre-alpha. [`Collection`] with discord.js' helpers is here. Track progress in the
> [roadmap](https://github.com/kotoanidev/djs-rs/blob/main/ROADMAP.md).

**discord.js counterpart:** `@discordjs/collection`

## The collection

[`Collection<K, V>`](Collection) is a hash map that remembers insertion order, like the JS
`Map` discord.js' `Collection` extends: iteration, `first`, `last`, `at` and `random` follow
the order keys were first inserted in, and `remove` keeps it. It is built on
[`indexmap`](https://docs.rs/indexmap) and hashes with foldhash by default.

```rust
use djs_collection::Collection;

let mut members: Collection<u64, &str> = Collection::new();
members.insert(1, "ada");
members.insert(2, "grace");
members.insert(3, "linus");

assert_eq!(members.first(), Some(&"ada"));
assert_eq!(members.at(-1), Some(&"linus"));
assert_eq!(members.find_key(|name, _| name.starts_with('g')), Some(&2));

// Transformations consume the collection and clone nothing; clone it to keep it.
let (short, long) = members.clone().partition(|name, _| name.len() <= 3);
assert_eq!(short.len(), 1);
assert_eq!(long.keys().copied().collect::<Vec<_>>(), [2, 3]);

// In-place changes.
assert_eq!(members.sweep(|name, _| name.contains('i')), 1);
let winner = members.random();
assert!(winner.is_some());
```

## Methods

| Kind | Methods | Takes |
|---|---|---|
| Map | `get`, `get_mut`, `insert`, `remove` (keeps order), `swap_remove`, `contains_key`, `len`, `iter`, `keys`, `values`, … | |
| Position | `first`, `first_key`, `last`, `last_key`, `at`, `key_at` (negative from the end), `index_of` | `&self` |
| Search | `find`, `find_key`, `find_last`, `find_last_key`, `has_all`, `has_any`, `map` → `Vec` | `&self` |
| Random | `random`, `random_key`, `random_n` | `&self` |
| In place | `ensure`, `sweep`, `retain`, `sort`, `sort_by`, `reverse` | `&mut self` |
| New collection | `filter`, `partition`, `map_values`, `flat_map`, `to_sorted`, `to_sorted_by`, `to_reversed`, `concat`, `union`, `intersection`, `difference`, `symmetric_difference`, `merge`, `Collection::combine_entries` | `self` |

discord.js' `each`, `some`, `every`, `reduce` and `tap` are the iterators' job
(`.values().any(…)`, `.iter().fold(…)`). `==` compares keys and values in any order, like
discord.js' `equals`. `as_index_map` and `into_index_map` reach the inner `IndexMap` for
anything else.

## Part of djs-rs

`djs-collection` is one crate of [djs-rs](https://github.com/kotoanidev/djs-rs), a high-performance
Rust library for building Discord bots modelled on discord.js v14. It can be used on its own
or through the main `djs-rs` crate, as `djs_rs::collection`.

## License

Licensed under either of [Apache License, Version 2.0](https://github.com/kotoanidev/djs-rs/blob/main/LICENSE-APACHE)
or [MIT license](https://github.com/kotoanidev/djs-rs/blob/main/LICENSE-MIT) at your option.
