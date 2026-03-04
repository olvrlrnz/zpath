#![doc = include_str!("../README.md")]

#[cfg(not(unix))]
compile_error!("unsupported platform");

use std::{
    borrow::Borrow,
    ffi::{OsStr, OsString},
    fmt::Display,
    ops::Deref,
    os::unix::ffi::{OsStrExt, OsStringExt},
    path::{Path, PathBuf},
};

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use zvariant::{self, OwnedValue, Signature, Type, Value};

/// A slice of a filesystem path for use over D-Bus (akin to [`str`]).
///
/// This type is a thin wrapper around [`Path`] that implements the
/// [`zvariant`] traits required to serialize and deserialize filesystem paths
/// over D-Bus. It serializes as `ay` (a D-Bus byte array), preserving the
/// exact byte content of the path — including invalid UTF-8 sequences and
/// embedded NUL bytes.
///
/// This is an *unsized* type, meaning that it must always be used behind a
/// pointer like `&`. For an owned version of this type, see [`ZPathBuf`].
///
/// Because `ZPath` is `#[repr(transparent)]` over [`Path`], converting
/// between the two is always free.
///
/// # Examples
///
/// ```
/// use zpath::ZPath;
///
/// let path = ZPath::new("/run/user/1000/bus");
///
/// // ZPath can also be created from raw bytes, preserving non-UTF-8 content.
/// let non_utf8 = ZPath::from_bytes(b"/tmp/\xff\xfe/data");
/// assert!(non_utf8.to_str().is_none());
/// assert_eq!(non_utf8.as_bytes(), b"/tmp/\xff\xfe/data");
/// ```
///
/// [`Path`]: std::path::Path
#[derive(Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct ZPath(Path);

/// An owned, mutable filesystem path for use over D-Bus (akin to [`String`]).
///
/// This type is a thin wrapper around [`PathBuf`] that implements the
/// [`zvariant`] traits required to serialize and deserialize filesystem paths
/// over D-Bus. It serializes as `ay` (a D-Bus byte array), preserving the
/// exact byte content of the path — including invalid UTF-8 sequences and
/// embedded NUL bytes.
///
/// `ZPathBuf` implements [`Deref`] to [`ZPath`], so all methods on [`ZPath`]
/// are available on `ZPathBuf` values as well.
///
/// # Examples
///
/// ```
/// use zpath::ZPathBuf;
///
/// // Construct from a string using From.
/// let path = ZPathBuf::from("/run/user/1000/bus");
///
/// // Construct from raw bytes, preserving non-UTF-8 content.
/// let non_utf8 = ZPathBuf::from_vec(b"/tmp/\xff\xfe/data".to_vec());
/// assert!(non_utf8.to_str().is_none());
/// assert_eq!(non_utf8.as_bytes(), b"/tmp/\xff\xfe/data");
/// ```
///
/// [`PathBuf`]: std::path::PathBuf
/// [`Deref`]: std::ops::Deref
#[derive(Clone, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ZPathBuf(PathBuf);

impl ZPath {
    /// Converts a `&Path` to a `&ZPath`.
    ///
    /// This is a cost-free conversion.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::Path;
    /// use zpath::ZPath;
    ///
    /// let p = Path::new("foo.txt");
    /// let zp = ZPath::from_path(p);
    /// ```
    #[inline]
    #[must_use]
    pub fn from_path(p: &Path) -> &Self {
        // SAFETY: `ZPath` is `repr(transparent)` over `Path`.
        unsafe { &*(p as *const Path as *const Self) }
    }

    /// Wraps an [`OsStr`] reference or related type as a `ZPath` slice.
    ///
    /// This is a cost-free conversion.
    ///
    /// [`OsStr`]: std::ffi::OsStr
    /// [`String`]: std::string::String
    ///
    /// # Examples
    ///
    /// ```
    /// use zpath::ZPath;
    ///
    /// ZPath::new("foo.txt");
    /// ```
    ///
    /// You can create `ZPath`s from [`String`]s, or even other `ZPath`s:
    ///
    /// ```
    /// use zpath::ZPath;
    ///
    /// let string = String::from("foo.txt");
    /// let from_string = ZPath::new(&string);
    /// let from_path = ZPath::new(&from_string);
    /// assert_eq!(from_string, from_path);
    /// ```
    #[inline]
    #[must_use]
    pub fn new<S: AsRef<OsStr> + ?Sized>(s: &S) -> &Self {
        Self::from_path(Path::new(s.as_ref()))
    }

