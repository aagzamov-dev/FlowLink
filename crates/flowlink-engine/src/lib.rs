pub mod discovery;
pub mod geometry;
pub mod input;
pub mod model;
pub mod pairing;
pub mod platform;
pub mod protocol;
pub mod runtime;
pub mod storage;
pub mod transport;

pub use runtime::Engine;
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
