//! The server: each player's mode, their first block, and the shape it fills when they place the
//! second. Each block goes in as the first one was placed (its state copied), only into a place
//! that may take it (replaceable, the player may build there), paid from the inventory first.
use crate::shape::{self, Mode, Pos};
use mod_api::{BlockFace, BlockState, Host, HostBlockStates, ItemStack, PlayerRef, ServerHost, Text};
#[allow(unused_imports)]
use mod_api::prelude::*;
use std::collections::BTreeMap;
use std::sync::Mutex;

/// The most a shape holds in the skeleton (the Skills tree sets it in the full build).
const CAP: usize = 64;
/// Blocks placed per player per tick.
const PER_TICK: usize = 64;

struct Start {
    pos: Pos,
    state: BlockState,
}

struct Job {
    player: u128,
    state: BlockState,
    /// The item that pays for each block.
    item: String,
    left: Vec<Pos>,
}

#[derive(Default)]
struct State {
    modes: BTreeMap<u128, Mode>,
    starts: BTreeMap<u128, Start>,
    jobs: Vec<Job>,
}

static STATE: Mutex<Option<State>> = Mutex::new(None);

fn with<R>(f: impl FnOnce(&mut State) -> R) -> R {
    f(STATE.lock().unwrap_or_else(|e| e.into_inner()).get_or_insert_with(State::default))
}

/// The player pressed the mode key: the next mode, named in the action bar.
pub fn next_mode(host: &mut dyn ServerHost, p: PlayerRef) {
    let mode = with(|s| {
        let m = s.modes.entry(p.uuid()).or_default();
        *m = m.next();
        s.starts.remove(&p.uuid());
        *m
    });
    let _ = p.action_bar(host, Text::translate(format!("builder.mode.{}", mode.key()), Vec::new()));
}

/// The held item's id, from the hand slot.
fn held_item(host: &dyn Host, p: PlayerRef) -> Option<String> {
    let slot = p.selected_slot(host)?;
    p.inventory_slots(host)?.into_iter().find(|(s, st)| *s as usize == slot && st.count() > 0).map(|(_, st)| st.item().id(host))
}

/// A player placed `state` at `pos`: the first block of a shape is remembered, the second fills it.
pub fn placed(host: &mut dyn ServerHost, pos: Pos, state: BlockState) {
    let Some(p) = host.players().acting_player() else { return };
    let mode = with(|s| s.modes.get(&p.uuid()).copied().unwrap_or_default());
    if mode == Mode::Off {
        return;
    }
    let Some(item) = held_item(&*host, p) else { return };
    let start = with(|s| s.starts.remove(&p.uuid()));
    let Some(start) = start else {
        with(|s| s.starts.insert(p.uuid(), Start { pos, state }));
        return;
    };
    let left: Vec<Pos> = shape::fill(mode, start.pos, pos, CAP).into_iter().filter(|q| *q != start.pos && *q != pos).collect();
    with(|s| s.jobs.push(Job { player: p.uuid(), state: start.state, item, left }));
}

/// A stack of `item` to pay with: the hand first, then the rest of the inventory.
fn payer(host: &dyn Host, p: PlayerRef, item: &str) -> Option<(u32, ItemStack)> {
    let hand = p.selected_slot(host)? as u32;
    let mut slots: Vec<(u32, ItemStack)> =
        p.inventory_slots(host)?.into_iter().filter(|(s, st)| *s < 36 && st.count() > 0 && st.item().id(host) == item).collect();
    slots.sort_by_key(|(s, _)| (*s != hand, *s));
    slots.into_iter().next()
}

pub fn tick(host: &mut dyn ServerHost) {
    let mut jobs = with(|s| std::mem::take(&mut s.jobs));
    jobs.retain_mut(|job| step(host, job));
    with(|s| {
        let newer = std::mem::take(&mut s.jobs);
        s.jobs = jobs;
        s.jobs.extend(newer);
    });
}

/// One tick of a job: false when it is over (done, out of blocks, or its player gone).
fn step(host: &mut dyn ServerHost, job: &mut Job) -> bool {
    let Some(p) = host.players().server_players().into_iter().find(|p| p.uuid() == job.player) else { return false };
    let Some(level) = p.level(&*host) else { return false };
    let creative = p.is_creative(&*host) == Some(true);
    for _ in 0..PER_TICK {
        let Some(pos) = job.left.first().copied() else { return false };
        job.left.remove(0);
        let free = level.block_state(&*host, pos).and_then(|s| host.block_states().replaceable(s)) == Some(true);
        if !free {
            continue;
        }
        let payment = if creative { None } else { payer(&*host, p, &job.item) };
        if !creative && payment.is_none() {
            return false;
        }
        if let Some((_, stack)) = &payment
            && p.may_use_item_at(&*host, pos, BlockFace::Up, stack) != Some(true)
        {
            continue;
        }
        // Paid before the block goes in; given back if the write is refused.
        if let Some((slot, stack)) = &payment
            && p.set_inventory_slot(host, *slot, stack.clone().shrink(1)).is_err()
        {
            return false;
        }
        if p.place_block(host, level, pos, job.state, true).is_err()
            && let Some((slot, stack)) = payment
        {
            let _ = p.set_inventory_slot(host, slot, Some(stack));
        }
    }
    !job.left.is_empty()
}

pub fn forget(player: u128) {
    with(|s| {
        s.modes.remove(&player);
        s.starts.remove(&player);
        s.jobs.retain(|j| j.player != player);
    });
}
