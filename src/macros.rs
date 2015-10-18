
macro_rules! bson {
    { [ $($e:tt),* ] } => {{
        let mut v = Vec::new();
        $(v.push(::bson::Bson::from(bson!($e)));)*
        v
    }};
    { { $($k:expr => $v:tt),* } } => {{
        let mut d = ::bson::Document::new();
        $(d.insert($k, ::bson::Bson::from(bson!($v)));)*
        d
    }};
    { $($k:expr => $v:tt),* } => { bson!{{ $($k => $v),* }} };
    { $e:expr } => { $e };
    { $($e:tt),+ } => { bson![[ $($e),+ ]] };
}
