// Example VoxelSeek mod (Rhai script).
//
// Hooks available: on_load, on_tick(dt), on_block_placed(x,y,z,block),
// on_block_broken(x,y,z,block), on_command(cmd, args).
// API: log, chat, set_block, fill, give, teleport, set_time, time_of_day,
// player_pos, get_block. See src/mods/mod.rs for the full reference.

fn on_load() {
    log("example-hello loaded");
    chat("example-hello mod loaded — try /castle or /hello in the console (press T)");
}

fn on_block_placed(x, y, z, block) {
    if block == "torch" {
        chat("Nice torch!");
    }
}

fn on_command(cmd, args) {
    if cmd == "hello" {
        chat("hello from the example-hello mod!");
    } else if cmd == "pos" {
        let p = player_pos();
        chat(`you are at ${p[0].floor()}, ${p[1].floor()}, ${p[2].floor()}`);
    } else if cmd == "castle" {
        let pos = player_pos();
        let px = pos[0].floor().to_int();
        let py = pos[1].floor().to_int();
        let pz = pos[2].floor().to_int();
        // 11x11 cobblestone floor just below the player's feet
        fill(px - 5, py - 1, pz - 5, px + 5, py - 1, pz + 5, "cobblestone");
        // four stone walls, 4 blocks high
        fill(px - 5, py, pz - 5, px + 5, py + 3, pz - 5, "stone");
        fill(px - 5, py, pz + 5, px + 5, py + 3, pz + 5, "stone");
        fill(px - 5, py, pz - 5, px - 5, py + 3, pz + 5, "stone");
        fill(px + 5, py, pz - 5, px + 5, py + 3, pz + 5, "stone");
        chat("castle built around you!");
    } else if cmd == "day" {
        set_time(0.25);
        chat("time set to noon");
    } else if cmd == "night" {
        set_time(0.75);
        chat("time set to midnight");
    } else {
        chat(`example-hello: unknown command /${cmd}`);
    }
}
