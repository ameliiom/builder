//! The server: each player's mode, their first block, and the shape it fills when they place the
//! second. Each block goes in as the first one was placed (its state copied), only into a place
//! that may take it (replaceable, nobody standing in it, the player may build there), paid from
//! the inventory first. The last shape can be taken back for a while (Undo).
use crate::kit;
use crate::payloads::{self, Start as StartView};
use crate::perks::Comforts;
use crate::shape::{self, Mode, Pos};
use mod_api::{BlockFace, BlockState, BlockUpdateFlags, Host, HostBlockStates, ItemStack, PlayerRef, ServerHost, Text, VarInt};
#[allow(unused_imports)]
use mod_api::prelude::*;
use std::collections::BTreeMap;
use std::sync::Mutex;

/// Blocks placed per player per tick.
const PER_TICK: usize = 64;
/// How long the last shape can be taken back, in ticks (10 seconds).
const UNDO_TICKS: u64 = 200;

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
    /// Where blocks went, for Undo.
    placed: Vec<Pos>,
}

/// The last shape a player finished: what Undo takes back.
struct Done {
    state: BlockState,
    item: String,
    placed: Vec<Pos>,
    at: u64,
}

#[derive(Default)]
struct State {
    modes: BTreeMap<u128, Mode>,
    starts: BTreeMap<u128, Start>,
    jobs: Vec<Job>,
    done: BTreeMap<u128, Done>,
    ticks: u64,
}

static STATE: Mutex<Option<State>> = Mutex::new(None);

fn with<R>(f: impl FnOnce(&mut State) -> R) -> R {
    f(STATE.lock().unwrap_or_else(|e| e.into_inner()).get_or_insert_with(State::default))
}

pub fn comforts(host: &dyn ServerHost, p: PlayerRef) -> Comforts {
    kit::comforts::read(host, crate::MOD_ID, p, Comforts::of)
}

/// Tells the client the shape waiting for its second block (or none), for its preview.
fn tell_start(host: &mut dyn ServerHost, p: PlayerRef, start: Option<(Pos, Mode, usize)>) {
    let Some(token) = payloads::START.get() else { return };
    let view = match start {
        Some((at, mode, size)) => StartView {
            at: at.iter().map(|v| VarInt(*v)).collect(),
            mode: VarInt(Mode::ALL.iter().position(|m| *m == mode).unwrap_or(0) as i32),
            size: VarInt(size as i32),
        },
        None => StartView { at: Vec::new(), mode: VarInt(0), size: VarInt(0) },
    };
    if let Err(e) = token.send(host, p, &view) {
        eprintln!("[builder] sending the preview: {e:?}");
    }
}

/// The player pressed the mode key: the next mode they have, named in the action bar.
pub fn next_mode(host: &mut dyn ServerHost, p: PlayerRef) {
    let c = comforts(&*host, p);
    let mode = with(|s| {
        let m = s.modes.entry(p.uuid()).or_default();
        *m = c.next_mode(*m);
        s.starts.remove(&p.uuid());
        *m
    });
    tell_start(host, p, None);
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
    let size = comforts(&*host, p).size;
    let start = with(|s| s.starts.remove(&p.uuid()));
    let Some(start) = start else {
        with(|s| s.starts.insert(p.uuid(), Start { pos, state }));
        tell_start(host, p, Some((pos, mode, size)));
        return;
    };
    tell_start(host, p, None);
    let left: Vec<Pos> = shape::fill(mode, start.pos, pos, size).into_iter().filter(|q| *q != start.pos && *q != pos).collect();
    with(|s| s.jobs.push(Job { player: p.uuid(), state: start.state, item, left, placed: Vec::new() }));
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
    let mut jobs = with(|s| {
        s.ticks += 1;
        std::mem::take(&mut s.jobs)
    });
    jobs.retain_mut(|job| {
        let going = step(host, job);
        if !going {
            // Each block a shape placed counts toward the Building tree.
            if let Some(p) = host.players().server_players().into_iter().find(|p| p.uuid() == job.player)
                && !job.placed.is_empty()
                && !p.award_stat(host, mod_api::StatType::Custom, &format!("{}:{}", crate::MOD_ID, crate::PLACED), job.placed.len() as i32)
            {
                eprintln!("[builder] could not award the placed statistic");
            }
            let now = with(|s| s.ticks);
            let done = Done { state: job.state, item: job.item.clone(), placed: std::mem::take(&mut job.placed), at: now };
            with(|s| s.done.insert(job.player, done));
        }
        going
    });
    with(|s| {
        let newer = std::mem::take(&mut s.jobs);
        s.jobs = jobs;
        s.jobs.extend(newer);
    });
}

/// Whether a living thing stands in the block at `pos`.
fn occupied(host: &dyn Host, level: mod_api::LevelRef, pos: Pos) -> bool {
    let min = [f64::from(pos[0]), f64::from(pos[1]), f64::from(pos[2])];
    let max = [min[0] + 1.0, min[1] + 1.0, min[2] + 1.0];
    level.entities_in(host, min, max).unwrap_or_default().into_iter().any(|e| host.living(e).is_some())
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
        if !free || occupied(&*host, level, pos) {
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
        if p.place_block(host, level, pos, job.state, true).is_ok() {
            job.placed.push(pos);
        } else if let Some((slot, stack)) = payment {
            let _ = p.set_inventory_slot(host, slot, Some(stack));
        }
    }
    !job.left.is_empty()
}

/// Undo: the player's last shape, within 10 seconds, comes back into their inventory (each block
/// still standing as it was placed).
pub fn undo(host: &mut dyn ServerHost, p: PlayerRef) {
    if !comforts(&*host, p).undo {
        return;
    }
    let Some(done) = with(|s| {
        let fresh = s.done.get(&p.uuid()).is_some_and(|d| s.ticks - d.at <= UNDO_TICKS);
        if fresh { s.done.remove(&p.uuid()) } else { None }
    }) else {
        let _ = p.action_bar(host, Text::translate("builder.undo.none", Vec::new()));
        return;
    };
    let (Some(level), Some(air)) = (p.level(&*host), host.block_state_id("minecraft:air")) else { return };
    let mut back = 0;
    for pos in done.placed.iter().rev() {
        if level.block_state(&*host, *pos) == Some(done.state)
            && level.set_block(host, *pos, air, BlockUpdateFlags::NEIGHBORS | BlockUpdateFlags::CLIENTS).is_ok_and(|w| w)
        {
            back += 1;
        }
    }
    let creative = p.is_creative(&*host) == Some(true);
    if back > 0
        && !creative
        && let Some(stack) = mod_api::Identifier::parse(&done.item).ok().and_then(|k| host.item_key(&k)).and_then(|k| ItemStack::new(k, 1))
    {
        let at = done.placed[0];
        let max = host.max_stack_size(stack.item()).unwrap_or(64);
        let mut left: u32 = back;
        let mut stacks = Vec::new();
        while left > 0 {
            let n = left.min(max);
            if let Some(s) = mod_api::ItemResource::of(&stack).to_stack(n) {
                stacks.push(s);
            }
            left -= n;
        }
        kit::inventory::give(host, crate::MOD_ID, p, level, at, stacks);
    }
    let _ = p.action_bar(host, Text::translate("builder.undo.done", vec![Text::literal(back.to_string())]));
}

pub fn forget(player: u128) {
    with(|s| {
        s.modes.remove(&player);
        s.starts.remove(&player);
        s.jobs.retain(|j| j.player != player);
        s.done.remove(&player);
    });
}
