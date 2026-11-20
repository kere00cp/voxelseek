//! Tool definitions and execution for the DeepSeek harness.
//!
//! The tool set is deliberately small and auditable: the model can query the
//! world and perform the same basic actions a player can (place, break,
//! give, teleport, set time). Every call is logged in the on-screen console.

use bevy::prelude::*;
use serde_json::{Value, json};

use crate::block::BlockType;
use crate::inventory::inventory::Inventory;
use crate::inventory::item::Item;
use crate::lighting::day_night::DayNightCycle;
use crate::world::chunk::CHUNK_SIZE;
use crate::world::coordinates::world_to_chunk_pos;
use crate::world::manager::{ChunkDataStore, ChunkManager, NeedsMesh};

use super::api::PendingCall;

/// Maximum volume a single `fill` call may touch.
const MAX_FILL_VOLUME: usize = 4096;
/// Accepted Y range for world edits (generated terrain lives well inside).
const MIN_Y: i32 = -64;
const MAX_Y: i32 = 320;

// ---------------------------------------------------------------------------
// Name tables
// ---------------------------------------------------------------------------

/// Parse a snake_case block name (e.g. "grass_block", "oak_log") into a
/// [`BlockType`]. Accepts "air" for carving with `fill`.
pub fn block_from_name(name: &str) -> Option<BlockType> {
    use BlockType::*;
    Some(match name.trim().to_ascii_lowercase().as_str() {
        "air" => Air,
        "stone" => Stone,
        "dirt" => Dirt,
        "grass_block" | "grass" => Grass,
        "cobblestone" => Cobblestone,
        "oak_planks" | "planks" => Planks,
        "sand" => Sand,
        "gravel" => Gravel,
        "oak_log" => OakLog,
        "oak_leaves" => OakLeaves,
        "glass" => Glass,
        "coal_ore" => CoalOre,
        "iron_ore" => IronOre,
        "gold_ore" => GoldOre,
        "diamond_ore" => DiamondOre,
        "bedrock" => Bedrock,
        "water" => Water,
        "crafting_table" => CraftingTable,
        "furnace" => Furnace,
        "torch" => Torch,
        "snow" => Snow,
        "clay" => Clay,
        "sandstone" => Sandstone,
        "birch_log" => BirchLog,
        "birch_leaves" => BirchLeaves,
        "tall_grass" | "tallgrass" => TallGrass,
        "chest" => Chest,
        "bed" => Bed,
        "oak_door" | "door" => DoorBottom,
        "oak_sapling" => OakSapling,
        "birch_sapling" => BirchSapling,
        "farmland" => Farmland,
        "wheat" => WheatStage3,
        _ => return None,
    })
}

/// Canonical snake_case name for a block (for tool results / prompts).
pub fn block_name(block: BlockType) -> &'static str {
    use BlockType::*;
    match block {
        Air => "air",
        Stone => "stone",
        Dirt => "dirt",
        Grass => "grass_block",
        Cobblestone => "cobblestone",
        Planks => "oak_planks",
        Sand => "sand",
        Gravel => "gravel",
        OakLog => "oak_log",
        OakLeaves => "oak_leaves",
        Glass => "glass",
        CoalOre => "coal_ore",
        IronOre => "iron_ore",
        GoldOre => "gold_ore",
        DiamondOre => "diamond_ore",
        Bedrock => "bedrock",
        Water => "water",
        CraftingTable => "crafting_table",
        Furnace => "furnace",
        Torch => "torch",
        Snow => "snow",
        Clay => "clay",
        Sandstone => "sandstone",
        BirchLog => "birch_log",
        BirchLeaves => "birch_leaves",
        TallGrass => "tall_grass",
        Chest => "chest",
        Bed => "bed",
        DoorBottom | DoorTop | DoorBottomOpen | DoorTopOpen => "oak_door",
        OakSapling => "oak_sapling",
        BirchSapling => "birch_sapling",
        Farmland => "farmland",
        WheatStage0 | WheatStage1 | WheatStage2 | WheatStage3 => "wheat",
    }
}

