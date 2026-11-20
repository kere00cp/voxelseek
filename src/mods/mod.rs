//! Mod interface: sandboxed [Rhai](https://rhai.rs) scripts.
//!
//! Mods live in `mods/<id>/` next to the game executable (or the working
//! directory when running via cargo):
//!
//! ```text
//! mods/
//! └── hello/
//!     ├── mod.toml    # metadata: id, name, version, author, description
//!     └── main.rhai   # the script
//! ```
//!
//! A mod script can define any of these hook functions:
//!
//! ```rhai
//! fn on_load()                          // called once when the game starts
//! fn on_tick(dt)                        // called every frame (dt in seconds)
//! fn on_block_placed(x, y, z, block)    // a block was placed (player or AI)
//! fn on_block_broken(x, y, z, block)    // a block was broken
//! fn on_command(cmd, args)              // "/cmd a b c" typed in the console (T)
//! ```
//!
//! Scripts call this API:
//!
//! ```rhai
//! log(msg)                                  // write to the game log/console
//! chat(msg)                                 // show a line in the console overlay
//! set_block(x, y, z, "stone")               // queue a block placement
//! fill(x1, y1, z1, x2, y2, z2, "glass")     // queue a box fill (≤ 4096 blocks)
//! give("diamond", 3)                        // give the player items
//! teleport(x, y, z)                         // teleport the player
//! set_time(0.75)                            // set time of day (0..1)
//! time_of_day()                             // current time of day (float)
//! player_pos()                              // [x, y, z] floats
//! get_block(x, y, z)                        // block name near the player
//! ```
//!
//! Scripts are sandboxed: no filesystem, no network, no FFI, a hard
//! operations cap per handler call, and all world mutations go through a
//! validated main-thread queue. Mods can be disabled per-id via the launcher
//! (`disabled_mods` in `voxelseek.toml`).

use bevy::prelude::*;
use rhai::{AST, Engine, Scope};
use serde::Deserialize;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::block::BlockType;
use crate::harness::tools;
use crate::harness::{ChatLine, HarnessConsole};
use crate::inventory::inventory::Inventory;
use crate::lighting::day_night::DayNightCycle;
use crate::player::Player;
use crate::settings::GameSettings;
use crate::world::chunk::CHUNK_SIZE;
use crate::world::manager::{ChunkDataStore, ChunkManager};

/// Directory scanned for mods.
pub const MODS_DIR: &str = "mods";
/// Maximum blocks a scripted `fill` may touch.
const MAX_FILL_VOLUME: i64 = 4096;
/// Radius of the read-only world snapshot around the player.
const SNAPSHOT_RADIUS: i32 = 16;
/// Operations budget for a single Rhai handler call.
const MAX_OPERATIONS: u64 = 500_000;

// ---------------------------------------------------------------------------
// Game events consumed by mods
// ---------------------------------------------------------------------------

/// Fired when a block is placed in the world (by the player or the harness).
#[derive(Message, Clone, Copy)]
pub struct BlockPlacedMsg {
    pub pos: IVec3,
    pub block: BlockType,
}

/// Fired when a block is broken in the world.
#[derive(Message, Clone, Copy)]
pub struct BlockBrokenMsg {
    pub pos: IVec3,
    pub block: BlockType,
}

// ---------------------------------------------------------------------------
// Script ↔ engine communication
// ---------------------------------------------------------------------------

/// World mutations queued by scripts, applied on the main thread.
#[derive(Debug)]
enum ModOp {
    SetBlock {
        pos: IVec3,
        block: BlockType,
    },
    Fill {
        min: IVec3,
        max: IVec3,
        block: BlockType,
    },
    Give {
        item: crate::inventory::item::Item,
        count: usize,
    },
    Teleport {
        pos: Vec3,
    },
    SetTime(f32),
    Chat(String),
    Log(String),
}

type SharedOutbox = Arc<Mutex<Vec<ModOp>>>;