    /// Creates a `ZPath` from a byte slice.
    ///
    /// # Examples
    ///
    /// ```
    /// use zpath::ZPath;
    ///
    /// let bytes = b"/some/path";
    /// let p = ZPath::from_bytes(bytes);
    /// assert_eq!(std::path::Path::new("/some/path"), p.as_path());    
    /// ```
    #[inline]
    #[must_use]
    pub fn from_bytes(bytes: &[u8]) -> &Self {
        Self::new(OsStr::from_bytes(bytes))
    }

    /// Gets the underlying byte view of the `ZPath` slice.
    ///
    /// # Examples
    ///
    /// ```
    /// use zpath::ZPath;
    ///
    /// let bytes = b"/some/path";
    /// let p = ZPath::new("/some/path");
    /// assert_eq!(b"/some/path", p.as_bytes());    
    /// ```
    #[inline]
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_os_str().as_bytes()
    }

    /// Yields the underlying [`Path`] slice.
    ///
    /// [`Path`]: std::path::Path
    ///
    /// # Examples
    ///
    /// ```
    /// use zpath::ZPath;
    ///
    /// let p = ZPath::new("/test");
    /// assert_eq!(std::path::Path::new("/test"), p.as_path());    
    /// ```
    #[inline]
    #[must_use]
    pub const fn as_path(&self) -> &Path {
        &self.0
    }

    /// Yields the underlying [`OsStr`] slice.
    ///
    /// [`OsStr`]: std::ffi::OsStr
    ///
    /// # Examples
    ///
    /// ```
    /// use zpath::ZPath;
    ///
    /// let os_str = ZPath::new("foo.txt").as_os_str();
    /// assert_eq!(os_str, std::ffi::OsStr::new("foo.txt"));
    /// ```
    #[inline]
    #[must_use]
    pub fn as_os_str(&self) -> &OsStr {
        self.0.as_os_str()
    }

    /// Converts a `ZPath` to a [`Cow<str>`][Cow].
    ///
    /// Any non-UTF-8 sequences are replaced with
    /// [`U+FFFD REPLACEMENT CHARACTER`][U+FFFD].
    ///
    /// [U+FFFD]: std::char::REPLACEMENT_CHARACTER
    /// [Cow]: std::borrow::Cow
    ///
    /// # Examples
    ///
    /// Calling `to_string_lossy` on a `ZPath` with valid unicode:
    ///
    /// ```
    /// use zpath::ZPath;
    ///
    /// let path = ZPath::new("foo.txt");
    /// assert_eq!(path.to_string_lossy(), "foo.txt");
    /// ```
    ///
    /// Had `path` contained invalid unicode, the `to_string_lossy` call might
    /// have returned `"fo�.txt"`.
    #[inline]
    #[must_use]
    pub fn to_string_lossy(&self) -> std::borrow::Cow<'_, str> {
        self.0.to_string_lossy()
    }

    /// Yields a [`&str`] slice if the `ZPath` is valid unicode.
    ///
    /// This conversion may entail doing a check for UTF-8 validity.
    /// Note that validation is performed because non-UTF-8 strings are
    /// perfectly valid for some OS.
    ///
    /// [`&str`]: str
    ///
    /// # Examples
    ///
    /// ```
    /// use zpath::ZPath;
    ///
    /// let path = ZPath::new("foo.txt");
    /// assert_eq!(path.to_str(), Some("foo.txt"));
    /// ```
    #[inline]
    #[must_use]
    pub fn to_str(&self) -> Option<&str> {
        self.0.to_str()
    }
}

impl ZPathBuf {
    /// Creates a `ZPathBuf` from a byte vector.
    ///
    /// # Examples
    ///
    /// ```
    /// use zpath::ZPathBuf;
    ///
    /// let bytes = b"/some/path";
    /// let p = ZPathBuf::from_vec(bytes.into());
    /// assert_eq!(std::path::Path::new("/some/path"), p.as_path());    
    /// ```
    #[inline]
    #[must_use]
    pub fn from_vec(vec: Vec<u8>) -> Self {
        Self(PathBuf::from(OsString::from_vec(vec)))
    }

    /// Consumes the `ZPathBuf`, yielding its internal byte vector.
    ///
    /// # Examples
    ///
    /// ```
    /// use zpath::ZPathBuf;
    ///
    /// let p = ZPathBuf::from_vec(b"/some/path".to_vec());
    /// let vec = p.into_vec();
    /// assert_eq!(vec, b"/some/path");
    /// ```
    #[inline]
    #[must_use]
    pub fn into_vec(self) -> Vec<u8> {
        self.0.into_os_string().into_vec()
    }
}

