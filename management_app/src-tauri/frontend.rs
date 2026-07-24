//! Embedded frontend assets via rust-embed

#[derive(rust_embed::Embed)]
#[folder = "frontend/dist"]
pub struct Assets;