/// Read-only snapshot of the area around the player, refreshed every frame.
struct Snapshot {
    center: IVec3,
    player_pos: Vec3,
    time_of_day: f32,
    /// `(2r+1)^3` block ids, indexed [y][z][x] relative to `center`.
    blocks: Vec<u8>,
    valid: bool,
}

impl Default for Snapshot {
    fn default() -> Self {
        Self {
            center: IVec3::ZERO,
            player_pos: Vec3::ZERO,
            time_of_day: 0.25,
            blocks: vec![0; ((2 * SNAPSHOT_RADIUS + 1) as usize).pow(3)],
            valid: false,
        }
    }
}

impl Snapshot {
    fn get_block(&self, x: i32, y: i32, z: i32) -> Option<BlockType> {
        if !self.valid {
            return None;
        }
        let r = SNAPSHOT_RADIUS;
        let (dx, dy, dz) = (x - self.center.x, y - self.center.y, z - self.center.z);
        if dx.abs() > r || dy.abs() > r || dz.abs() > r {
            return None;
        }
        let side = (2 * r + 1) as usize;
        let idx =
            ((dy + r) as usize) * side * side + ((dz + r) as usize) * side + (dx + r) as usize;
        self.blocks.get(idx).map(|&b| BlockType::from_id(b))
    }
}

type SharedSnapshot = Arc<Mutex<Snapshot>>;

// ---------------------------------------------------------------------------
// Loaded mod & runtime resource
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct ModMeta {
    pub id: String,
    pub name: Option<String>,
    pub version: Option<String>,
    pub author: Option<String>,
    pub description: Option<String>,
}

struct LoadedMod {
    meta: ModMeta,
    engine: Engine,
    ast: AST,
    scope: Scope<'static>,
    on_tick: bool,
    on_block_placed: bool,
    on_block_broken: bool,
    on_command: bool,
}

/// Bevy resource holding all loaded mods.
#[derive(Resource)]
pub struct ModRuntime {
    mods: Vec<LoadedMod>,
    outbox: SharedOutbox,
    snapshot: SharedSnapshot,
    /// `/command args...` lines typed in the console, awaiting dispatch.
    command_queue: VecDeque<(String, Vec<String>)>,
}

impl Default for ModRuntime {
    fn default() -> Self {
        Self {
            mods: Vec::new(),
            outbox: Arc::new(Mutex::new(Vec::new())),
            snapshot: Arc::new(Mutex::new(Snapshot::default())),
            command_queue: VecDeque::new(),
        }
    }
}

impl ModRuntime {
    /// Queue a console command (called by the harness console on "/cmd ...").
    pub fn queue_command(&mut self, name: String, args: Vec<String>) {
        self.command_queue.push_back((name, args));
    }

    /// Number of currently loaded (enabled) mods.
    pub fn loaded_count(&self) -> usize {
        self.mods.len()
    }

    /// Names of loaded mods, for display.
    pub fn loaded_names(&self) -> Vec<String> {
        self.mods
            .iter()
            .map(|m| m.meta.name.clone().unwrap_or_else(|| m.meta.id.clone()))
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct ModsPlugin;

impl Plugin for ModsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ModRuntime>()
            .add_message::<BlockPlacedMsg>()
            .add_message::<BlockBrokenMsg>()
            .add_systems(Startup, load_mods)
            .add_systems(Update, (update_snapshot, mod_runtime_system).chain());
    }
}

// ---------------------------------------------------------------------------
// Script engine construction
// ---------------------------------------------------------------------------

