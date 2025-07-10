use std::fmt;

#[derive(Debug)]
pub enum FrameworkError {
    Runner(RunnerError),
    Analyzer(AnalyzerError),
    Storage(StorageError),
    Stream(StreamError),
    Config(ConfigError),
    Server(ServerError),
}

#[derive(Debug)]
pub enum RunnerError {
    StartupFailed(String),
    AlreadyRunning,
    NotRunning,
    ConfigurationError(String),
    ExecutionError(String),
    TimeoutError,
    HealthCheckFailed(String),
}

#[derive(Debug)]
pub enum AnalyzerError {
    InitializationFailed(String),
    ProcessingError(String),
    DependencyError(String),
    ConfigurationError(String),
    SchemaValidationError(String),
}

#[derive(Debug)]
pub enum StorageError {
    ConnectionFailed(String),
    WriteError(String),
    ReadError(String),
    QueryError(String),
    CapacityExceeded,
    SerializationError(String),
}

#[derive(Debug)]
pub enum StreamError {
    ChannelClosed,
    BroadcastFailed { failed_count: usize, total_count: usize },
    BackpressureExceeded,
    FilterError(String),
}

#[derive(Debug)]
pub enum ConfigError {
    ParseError(String),
    ValidationError(String),
    FileNotFound(String),
    PermissionDenied(String),
    InvalidFormat(String),
}

#[derive(Debug)]
pub enum ServerError {
    BindError(String),
    AuthenticationError(String),
    AuthorizationError(String),
    RequestError(String),
    InternalError(String),
}

// Implement Display and Error traits for all error types
impl fmt::Display for FrameworkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameworkError::Runner(e) => write!(f, "Runner error: {}", e),
            FrameworkError::Analyzer(e) => write!(f, "Analyzer error: {}", e),
            FrameworkError::Storage(e) => write!(f, "Storage error: {}", e),
            FrameworkError::Stream(e) => write!(f, "Stream error: {}", e),
            FrameworkError::Config(e) => write!(f, "Configuration error: {}", e),
            FrameworkError::Server(e) => write!(f, "Server error: {}", e),
        }
    }
}

impl fmt::Display for RunnerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RunnerError::StartupFailed(msg) => write!(f, "Startup failed: {}", msg),
            RunnerError::AlreadyRunning => write!(f, "Runner is already running"),
            RunnerError::NotRunning => write!(f, "Runner is not running"),
            RunnerError::ConfigurationError(msg) => write!(f, "Configuration error: {}", msg),
            RunnerError::ExecutionError(msg) => write!(f, "Execution error: {}", msg),
            RunnerError::TimeoutError => write!(f, "Timeout occurred"),
            RunnerError::HealthCheckFailed(msg) => write!(f, "Health check failed: {}", msg),
        }
    }
}

impl fmt::Display for AnalyzerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AnalyzerError::InitializationFailed(msg) => write!(f, "Initialization failed: {}", msg),
            AnalyzerError::ProcessingError(msg) => write!(f, "Processing error: {}", msg),
            AnalyzerError::DependencyError(msg) => write!(f, "Dependency error: {}", msg),
            AnalyzerError::ConfigurationError(msg) => write!(f, "Configuration error: {}", msg),
            AnalyzerError::SchemaValidationError(msg) => write!(f, "Schema validation error: {}", msg),
        }
    }
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StorageError::ConnectionFailed(msg) => write!(f, "Connection failed: {}", msg),
            StorageError::WriteError(msg) => write!(f, "Write error: {}", msg),
            StorageError::ReadError(msg) => write!(f, "Read error: {}", msg),
            StorageError::QueryError(msg) => write!(f, "Query error: {}", msg),
            StorageError::CapacityExceeded => write!(f, "Storage capacity exceeded"),
            StorageError::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
        }
    }
}

impl fmt::Display for StreamError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StreamError::ChannelClosed => write!(f, "Channel is closed"),
            StreamError::BroadcastFailed { failed_count, total_count } => {
                write!(f, "Broadcast failed: {}/{} subscribers", failed_count, total_count)
            }
            StreamError::BackpressureExceeded => write!(f, "Backpressure exceeded"),
            StreamError::FilterError(msg) => write!(f, "Filter error: {}", msg),
        }
    }
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            ConfigError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            ConfigError::FileNotFound(path) => write!(f, "File not found: {}", path),
            ConfigError::PermissionDenied(path) => write!(f, "Permission denied: {}", path),
            ConfigError::InvalidFormat(msg) => write!(f, "Invalid format: {}", msg),
        }
    }
}

impl fmt::Display for ServerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ServerError::BindError(msg) => write!(f, "Bind error: {}", msg),
            ServerError::AuthenticationError(msg) => write!(f, "Authentication error: {}", msg),
            ServerError::AuthorizationError(msg) => write!(f, "Authorization error: {}", msg),
            ServerError::RequestError(msg) => write!(f, "Request error: {}", msg),
            ServerError::InternalError(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl std::error::Error for FrameworkError {}
impl std::error::Error for RunnerError {}
impl std::error::Error for AnalyzerError {}
impl std::error::Error for StorageError {}
impl std::error::Error for StreamError {}
impl std::error::Error for ConfigError {}
impl std::error::Error for ServerError {}

// Conversion traits
impl From<RunnerError> for FrameworkError {
    fn from(error: RunnerError) -> Self {
        FrameworkError::Runner(error)
    }
}

impl From<AnalyzerError> for FrameworkError {
    fn from(error: AnalyzerError) -> Self {
        FrameworkError::Analyzer(error)
    }
}

impl From<StorageError> for FrameworkError {
    fn from(error: StorageError) -> Self {
        FrameworkError::Storage(error)
    }
}

impl From<StreamError> for FrameworkError {
    fn from(error: StreamError) -> Self {
        FrameworkError::Stream(error)
    }
}

impl From<ConfigError> for FrameworkError {
    fn from(error: ConfigError) -> Self {
        FrameworkError::Config(error)
    }
}

impl From<ServerError> for FrameworkError {
    fn from(error: ServerError) -> Self {
        FrameworkError::Server(error)
    }
} 