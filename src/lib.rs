//! Builder: place a block, then another further on, and a line, a wall or a floor of it fills the
//! space between, previewed in the game's own outline; the bigger comforts are earned in the Skills
//! tree Building (nodes Builder adds to it). See `docs/design.md`.
//!
//! - **Shapes** (`shape.rs`): the positions between two blocks, by mode, on plain values.
//! - **The comforts** (`perks.rs`): the tags the tree gives, read with no code tie to Skills.
//! - **The server** (`server.rs`): modes, first blocks, the filling paid and checked, undo.
//! - **The client** (`client.rs`): the preview while a shape waits for its second block.
mod bag;
mod client;
mod payloads;
pub mod perks;
mod server;
pub mod shape;

use mod_api::{
    ClientHost, ClientTickPreEvent, EntityPlaceEvent, Flow, Hooks, IPayloadContext, KeyRegistration, PayloadId, PlayerLoggedOutEvent, PlayerRef,
    Registrar, RegistrationContext, RenderLevelStageEvent, ServerHost, ServerTickEvent, ServerTickPhase, SettingSpec, VarInt,
};
#[allow(unused_imports)]
use mod_api::prelude::*;

pub(crate) const MOD_ID: &str = "builder";
/// `builder:placed`: every block a shape places (Building experience Skills can't see: `place_block` fires no place event).
pub(crate) const PLACED: &str = "placed";
/// The mode key: Z, free in vanilla and the other mods (B opens Horizon's emote wheel, Y a mod screen;
/// measured 2026-11-25). Undo: U, free as well.
const MODE_KEY: &str = "mode";
const UNDO_KEY: &str = "undo";

/// Settings (Options > Mods > Builder).
pub(crate) mod setting {
    /// Opens every comfort for a world that doesn't use Skills.
    pub const ALL_COMFORTS: &str = "all_comforts";
}

fn on_key(request: payloads::Key, cx: &mut dyn IPayloadContext) -> Result<(), String> {
    let p: PlayerRef = cx.player().map_err(|e| format!("{e:?}"))?;
    let host = cx.server_host().map_err(|e| format!("{e:?}"))?;
    if request.undo {
        server::undo(host, p);
    }
    for _ in 0..request.presses.0.clamp(0, 4) {
        server::next_mode(host, p);
    }
    Ok(())
}

fn on_start(view: payloads::Start, _: &mut dyn IPayloadContext) -> Result<(), String> {
    let mode = shape::Mode::ALL.get(view.mode.0.max(0) as usize).copied().unwrap_or_default();
    let start = match view.at.as_slice() {
        [x, y, z] if mode != shape::Mode::Off => Some(([x.0, y.0, z.0], mode, view.size.0.max(0) as usize)),
        _ => None,
    };
    client::start(start);
    Ok(())
}

#[derive(Default)]
pub struct Mod;

#[mod_api::horizon_mod]
impl Hooks for Mod {
    fn register(&self, r: &mut dyn Registrar, _cx: &RegistrationContext) {
        for (name, default, label) in [(MODE_KEY, "key.keyboard.z", "key.builder.mode"), (UNDO_KEY, "key.keyboard.u", "key.builder.undo")] {
            let mut key = KeyRegistration::new(name, default);
            key.label = label.into();
            r.register_key_mapping(key);
        }
        if let Err(e) = r.register_custom_stat(PLACED) {
            eprintln!("[builder] declaring statistic {PLACED}: {e}");
        }
        if let Err(e) = r.register_setting(SettingSpec::toggle(setting::ALL_COMFORTS, "builder.config.all_comforts", false)) {
            eprintln!("[builder] setting: {e:?}");
        }
        let mut handlers = r.payload_handlers();
        let mut v1 = handlers.registrar("1").optional();
        let _ = payloads::KEY.set(v1.play_to_server_typed(PayloadId("builder:key".into()), on_key));
        let _ = payloads::START.set(v1.play_to_client_typed(PayloadId("builder:start".into()), on_start));
    }

    fn client_tick_pre(&self, host: &mut dyn ClientHost, _: &mut ClientTickPreEvent) {
        let mut presses = 0;
        while host.consume_click(MOD_ID, MODE_KEY) {
            presses += 1;
        }
        let mut undo = false;
        while host.consume_click(MOD_ID, UNDO_KEY) {
            undo = true;
        }
        if (presses > 0 || undo)
            && let Some(token) = payloads::KEY.get()
            && let Err(e) = token.send(host, &payloads::Key { presses: VarInt(presses), undo })
        {
            eprintln!("[builder] sending a key: {e:?}");
        }
    }

    /// The preview of the shape waiting for its second block.
    fn render_level_stage(&self, host: &dyn ClientHost, event: &mut RenderLevelStageEvent<'_>) {
        client::draw(host, event);
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
