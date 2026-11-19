# DeepSeek Harness

The harness embeds a DeepSeek assistant into the game. It can chat and act
in your local single-player world through a small, auditable tool set.

## Setup

The assistant needs a DeepSeek API key (get one at <https://platform.deepseek.com>):

```bash
# Windows (PowerShell)
$env:DEEPSEEK_API_KEY = "sk-..."
# Linux / macOS
export DEEPSEEK_API_KEY=sk-...
```

Optional overrides:

| Env var | Default | Purpose |
|---|---|---|
| `DEEPSEEK_API_KEY` | — | **Required** API key |
| `DEEPSEEK_BASE_URL` | `https://api.deepseek.com` | API endpoint (any OpenAI-compatible endpoint works) |
| `DEEPSEEK_MODEL` | `deepseek-chat` | Model name (e.g. `deepseek-chat`, `deepseek-reasoner`) |

The launcher can store these and pass them to the game automatically.

## Usage

1. Press **T** in-game to open the console (Esc closes it).
2. Type a message and press Enter.
3. The assistant replies in the console; any tool calls are shown as
   `Tool: name(args)` lines.

Examples:

- `give me 64 oak planks and a diamond pickaxe`
- `build a 7x7 stone wall 5 blocks north of me`
- `fill a 10x10x1 glass floor under my feet, then set time to midnight`
- `what's around me right now?`

Lines starting with `/` are **mod commands**, not chat messages
(see [mods.md](mods.md)).

## Tool set

| Tool | Description |
|---|---|
| `get_player_position()` | Player coordinates |
| `teleport(x, y, z)` | Teleport the player |
| `get_time_of_day()` / `set_time_of_day(t)` | 0=sunrise, 0.25=noon, 0.5=sunset, 0.75=midnight |
| `place_block(x, y, z, block)` | Place one block (refuses to overlap the player, bedrock, water) |
| `break_block(x, y, z)` | Remove one block (bedrock refused) |
| `fill(x1..z2, block)` | Fill a box, capped at 4096 blocks; never entombs the player |
| `give_item(item, count)` | Add items to the inventory |
| `get_surroundings(radius)` | Block-type counts near the player |
| `list_inventory()` | Aggregated inventory contents |

Block and item names are snake_case (`stone`, `grass_block`, `oak_planks`,
`diamond_pickaxe`, …); the full list is injected into the model's system
prompt — see `src/harness/tools.rs`.

## Safety & privacy

- The key never leaves your machine except as the `Authorization` header to
  `DEEPSEEK_BASE_URL`.
- All world changes happen in your local single-player world only.
- Conversation history is kept in memory for the session (last ~24 messages)
  and is never written to disk.
- Every tool call is printed in the console before it executes.
- Requests run on background threads; the game never blocks on the network.

## Architecture

```
console (T) ──> HarnessHistory ──> background thread ──> DeepSeek API
                                       │
            ┌──────────────────────────┘ tool_calls
            ▼
   poll_network (main thread) ──> tools::execute ──> world (place/break/give/…)
            │
            └───── tool results appended ──> next request (≤ 8 rounds)
```

Relevant files: `src/harness/mod.rs`, `console.rs`, `api.rs`, `tools.rs`.
