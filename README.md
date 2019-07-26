# qjs [![travis](https://travis-ci.org/flier/rust-quickjs.svg?branch=master)](https://travis-ci.org/flier/rust-quickjs) [![Build status](https://ci.appveyor.com/api/projects/status/jhlk20pjdrx0jh5u?svg=true)](https://ci.appveyor.com/project/flier/rust-quickjs) [![crate](https://img.shields.io/crates/v/qjs.svg)](https://crates.io/crates/qjs) [![docs](https://docs.rs/qjs/badge.svg)](https://docs.rs/crate/qjs/) [![dependency status](https://deps.rs/repo/github/flier/rust-quickjs/status.svg)](https://deps.rs/repo/github/flier/rust-quickjs)

`qjs` is an experimental Rust binding for the QuickJS Javascript Engine

## Usage

To use `qjs` in your project, add the following to your Cargo.toml:

```toml
[dependencies]
qjs = "0.1"
```

## Example

```rust
let v: Option<i32> = qjs::eval("1+2").unwrap();

assert_eq!(v, Some(3));
```
