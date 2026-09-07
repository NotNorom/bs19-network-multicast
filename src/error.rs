/// Receiver specific errors
#[derive(Debug, thiserror::Error)]
pub enum ReceiveError {
    /// IO errors
    #[error("std error: {0:?}")]
    Io(#[from] std::io::Error),

    /// Failed to set a timeout value for the receiver
    #[error("Failed to sent a timeout value for the receiver")]
    SetTimeoutValue(#[source] std::io::Error),

    /// Ip version (ipv4 or ipv6) used when the other is expected.
    ///
    /// # Arguments
    /// A string describing the situation where the wrong IpVersion was encountered.
    #[error("Ip version (ipv4 or ipv6) used when the other is expected, msg: {0}")]
    IpVersionError(String),

    /// Thrown to indicate that the operation attempted is unsupported on the current OS
    /// For example this is used to indicate that multicast-IPv6 isn't supported current on Windows.
    ///
    /// # Arguments
    /// A message describing why this error was returned / the operation that was not supported.
    #[error("Operation attempted is unsupported on the current OS, msg: {0}")]
    OsOperationUnsupported(String),

    #[error("Program was quit using Ctrl-C")]
    CtrlC,
}