fn build_engine(outbox: SharedOutbox, snapshot: SharedSnapshot) -> Engine {
    let mut engine = Engine::new();
    engine.set_max_operations(MAX_OPERATIONS);
    engine.set_max_call_levels(32);

    {
        let ob = outbox.clone();
        engine.register_fn("log", move |msg: &str| {
            ob.lock().unwrap().push(ModOp::Log(msg.to_string()));
        });
    }
    {
        let ob = outbox.clone();
        engine.register_fn("chat", move |msg: &str| {
            ob.lock().unwrap().push(ModOp::Chat(msg.to_string()));
        });
    }
    {
        let ob = outbox.clone();
        engine.register_fn(
            "set_block",
            move |x: i64, y: i64, z: i64, block: &str| -> bool {
                match tools::block_from_name(block) {
                    Some(b)
                        if !matches!(b, BlockType::Air | BlockType::Water | BlockType::Bedrock) =>
                    {
                        ob.lock().unwrap().push(ModOp::SetBlock {
                            pos: IVec3::new(x as i32, y as i32, z as i32),
                            block: b,
                        });
                        true
                    }
                    _ => false,
                }
            },
        );
    }
    {
        let ob = outbox.clone();
        engine.register_fn(
            "fill",
            move |x1: i64, y1: i64, z1: i64, x2: i64, y2: i64, z2: i64, block: &str| -> i64 {
                let Some(b) = tools::block_from_name(block) else {
                    return -1;
                };
                let (min, max) = (
                    IVec3::new(
                        (x1 as i32).min(x2 as i32),
                        (y1 as i32).min(y2 as i32),
                        (z1 as i32).min(z2 as i32),
                    ),
                    IVec3::new(
                        (x1 as i32).max(x2 as i32),
                        (y1 as i32).max(y2 as i32),
                        (z1 as i32).max(z2 as i32),
                    ),
                );
                let size = (max - min + IVec3::ONE).as_i64vec3();
                let volume = size.x * size.y * size.z;
                if volume > MAX_FILL_VOLUME {
                    return -2;
                }
                ob.lock().unwrap().push(ModOp::Fill { min, max, block: b });
                volume
            },
        );
    }
    {
        let ob = outbox.clone();
        engine.register_fn("give", move |item: &str, count: i64| -> i64 {
            match tools::item_from_name(item) {
                Some(it) => {
                    let n = count.clamp(1, 64) as usize;
                    ob.lock().unwrap().push(ModOp::Give { item: it, count: n });
                    n as i64
                }
                None => -1,
            }
        });
    }
    {
        let ob = outbox.clone();
        engine.register_fn("teleport", move |x: f64, y: f64, z: f64| {
            ob.lock().unwrap().push(ModOp::Teleport {
                pos: Vec3::new(x as f32, y as f32, z as f32),
            });
        });
    }
    {
        let ob = outbox.clone();
        engine.register_fn("set_time", move |t: f64| {
            ob.lock()
                .unwrap()
                .push(ModOp::SetTime(t.rem_euclid(1.0) as f32));
        });
    }
    {
        let sn = snapshot.clone();
        engine.register_fn("time_of_day", move || -> f64 {
            sn.lock().unwrap().time_of_day as f64
        });
    }
    {
        let sn = snapshot.clone();
        engine.register_fn("player_pos", move || -> Vec<rhai::Dynamic> {
            let s = sn.lock().unwrap();
            vec![
                (s.player_pos.x as f64).into(),
                (s.player_pos.y as f64).into(),
                (s.player_pos.z as f64).into(),
            ]
        });
    }
    {
        let sn = snapshot.clone();
        engine.register_fn("get_block", move |x: i64, y: i64, z: i64| -> String {
            let s = sn.lock().unwrap();
            match s.get_block(x as i32, y as i32, z as i32) {
                Some(b) => tools::block_name(b).to_string(),
                None => "unknown".to_string(),
            }
        });
    }

    engine
}

// ---------------------------------------------------------------------------
// Loading
// ---------------------------------------------------------------------------