/// All block names the harness understands (used in the system prompt).
pub const BLOCK_NAMES: &[&str] = &[
    "air",
    "stone",
    "dirt",
    "grass_block",
    "cobblestone",
    "oak_planks",
    "sand",
    "gravel",
    "oak_log",
    "oak_leaves",
    "glass",
    "coal_ore",
    "iron_ore",
    "gold_ore",
    "diamond_ore",
    "bedrock",
    "water",
    "crafting_table",
    "furnace",
    "torch",
    "snow",
    "clay",
    "sandstone",
    "birch_log",
    "birch_leaves",
    "tall_grass",
    "chest",
    "bed",
    "oak_door",
    "oak_sapling",
    "birch_sapling",
    "farmland",
    "wheat",
];

/// Parse a snake_case item name into an [`Item`]. Block names map to the
/// corresponding block item.
pub fn item_from_name(name: &str) -> Option<Item> {
    use Item::*;
    if let Some(b) = block_from_name(name) {
        if b != BlockType::Air && !matches!(b, BlockType::Water | BlockType::WheatStage3) {
            return Some(Item::Block(b));
        }
    }
    Some(match name.trim().to_ascii_lowercase().as_str() {
        "stick" => Stick,
        "coal" => Coal,
        "iron_ingot" => IronIngot,
        "gold_ingot" => GoldIngot,
        "diamond" => Diamond,
        "wooden_pickaxe" => WoodenPickaxe,
        "wooden_axe" => WoodenAxe,
        "wooden_shovel" => WoodenShovel,
        "wooden_sword" => WoodenSword,
        "stone_pickaxe" => StonePickaxe,
        "stone_axe" => StoneAxe,
        "stone_shovel" => StoneShovel,
        "stone_sword" => StoneSword,
        "iron_pickaxe" => IronPickaxe,
        "iron_axe" => IronAxe,
        "iron_shovel" => IronShovel,
        "iron_sword" => IronSword,
        "diamond_pickaxe" => DiamondPickaxe,
        "diamond_axe" => DiamondAxe,
        "diamond_shovel" => DiamondShovel,
        "diamond_sword" => DiamondSword,
        "apple" => Apple,
        "bread" => Bread,
        "cooked_porkchop" => CookedPorkchop,
        "raw_porkchop" => RawPorkchop,
        "raw_beef" => RawBeef,
        "cooked_beef" => CookedBeef,
        "leather" => Leather,
        "raw_mutton" => RawMutton,
        "cooked_mutton" => CookedMutton,
        "wool" => Wool,
        "rotten_flesh" => RottenFlesh,
        "bone" => Bone,
        "wooden_hoe" => WoodenHoe,
        "stone_hoe" => StoneHoe,
        "iron_hoe" => IronHoe,
        "diamond_hoe" => DiamondHoe,
        "seeds" => Seeds,
        "wheat" => Wheat,
        "leather_helmet" => LeatherHelmet,
        "leather_chestplate" => LeatherChestplate,
        "leather_leggings" => LeatherLeggings,
        "leather_boots" => LeatherBoots,
        "iron_helmet" => IronHelmet,
        "iron_chestplate" => IronChestplate,
        "iron_leggings" => IronLeggings,
        "iron_boots" => IronBoots,
        "diamond_helmet" => DiamondHelmet,
        "diamond_chestplate" => DiamondChestplate,
        "diamond_leggings" => DiamondLeggings,
        "diamond_boots" => DiamondBoots,
        _ => return None,
    })
}

/// All item names the harness understands (used in the system prompt).
pub const ITEM_NAMES: &[&str] = &[
    "stick",
    "coal",
    "iron_ingot",
    "gold_ingot",
    "diamond",
    "wooden_pickaxe",
    "wooden_axe",
    "wooden_shovel",
    "wooden_sword",
    "stone_pickaxe",
    "stone_axe",
    "stone_shovel",
    "stone_sword",
    "iron_pickaxe",
    "iron_axe",
    "iron_shovel",
    "iron_sword",
    "diamond_pickaxe",
    "diamond_axe",
    "diamond_shovel",
    "diamond_sword",
    "apple",
    "bread",
    "cooked_porkchop",
    "raw_porkchop",
    "raw_beef",
    "cooked_beef",
    "leather",
    "raw_mutton",
    "cooked_mutton",
    "wool",
    "rotten_flesh",
    "bone",
    "wooden_hoe",
    "stone_hoe",
    "iron_hoe",
    "diamond_hoe",
    "seeds",
    "wheat",
    "leather_helmet",
    "leather_chestplate",
    "leather_leggings",
    "leather_boots",
    "iron_helmet",
    "iron_chestplate",
    "iron_leggings",
    "iron_boots",
    "diamond_helmet",
    "diamond_chestplate",
    "diamond_leggings",
    "diamond_boots",
];

