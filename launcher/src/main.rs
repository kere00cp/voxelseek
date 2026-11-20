//! VoxelSeek Launcher — a small native launcher for the game.
//!
//! Features:
//! - edit game settings (window size, FOV, render distance, texture pack)
//! - manage the DeepSeek API key (passed to the game as an env var)
//! - enable/disable mods installed in the game's `mods/` folder
//! - build the game with cargo and launch it with one click
//!
//! The launcher stores its own state in `launcher.toml` next to the game
//! directory and writes the game's settings file `voxelseek.toml`.

use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::sync::mpsc;

use eframe::egui;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([560.0, 720.0])
            .with_min_inner_size([480.0, 600.0])
            .with_title("VoxelSeek Launcher"),
        ..Default::default()
    };
    eframe::run_native(
        "voxelseek-launcher",
        options,
        Box::new(|_cc| Ok(Box::new(LauncherApp::new()))),
    )
}

// ---------------------------------------------------------------------------
// Data model
// ---------------------------------------------------------------------------

/// The game's `voxelseek.toml` (subset mirrored from src/settings.rs).
#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(default)]
struct GameSettings {
    texture_pack: String,
    window_width: f32,
    window_height: f32,
    render_distance: i32,
    fov: f32,
    disabled_mods: Vec<String>,
}

impl Default for GameSettings {
    fn default() -> Self {
        Self {
            texture_pack: "programmer-art".into(),
            window_width: 1280.0,
            window_height: 720.0,
            render_distance: 16,
            fov: 80.0,
            disabled_mods: Vec::new(),
        }
    }
}

/// The launcher's own persistent state (`launcher.toml`).
#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
struct LauncherConfig {
    game_dir: String,
    deepseek_api_key: String,
    deepseek_base_url: String,
    deepseek_model: String,
}

#[derive(Clone)]
struct PackInfo {
    id: String,
    name: String,
    license: String,
}

#[derive(Clone)]
struct ModInfo {
    id: String,
    name: String,
    version: String,
    description: String,
    enabled: bool,
}

struct LauncherApp {
    game_dir: PathBuf,
    config: LauncherConfig,
    settings: GameSettings,
    packs: Vec<PackInfo>,
    mods: Vec<ModInfo>,
    status: String,
    child: Option<Child>,
    build_rx: Option<mpsc::Receiver<String>>,
    show_api_key: bool,
}

// ---------------------------------------------------------------------------
// Discovery helpers
// ---------------------------------------------------------------------------

fn exe_name() -> &'static str {
    if cfg!(windows) {
        "voxelseek.exe"
    } else {
        "voxelseek"
    }
}

/// Locate the game directory: a place that contains (or will contain) the
/// `assets/packs` tree. Preference order: saved config → launcher exe dir →
/// cwd → parent dirs of cwd (covers `cargo run -p voxelseek-launcher`).
fn find_game_dir(config: &LauncherConfig) -> PathBuf {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if !config.game_dir.is_empty() {
        candidates.push(PathBuf::from(&config.game_dir));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.to_path_buf());
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.clone());
        for up in cwd.ancestors().skip(1).take(3) {
            candidates.push(up.to_path_buf());
        }
    }
    for cand in &candidates {
        if cand.join("assets/packs").is_dir() {
            return cand.clone();
        }
    }
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

fn game_exe_path(game_dir: &Path) -> Option<PathBuf> {
    let direct = game_dir.join(exe_name());
    if direct.exists() {
        return Some(direct);
    }
    for profile in ["release", "debug"] {
        let p = game_dir.join("target").join(profile).join(exe_name());
        if p.exists() {
            return Some(p);
        }
    }
    None
}