impl AsRef<[u8]> for ZPath {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl AsRef<OsStr> for ZPath {
    #[inline]
    fn as_ref(&self) -> &OsStr {
        self.0.as_os_str()
    }
}

impl AsRef<Path> for ZPath {
    #[inline]
    fn as_ref(&self) -> &Path {
        self
    }
}

impl<T> AsRef<T> for ZPathBuf
where
    T: ?Sized,
    <Self as Deref>::Target: AsRef<T>,
{
    #[inline]
    fn as_ref(&self) -> &T {
        self.deref().as_ref()
    }
}

impl Borrow<ZPath> for ZPathBuf {
    #[inline]
    fn borrow(&self) -> &ZPath {
        self
    }
}

impl Borrow<Path> for ZPathBuf {
    #[inline]
    fn borrow(&self) -> &Path {
        &self.0
    }
}

impl Deref for ZPath {
    type Target = Path;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Deref for ZPathBuf {
    type Target = ZPath;

    #[inline]
    fn deref(&self) -> &Self::Target {
        ZPath::from_path(&self.0)
    }
}

impl Display for ZPath {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.display().fmt(f)
    }
}

impl Display for ZPathBuf {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&**self, f)
    }
}

impl From<&ZPath> for ZPathBuf {
    #[inline]
    fn from(value: &ZPath) -> Self {
        value.to_owned()
    }
}

impl From<&Path> for ZPathBuf {
    #[inline]
    fn from(value: &Path) -> Self {
        Self(value.to_owned())
    }
}

impl From<PathBuf> for ZPathBuf {
    #[inline]
    fn from(value: PathBuf) -> Self {
        Self(value)
    }
}

impl From<&OsStr> for ZPathBuf {
    #[inline]
    fn from(value: &OsStr) -> Self {
        Self(PathBuf::from(value))
    }
}

impl From<OsString> for ZPathBuf {
    #[inline]
    fn from(value: OsString) -> Self {
        Self(PathBuf::from(value))
    }
}

impl From<&str> for ZPathBuf {
    #[inline]
    fn from(value: &str) -> Self {
        Self(PathBuf::from(value))
    }
}

impl From<String> for ZPathBuf {
    #[inline]
    fn from(value: String) -> Self {
        Self(PathBuf::from(value))
    }
}

impl From<ZPathBuf> for PathBuf {
    #[inline]
    fn from(path: ZPathBuf) -> Self {
        path.0
    }
}

impl From<ZPathBuf> for OsString {
    #[inline]
    fn from(value: ZPathBuf) -> Self {
        value.0.into_os_string()
    }
}

impl PartialEq<ZPath> for ZPathBuf {
    #[inline]
    fn eq(&self, other: &ZPath) -> bool {
        self.0 == other.0
    }
}

impl PartialEq<ZPathBuf> for ZPath {
    #[inline]
    fn eq(&self, other: &ZPathBuf) -> bool {
        self.0 == other.0
    }
}

impl PartialEq<ZPathBuf> for &ZPath {
    #[inline]
    fn eq(&self, other: &ZPathBuf) -> bool {
        self.0 == other.0
    }
}

impl ToOwned for ZPath {
    type Owned = ZPathBuf;

    #[inline]
    fn to_owned(&self) -> Self::Owned {
        ZPathBuf(self.0.to_path_buf())
    }
}

// serde

impl Serialize for ZPath {
    #[inline]
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_bytes(self.as_bytes())
    }
}

impl Serialize for ZPathBuf {
    #[inline]
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.deref().serialize(serializer)
    }
}

impl<'de: 'zp, 'zp> Deserialize<'de> for &'zp ZPath {
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        <&'zp [u8]>::deserialize(deserializer).map(ZPath::from_bytes)
    }
}

impl<'de> Deserialize<'de> for ZPathBuf {
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Vec::<u8>::deserialize(deserializer).map(Self::from_vec)
    }
}

// zvariant - type

impl Type for ZPath {
    const SIGNATURE: &'static Signature = <Vec<u8>>::SIGNATURE;
}

impl Type for ZPathBuf {
    const SIGNATURE: &'static Signature = ZPath::SIGNATURE;
}

// zvariant - value

impl From<&ZPath> for Value<'_> {
    #[inline]
    fn from(value: &ZPath) -> Self {
        Value::from(value.as_bytes())
    }
}

impl From<&ZPathBuf> for Value<'_> {
    #[inline]
    fn from(value: &ZPathBuf) -> Self {
        Value::from(value.as_bytes())
    }
}

impl From<ZPathBuf> for Value<'_> {
    #[inline]
    fn from(value: ZPathBuf) -> Self {
        Value::from(value.as_bytes())
    }
}

