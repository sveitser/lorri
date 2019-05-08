//! Ops are command-line callables.

pub mod build;
pub mod daemon;
pub mod direnv;
pub mod info;
pub mod ping;
pub mod shell;
pub mod upgrade;
pub mod watch;

/// Non-zero exit status from an op
#[derive(Debug)]
pub struct ExitError {
    /// Exit code of the process, should be non-zero
    exitcode: i32,

    /// Final dying words
    message: String,
}

/// Final result from a CLI operation
pub type OpResult = Result<Option<String>, ExitError>;

/// Return an OpResult with a final message to print before exit 0
/// Note, the final message is possibly intended to be consumed
/// by automated tests.
pub fn ok_msg<T>(message: T) -> OpResult
where
    T: Into<String>,
{
    Ok(Some(message.into()))
}

/// Return an OpResult with no message to be printed, producing
/// a silent exit 0
pub fn ok() -> OpResult {
    Ok(None)
}

impl ExitError {
    /// Exit 1 with an exit message
    pub fn errmsg<T>(message: T) -> OpResult
    where
        T: Into<String>,
    {
        ExitError::err(1, message.into())
    }

    /// Helpers to create exit results
    ///
    /// Note: err panics if exitcode is zero.
    fn err<T>(exitcode: i32, message: T) -> OpResult
    where
        T: Into<String>,
    {
        assert!(exitcode != 0, "ExitError exitcode must be > 0!");

        Err(ExitError {
            exitcode,
            message: message.into(),
        })
    }

    /// Exit code of the failure message, guaranteed to be > 0
    pub fn exitcode(&self) -> i32 {
        self.exitcode
    }

    /// Exit message to be displayed to the user on stderr
    pub fn message(&self) -> &str {
        &self.message
    }
}

#[cfg(test)]
mod tests {
    use super::ExitError;

    #[test]
    #[should_panic]
    fn err_requires_nonzero() {
        // is_err() because we're forced to use the result
        ExitError::err(0, "bogus").is_err();
    }

    #[test]
    fn err_doesnt_always_panic() {
        assert!(ExitError::err(1, "bogus").is_err());
    }

    #[test]
    fn getters() {
        match ExitError::err(1, "bogus") {
            Err(e) => {
                assert_eq!(e.exitcode(), 1);
                assert_eq!(e.message(), "bogus");
            }
            otherwise => {
                panic!("{:?}", otherwise);
            }
        }
    }
}
