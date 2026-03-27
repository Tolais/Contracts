use crate::errors::codes::Error;

// Generic condition checker
pub fn ensure(condition: bool, err: Error) -> Result<(), Error> {
    if !condition {
        return Err(err);
    }
    Ok(())
}

// Option → Result converter
pub fn unwrap_or_error<T>(opt: Option<T>, err: Error) -> Result<T, Error> {
    match opt {
        Some(val) => Ok(val),
        None => Err(err),
    }
}