impl TryFrom<Value<'_>> for ZPathBuf {
    type Error = zvariant::Error;

    #[inline]
    fn try_from(value: Value<'_>) -> Result<Self, Self::Error> {
        Vec::<u8>::try_from(value).map(ZPathBuf::from_vec)
    }
}

// zvariant - owned value

impl TryFrom<&ZPath> for OwnedValue {
    type Error = zvariant::Error;

    #[inline]
    fn try_from(value: &ZPath) -> Result<Self, Self::Error> {
        Self::try_from(Value::from(value))
    }
}

impl TryFrom<&ZPathBuf> for OwnedValue {
    type Error = zvariant::Error;

    #[inline]
    fn try_from(value: &ZPathBuf) -> Result<Self, Self::Error> {
        Self::try_from(Value::from(value.as_bytes()))
    }
}

impl TryFrom<ZPathBuf> for OwnedValue {
    type Error = zvariant::Error;

    #[inline]
    fn try_from(value: ZPathBuf) -> Result<Self, Self::Error> {
        Self::try_from(Value::from(value.as_bytes()))
    }
}

impl TryFrom<OwnedValue> for ZPathBuf {
    type Error = zvariant::Error;

    #[inline]
    fn try_from(value: OwnedValue) -> Result<Self, Self::Error> {
        Self::try_from(Value::from(value))
    }
}

#[cfg(test)]
mod tests {
    use std::{
        borrow::Borrow,
        collections::{HashMap, HashSet, hash_map::DefaultHasher},
        ffi::{OsStr, OsString},
        hash::{Hash, Hasher},
        os::unix::ffi::{OsStrExt, OsStringExt},
        path::{Path, PathBuf},
    };

    use zvariant::{OwnedValue, Type, Value};

    use crate::{ZPath, ZPathBuf};

    const INVALID_UTF8: &[u8] = b"/tmp/\xff\xfe/data";
    const EMBEDDED_NUL: &[u8] = b"/tmp/foo\x00bar";
    const LONE_CONTINUATION: &[u8] = b"/\x80";
    const OVERLONG_SLASH: &[u8] = b"/tmp/\xc0\xaf";
    const TRUNCATED_MULTIBYTE: &[u8] = b"/dir/\xe0\xa0";

    fn hash_of<T: Hash>(val: &T) -> u64 {
        let mut h = DefaultHasher::new();
        val.hash(&mut h);
        h.finish()
    }

    #[test]
    fn zpath_new_from_str() {
        let zp = ZPath::new("/some/path");
        assert_eq!(zp.as_path(), Path::new("/some/path"));
    }

    #[test]
    fn zpath_new_from_osstr() {
        let os = OsStr::new("/some/path");
        let zp = ZPath::new(os);
        assert_eq!(zp.as_os_str(), os);
    }

    #[test]
    fn zpath_new_from_path() {
        let p = Path::new("/some/path");
        let zp = ZPath::new(p);
        assert_eq!(zp.as_path(), p);
    }

    #[test]
    fn zpath_from_bytes_valid_utf8() {
        let zp = ZPath::from_bytes(b"/some/path");
        assert_eq!(zp.to_str(), Some("/some/path"));
    }

    #[test]
    fn zpath_from_bytes_invalid_utf8() {
        let zp = ZPath::from_bytes(INVALID_UTF8);
        assert!(zp.to_str().is_none());
        assert_eq!(zp.as_bytes(), INVALID_UTF8);
    }

    #[test]
    fn zpath_from_bytes_empty() {
        let zp = ZPath::from_bytes(b"");
        assert_eq!(zp.as_bytes(), b"");
        assert_eq!(zp.as_path(), Path::new(""));
    }

    #[test]
    fn zpath_as_bytes_roundtrip() {
        let bytes = b"/some/path";
        let zp = ZPath::from_bytes(bytes);
        assert_eq!(zp.as_bytes(), bytes);
        assert_eq!(<ZPath as AsRef::<[u8]>>::as_ref(zp), bytes);
    }

    #[test]
    fn zpath_embedded_nul() {
        let zp = ZPath::from_bytes(EMBEDDED_NUL);
        assert_eq!(zp.as_bytes(), EMBEDDED_NUL);
        assert_eq!(zp.as_os_str().as_bytes(), EMBEDDED_NUL);
    }

    #[test]
    fn zpath_lone_continuation_byte() {
        let zp = ZPath::from_bytes(LONE_CONTINUATION);
        assert!(zp.to_str().is_none());
        assert_eq!(zp.as_bytes(), LONE_CONTINUATION);
    }

