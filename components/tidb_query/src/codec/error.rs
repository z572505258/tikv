// Copyright 2017 TiKV Project Authors. Licensed under Apache-2.0.

use regex::Error as RegexpError;
use serde_json::error::Error as SerdeError;
use std::error::Error as StdError;
use std::fmt::Display;
use std::io;
use std::num::ParseFloatError;
use std::str::Utf8Error;
use std::string::FromUtf8Error;
use std::{error, str};
use tipb::expression::ScalarFuncSig;
use tipb::select;

pub const ERR_UNKNOWN: i32 = 1105;
pub const ERR_REGEXP: i32 = 1139;
pub const ZLIB_LENGTH_CORRUPTED: i32 = 1258;
pub const ZLIB_DATA_CORRUPTED: i32 = 1259;
pub const WARN_DATA_TRUNCATED: i32 = 1265;
pub const ERR_TRUNCATE_WRONG_VALUE: i32 = 1292;
pub const ERR_UNKNOWN_TIMEZONE: i32 = 1298;
pub const ERR_DIVISION_BY_ZERO: i32 = 1365;
pub const ERR_DATA_TOO_LONG: i32 = 1406;
pub const ERR_DATA_OUT_OF_RANGE: i32 = 1690;

quick_error! {
    #[derive(Debug)]
    pub enum Error {
        InvalidDataType(reason: String) {
            description("invalid data type")
            display("{}", reason)
        }
        Encoding(err: Utf8Error) {
            from()
            cause(err)
            description("encoding failed")
        }
        ColumnOffset(offset: usize) {
            description("column offset not found")
            display("illegal column offset: {}", offset)
        }
        UnknownSignature(sig: ScalarFuncSig) {
            description("Unknown signature")
            display("Unknown signature: {:?}", sig)
        }
        Eval(s: String,code:i32) {
            description("evaluation failed")
            display("{}", s)
        }
        Other(err: Box<dyn error::Error + Send + Sync>) {
            from()
            cause(err.as_ref())
            description(err.description())
            display("{}", err)
        }
    }
}

impl Error {
    pub fn overflow(data: impl Display, expr: impl Display) -> Error {
        let msg = format!("{} value is out of range in '{}'", data, expr);
        Error::Eval(msg, ERR_DATA_OUT_OF_RANGE)
    }

    pub fn truncated_wrong_val(data_type: impl Display, val: impl Display) -> Error {
        let msg = format!("Truncated incorrect {} value: '{}'", data_type, val);
        Error::Eval(msg, ERR_TRUNCATE_WRONG_VALUE)
    }

    pub fn truncated() -> Error {
        Error::Eval("Data Truncated".into(), WARN_DATA_TRUNCATED)
    }

    pub fn cast_neg_int_as_unsigned() -> Error {
        let msg = "Cast to unsigned converted negative integer to it's positive complement";
        Error::Eval(msg.into(), ERR_UNKNOWN)
    }

    pub fn cast_as_signed_overflow() -> Error {
        let msg =
            "Cast to signed converted positive out-of-range integer to it's negative complement";
        Error::Eval(msg.into(), ERR_UNKNOWN)
    }

    pub fn invalid_timezone(given_time_zone: impl Display) -> Error {
        let msg = format!("unknown or incorrect time zone: {}", given_time_zone);
        Error::Eval(msg, ERR_UNKNOWN_TIMEZONE)
    }

    pub fn division_by_zero() -> Error {
        let msg = "Division by 0";
        Error::Eval(msg.into(), ERR_DIVISION_BY_ZERO)
    }

    pub fn data_too_long(msg: String) -> Error {
        if msg.is_empty() {
            Error::Eval("Data Too Long".into(), ERR_DATA_TOO_LONG)
        } else {
            Error::Eval(msg, ERR_DATA_TOO_LONG)
        }
    }

    pub fn code(&self) -> i32 {
        match *self {
            Error::Eval(_, code) => code,
            _ => ERR_UNKNOWN,
        }
    }

    pub fn is_overflow(&self) -> bool {
        self.code() == ERR_DATA_OUT_OF_RANGE
    }

    pub fn unexpected_eof() -> Error {
        tikv_util::codec::Error::unexpected_eof().into()
    }

    pub fn invalid_time_format(val: impl Display) -> Error {
        let msg = format!("invalid time format: '{}'", val);
        Error::Eval(msg, ERR_TRUNCATE_WRONG_VALUE)
    }

    pub fn incorrect_datetime_value(val: impl Display) -> Error {
        let msg = format!("Incorrect datetime value: '{}'", val);
        Error::Eval(msg, ERR_TRUNCATE_WRONG_VALUE)
    }

    pub fn zlib_length_corrupted() -> Error {
        let msg = "ZLIB: Not enough room in the output buffer (probably, length of uncompressed data was corrupted)";
        Error::Eval(msg.into(), ZLIB_LENGTH_CORRUPTED)
    }

    pub fn zlib_data_corrupted() -> Error {
        Error::Eval("ZLIB: Input data corrupted".into(), ZLIB_DATA_CORRUPTED)
    }
}

impl From<Error> for select::Error {
    fn from(error: Error) -> select::Error {
        let mut err = select::Error::default();
        err.set_code(error.code());
        err.set_msg(format!("{:?}", error));
        err
    }
}

impl From<FromUtf8Error> for Error {
    fn from(err: FromUtf8Error) -> Error {
        Error::Encoding(err.utf8_error())
    }
}

impl From<SerdeError> for Error {
    fn from(err: SerdeError) -> Error {
        box_err!("serde:{:?}", err)
    }
}

impl From<ParseFloatError> for Error {
    fn from(err: ParseFloatError) -> Error {
        box_err!("parse float: {:?}", err)
    }
}

impl From<tikv_util::codec::Error> for Error {
    fn from(err: tikv_util::codec::Error) -> Error {
        box_err!("codec:{:?}", err)
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        let uerr: tikv_util::codec::Error = err.into();
        uerr.into()
    }
}

impl From<RegexpError> for Error {
    fn from(err: RegexpError) -> Error {
        let msg = format!("Got error '{:.64}' from regexp", err.description());
        Error::Eval(msg, ERR_REGEXP)
    }
}

pub type Result<T> = std::result::Result<T, Error>;
