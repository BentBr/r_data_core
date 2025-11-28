#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

pub mod backend;
pub mod in_memory;
pub mod manager;
pub mod redis;

pub use backend::CacheBackend;
pub use in_memory::InMemoryCache;
pub use manager::CacheManager;
pub use redis::RedisCache;
