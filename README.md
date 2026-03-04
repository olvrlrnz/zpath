# zpath

Filesystem paths for use with D-Bus via [`zvariant`].

This crate provides two types, [`ZPathBuf`] and [`ZPath`] (akin to [`PathBuf`]
and [`Path`]), for sending and receiving filesystem paths over D-Bus. They are
thin wrappers around [`PathBuf`] and [`Path`] that implement the [`zvariant`]
traits for serializing and deserializing as D-Bus byte arrays (`ay`), preserving
the full byte content of a path, including invalid UTF-8 and embedded NUL bytes.

## Differences from `zvariant::FilePath`

[`zvariant`] provides [`FilePath`] for this purpose, but it has two limitations:

- It wraps [`CStr`] internally, which cannot represent paths containing embedded
  NUL bytes and does not follow the [`Path`]-based idioms common in Rust.
- It does not implement [`From`]/[`TryFrom`] for [`Value`] and [`OwnedValue`],
  which [`zbus`] requires for any type used as a `#[zbus(property)]`.

## Platform support

Unix only. On Unix, [`OsString`] is an arbitrary byte sequence, which maps
naturally to a D-Bus `ay`. On Windows, [`OsString`] uses WTF-8 encoding, which
does not have the same property.

## Examples

```rust
use zpath::{ZPath, ZPathBuf};
use zvariant::Value;

// Construct from a string or raw bytes.
let path = ZPathBuf::from("/some/path");
let non_utf8 = ZPathBuf::from_vec(b"/tmp/\xff\xfe/data".to_vec());

// Convert to a zvariant Value and back.
let value = Value::from(path.clone());
let roundtrip = ZPathBuf::try_from(value).unwrap();
assert_eq!(path, roundtrip);
```

## Using `ZPath` with `zbus`

In addition to regular method arguments, `ZPathBuf` can also be used as
a property due to its support for [`OwnedValue`].
`ZPath` cannot be used for anything other than arguments.

```rust
use serde::{Serialize, Deserialize};
use zbus::zvariant::{Type, OwnedValue, Value};
use zpath::{ZPath, ZPathBuf};

#[derive(Serialize, Deserialize, Type, Value, OwnedValue)]
struct Container {
    path: ZPathBuf,
}

#[zbus::proxy(
    interface = "org.example.MyService1",
    default_service = "org.example.MyService",
    default_path = "/org/example/MyService"
)]
trait MyService {
    /// Property holding a single ZPathBuf
    #[zbus(property)]
    fn path_prop(&self) -> zbus::fdo::Result<ZPathBuf>;

    /// Property holding a vector of ZPathBuf
    #[zbus(property)]
    fn many_path_prop(&self) -> zbus::fdo::Result<Vec<ZPathBuf>>;

    /// Property holding a vector of containers
    #[zbus(property)]
    fn many_container_prop(&self) -> zbus::fdo::Result<Vec<Container>>;

    /// Method taking a reference to a ZPathBuf
    async fn takes_a_path(&self, path: &ZPath) -> Result<String, zbus::Error>;

    /// Method taking a ZPathBuf by value
    async fn takes_a_pathbuf(&self, path: ZPathBuf) -> Result<String, zbus::Error>;

    /// Method returning a ZPathBuf
    async fn ret_a_pathbuf(&self, path: &ZPath) -> Result<ZPathBuf, zbus::Error>;

    /// Method returning a vector of ZPathBuf
    async fn ret_many_pathbufs(&self, path: &ZPath) -> Result<Vec<ZPathBuf>, zbus::Error>;

    /// Method returning a vector of containers
    async fn ret_many_containers(&self, path: &ZPath) -> Result<Vec<Container>, zbus::Error>;

    // The following method signatures are not supported:
    //
    //fn ret_dst(&self, path: ZPathBuf) -> ZPath;
    //fn ret_dst_ref(&self, path: ZPathBuf) -> &ZPath;
}
```

[`From`]: https://doc.rust-lang.org/stable/std/convert/trait.From.html
[`TryFrom`]: https://doc.rust-lang.org/stable/std/convert/trait.TryFrom.html
[`zvariant`]: https://docs.rs/zvariant/latest/zvariant/index.html
[`zbus`]: https://docs.rs/zbus/latest/zbus/index.html
[`FilePath`]: https://docs.rs/zvariant/latest/zvariant/struct.FilePath.html
[`CStr`]: https://doc.rust-lang.org/stable/std/ffi/struct.CStr.html
[`Path`]: https://doc.rust-lang.org/stable/std/path/struct.Path.html
[`PathBuf`]: https://doc.rust-lang.org/stable/std/path/struct.PathBuf.html
[`OsString`]: https://doc.rust-lang.org/stable/std/ffi/struct.OsString.html
[`Value`]: https://docs.rs/zvariant/latest/zvariant/enum.Value.html
[`OwnedValue`]: https://docs.rs/zvariant/latest/zvariant/struct.OwnedValue.html