fn load_mods(
    mut runtime: ResMut<ModRuntime>,
    settings: Res<GameSettings>,
    mut console: ResMut<HarnessConsole>,
) {
    let mods_root = PathBuf::from(MODS_DIR);
    if !mods_root.is_dir() {
        return;
    }

    let mut entries: Vec<PathBuf> = std::fs::read_dir(&mods_root)
        .map(|rd| rd.filter_map(|e| e.ok().map(|e| e.path())).collect())
        .unwrap_or_default();
    entries.sort();

    for dir in entries {
        if !dir.is_dir() {
            continue;
        }
        let meta_path = dir.join("mod.toml");
        let script_path = dir.join("main.rhai");
        if !meta_path.exists() || !script_path.exists() {
            continue;
        }

        let meta: ModMeta = match std::fs::read_to_string(&meta_path)
            .ok()
            .and_then(|text| toml::from_str(&text).ok())
        {
            Some(m) => m,
            None => {
                console.push(ChatLine::system(format!(
                    "mod at {}: invalid mod.toml, skipped",
                    dir.display()
                )));
                continue;
            }
        };

        if settings.disabled_mods.iter().any(|d| d == &meta.id) {
            console.push(ChatLine::system(format!(
                "mod '{}' is disabled in settings, skipped",
                meta.id
            )));
            continue;
        }

        let source = match std::fs::read_to_string(&script_path) {
            Ok(s) => s,
            Err(e) => {
                console.push(ChatLine::system(format!(
                    "mod '{}': cannot read main.rhai: {e}",
                    meta.id
                )));
                continue;
            }
        };

        let engine = build_engine(runtime.outbox.clone(), runtime.snapshot.clone());
        let ast = match engine.compile(&source) {
            Ok(ast) => ast,
            Err(e) => {
                console.push(ChatLine::system(format!(
                    "mod '{}': script error: {e}",
                    meta.id
                )));
                continue;
            }
        };

        let has = |name: &str, params: usize| {
            ast.iter_functions()
                .any(|f| f.name == name && f.params.len() == params)
        };
        let on_load = has("on_load", 0);

        let mut loaded = LoadedMod {
            on_tick: has("on_tick", 1),
            on_block_placed: has("on_block_placed", 4),
            on_block_broken: has("on_block_broken", 4),
            on_command: has("on_command", 2),
            meta,
            engine,
            ast,
            scope: Scope::new(),
        };

        if on_load {
            if let Err(e) =
                loaded
                    .engine
                    .call_fn::<()>(&mut loaded.scope, &loaded.ast, "on_load", ())
            {
                console.push(ChatLine::system(format!(
                    "mod '{}': on_load failed: {e}",
                    loaded.meta.id
                )));
            }
        }

        console.push(ChatLine::system(format!(
            "mod loaded: {} {}",
            loaded
                .meta
                .name
                .clone()
                .unwrap_or_else(|| loaded.meta.id.clone()),
            loaded.meta.version.clone().unwrap_or_default(),
        )));
        runtime.mods.push(loaded);
    }

    let count = runtime.mods.len();
    if count > 0 {
        info!("loaded {count} mod(s)");
    }
}

// ---------------------------------------------------------------------------
// Per-frame snapshot refresh
// ---------------------------------------------------------------------------

fn update_snapshot(
    runtime: Res<ModRuntime>,
    store: Res<ChunkDataStore>,
    player_q: Query<&Transform, With<Player>>,
    cycle: Res<DayNightCycle>,
) {
    if runtime.mods.is_empty() {
        return;
    }
    let Ok(player_tf) = player_q.single() else {
        return;
    };
    let center = player_tf.translation.floor().as_ivec3();
    let mut snapshot = runtime.snapshot.lock().unwrap();
    snapshot.player_pos = player_tf.translation;
    snapshot.time_of_day = cycle.time_of_day;
    snapshot.center = center;

    let r = SNAPSHOT_RADIUS;
    let side = 2 * r + 1;
    let size = CHUNK_SIZE as i32;
    for dy in -r..=r {
        for dz in -r..=r {
            for dx in -r..=r {
                let (x, y, z) = (center.x + dx, center.y + dy, center.z + dz);
                let chunk_pos =
                    IVec3::new(x.div_euclid(size), y.div_euclid(size), z.div_euclid(size));
                let id = store
                    .chunks
                    .get(&chunk_pos)
                    .map(|c| {
                        c.get(
                            x.rem_euclid(size) as usize,
                            y.rem_euclid(size) as usize,
                            z.rem_euclid(size) as usize,
                        ) as u8
                    })
                    .unwrap_or(0);
                let idx = ((dy + r) as usize) * (side as usize) * (side as usize)
                    + ((dz + r) as usize) * (side as usize)
                    + (dx + r) as usize;
                snapshot.blocks[idx] = id;
            }
        }
    }
    snapshot.valid = true;
}

