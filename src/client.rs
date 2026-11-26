//! The client: while a shape waits for its second block, the blocks it would fill are outlined as
//! the game outlines the block you aim at, from the first block to where your next one would go.
use crate::kit;
use crate::shape::{self, Mode, Pos};
use mod_api::{ClientHost, CrosshairTarget, Host, HostBlockStates, RenderLevelStage, RenderLevelStageEvent};
#[allow(unused_imports)]
use mod_api::prelude::*;
use std::sync::Mutex;

/// The waiting shape: first block, mode, size.
static START: Mutex<Option<(Pos, Mode, usize)>> = Mutex::new(None);

pub fn start(at: Option<(Pos, Mode, usize)>) {
    *START.lock().unwrap_or_else(|e| e.into_inner()) = at;
}

/// Where a block placed on the aimed face would go: the aimed block itself when it is
/// replaceable (grass, snow), else the block beside the face.
fn next_block(host: &dyn ClientHost) -> Option<Pos> {
    let Some(CrosshairTarget::Block { pos, state, hit }) = host.crosshair_target() else { return None };
    let read: &dyn Host = host;
    let p = [pos.x, pos.y, pos.z];
    if read.block_states().replaceable(state) == Some(true) {
        return Some(p);
    }
    Some([p[0] + hit.face[0], p[1] + hit.face[1], p[2] + hit.face[2]])
}

pub fn draw(host: &dyn ClientHost, event: &mut RenderLevelStageEvent<'_>) {
    if !matches!(event.stage, RenderLevelStage::AfterTranslucentBlocks) {
        return;
    }
    let Some((first, mode, size)) = *START.lock().unwrap_or_else(|e| e.into_inner()) else { return };
    let Some(end) = next_block(host) else { return };
    let lines = kit::outline::lines(shape::fill(mode, first, end, size).into_iter().filter(|p| *p != first));
    if !lines.is_empty()
        && let Err(e) = event.draw.lines(&lines)
    {
        eprintln!("[builder] drawing the preview: {e:?}");
    }
}