    #[test]
    fn zpath_overlong_encoding() {
        let zp = ZPath::from_bytes(OVERLONG_SLASH);
        assert!(zp.to_str().is_none());
        assert_eq!(zp.as_bytes(), OVERLONG_SLASH);
    }

    #[test]
    fn zpath_truncated_multibyte() {
        let zp = ZPath::from_bytes(TRUNCATED_MULTIBYTE);
        assert!(zp.to_str().is_none());
        assert_eq!(zp.as_bytes(), TRUNCATED_MULTIBYTE);
    }

    #[test]
    fn zpath_all_0xff_bytes() {
        let garbage: &[u8] = &[0xff; 64];
        let zp = ZPath::from_bytes(garbage);
        assert!(zp.to_str().is_none());
        assert_eq!(zp.as_bytes(), garbage);
    }

    #[test]
    fn zpath_single_dot() {
        let zp = ZPath::new(".");
        assert_eq!(zp.as_bytes(), b".");
    }

    #[test]
    fn zpath_trailing_slashes() {
        let zp = ZPath::from_bytes(b"/tmp///");
        assert_eq!(zp.as_bytes(), b"/tmp///");
    }

    #[test]
    fn zpath_very_long_path() {
        let long = vec![b'a'; 8192];
        let zp = ZPath::from_bytes(&long);
        assert_eq!(zp.as_bytes().len(), 8192);
    }

    #[test]
    fn zpath_newline_and_control_chars() {
        let bytes = b"/tmp/has\nnewline\tand\ttabs\x01";
        let zp = ZPath::from_bytes(bytes);
        assert_eq!(zp.as_bytes(), bytes);
    }

    #[test]
    fn zpath_space_only_component() {
        let zp = ZPath::new("/tmp/ /file");
        assert_eq!(zp.as_bytes(), b"/tmp/ /file");
    }

    #[test]
    fn zpath_to_string_lossy_non_utf8() {
        let zp = ZPath::from_bytes(INVALID_UTF8);
        let lossy = zp.to_string_lossy();
        assert!(lossy.contains('\u{FFFD}'));
    }

    #[test]
    fn zpathbuf_default_is_empty() {
        let zp = ZPathBuf::default();
        assert_eq!(zp.as_bytes(), b"");
    }

    #[test]
    fn zpathbuf_from_vec_valid_utf8() {
        let zp = ZPathBuf::from_vec(b"/some/path".to_vec());
        assert_eq!(zp.to_str(), Some("/some/path"));
    }

    #[test]
    fn zpathbuf_from_vec_invalid_utf8() {
        let zp = ZPathBuf::from_vec(INVALID_UTF8.to_vec());
        assert!(zp.to_str().is_none());
        assert_eq!(zp.as_bytes(), INVALID_UTF8);
    }

    #[test]
    fn zpathbuf_from_vec_embedded_nul() {
        let zp = ZPathBuf::from_vec(EMBEDDED_NUL.to_vec());
        assert_eq!(zp.as_bytes(), EMBEDDED_NUL);
    }

    #[test]
    fn zpathbuf_from_vec_empty() {
        let zp = ZPathBuf::from_vec(vec![]);
        assert_eq!(zp.as_bytes(), b"");
    }

    #[test]
    fn zpath_mountinfo_octal_escaped_space() {
        let zp = ZPath::new(r"/mnt/my\040drive");
        assert!(zp.as_bytes().windows(4).any(|w| w == br"\040"));
    }

    #[test]
    fn zpath_mixed_valid_invalid_components() {
        let bytes = b"/valid/\xff\xfe/also_valid/\x80\x81/end";
        let zp = ZPath::from_bytes(bytes);
        assert!(zp.to_str().is_none());
        assert_eq!(zp.as_bytes(), bytes);
    }

    #[test]
    fn zpath_high_bytes_all_positions() {
        let mut bytes = vec![b'/'];
        bytes.extend(0x80_u8..=0xFF);
        let zp = ZPath::from_bytes(&bytes);
        assert_eq!(zp.as_bytes(), &*bytes);
        assert!(zp.to_str().is_none());
    }

    #[test]
    fn zpath_only_nul_byte() {
        let zp = ZPath::from_bytes(b"\x00");
        assert_eq!(zp.as_bytes(), b"\x00");
    }

    #[test]
    fn zpath_multiple_nuls() {
        let bytes = b"\x00\x00\x00";
        let zp = ZPath::from_bytes(bytes);
        assert_eq!(zp.as_bytes(), bytes);
    }

    // Clone

    #[test]
    fn zpathbuf_clone() {
        let orig = ZPathBuf::from_vec(INVALID_UTF8.to_vec());
        let cloned = orig.clone();
        assert_eq!(orig, cloned);
        assert_eq!(cloned.as_bytes(), INVALID_UTF8);
    }