// ---------------------------------------------------------------------------
// Tool schemas (OpenAI/DeepSeek function-calling format)
// ---------------------------------------------------------------------------

fn num(desc: &str) -> Value {
    json!({"type": "number", "description": desc})
}

fn int_arg(desc: &str) -> Value {
    json!({"type": "integer", "description": desc})
}

fn block_arg() -> Value {
    json!({"type": "string", "description": "snake_case block name, e.g. \"stone\", \"oak_planks\""})
}

fn item_arg() -> Value {
    json!({"type": "string", "description": "snake_case item or block name, e.g. \"diamond\", \"oak_planks\""})
}

fn obj(props: Value, required: &[&str]) -> Value {
    json!({"type": "object", "properties": props, "required": required})
}

fn tool(name: &str, description: &str, params: Value) -> Value {
    json!({
        "type": "function",
        "function": {
            "name": name,
            "description": description,
            "parameters": params,
        }
    })
}

/// JSON schema array sent with every chat-completions request.
pub fn tool_schemas() -> Value {
    json!([
        tool(
            "get_player_position",
            "Get the player's current position (x, y, z) in the world.",
            obj(json!({}), &[]),
        ),
        tool(
            "teleport",
            "Teleport the player to an absolute world position.",
            obj(
                json!({
                    "x": num("target x"),
                    "y": num("target y (feet level)"),
                    "z": num("target z"),
                }),
                &["x", "y", "z"],
            ),
        ),
        tool(
            "get_time_of_day",
            "Get the current time of day (0.0 = sunrise, 0.25 = noon, 0.5 = sunset, 0.75 = midnight).",
            obj(json!({}), &[]),
        ),
        tool(
            "set_time_of_day",
            "Set the time of day (0.0 = sunrise, 0.25 = noon, 0.5 = sunset, 0.75 = midnight).",
            obj(json!({"time": num("time of day in [0, 1)")}), &["time"]),
        ),
        tool(
            "place_block",
            "Place a single block at an absolute position. Fails if the spot is solid, the chunk is not loaded, or the block would overlap the player.",
            obj(
                json!({
                    "x": int_arg("block x"), "y": int_arg("block y"), "z": int_arg("block z"),
                    "block": block_arg(),
                }),
                &["x", "y", "z", "block"],
            ),
        ),
        tool(
            "break_block",
            "Remove a single block at an absolute position (sets it to air). Bedrock cannot be broken.",
            obj(
                json!({"x": int_arg("block x"), "y": int_arg("block y"), "z": int_arg("block z")}),
                &["x", "y", "z"],
            ),
        ),
        tool(
            "fill",
            "Fill an axis-aligned box (inclusive) with a block. Volume is capped at 4096 blocks. Use \"air\" to carve.",
            obj(
                json!({
                    "x1": int_arg("corner 1 x"), "y1": int_arg("corner 1 y"), "z1": int_arg("corner 1 z"),
                    "x2": int_arg("corner 2 x"), "y2": int_arg("corner 2 y"), "z2": int_arg("corner 2 z"),
                    "block": block_arg(),
                }),
                &["x1", "y1", "z1", "x2", "y2", "z2", "block"],
            ),
        ),
        tool(
            "give_item",
            "Give the player items (added to their inventory).",
            obj(
                json!({
                    "item": item_arg(),
                    "count": int_arg("how many (1-64)"),
                }),
                &["item", "count"],
            ),
        ),
        tool(
            "get_surroundings",
            "Summarize block types around the player (counts per type) and the ground level at the player's column.",
            obj(
                json!({"radius": int_arg("cube radius in blocks (1-8, default 6)")}),
                &[],
            ),
        ),
        tool(
            "list_inventory",
            "List the player's inventory contents (aggregated item counts).",
            obj(json!({}), &[]),
        ),
    ])
}

