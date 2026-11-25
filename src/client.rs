//! The client: while a shape waits for its second block, the blocks it would fill are outlined as
//! the game outlines the block you aim at, from the first block to where your next one would go.
use crate::shape::{self, Mode, Pos};
use mod_api::{ClientHost, CrosshairTarget, Host, HostBlockStates, RenderLevelStage, RenderLevelStageEvent, WorldDepth, WorldLine};
#[allow(unused_imports)]
use mod_api::prelude::*;
use std::sync::Mutex;

const COLOUR: u32 = 0x6600_0000;
const WIDTH: f32 = 2.0;

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

/// The twelve edges of the block at `p`, a hair outside it.
fn edges([x, y, z]: Pos) -> impl Iterator<Item = WorldLine> {
    let (lo, hi) = (-0.002, 1.002);
    let c = move |i: f64, j: f64, k: f64| [f64::from(x) + i, f64::from(y) + j, f64::from(z) + k];
    let corners = [(lo, lo), (hi, lo), (hi, hi), (lo, hi)];
    (0..4).flat_map(move |n| {
        let (a, b) = (corners[n], corners[(n + 1) % 4]);
        [
            WorldLine::new(c(a.0, lo, a.1), c(b.0, lo, b.1), COLOUR, WIDTH, WorldDepth::Tested),
            WorldLine::new(c(a.0, hi, a.1), c(b.0, hi, b.1), COLOUR, WIDTH, WorldDepth::Tested),
            WorldLine::new(c(a.0, lo, a.1), c(a.0, hi, a.1), COLOUR, WIDTH, WorldDepth::Tested),
        ]
    })
}

pub fn draw(host: &dyn ClientHost, event: &mut RenderLevelStageEvent<'_>) {
    if !matches!(event.stage, RenderLevelStage::AfterTranslucentBlocks) {
        return;
    }
    let Some((first, mode, size)) = *START.lock().unwrap_or_else(|e| e.into_inner()) else { return };
    let Some(end) = next_block(host) else { return };
    let lines: Vec<WorldLine> = shape::fill(mode, first, end, size).into_iter().filter(|p| *p != first).flat_map(edges).collect();
    if !lines.is_empty()
        && let Err(e) = event.draw.lines(&lines)
    {
        eprintln!("[builder] drawing the preview: {e:?}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_block_has_twelve_edges() {
        assert_eq!(edges([0, 0, 0]).count(), 12);
    }
}