    // From<T> for ZPathBuf

    #[test]
    fn zpathbuf_from_path_ref() {
        let p = Path::new("/some/path");
        let zp = ZPathBuf::from(p);
        assert_eq!(zp.as_path(), p);
    }

    #[test]
    fn zpathbuf_from_pathbuf() {
        let pb = PathBuf::from("/some/path");
        let zp = ZPathBuf::from(pb.clone());
        assert_eq!(zp.as_path(), pb.as_path());
    }

    #[test]
    fn zpathbuf_from_osstr() {
        let os = OsStr::new("/some/path");
        let zp = ZPathBuf::from(os);
        assert_eq!(zp.as_os_str(), os);
    }

    #[test]
    fn zpathbuf_from_osstring() {
        let os = OsString::from("/some/path");
        let zp = ZPathBuf::from(os.clone());
        assert_eq!(zp.as_os_str(), &os);
    }

    #[test]
    fn zpathbuf_from_str_ref() {
        let zp = ZPathBuf::from("/some/path");
        assert_eq!(zp.as_path(), Path::new("/some/path"));
    }

    #[test]
    fn zpathbuf_from_string() {
        let zp = ZPathBuf::from(String::from("/some/path"));
        assert_eq!(zp.as_path(), Path::new("/some/path"));
    }

    #[test]
    fn zpathbuf_from_zpath_ref() {
        let zp = ZPath::new("/some/path");
        let owned = ZPathBuf::from(zp);
        assert_eq!(owned.as_path(), Path::new("/some/path"));
    }

    #[test]
    fn zpathbuf_from_osstring_non_utf8() {
        let os = OsString::from_vec(INVALID_UTF8.to_vec());
        let zp = ZPathBuf::from(os);
        assert_eq!(zp.as_bytes(), INVALID_UTF8);
    }

    // From<ZPathBuf> for T

    #[test]
    fn pathbuf_from_zpathbuf() {
        let zp = ZPathBuf::from("/some/path");
        let pb: PathBuf = zp.into();
        assert_eq!(pb, PathBuf::from("/some/path"));
    }

    #[test]
    fn osstring_from_zpathbuf() {
        let zp = ZPathBuf::from("/some/path");
        let os: OsString = zp.into();
        assert_eq!(os, OsString::from("/some/path"));
    }

    #[test]
    fn pathbuf_from_zpathbuf_non_utf8() {
        let zp = ZPathBuf::from_vec(INVALID_UTF8.to_vec());
        let pb: PathBuf = zp.into();
        assert_eq!(pb.as_os_str().as_bytes(), INVALID_UTF8);
    }

    #[test]
    fn osstring_from_zpathbuf_non_utf8() {
        let zp = ZPathBuf::from_vec(INVALID_UTF8.to_vec());
        let os: OsString = zp.into();
        assert_eq!(os.as_bytes(), INVALID_UTF8);
    }

    // AsRef<T> for ZPath

    #[test]
    fn zpath_asref_self() {
        let zp = ZPath::new("/some/path");
        let r: &ZPath = zp;
        assert_eq!(r, zp);
    }

    #[test]
    fn zpath_asref_osstr() {
        let zp = ZPath::new("/some/path");
        let os: &OsStr = zp.as_ref();
        assert_eq!(os, OsStr::new("/some/path"));
    }

    #[test]
    fn zpath_asref_path() {
        let zp = ZPath::new("/some/path");
        let p: &Path = zp.as_ref();
        assert_eq!(p, Path::new("/some/path"));
    }

    // AsRef<T> for ZPathBuf

    #[test]
    fn zpathbuf_asref_path() {
        let zp = ZPathBuf::from("/some/path");
        let p: &Path = zp.as_ref();
        assert_eq!(p, Path::new("/some/path"));
    }

    #[test]
    fn zpathbuf_asref_osstr() {
        let zp = ZPathBuf::from("/some/path");
        let os: &OsStr = zp.as_ref();
        assert_eq!(os, OsStr::new("/some/path"));
    }

    // Borrow<T> for ZPath

    #[test]
    fn zpathbuf_borrow_zpath() {
        let zp = ZPathBuf::from("/some/path");
        let b: &ZPath = Borrow::borrow(&zp);
        assert_eq!(b.as_path(), Path::new("/some/path"));
    }

    #[test]
    fn zpathbuf_borrow_path() {
        let zp = ZPathBuf::from("/some/path");
        let b: &Path = Borrow::borrow(&zp);
        assert_eq!(b, Path::new("/some/path"));
    }