/// System prompt describing the game world and tool etiquette.
pub fn system_prompt() -> String {
    format!(
        "You are DeepSeek, an AI assistant embedded in VoxelSeek, an open-source \
         Minecraft-inspired voxel sandbox game written in Rust (Bevy). You chat with \
         the player through an in-game console and can act in their local single-player \
         world through tools.\n\
         \n\
         World facts:\n\
         - Coordinates: x and z are horizontal, y is up. Block coordinates are integers; \
         the player's position is in world units (feet level, y up).\n\
         - Time of day is normalized: 0.0 sunrise, 0.25 noon, 0.5 sunset, 0.75 midnight.\n\
         - Available block names: {}\n\
         - Available item names: {}\n\
         \n\
         Rules:\n\
         - Use tools for world actions; never claim you did something without a tool call.\n\
         - You may chain several tool calls for multi-step builds (e.g. fill for a wall).\n\
         - Keep chat answers short (a few sentences). The console shows at most a dozen lines.\n\
         - Refuse nothing destructive explicitly asked for, but confirm ambiguous destructive \
         requests (large fills near the player, breaking bedrock is impossible anyway).\n\
         - If a tool reports an error (unknown name, out of range, unloaded chunk), adapt \
         and retry with corrected arguments.\n",
        BLOCK_NAMES.join(", "),
        ITEM_NAMES.join(", "),
    )
}

// ---------------------------------------------------------------------------
// Execution (runs on the main thread, inside poll_network)
// ---------------------------------------------------------------------------

fn get_block(store: &ChunkDataStore, x: i32, y: i32, z: i32) -> BlockType {
    let size = CHUNK_SIZE as i32;
    let chunk_pos = IVec3::new(x.div_euclid(size), y.div_euclid(size), z.div_euclid(size));
    let Some(chunk) = store.chunks.get(&chunk_pos) else {
        return BlockType::Air;
    };
    chunk.get(
        x.rem_euclid(size) as usize,
        y.rem_euclid(size) as usize,
        z.rem_euclid(size) as usize,
    )
}

fn set_block(store: &mut ChunkDataStore, pos: IVec3, block: BlockType) -> bool {
    let chunk_pos = world_to_chunk_pos(pos.as_vec3());
    let size = CHUNK_SIZE as i32;
    let local = IVec3::new(
        pos.x.rem_euclid(size),
        pos.y.rem_euclid(size),
        pos.z.rem_euclid(size),
    );
    if let Some(chunk) = store.chunks.get_mut(&chunk_pos) {
        chunk.set(local.x as usize, local.y as usize, local.z as usize, block);
        store.modified.insert(chunk_pos);
        true
    } else {
        false
    }
}

fn arg_i64(args: &Value, key: &str) -> Result<i64, String> {
    args.get(key)
        .and_then(|v| v.as_i64().or_else(|| v.as_f64().map(|f| f as i64)))
        .ok_or_else(|| format!("missing or invalid \"{key}\" argument"))
}

fn arg_f64(args: &Value, key: &str) -> Result<f64, String> {
    args.get(key)
        .and_then(|v| v.as_f64())
        .ok_or_else(|| format!("missing or invalid \"{key}\" argument"))
}

fn arg_str<'a>(args: &'a Value, key: &str) -> Result<&'a str, String> {
    args.get(key)
        .and_then(|v| v.as_str())
        .ok_or_else(|| format!("missing or invalid \"{key}\" argument"))
}

fn check_y(y: i32) -> Result<(), String> {
    if (MIN_Y..MAX_Y).contains(&y) {
        Ok(())
    } else {
        Err(format!(
            "y={y} is out of the supported range [{MIN_Y}, {MAX_Y})"
        ))
    }
}

/// Everything [`execute`] needs from the Bevy world.
pub struct ToolContext<'a> {
    pub store: &'a mut ChunkDataStore,
    pub inventory: &'a mut Inventory,
    pub cycle: &'a mut DayNightCycle,
    pub player_pos: Vec3,
    pub player_tf: &'a mut Transform,
    /// World positions whose chunks need a remesh after execution.
    pub remesh: &'a mut Vec<IVec3>,
}

