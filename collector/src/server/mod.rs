//! Web server module for serving embedded frontend assets
//! 
//! This module provides functionality to embed the Next.js frontend build artifacts
//! directly into the Rust binary and serve them via HTTP.

pub mod assets;
pub mod web;

pub use assets::FrontendAssets;
pub use web::WebServer;