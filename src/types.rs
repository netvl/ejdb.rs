use std::result;
use std::borrow::Cow;
use std::io;

pub type Result<T> = result::Result<T, EjdbError>;

error_type! {
    #[derive(Debug)]
    pub enum EjdbError {
        Io(io::Error) {
            cause (e) Some(e);
        },
        Other(Cow<'static, str>) {
            desc (e) &**e;
            from (s: &'static str) s.into();
            from (s: String) s.into();
        }
    }
}