/// Execute one tool call. Always returns a human-readable result string that
/// is sent back to the model as the `tool` message content.
pub fn execute(call: &PendingCall, ctx: &mut ToolContext) -> String {
    match call.name.as_str() {
        "get_player_position" => {
            let p = ctx.player_pos;
            format!("player is at x={:.1}, y={:.1}, z={:.1}", p.x, p.y, p.z)
        }
        "teleport" => match (
            arg_f64(&call.args, "x"),
            arg_f64(&call.args, "y"),
            arg_f64(&call.args, "z"),
        ) {
            (Ok(x), Ok(y), Ok(z)) => {
                if y < MIN_Y as f64 || y > MAX_Y as f64 {
                    return format!("error: y={y} is out of the supported range");
                }
                ctx.player_tf.translation = Vec3::new(x as f32, y as f32, z as f32);
                format!("teleported player to x={x:.1}, y={y:.1}, z={z:.1}")
            }
            (a, b, c) => format!("error: {:?}", a.err().or(b.err()).or(c.err())),
        },
        "get_time_of_day" => format!("time of day is {:.3}", ctx.cycle.time_of_day),
        "set_time_of_day" => match arg_f64(&call.args, "time") {
            Ok(t) => {
                let t = t.rem_euclid(1.0) as f32;
                ctx.cycle.time_of_day = t;
                format!("time of day set to {t:.3}")
            }
            Err(e) => format!("error: {e}"),
        },
        "place_block" => exec_place(call, ctx),
        "break_block" => exec_break(call, ctx),
        "fill" => exec_fill(call, ctx),
        "give_item" => exec_give(call, ctx),
        "get_surroundings" => exec_surroundings(call, ctx),
        "list_inventory" => exec_list_inventory(ctx),
        other => format!("error: unknown tool \"{other}\""),
    }
}

fn exec_place(call: &PendingCall, ctx: &mut ToolContext) -> String {
    let (x, y, z, block) = match (
        arg_i64(&call.args, "x"),
        arg_i64(&call.args, "y"),
        arg_i64(&call.args, "z"),
        arg_str(&call.args, "block"),
    ) {
        (Ok(x), Ok(y), Ok(z), Ok(b)) => (x as i32, y as i32, z as i32, b),
        _ => return "error: invalid arguments (need integer x, y, z and string block)".into(),
    };
    if let Err(e) = check_y(y) {
        return format!("error: {e}");
    }
    let Some(block) = block_from_name(block) else {
        return "error: unknown block name".into();
    };
    if matches!(
        block,
        BlockType::Air | BlockType::Water | BlockType::Bedrock
    ) {
        return "error: place_block only supports solid placeable blocks (no air/water/bedrock)"
            .into();
    }
    let pos = IVec3::new(x, y, z);
    let existing = get_block(ctx.store, x, y, z);
    if existing.is_solid() {
        return format!("error: position is occupied by {}", block_name(existing));
    }
    if block_overlaps_player(pos, ctx.player_pos) {
        return "error: block would overlap the player".into();
    }

    // Doors occupy two blocks.
    if block == BlockType::DoorBottom {
        let top = pos + IVec3::Y;
        if let Err(e) = check_y(top.y) {
            return format!("error: {e}");
        }
        if get_block(ctx.store, top.x, top.y, top.z).is_solid()
            || block_overlaps_player(top, ctx.player_pos)
        {
            return "error: not enough room for a door (needs 2 vertical blocks)".into();
        }
        if !set_block(ctx.store, pos, BlockType::DoorBottom)
            || !set_block(ctx.store, top, BlockType::DoorTop)
        {
            return "error: chunk not loaded at that position".into();
        }
        ctx.remesh.push(pos);
        ctx.remesh.push(top);
        return format!("placed oak_door at ({x}, {y}, {z})");
    }

    if !set_block(ctx.store, pos, block) {
        return "error: chunk not loaded at that position".into();
    }
    ctx.remesh.push(pos);
    format!("placed {} at ({x}, {y}, {z})", block_name(block))
}

