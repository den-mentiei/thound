# T H O U N D

[![Latest Version]][crates.io]
[![MIT licensed][mit-badge]][mit-url]
[![API](https://docs.rs/thound/badge.svg)][docs.rs]

[Latest Version]: https://img.shields.io/crates/v/thound.svg
[crates.io]: https://crates.io/crates/thound
[docs.rs]: https://docs.rs/thound
[mit-badge]: https://img.shields.io/badge/license-MIT-blue.svg
[mit-url]: https://github.com/den-mentiei/thound/blob/main/LICENSE

## Overview

The purpose of this is to find the folders that contain libraries you
may need to link against, on Windows, if you are linking with any
compiled C or C++ code. This will be necessary for many non-C++
programming language environments that wnat to provide compatibility.

## Getting started

Add `thound` to your `Cargo.toml`:

``` toml
[dependencies]
thound = "0.1.0"
```

## Usage

The usage is pretty straight-forward, just call the only available
function and use the result with information contained:

``` rust
fn main() {
	let info = thound::find_vc_and_windows_sdk();
	println!("{info:#?}");
}
```

## Implementation notes

One of the goals for this implementation was to be as dependency-free
and quick compiling as possible. As a result, this crate has zero
dependencies except `std`.

Having no dependencies, means the code contains the minimal Windows
API & COM shenanigans, required to do its job. Those could be replaced
by `windows-rs`, but it wasn't done on purpose.

## Note

The following is originally written by Jonathan Blow and it mirrors my
thoughts and explains the reason for this crate to exist.

```c++
// I hate this kind of code. The fact that we have to do this at all
// is stupid, and the actual maneuvers we need to go through
// are just painful. If programming were like this all the time,
// I would quit.
//
// Because this is such an absurd waste of time, I felt it would be
// useful to package the code in an easily-reusable way
```

## Alternatives

Microsoft provides its own solution to this problem, called
"[vswhere]", is a much bigger program (a binary then!) I don't want to
deal with in most cases.

[vswhere]: https://github.com/Microsoft/vswhere

## License

This crate is licensed under the [MIT license].
Implementation is a Rust port of the [Original code] by Jonathan Blow,
which was released under the MIT license.

[MIT license]: https://github.com/den-mentiei/thound/blob/main/LICENSE
[Original code]: https://gist.github.com/den-mentiei/2b5319da0ac5a128d89cc611b2d4d75a

### Contribution

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in Thound by you, shall be licensed as MIT,
without any additional terms or conditions.
