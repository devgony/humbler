use anyhow::{anyhow, Error, Result};

pub trait OptionExt<T, E: ToString> {
    fn to_result(self, error: E) -> Result<T, Error>;
}

impl<T, E: ToString> OptionExt<T, E> for Option<T> {
    fn to_result(self, error_msg: E) -> Result<T, Error> {
        self.ok_or_else(|| anyhow!(error_msg.to_string()))
    }
}