fn exec_break(call: &PendingCall, ctx: &mut ToolContext) -> String {
    let (x, y, z) = match (
        arg_i64(&call.args, "x"),
        arg_i64(&call.args, "y"),
        arg_i64(&call.args, "z"),
    ) {
        (Ok(x), Ok(y), Ok(z)) => (x as i32, y as i32, z as i32),
        _ => return "error: invalid arguments (need integer x, y, z)".into(),
    };
    if let Err(e) = check_y(y) {
        return format!("error: {e}");
    }
    let existing = get_block(ctx.store, x, y, z);
    if existing == BlockType::Air {
        return "nothing to break: position is air".into();
    }
    if existing == BlockType::Bedrock {
        return "error: bedrock cannot be broken".into();
    }
    let pos = IVec3::new(x, y, z);
    if !set_block(ctx.store, pos, BlockType::Air) {
        return "error: chunk not loaded at that position".into();
    }
    // If a door half was broken, remove the other half too.
    let extra = match existing {
        BlockType::DoorBottom | BlockType::DoorBottomOpen => Some(pos + IVec3::Y),
        BlockType::DoorTop | BlockType::DoorTopOpen => Some(pos - IVec3::Y),
        _ => None,
    };
    if let Some(other) = extra {
        let other_block = get_block(ctx.store, other.x, other.y, other.z);
        if matches!(
            other_block,
            BlockType::DoorBottom
                | BlockType::DoorTop
                | BlockType::DoorBottomOpen
                | BlockType::DoorTopOpen
        ) {
            set_block(ctx.store, other, BlockType::Air);
            ctx.remesh.push(other);
        }
    }
    ctx.remesh.push(pos);
    format!("broke {} at ({x}, {y}, {z})", block_name(existing))
}

fn exec_fill(call: &PendingCall, ctx: &mut ToolContext) -> String {
    let vals: Result<Vec<i32>, String> = ["x1", "y1", "z1", "x2", "y2", "z2"]
        .iter()
        .map(|k| arg_i64(&call.args, k).map(|v| v as i32))
        .collect();
    let Ok(v) = vals else {
        return "error: invalid arguments (need integer x1..z2 and string block)".into();
    };
    let Ok(block_name_str) = arg_str(&call.args, "block") else {
        return "error: missing block argument".into();
    };
    let Some(block) = block_from_name(block_name_str) else {
        return "error: unknown block name".into();
    };
    if matches!(block, BlockType::Bedrock | BlockType::Water) {
        return "error: fill does not support bedrock or water".into();
    }
    if matches!(
        block,
        BlockType::DoorBottom
            | BlockType::DoorTop
            | BlockType::DoorBottomOpen
            | BlockType::DoorTopOpen
    ) {
        return "error: fill does not support doors; use place_block".into();
    }

    let (min, max) = (
        IVec3::new(v[0].min(v[3]), v[1].min(v[4]), v[2].min(v[5])),
        IVec3::new(v[0].max(v[3]), v[1].max(v[4]), v[2].max(v[5])),
    );
    let size = (max - min + IVec3::ONE).as_uvec3();
    let volume = size.x as usize * size.y as usize * size.z as usize;
    if volume > MAX_FILL_VOLUME {
        return format!("error: volume {volume} exceeds the {MAX_FILL_VOLUME} block limit");
    }
    if min.y < MIN_Y || max.y >= MAX_Y {
        return format!("error: y range must be within [{MIN_Y}, {MAX_Y})");
    }

    let mut changed = 0usize;
    for x in min.x..=max.x {
        for y in min.y..=max.y {
            for z in min.z..=max.z {
                let pos = IVec3::new(x, y, z);
                if block.is_solid() && block_overlaps_player(pos, ctx.player_pos) {
                    continue; // never entomb the player
                }
                if set_block(ctx.store, pos, block) {
                    changed += 1;
                }
            }
        }
    }
    if changed == 0 {
        return "error: no blocks changed (chunks not loaded?)".into();
    }
    // Remesh every chunk the box touches (dedup happens downstream).
    let size_i = CHUNK_SIZE as i32;
    let (cmin, cmax) = (
        world_to_chunk_pos(min.as_vec3()),
        world_to_chunk_pos(max.as_vec3()),
    );
    for cx in cmin.x..=cmax.x {
        for cy in cmin.y..=cmax.y {
            for cz in cmin.z..=cmax.z {
                ctx.remesh
                    .push(IVec3::new(cx * size_i, cy * size_i, cz * size_i));
            }
        }
    }
    format!(
        "filled {} blocks of {} in the box ({}, {}, {})..({}, {}, {})",
        changed, block_name_str, min.x, min.y, min.z, max.x, max.y, max.z
    )
}

