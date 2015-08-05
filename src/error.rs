#[derive(Debug)]
pub enum Error {
    Utf,
    IO,
}
impl From<::std::io::Error> for Error {
    fn from(_: ::std::io::Error) -> Error {
        Error::IO
    }
}
impl From<::std::string::FromUtf8Error> for Error {
    fn from(_: ::std::string::FromUtf8Error) -> Error {
        Error::Utf
    }
}
impl ::std::fmt::Display for Error {
    fn fmt(&self, fmt: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(fmt, "{}", (self as &::std::error::Error).description())
    }
}
impl ::std::error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::Utf => "Plugin result must be valid UTF-8",
            Error::IO => "Could not send plugin result to the bot"
        }
    }
}
