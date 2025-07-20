use std::error::Error;

pub fn return_error_if_true(
    condition: bool,
    status: Box<dyn Error + Send + Sync>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    if condition { Err(status) } else { Ok(()) }
}
