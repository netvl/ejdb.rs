
/// A convenience macro to construct BSON documents.
///
/// It is very similar to `bson!/doc!` macros provided by `bson` crate, but somewhat
/// more versatile.
///
/// Note that due to limitations of Rust macros, any expression which is not a single token tree
/// must be wrapped in parentheses. See key `"a"` in the example below.
///
/// # Examples
///
/// ```
/// #[macro_use] extern crate ejdb;
/// use ejdb::bson::{Bson, Document};
///
/// # fn main() {
/// let mut d1 = Document::new();
/// d1.insert("a", -123i32);
/// d1.insert("b", "hello");
/// d1.insert("c", vec![Bson::I32(456), Bson::FloatingPoint(12.3)]);
/// let mut d2 = Document::new();
/// d2.insert("x", 897);
/// d2.insert("y", "world");
/// d1.insert("d", d2);
///
/// assert_eq!(d1, bson! {
///     "a" => (-123i32),
///     "b" => "hello",
///     "c" => [456, 12.3],
///     "d" => {
///         "x" => 897,
///         "y" => "world"
///     }
/// });
/// # }
/// ```
///
/// Constructing arrays is supported as well:
///
/// ```
/// #[macro_use] extern crate ejdb;
/// use ejdb::bson::Bson;
///
/// # fn main() {
/// let arr = vec![Bson::I64(1_024_000), Bson::String("hello".into())];
/// assert_eq!(arr, bson![1_024_000_i64, "hello"]);
/// # }
/// ```
///
/// Single values will be converted to `Bson` directly:
///
/// ```
/// #[macro_use] extern crate ejdb;
/// use ejdb::bson::Bson;
///
/// # fn main() {
/// assert_eq!(bson!("hello world"), Bson::String("hello world".into()));
/// assert_eq!(bson!(("[ab]+".to_owned(), "i".to_owned())), Bson::RegExp("[ab]+".into(), "i".into()));
/// assert_eq!(bson!(true), Bson::Boolean(true));
/// # }
/// ```
#[macro_export]
macro_rules! bson {
    { [ $($e:tt),* ] } => {{
        let mut v = Vec::new();
        $(v.push($crate::bson::Bson::from(bson!($e)));)*
        v
    }};
    { { $($k:expr => $v:tt),* } } => {{
        let mut d = $crate::bson::Document::new();
        $(d.insert($k, bson!($v));)*
        d
    }};
    { $($k:expr => $v:tt),* } => { bson!{{ $($k => $v),* }} };
    { $e:expr } => { $crate::bson::Bson::from($e) };
    { $($e:tt),+ } => { bson![[ $($e),+ ]] };
}