fn scan_packs(game_dir: &Path) -> Vec<PackInfo> {
    let mut out = Vec::new();
    let root = game_dir.join("assets/packs");
    let Ok(rd) = std::fs::read_dir(&root) else {
        return out;
    };
    for entry in rd.flatten() {
        let dir = entry.path();
        if !dir.is_dir() {
            continue;
        }
        let id = entry.file_name().to_string_lossy().into_owned();
        let (mut name, mut license) = (id.clone(), String::new());
        if let Ok(text) = std::fs::read_to_string(dir.join("pack.toml")) {
            if let Ok(v) = toml::from_str::<toml::Value>(&text) {
                name = v
                    .get("name")
                    .and_then(|x| x.as_str())
                    .unwrap_or(&id)
                    .to_string();
                license = v
                    .get("license")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_string();
            }
        }
        out.push(PackInfo { id, name, license });
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    out
}

fn scan_mods(game_dir: &Path, disabled: &[String]) -> Vec<ModInfo> {
    let mut out = Vec::new();
    let root = game_dir.join("mods");
    let Ok(rd) = std::fs::read_dir(&root) else {
        return out;
    };
    for entry in rd.flatten() {
        let dir = entry.path();
        if !dir.is_dir() || !dir.join("mod.toml").exists() {
            continue;
        }
        let mut info = ModInfo {
            id: entry.file_name().to_string_lossy().into_owned(),
            name: entry.file_name().to_string_lossy().into_owned(),
            version: String::new(),
            description: String::new(),
            enabled: true,
        };
        if let Ok(text) = std::fs::read_to_string(dir.join("mod.toml")) {
            if let Ok(v) = toml::from_str::<toml::Value>(&text) {
                info.id = v
                    .get("id")
                    .and_then(|x| x.as_str())
                    .unwrap_or(&info.id)
                    .to_string();
                info.name = v
                    .get("name")
                    .and_then(|x| x.as_str())
                    .unwrap_or(&info.name)
                    .to_string();
                info.version = v
                    .get("version")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_string();
                info.description = v
                    .get("description")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_string();
            }
        }
        info.enabled = !disabled.iter().any(|d| d == &info.id);
        out.push(info);
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    out
}

// ---------------------------------------------------------------------------
// App
// ---------------------------------------------------------------------------

impl LauncherApp {
    fn new() -> Self {
        let config: LauncherConfig = std::env::current_dir()
            .ok()
            .and_then(|d| std::fs::read_to_string(d.join("launcher.toml")).ok())
            .and_then(|text| toml::from_str(&text).ok())
            .unwrap_or_default();

        let game_dir = find_game_dir(&config);
        let settings: GameSettings = std::fs::read_to_string(game_dir.join("voxelseek.toml"))
            .ok()
            .and_then(|text| toml::from_str(&text).ok())
            .unwrap_or_default();
        let packs = scan_packs(&game_dir);
        let mods = scan_mods(&game_dir, &settings.disabled_mods);

        let mut config = config;
        config.game_dir = game_dir.to_string_lossy().into_owned();

        Self {
            game_dir,
            config,
            settings,
            packs,
            mods,
            status: "就绪".into(),
            child: None,
            build_rx: None,
            show_api_key: false,
        }
    }

    fn save_all(&mut self) {
        // Sync disabled mods from the checkboxes.
        self.settings.disabled_mods = self
            .mods
            .iter()
            .filter(|m| !m.enabled)
            .map(|m| m.id.clone())
            .collect();

        if let Ok(text) = toml::to_string_pretty(&self.settings) {
            if let Err(e) = std::fs::write(self.game_dir.join("voxelseek.toml"), text) {
                self.status = format!("保存 voxelseek.toml 失败: {e}");
                return;
            }
        }
        if let Ok(text) = toml::to_string_pretty(&self.config) {
            let _ = std::fs::write(
                std::env::current_dir()
                    .unwrap_or_else(|_| PathBuf::from("."))
                    .join("launcher.toml"),
                text,
            );
        }
        self.status = "已保存设置".into();
    }

    fn launch(&mut self) {
        self.save_all();
        let Some(exe) = game_exe_path(&self.game_dir) else {
            self.status = "找不到游戏可执行文件，请先构建（点下方“构建游戏”）".into();
            return;
        };
        let mut cmd = Command::new(&exe);
        cmd.current_dir(&self.game_dir);
        if !self.config.deepseek_api_key.trim().is_empty() {
            cmd.env("DEEPSEEK_API_KEY", self.config.deepseek_api_key.trim());
        }
        if !self.config.deepseek_base_url.trim().is_empty() {
            cmd.env("DEEPSEEK_BASE_URL", self.config.deepseek_base_url.trim());
        }
        if !self.config.deepseek_model.trim().is_empty() {
            cmd.env("DEEPSEEK_MODEL", self.config.deepseek_model.trim());
        }
        match cmd.spawn() {
            Ok(child) => {
                self.status = "游戏已启动".into();
                self.child = Some(child);
            }
            Err(e) => {
                self.status = format!("启动失败: {e}");
            }
        }
    }

    fn build_game(&mut self) {
        if self.build_rx.is_some() {
            return;
        }
        let (tx, rx) = mpsc::channel();
        let dir = self.game_dir.clone();
        std::thread::spawn(move || {
            let out = Command::new("cargo")
                .args(["build", "--release", "--package", "voxelseek"])
                .current_dir(&dir)
                .output();
            let msg = match out {
                Ok(o) if o.status.success() => "构建完成 ✔".to_string(),
                Ok(o) => format!(
                    "构建失败:\n{}",
                    String::from_utf8_lossy(&o.stderr)
                        .chars()
                        .take(800)
                        .collect::<String>()
                ),
                Err(e) => format!("无法运行 cargo: {e}"),
            };
            let _ = tx.send(msg);
        });
        self.build_rx = Some(rx);
        self.status = "正在构建（首次可能需要几分钟）…".into();
    }
}

// ---------------------------------------------------------------------------
// UI
// ---------------------------------------------------------------------------

impl eframe::App for LauncherApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Poll background build result.
        if let Some(rx) = &self.build_rx {
            if let Ok(msg) = rx.try_recv() {
                self.status = msg;
                self.build_rx = None;
            }
        }
        // Poll running game.
        if let Some(child) = &mut self.child {
            match child.try_wait() {
                Ok(Some(code)) => {
                    self.status = format!("游戏已退出 ({code})");
                    self.child = None;
                }
                Ok(None) => {}
                Err(_) => {
                    self.child = None;
                }
            }
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("VoxelSeek 启动器");
            ui.label(format!("游戏目录: {}", self.game_dir.display()));
            ui.separator();

            // --- DeepSeek ---
            egui::CollapsingHeader::new("DeepSeek 设置")
                .default_open(true)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("API Key:");
                        let mut edit =
                            egui::TextEdit::singleline(&mut self.config.deepseek_api_key)
                                .desired_width(280.0);
                        edit = edit.password(!self.show_api_key);
                        ui.add(edit);
                        ui.checkbox(&mut self.show_api_key, "显示");
                    });
                    ui.horizontal(|ui| {
                        ui.label("Base URL:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.config.deepseek_base_url)
                                .desired_width(280.0)
                                .hint_text("https://api.deepseek.com"),
                        );
                    });
                    ui.horizontal(|ui| {
                        ui.label("模型:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.config.deepseek_model)
                                .desired_width(180.0)
                                .hint_text("deepseek-chat"),
                        );
                    });
                    ui.label("密钥仅保存在本地 launcher.toml，启动时以环境变量传给游戏。");
                });

            // --- Game settings ---
            egui::CollapsingHeader::new("游戏设置")
                .default_open(true)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("资源包:");
                        egui::ComboBox::from_id_salt("pack")
                            .selected_text(&self.settings.texture_pack)
                            .show_ui(ui, |ui| {
                                for pack in &self.packs {
                                    ui.selectable_value(
                                        &mut self.settings.texture_pack,
                                        pack.id.clone(),
                                        format!("{} ({})", pack.name, pack.license),
                                    );
                                }
                            });
                    });
                    ui.horizontal(|ui| {
                        ui.label("窗口:");
                        ui.add(
                            egui::DragValue::new(&mut self.settings.window_width)
                                .range(640.0..=3840.0),
                        );
                        ui.label("x");
                        ui.add(
                            egui::DragValue::new(&mut self.settings.window_height)
                                .range(480.0..=2160.0),
                        );
                    });
                    ui.horizontal(|ui| {
                        ui.label("渲染距离 (区块):");
                        ui.add(egui::Slider::new(
                            &mut self.settings.render_distance,
                            2..=24,
                        ));
                    });
                    ui.horizontal(|ui| {
                        ui.label("视场角 FOV:");
                        ui.add(egui::Slider::new(&mut self.settings.fov, 60.0..=110.0));
                    });
                });

            // --- Mods ---
            egui::CollapsingHeader::new(format!("模组 ({})", self.mods.len()))
                .default_open(false)
                .show(ui, |ui| {
                    if self.mods.is_empty() {
                        ui.label("未检测到模组。把模组放进游戏目录的 mods/<id>/ 即可。");
                    }
                    for m in &mut self.mods {
                        ui.horizontal(|ui| {
                            ui.checkbox(&mut m.enabled, "");
                            ui.label(format!("{} {}", m.name, m.version));
                            if !m.description.is_empty() {
                                ui.small(m.description.clone());
                            }
                        });
                    }
                    if ui.button("重新扫描 mods 文件夹").clicked() {
                        self.mods = scan_mods(&self.game_dir, &self.settings.disabled_mods);
                    }
                });

            ui.separator();
            ui.horizontal(|ui| {
                let running = self.child.is_some();
                let building = self.build_rx.is_some();

                let launch_text = if running {
                    "运行中…"
                } else {
                    "▶ 启动游戏"
                };
                if ui
                    .add_enabled(
                        !running && !building,
                        egui::Button::new(launch_text).min_size([120.0, 36.0].into()),
                    )
                    .clicked()
                {
                    self.launch();
                }
                if running && ui.button("结束游戏").clicked() {
                    if let Some(mut child) = self.child.take() {
                        let _ = child.kill();
                    }
                    self.status = "已结束游戏进程".into();
                }
                if ui
                    .add_enabled(
                        !building,
                        egui::Button::new(if building {
                            "构建中…"
                        } else {
                            "构建游戏"
                        }),
                    )
                    .clicked()
                {
                    self.build_game();
                }
                if ui.button("保存设置").clicked() {
                    self.save_all();
                }
            });

            ui.separator();
            ui.label(&self.status);
        });

        // Repaint while a game or build is running so status updates.
        if self.child.is_some() || self.build_rx.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(250));
        }
    }
}
