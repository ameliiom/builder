//! Builder: place a block, then another further on, and a line, a wall or a floor of it fills the
//! space between. See `docs/design.md`.
//!
//! - **Shapes** (`shape.rs`): the positions between two blocks, by mode, on plain values.
//! - **The server** (`server.rs`): modes, first blocks, and the filling, paid and checked.
pub mod shape;
mod server;

use mod_api::{
    ClientHost, ClientTickPreEvent, EntityPlaceEvent, Flow, Hooks, IPayloadContext, KeyRegistration, PayloadCodec, PayloadId, PlayerLoggedOutEvent,
    PlayerRef, Registrar, RegistrationContext, ServerHost, ServerTickEvent, ServerTickPhase, ToServer,
};
#[allow(unused_imports)]
use mod_api::prelude::*;
use std::sync::OnceLock;

pub(crate) const MOD_ID: &str = "builder";
/// The mode key: Z, free in vanilla and the other mods (B opens Horizon's emote wheel, Y a mod screen;
/// measured 2026-11-25).
const MODE_KEY: &str = "mode";

/// Client → server: the mode key was pressed.
#[derive(PayloadCodec, Clone, Debug)]
pub struct NextMode {
    pub presses: mod_api::VarInt,
}

static NEXT_MODE: OnceLock<ToServer<NextMode>> = OnceLock::new();

fn on_next_mode(request: NextMode, cx: &mut dyn IPayloadContext) -> Result<(), String> {
    let p: PlayerRef = cx.player().map_err(|e| format!("{e:?}"))?;
    let host = cx.server_host().map_err(|e| format!("{e:?}"))?;
    for _ in 0..request.presses.0.clamp(0, 4) {
        server::next_mode(host, p);
    }
    Ok(())
}

#[derive(Default)]
pub struct Mod;

#[mod_api::horizon_mod]
impl Hooks for Mod {
    fn register(&self, r: &mut dyn Registrar, _cx: &RegistrationContext) {
        let mut key = KeyRegistration::new(MODE_KEY, "key.keyboard.z");
        key.label = "key.builder.mode".into();
        r.register_key_mapping(key);
        let mut handlers = r.payload_handlers();
        let mut v1 = handlers.registrar("1").optional();
        let _ = NEXT_MODE.set(v1.play_to_server_typed(PayloadId("builder:next_mode".into()), on_next_mode));
    }

    fn client_tick_pre(&self, host: &mut dyn ClientHost, _: &mut ClientTickPreEvent) {
        let mut presses = 0;
        while host.consume_click(MOD_ID, MODE_KEY) {
            presses += 1;
        }
        if presses > 0
            && let Some(token) = NEXT_MODE.get()
            && let Err(e) = token.send(host, &NextMode { presses: mod_api::VarInt(presses) })
        {
            eprintln!("[builder] sending the mode: {e:?}");
        }
    }

    /// A player placed a block: the first of a shape, or the second, which fills it.
    fn entity_place(&self, host: &mut dyn ServerHost, event: &mut EntityPlaceEvent) -> Flow {
        server::placed(host, [event.pos.x, event.pos.y, event.pos.z], event.state);
        Flow::Continue
    }

    fn server_tick(&self, host: &mut dyn ServerHost, event: &mut ServerTickEvent<'_>) {
        if matches!(event.phase, ServerTickPhase::ServerTickEnd { .. }) {
            server::tick(host);
        }
    }

    fn player_logged_out(&self, _: &mut dyn ServerHost, event: &mut PlayerLoggedOutEvent<'_>) {
        server::forget(event.player);
    }
}