// ---------------------------------------------------------------------------
// Handler dispatch + outbox draining
// ---------------------------------------------------------------------------

/// Bundled world access for applying scripted operations.
#[derive(bevy::ecs::system::SystemParam)]
struct ModWorld<'w, 's> {
    store: ResMut<'w, ChunkDataStore>,
    manager: Res<'w, ChunkManager>,
    inventory: ResMut<'w, Inventory>,
    cycle: ResMut<'w, DayNightCycle>,
    player_q: Query<'w, 's, &'static mut Transform, With<Player>>,
    commands: Commands<'w, 's>,
}

fn mod_runtime_system(
    time: Res<Time>,
    mut runtime: ResMut<ModRuntime>,
    mut placed: bevy::ecs::message::MessageReader<BlockPlacedMsg>,
    mut broken: bevy::ecs::message::MessageReader<BlockBrokenMsg>,
    mut console: ResMut<HarnessConsole>,
    mut world: ModWorld,
) {
    if runtime.mods.is_empty() {
        // Still drain readers so they don't grow unboundedly.
        placed.clear();
        broken.clear();
        runtime.command_queue.clear();
        return;
    }

    let dt = time.delta_secs() as f64;

    // --- tick handlers ---
    for i in 0..runtime.mods.len() {
        if runtime.mods[i].on_tick {
            let m = &mut runtime.mods[i];
            if let Err(e) = m
                .engine
                .call_fn::<()>(&mut m.scope, &m.ast, "on_tick", (dt,))
            {
                let id = m.meta.id.clone();
                console.push(ChatLine::system(format!("mod '{id}' on_tick: {e}")));
            }
        }
    }

    // --- block events ---
    let placed_events: Vec<BlockPlacedMsg> = placed.read().copied().collect();
    let broken_events: Vec<BlockBrokenMsg> = broken.read().copied().collect();

    for ev in &placed_events {
        let block_name = tools::block_name(ev.block).to_string();
        for i in 0..runtime.mods.len() {
            if runtime.mods[i].on_block_placed {
                let m = &mut runtime.mods[i];
                let args = (
                    ev.pos.x as i64,
                    ev.pos.y as i64,
                    ev.pos.z as i64,
                    block_name.clone(),
                );
                if let Err(e) =
                    m.engine
                        .call_fn::<()>(&mut m.scope, &m.ast, "on_block_placed", args)
                {
                    let id = m.meta.id.clone();
                    console.push(ChatLine::system(format!("mod '{id}' on_block_placed: {e}")));
                }
            }
        }
    }
    for ev in &broken_events {
        let block_name = tools::block_name(ev.block).to_string();
        for i in 0..runtime.mods.len() {
            if runtime.mods[i].on_block_broken {
                let m = &mut runtime.mods[i];
                let args = (
                    ev.pos.x as i64,
                    ev.pos.y as i64,
                    ev.pos.z as i64,
                    block_name.clone(),
                );
                if let Err(e) =
                    m.engine
                        .call_fn::<()>(&mut m.scope, &m.ast, "on_block_broken", args)
                {
                    let id = m.meta.id.clone();
                    console.push(ChatLine::system(format!("mod '{id}' on_block_broken: {e}")));
                }
            }
        }
    }

    // --- console commands ("/cmd args...") ---
    let queued: Vec<(String, Vec<String>)> = runtime.command_queue.drain(..).collect();
    for (cmd, args) in queued {
        let mut handled_any = false;
        for i in 0..runtime.mods.len() {
            if runtime.mods[i].on_command {
                handled_any = true;
                let m = &mut runtime.mods[i];
                let dyn_args: Vec<rhai::Dynamic> = args
                    .iter()
                    .map(|a| rhai::Dynamic::from(a.clone()))
                    .collect();
                let call_args = (cmd.clone(), dyn_args);
                if let Err(e) =
                    m.engine
                        .call_fn::<()>(&mut m.scope, &m.ast, "on_command", call_args)
                {
                    let id = m.meta.id.clone();
                    console.push(ChatLine::system(format!("mod '{id}' on_command: {e}")));
                }
            }
        }
        if !handled_any {
            console.push(ChatLine::system(format!(
                "no mod handles '/{cmd}' — loaded mods: {}",
                runtime.loaded_names().join(", ")
            )));
        }
    }

    // --- drain queued world operations ---
    let ops: Vec<ModOp> = std::mem::take(&mut *runtime.outbox.lock().unwrap());
    if ops.is_empty() {
        return;
    }

    let Ok(mut player_tf) = world.player_q.single_mut() else {
        return;
    };
    let mut remesh = Vec::new();

    for op in ops {
        match op {
            ModOp::Chat(msg) => console.push(ChatLine::system(format!("[mod] {msg}"))),
            ModOp::Log(msg) => info!("[mod] {msg}"),
            ModOp::SetBlock { pos, block } => {
                let call = super::harness::api::PendingCall {
                    id: String::new(),
                    name: "place_block".to_string(),
                    args: serde_json::json!({"x": pos.x, "y": pos.y, "z": pos.z, "block": tools::block_name(block)}),
                };
                let mut ctx = tools::ToolContext {
                    store: &mut world.store,
                    inventory: &mut world.inventory,
                    cycle: &mut world.cycle,
                    player_pos: player_tf.translation,
                    player_tf: &mut *player_tf,
                    remesh: &mut remesh,
                };
                let result = tools::execute(&call, &mut ctx);
                if result.starts_with("error") {
                    console.push(ChatLine::system(format!("[mod] {result}")));
                }
            }
            ModOp::Fill { min, max, block } => {
                let call = super::harness::api::PendingCall {
                    id: String::new(),
                    name: "fill".to_string(),
                    args: serde_json::json!({
                        "x1": min.x, "y1": min.y, "z1": min.z,
                        "x2": max.x, "y2": max.y, "z2": max.z,
                        "block": tools::block_name(block),
                    }),
                };
                let mut ctx = tools::ToolContext {
                    store: &mut world.store,
                    inventory: &mut world.inventory,
                    cycle: &mut world.cycle,
                    player_pos: player_tf.translation,
                    player_tf: &mut *player_tf,
                    remesh: &mut remesh,
                };
                let result = tools::execute(&call, &mut ctx);
                if result.starts_with("error") {
                    console.push(ChatLine::system(format!("[mod] {result}")));
                }
            }
            ModOp::Give { item, count } => {
                let mut added = 0usize;
                for _ in 0..count {
                    if world.inventory.add_item(item) {
                        added += 1;
                    } else {
                        break;
                    }
                }
                if added < count {
                    console.push(ChatLine::system(format!(
                        "[mod] gave {added} x {} (inventory full)",
                        item.display_name()
                    )));
                }
            }
            ModOp::Teleport { pos } => {
                player_tf.translation = pos;
            }
            ModOp::SetTime(t) => {
                world.cycle.time_of_day = t;
            }
        }
    }

    drop(player_tf);
    tools::apply_remesh(&world.manager, &mut world.commands, remesh);
}
