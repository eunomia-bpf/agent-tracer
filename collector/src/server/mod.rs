//! Web server module for serving embedded frontend assets
//! 
//! This module provides functionality to embed the Next.js frontend build artifacts
//! directly into the Rust binary and serve them via HTTP.

pub mod assets;
pub mod web;

#[cfg(test)]
mod test_assets;
#[cfg(test)]
mod test_web;
#[cfg(test)]
mod test_integration;

pub use web::WebServer;