    // Deref<T> for ZPath

    #[test]
    fn zpath_deref_to_path() {
        let zp = ZPath::new("/some/path");
        let p: &Path = zp;
        assert_eq!(p, Path::new("/some/path"));
    }

    // Deref<T> for ZPathBuf

    #[test]
    fn zpathbuf_deref_to_zpath() {
        let zp = ZPathBuf::from("/some/path");
        let r: &ZPath = &zp;
        assert_eq!(r.as_path(), Path::new("/some/path"));
    }

    // Display

    #[test]
    fn zpath_display_valid_utf8() {
        let zp = ZPath::new("/some/path");
        let s = zp.to_string();
        assert!(!s.contains(std::char::REPLACEMENT_CHARACTER));
    }

    #[test]
    fn zpath_display_invalid_utf8() {
        let zp = ZPath::from_bytes(INVALID_UTF8);
        let s = zp.to_string();
        assert!(s.contains(std::char::REPLACEMENT_CHARACTER));
    }

    #[test]
    fn zpathbuf_display_valid_utf8() {
        let zp = ZPathBuf::from("/some/path");
        let s = zp.to_string();
        assert!(!s.contains(std::char::REPLACEMENT_CHARACTER));
    }

    #[test]
    fn zpathbuf_display_invalid_utf8() {
        let zp = ZPathBuf::from_vec(INVALID_UTF8.to_vec());
        let s = zp.to_string();
        assert!(s.contains(std::char::REPLACEMENT_CHARACTER));
    }

    // Hash

    #[test]
    fn hash_consistent_zpath_zpathbuf() {
        let zp = ZPath::new("/some/path");
        let zpb = ZPathBuf::from("/some/path");
        assert_eq!(hash_of(&zp), hash_of(&zpb));
    }

    #[test]
    fn hash_consistent_non_utf8() {
        let zp = ZPath::from_bytes(INVALID_UTF8);
        let zpb = ZPathBuf::from_vec(INVALID_UTF8.to_vec());
        assert_eq!(hash_of(&zp), hash_of(&zpb));
    }