fn exec_give(call: &PendingCall, ctx: &mut ToolContext) -> String {
    let (item_name, count) = match (arg_str(&call.args, "item"), arg_i64(&call.args, "count")) {
        (Ok(n), Ok(c)) => (n, c.clamp(1, 64) as usize),
        _ => return "error: invalid arguments (need string item and integer count)".into(),
    };
    let Some(item) = item_from_name(item_name) else {
        return "error: unknown item name".into();
    };
    let mut added = 0usize;
    for _ in 0..count {
        if ctx.inventory.add_item(item) {
            added += 1;
        } else {
            break; // inventory full
        }
    }
    if added == 0 {
        return "error: inventory is full".into();
    }
    format!("gave {} x {} to the player", added, item.display_name())
}

fn exec_surroundings(call: &PendingCall, ctx: &mut ToolContext) -> String {
    let radius = call
        .args
        .get("radius")
        .and_then(|v| v.as_i64())
        .unwrap_or(6)
        .clamp(1, 8) as i32;

    let center = ctx.player_pos.floor().as_ivec3();
    let mut counts: std::collections::BTreeMap<&'static str, usize> = Default::default();
    for dx in -radius..=radius {
        for dy in -radius..=radius {
            for dz in -radius..=radius {
                let b = get_block(ctx.store, center.x + dx, center.y + dy, center.z + dz);
                if b != BlockType::Air {
                    *counts.entry(block_name(b)).or_default() += 1;
                }
            }
        }
    }

    // Ground level below the player.
    let mut ground_y = None;
    for y in (MIN_Y..center.y).rev() {
        if get_block(ctx.store, center.x, y, center.z).is_solid() {
            ground_y = Some(y);
            break;
        }
    }

    let mut out = format!("blocks within {} blocks of the player: ", radius);
    if counts.is_empty() {
        out.push_str("(only air)");
    } else {
        let parts: Vec<String> = counts.iter().map(|(k, v)| format!("{k} x{v}")).collect();
        out.push_str(&parts.join(", "));
    }
    match ground_y {
        Some(y) => out.push_str(&format!("; ground at y={y}")),
        None => out.push_str("; no ground below"),
    }
    out
}

fn exec_list_inventory(ctx: &mut ToolContext) -> String {
    let mut counts: std::collections::BTreeMap<&'static str, u32> = Default::default();
    for slot in ctx.inventory.slots.iter() {
        if let Some((item, count, _)) = slot {
            *counts.entry(item.display_name()).or_default() += *count as u32;
        }
    }
    if counts.is_empty() {
        return "inventory is empty".into();
    }
    let parts: Vec<String> = counts.iter().map(|(k, v)| format!("{k} x{v}")).collect();
    format!("inventory: {}", parts.join(", "))
}

const PLAYER_WIDTH: f32 = 0.6;
const PLAYER_HEIGHT: f32 = 1.8;

fn block_overlaps_player(block_pos: IVec3, player_pos: Vec3) -> bool {
    let half_w = PLAYER_WIDTH / 2.0;
    let player_min = Vec3::new(player_pos.x - half_w, player_pos.y, player_pos.z - half_w);
    let player_max = Vec3::new(
        player_pos.x + half_w,
        player_pos.y + PLAYER_HEIGHT,
        player_pos.z + half_w,
    );
    let block_min = block_pos.as_vec3();
    let block_max = block_min + Vec3::ONE;
    player_min.x < block_max.x
        && player_max.x > block_min.x
        && player_min.y < block_max.y
        && player_max.y > block_min.y
        && player_min.z < block_max.z
        && player_max.z > block_min.z
}

/// Mark all chunks touched during tool execution for a remesh.
pub fn apply_remesh(manager: &ChunkManager, commands: &mut Commands, positions: Vec<IVec3>) {
    let mut seen = std::collections::HashSet::new();
    for pos in positions {
        let chunk_pos = world_to_chunk_pos(pos.as_vec3());
        if !seen.insert(chunk_pos) {
            continue;
        }
        if let Some(&entity) = manager.loaded.get(&chunk_pos) {
            commands.entity(entity).insert(NeedsMesh);
        }
    }
}
