pub mod admin;
pub mod client;
pub mod errors;
pub mod types;

mod ingest;
mod metadata;
mod sse;

pub use client::{Client, ClientBuilder, ExecuteOptions};
pub use errors::{Error, ExecuteError};
pub use types::{
    BreakerMeta, BreakerStateValue, BreakerStatus, MetricValue, ReportInput, RouterMeta, SdkStats,
    Status,
};

/// The contract version this SDK implements.
pub const CONTRACT_VERSION: &str = "0.2";