    #[test]
    fn zpathbuf_in_hashset() {
        let mut set = HashSet::new();
        set.insert(ZPathBuf::from("/a"));
        set.insert(ZPathBuf::from("/b"));
        set.insert(ZPathBuf::from("/a"));
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn zpathbuf_in_hashmap_lookup_by_zpath() {
        let mut map = HashMap::new();
        map.insert(ZPathBuf::from("/key"), 42);
        assert_eq!(map.get(ZPath::new("/key")), Some(&42));
    }

    // PartialEq

    #[test]
    fn zpath_eq_zpathbuf() {
        let zp = ZPath::new("/some/path");
        let zpb = ZPathBuf::from("/some/path");
        assert_eq!(zp, zpb);
        assert_eq!(zpb, *zp);
    }

    #[test]
    fn zpath_ne_zpathbuf() {
        let zp = ZPath::new("/some/path");
        let zpb = ZPathBuf::from("/some/other/path");
        assert_ne!(zp, &zpb);
    }

    #[test]
    fn zpath_eq_zpathbuf_non_utf8() {
        let zp = ZPath::from_bytes(INVALID_UTF8);
        let zpb = ZPathBuf::from_vec(INVALID_UTF8.to_vec());
        assert_eq!(zp, zpb);
        assert_eq!(zpb, *zp);
    }

    // PartialOrd

    #[test]
    fn zpathbuf_ordering() {
        let pa = PathBuf::from("/a");
        let pb = PathBuf::from("/b");

        let za = ZPathBuf::from("/a");
        let zb = ZPathBuf::from("/b");
        assert_eq!(za < zb, pa < pb);
    }

    #[test]
    fn zpath_ordering_non_utf8() {
        let plo = Path::new(OsStr::from_bytes(b"/a"));
        let phi = Path::new(OsStr::from_bytes(b"/\xfe"));

        let zlo = ZPath::from_bytes(b"/a");
        let zhi = ZPath::from_bytes(b"/\xfe");
        assert_eq!(zlo < zhi, plo < phi);
    }

    // ToOwned for ZPath

    #[test]
    fn zpath_to_owned() {
        let zp = ZPath::new("/some/path");
        let owned: ZPathBuf = zp.to_owned();
        assert_eq!(owned.as_path(), Path::new("/some/path"));
    }

    #[test]
    fn zpath_to_owned_non_utf8() {
        let zp = ZPath::from_bytes(INVALID_UTF8);
        let owned = zp.to_owned();
        assert_eq!(owned.as_bytes(), INVALID_UTF8);
    }

    // serde

    #[test]
    fn zpathbuf_serde_json_roundtrip_utf8() {
        let orig = ZPathBuf::from("/some/path");
        let json = serde_json::to_string(&orig).unwrap();
        let back: ZPathBuf = serde_json::from_str(&json).unwrap();
        assert_eq!(orig, back);
    }

    #[test]
    fn zpathbuf_serde_json_roundtrip_non_utf8() {
        let orig = ZPathBuf::from_vec(INVALID_UTF8.to_vec());
        let json = serde_json::to_string(&orig).unwrap();
        let back: ZPathBuf = serde_json::from_str(&json).unwrap();
        assert_eq!(orig, back);
    }

    #[test]
    fn zpathbuf_serde_json_roundtrip_embedded_nul() {
        let orig = ZPathBuf::from_vec(EMBEDDED_NUL.to_vec());
        let json = serde_json::to_string(&orig).unwrap();
        let back: ZPathBuf = serde_json::from_str(&json).unwrap();
        assert_eq!(orig, back);
    }

    #[test]
    fn zpathbuf_serde_json_roundtrip_empty() {
        let orig = ZPathBuf::default();
        let json = serde_json::to_string(&orig).unwrap();
        let back: ZPathBuf = serde_json::from_str(&json).unwrap();
        assert_eq!(orig, back);
    }

    // zvariant

    #[test]
    fn zpath_signature_is_ay() {
        assert_eq!(ZPath::SIGNATURE, "ay");
    }

    #[test]
    fn zpathbuf_signature_is_ay() {
        assert_eq!(ZPathBuf::SIGNATURE, "ay");
    }

    #[test]
    fn zpath_into_value() {
        let zp = ZPath::new("/some/path");
        let val = Value::from(zp);
        let bytes: Vec<u8> = val.try_into().unwrap();
        assert_eq!(bytes, b"/some/path");
    }

    #[test]
    fn zpath_into_value_non_utf8() {
        let zp = ZPath::from_bytes(INVALID_UTF8);
        let val = Value::from(zp);
        let bytes: Vec<u8> = val.try_into().unwrap();
        assert_eq!(bytes, INVALID_UTF8);
    }

    #[test]
    fn zpathbuf_into_value_and_back() {
        let orig = ZPathBuf::from("/some/path");
        let val = Value::from(&orig);
        let back = ZPathBuf::try_from(val).unwrap();
        assert_eq!(orig, back);
    }

    #[test]
    fn zpathbuf_into_value_and_back_non_utf8() {
        let orig = ZPathBuf::from_vec(INVALID_UTF8.to_vec());
        let val = Value::from(&orig);
        let back = ZPathBuf::try_from(val).unwrap();
        assert_eq!(orig, back);
    }

    #[test]
    fn zpathbuf_into_value_and_back_embedded_nul() {
        let orig = ZPathBuf::from_vec(EMBEDDED_NUL.to_vec());
        let val = Value::from(&orig);
        let back = ZPathBuf::try_from(val).unwrap();
        assert_eq!(orig, back);
    }

    #[test]
    fn zpathbuf_into_value_and_back_empty() {
        let orig = ZPathBuf::default();
        let val = Value::from(&orig);
        let back = ZPathBuf::try_from(val).unwrap();
        assert_eq!(orig, back);
    }

    #[test]
    fn zpath_to_ownedvalue_and_back() {
        let orig = ZPath::new("/some/path");
        let ov = OwnedValue::try_from(orig).unwrap();
        let back = ZPathBuf::try_from(ov).unwrap();
        assert_eq!(orig, back);
    }

    #[test]
    fn zpathbuf_to_ownedvalue_and_back() {
        let orig = ZPathBuf::from("/some/path");
        let ov = OwnedValue::try_from(orig.clone()).unwrap();
        let back = ZPathBuf::try_from(ov).unwrap();
        assert_eq!(orig, back);
    }

    #[test]
    fn zpathbuf_to_ownedvalue_and_back_non_utf8() {
        let orig = ZPathBuf::from_vec(INVALID_UTF8.to_vec());
        let ov = OwnedValue::try_from(&orig).unwrap();
        let back = ZPathBuf::try_from(ov).unwrap();
        assert_eq!(orig, back);
    }

    #[test]
    fn zpathbuf_to_ownedvalue_and_back_embedded_nul() {
        let orig = ZPathBuf::from_vec(EMBEDDED_NUL.to_vec());
        let ov = OwnedValue::try_from(&orig).unwrap();
        let back = ZPathBuf::try_from(ov).unwrap();
        assert_eq!(orig, back);
    }
}
