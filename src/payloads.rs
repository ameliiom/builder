//! What crosses the network. Registered (version "1", optional) in `register`.
use mod_api::{PayloadCodec, ToClient, ToServer, VarInt};
use std::sync::OnceLock;

/// Client → server: the mode key (`presses` times), or the undo key.
#[derive(PayloadCodec, Clone, Debug)]
pub struct Key {
    pub presses: VarInt,
    pub undo: bool,
}

/// Server → client: the shape waiting for its second block, for the preview: its first block
/// `at` (x, y, z), its mode (`Mode` index, 0 for none waiting) and its size.
#[derive(PayloadCodec, Clone, Debug, PartialEq)]
pub struct Start {
    pub at: Vec<VarInt>,
    pub mode: VarInt,
    pub size: VarInt,
}

pub static KEY: OnceLock<ToServer<Key>> = OnceLock::new();
pub static START: OnceLock<ToClient<Start>> = OnceLock::new();
