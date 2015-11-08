use std::result;
use std::borrow::Cow;
use std::io;
use std::fmt;
use std::error;

use bson::{self, oid};
use itertools::Itertools;

pub type Result<T> = result::Result<T, Error>;

#[derive(Debug)]
pub struct PartialSave {
    pub cause: Box<error::Error>,
    pub successful_ids: Vec<oid::ObjectId>
}

impl error::Error for PartialSave {
    fn description(&self) -> &str { "save operation completed partially" }
    fn cause(&self) -> Option<&error::Error> { Some(&*self.cause) }
}

struct OidHexDisplay(oid::ObjectId);

impl fmt::Display for OidHexDisplay {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        static CHARS: &'static [u8] = b"0123456789abcdef";
        for &byte in &self.0.bytes() {
            try!(write!(f,
                "{}{}",
                CHARS[(byte >> 4) as usize] as char,
                CHARS[(byte & 0xf) as usize] as char
            ));
        }
        Ok(())
    }
}

impl fmt::Display for PartialSave {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.successful_ids.is_empty() {
            write!(f, "saved nothing due to an error: {}", self.cause)
        } else {
            write!(f,
                "only saved objects with ids: [{}] due to an error: {}",
                self.successful_ids.iter().cloned().map(OidHexDisplay).join(", "),
                self.cause
            )
        }
    }
}

quick_error! {
    #[derive(Debug)]
    pub enum Error {
        Io(err: io::Error) {
            from()
            description("I/O error")
            display("I/O error: {}", err)
            cause(err)
        }
        BsonEncoding(err: bson::EncoderError) {
            from()
            description("BSON encoding error")
            display("BSON encoding error: {}", err)
            cause(err)
        }
        BsonDecoding(err: bson::DecoderError) {
            from()
            description("BSON decoding error")
            display("BSON decoding error: {}", err)
            cause(err)
        }
        PartialSave(err: PartialSave) {
            from()
            description("partial save")
            display("partial save: {}", err)
            cause(&*err.cause)
        }
        Other(msg: Cow<'static, str>) {
            description(&*msg)
            display("{}", msg)
            from(s: &'static str) -> (s.into())
            from(s: String) -> (s.into())
        }
    }
}
