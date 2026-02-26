use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use alacritty_terminal::event::{Event, EventListener};
use alacritty_terminal::grid::{Dimensions, Scroll};
use alacritty_terminal::sync::FairMutex;
use alacritty_terminal::term::cell::Flags;
use alacritty_terminal::term::Config;
use alacritty_terminal::term::TermDamage;
use alacritty_terminal::vte::ansi::{Color, CursorShape, NamedColor, Processor, Rgb};
use alacritty_terminal::Term;

use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::mouse::{AccumulatedMouseScroll, MouseScrollUnit};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy::asset::RenderAssetUsages;

use nu_cli::{gather_parent_env_vars, eval_source};
use nu_cmd_lang::create_default_context;
use nu_command::add_shell_command_context;
use nu_engine::eval_block;
use nu_parser::parse;
use nu_protocol::engine::{EngineState, Redirection, Stack, StateWorkingSet, Closure};
use nu_protocol::debugger::WithoutDebug;
use nu_protocol::{OutDest, PipelineData, Signals, Value};
use nu_engine::ClosureEvalOnce;
use nu_std::load_standard_library;

use sugarloaf::{
    FragmentStyle, FragmentStyleDecoration, Object, RichText, Sugarloaf,
    SugarloafWindowSize, UnderlineInfo, UnderlineShape,
};
use sugarloaf::context::Context as SugarloafContext;
use sugarloaf::font::FontLibrary;
use sugarloaf::layout::RootStyle;

use super::WorldState;

const FONT_SIZE: f32 = 16.0;

const NU_ENV_SOURCE: &str = include_str!("../../assets/nu-config/env.nu");
const NU_CONFIG_SOURCE: &str = include_str!("../../assets/nu-config/config.nu");

pub struct TerminalWorldPlugin;

#[derive(Component)]
struct TerminalMarker;

impl Plugin for TerminalWorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(WorldState::Terminal), setup_terminal)
            .add_systems(OnExit(WorldState::Terminal), destroy_terminal)
            .add_systems(
                Update,
                (
                    terminal_update,
                )
                    .run_if(in_state(WorldState::Terminal)),
            );
    }
}

// --- ANSI 16-color theme ---

fn default_ansi_rgb(color: NamedColor) -> Rgb {
    match color {
        NamedColor::Black => Rgb { r: 0, g: 0, b: 0 },
        NamedColor::Red => Rgb { r: 204, g: 0, b: 0 },
        NamedColor::Green => Rgb { r: 0, g: 204, b: 0 },
        NamedColor::Yellow => Rgb { r: 204, g: 204, b: 0 },
        NamedColor::Blue => Rgb { r: 0, g: 0, b: 204 },
        NamedColor::Magenta => Rgb { r: 204, g: 0, b: 204 },
        NamedColor::Cyan => Rgb { r: 0, g: 204, b: 204 },
        NamedColor::White => Rgb { r: 191, g: 191, b: 191 },
        NamedColor::BrightBlack => Rgb { r: 102, g: 102, b: 102 },
        NamedColor::BrightRed => Rgb { r: 255, g: 85, b: 85 },
        NamedColor::BrightGreen => Rgb { r: 85, g: 255, b: 85 },
        NamedColor::BrightYellow => Rgb { r: 255, g: 255, b: 85 },
        NamedColor::BrightBlue => Rgb { r: 85, g: 85, b: 255 },
        NamedColor::BrightMagenta => Rgb { r: 255, g: 85, b: 255 },
        NamedColor::BrightCyan => Rgb { r: 85, g: 255, b: 255 },
        NamedColor::BrightWhite => Rgb { r: 255, g: 255, b: 255 },
        NamedColor::Foreground | NamedColor::BrightForeground => Rgb { r: 230, g: 230, b: 230 },
        NamedColor::Background => Rgb { r: 0, g: 0, b: 0 },
        NamedColor::Cursor => Rgb { r: 230, g: 230, b: 230 },
        NamedColor::DimBlack => Rgb { r: 0, g: 0, b: 0 },
        NamedColor::DimRed => Rgb { r: 128, g: 0, b: 0 },
        NamedColor::DimGreen => Rgb { r: 0, g: 128, b: 0 },
        NamedColor::DimYellow => Rgb { r: 128, g: 128, b: 0 },
        NamedColor::DimBlue => Rgb { r: 0, g: 0, b: 128 },
        NamedColor::DimMagenta => Rgb { r: 128, g: 0, b: 128 },
        NamedColor::DimCyan => Rgb { r: 0, g: 128, b: 128 },
        NamedColor::DimWhite => Rgb { r: 128, g: 128, b: 128 },
        NamedColor::DimForeground => Rgb { r: 128, g: 128, b: 128 },
    }
}

fn resolve_color(color: Color, colors: &alacritty_terminal::term::color::Colors) -> Rgb {
    match color {
        Color::Spec(rgb) => rgb,
        Color::Named(named) => colors[named].unwrap_or_else(|| default_ansi_rgb(named)),
        Color::Indexed(idx) => {
            if let Some(rgb) = colors[idx as usize] {
                return rgb;
            }
            if idx < 16 {
                let named = match idx {
                    0 => NamedColor::Black,
                    1 => NamedColor::Red,
                    2 => NamedColor::Green,
                    3 => NamedColor::Yellow,
                    4 => NamedColor::Blue,
                    5 => NamedColor::Magenta,
                    6 => NamedColor::Cyan,
                    7 => NamedColor::White,
                    8 => NamedColor::BrightBlack,
                    9 => NamedColor::BrightRed,
                    10 => NamedColor::BrightGreen,
                    11 => NamedColor::BrightYellow,
                    12 => NamedColor::BrightBlue,
                    13 => NamedColor::BrightMagenta,
                    14 => NamedColor::BrightCyan,
                    15 => NamedColor::BrightWhite,
                    _ => unreachable!(),
                };
                default_ansi_rgb(named)
            } else if idx < 232 {
                let i = idx - 16;
                let r = (i / 36) % 6;
                let g = (i / 6) % 6;
                let b = i % 6;
                let to_byte = |v: u8| -> u8 { if v == 0 { 0 } else { 55 + 40 * v } };
                Rgb { r: to_byte(r), g: to_byte(g), b: to_byte(b) }
            } else {
                let level = 8 + 10 * (idx - 232);
                Rgb { r: level, g: level, b: level }
            }
        }
    }
}

fn rgb_to_f32(rgb: Rgb) -> [f32; 4] {
    [rgb.r as f32 / 255.0, rgb.g as f32 / 255.0, rgb.b as f32 / 255.0, 1.0]
}

// --- Event listener for alacritty_terminal ---

#[derive(Clone)]
struct BevyEventProxy;

impl EventListener for BevyEventProxy {
    fn send_event(&self, event: Event) {
        match event {
            Event::Title(title) => debug!("Terminal title: {}", title),
            Event::Bell => debug!("Terminal bell"),
            _ => {}
        }
    }
}

// --- Dimensions impl ---

struct TermDimensions {
    cols: usize,
    lines: usize,
}

impl Dimensions for TermDimensions {
    fn total_lines(&self) -> usize { self.lines }
    fn screen_lines(&self) -> usize { self.lines }
    fn columns(&self) -> usize { self.cols }
}

// --- Nushell engine wrapper ---

struct NuShellEngine {
    engine_state: EngineState,
    stack: Stack,
}

// --- Line buffer for input accumulation ---

struct LineBuffer {
    buffer: String,
    cursor_pos: usize,
    history: Vec<String>,
    history_index: Option<usize>,
}

impl LineBuffer {
    fn new() -> Self {
        Self {
            buffer: String::new(),
            cursor_pos: 0,
            history: Vec::new(),
            history_index: None,
        }
    }

    fn insert_char(&mut self, ch: char) {
        self.buffer.insert(self.cursor_pos, ch);
        self.cursor_pos += ch.len_utf8();
    }

    fn backspace(&mut self) -> bool {
        if self.cursor_pos > 0 {
            let prev = self.buffer[..self.cursor_pos]
                .chars()
                .last()
                .map(|c| c.len_utf8())
                .unwrap_or(0);
            self.cursor_pos -= prev;
            self.buffer.remove(self.cursor_pos);
            true
        } else {
            false
        }
    }

    fn take_line(&mut self) -> String {
        let line = self.buffer.clone();
        if !line.trim().is_empty() {
            self.history.push(line.clone());
        }
        self.buffer.clear();
        self.cursor_pos = 0;
        self.history_index = None;
        line
    }

    fn history_up(&mut self) -> bool {
        if self.history.is_empty() {
            return false;
        }
        let idx = match self.history_index {
            None => self.history.len() - 1,
            Some(0) => return false,
            Some(i) => i - 1,
        };
        self.history_index = Some(idx);
        self.buffer = self.history[idx].clone();
        self.cursor_pos = self.buffer.len();
        true
    }

    fn history_down(&mut self) -> bool {
        let idx = match self.history_index {
            None => return false,
            Some(i) => i + 1,
        };
        if idx >= self.history.len() {
            self.history_index = None;
            self.buffer.clear();
            self.cursor_pos = 0;
            return true;
        }
        self.history_index = Some(idx);
        self.buffer = self.history[idx].clone();
        self.cursor_pos = self.buffer.len();
        true
    }
}

// --- Eval result from background thread ---

struct EvalResult {
    output: Vec<u8>,
    error: Option<String>,
}

// --- NonSend terminal state (contains !Sync types) ---

struct TerminalNonSendState {
    nu_engine: Option<NuShellEngine>,
    term: Arc<FairMutex<Term<BevyEventProxy>>>,
    processor: Processor,
    line_buffer: LineBuffer,
    cols: usize,
    rows: usize,
    rich_text_id: usize,
    sugarloaf: Sugarloaf<'static>,
    image_handle: Handle<Image>,
    eval_rx: Option<std::sync::mpsc::Receiver<(EvalResult, NuShellEngine)>>,
    eval_in_progress: bool,
    key_cursor: bevy::ecs::message::MessageCursor<KeyboardInput>,
    last_width: u32,
    last_height: u32,
    ctrlc_flag: Arc<AtomicBool>,
    force_full_render: bool,
}

// --- Nushell engine initialization ---

fn init_nushell_engine() -> NuShellEngine {
    let engine_state = create_default_context();
    let mut engine_state = add_shell_command_context(engine_state);

    let home = std::env::var("HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("/"));
    // Always start in HOME (DMG apps have CWD=/ which is not useful)
    let _ = std::env::set_current_dir(&home);

    // macOS apps launched from Finder/DMG get a minimal PATH.
    // Ensure common dev directories are included.
    {
        let current_path = std::env::var("PATH").unwrap_or_default();
        let home_str = home.to_string_lossy();
        let extra_paths = [
            format!("{}/.cargo/bin", home_str),
            "/opt/homebrew/bin".to_string(),
            "/opt/homebrew/sbin".to_string(),
            "/usr/local/bin".to_string(),
            "/usr/local/sbin".to_string(),
            format!("{}/.local/bin", home_str),
            format!("{}/go/bin", home_str),
            format!("{}/.deno/bin", home_str),
        ];
        let mut paths: Vec<&str> = extra_paths.iter().map(|s| s.as_str()).collect();
        for p in current_path.split(':') {
            if !paths.contains(&p) {
                paths.push(p);
            }
        }
        // SAFETY: called once during init, before any threads are spawned
        unsafe { std::env::set_var("PATH", paths.join(":")); }
    }

    gather_parent_env_vars(&mut engine_state, &home);

    if let Err(e) = load_standard_library(&mut engine_state) {
        warn!("Failed to load nu standard library: {:?}", e);
    }

    let mut stack = Stack::new();

    eval_source(
        &mut engine_state,
        &mut stack,
        NU_ENV_SOURCE.as_bytes(),
        "env.nu",
        PipelineData::empty(),
        false,
    );

    eval_source(
        &mut engine_state,
        &mut stack,
        NU_CONFIG_SOURCE.as_bytes(),
        "config.nu",
        PipelineData::empty(),
        false,
    );

    // Force ANSI coloring
    {
        let mut config: nu_protocol::Config = (*engine_state.get_config()).as_ref().clone();
        config.use_ansi_coloring = nu_protocol::config::UseAnsiColoring::True;
        engine_state.set_config(config);
    }

    if let Err(e) = nu_engine::env::convert_env_values(&mut engine_state, &mut stack) {
        warn!("Failed to convert env values: {:?}", e);
    }

    info!("Nushell engine initialized");
    NuShellEngine { engine_state, stack }
}

fn wire_ctrlc_signal(engine: &mut NuShellEngine, flag: Arc<AtomicBool>) {
    engine.engine_state.set_signals(Signals::new(flag));
}

// --- Window resize handling ---

/// Resize sugarloaf + term grid. Returns true if dimensions actually changed.
fn handle_resize(state: &mut TerminalNonSendState, new_width: u32, new_height: u32) -> bool {
    if (new_width, new_height) == (state.last_width, state.last_height) {
        return false;
    }
    if new_width == 0 || new_height == 0 {
        return false; // minimized
    }

    // 1. Update sugarloaf context dimensions
    state.sugarloaf.resize(new_width, new_height);

    // 2. Recreate offscreen texture at new size
    state.sugarloaf.ctx.enable_offscreen();

    // 3. Recompute grid cols/rows from cell dimensions
    let dims = state.sugarloaf.get_rich_text_dimensions(&state.rich_text_id);
    let cell_w = if dims.width > 0.0 { dims.width } else { 9.0 };
    let cell_h = if dims.height > 0.0 { dims.height } else { 18.0 };

    let new_cols = (new_width as f32 / cell_w).floor().max(2.0) as usize;
    let new_rows = (new_height as f32 / cell_h).floor().max(1.0) as usize;

    // 4. Resize alacritty terminal grid if needed
    if new_cols != state.cols || new_rows != state.rows {
        let term_dims = TermDimensions { cols: new_cols, lines: new_rows };
        let mut term = state.term.lock();
        term.resize(term_dims);
        state.cols = new_cols;
        state.rows = new_rows;
        info!("Terminal resized to {}x{} ({}x{}px)", new_cols, new_rows, new_width, new_height);
    }

    // 5. Update stored dimensions
    state.last_width = new_width;
    state.last_height = new_height;
    state.force_full_render = true;
    true
}

fn check_resize(world: &mut World) {
    let win_dims: Option<(u32, u32, f32, f32)> = world
        .query_filtered::<Entity, With<PrimaryWindow>>()
        .single(world)
        .ok()
        .map(|e| {
            let w = world.get::<Window>(e).unwrap();
            (w.physical_width(), w.physical_height(), w.width(), w.height())
        });
    let Some((phys_w, phys_h, logical_w, logical_h)) = win_dims else { return };

    // Phase 1: resize sugarloaf + term (only borrows NonSend, uses physical dims)
    let (resized, image_handle) = {
        let state = world.get_non_send_resource_mut::<TerminalNonSendState>().unwrap().into_inner();
        let resized = handle_resize(state, phys_w, phys_h);
        (resized, state.image_handle.clone())
    };

    if !resized { return; }

    // Phase 2: update Bevy Image at physical size (borrows Assets<Image>)
    {
        let mut images = world.resource_mut::<Assets<Image>>();
        if let Some(image) = images.get_mut(&image_handle) {
            *image = Image::new_fill(
                bevy::render::render_resource::Extent3d {
                    width: phys_w,
                    height: phys_h,
                    depth_or_array_layers: 1,
                },
                bevy::render::render_resource::TextureDimension::D2,
                &[0, 0, 0, 255],
                bevy::render::render_resource::TextureFormat::Bgra8UnormSrgb,
                RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
            );
        }
    }

    // Phase 3: update Sprite custom_size at logical size (borrows ECS components)
    {
        let mut sprites = world.query_filtered::<&mut Sprite, With<TerminalMarker>>();
        for mut sprite in sprites.iter_mut(world) {
            sprite.custom_size = Some(Vec2::new(logical_w, logical_h));
        }
    }
}

// --- Evaluate nushell command and capture output as bytes ---

fn evaluate_and_capture(engine: &mut NuShellEngine, input: &str) -> EvalResult {
    let input_bytes = input.as_bytes();

    // Parse
    let mut working_set = StateWorkingSet::new(&engine.engine_state);
    let block = parse(&mut working_set, Some("input"), input_bytes, false);

    if let Some(err) = working_set.parse_errors.first() {
        return EvalResult {
            output: Vec::new(),
            error: Some(format!("Parse error: {:?}", err)),
        };
    }

    let delta = working_set.render();
    if let Err(e) = engine.engine_state.merge_delta(delta) {
        return EvalResult {
            output: Vec::new(),
            error: Some(format!("Merge error: {:?}", e)),
        };
    }

    // Redirect stdout/stderr to Pipe so external commands output through PipelineData
    // (instead of writing to real stdout/stderr)
    let pipeline_data = {
        let mut guard = engine.stack.push_redirection(
            Some(Redirection::Pipe(OutDest::Pipe)),
            Some(Redirection::Pipe(OutDest::Pipe)),
        );

        let result = eval_block::<WithoutDebug>(
            &engine.engine_state,
            &mut guard,
            &block,
            PipelineData::empty(),
        );

        match result {
            Ok(exec_data) => exec_data.body,
            Err(e) => {
                return EvalResult {
                    output: Vec::new(),
                    error: Some(format!("{:?}", e)),
                };
            }
        }
        // guard drops here, restoring stack redirection state
    };

    // Capture output — pipe through `table` command for proper formatting
    let output = capture_pipeline_output(pipeline_data, engine);

    // Merge env changes (cd, export, etc.)
    if let Err(e) = engine.engine_state.merge_env(&mut engine.stack) {
        warn!("Failed to merge env: {:?}", e);
    }

    EvalResult {
        output,
        error: None,
    }
}

fn capture_pipeline_output(data: PipelineData, engine: &mut NuShellEngine) -> Vec<u8> {
    match data {
        PipelineData::Empty => Vec::new(),
        PipelineData::Value(Value::Nothing { .. }, _) => Vec::new(),
        PipelineData::ByteStream(stream, _) => {
            match stream.into_bytes() {
                Ok(bytes) => bytes,
                Err(e) => format!("Error: {}", e).into_bytes(),
            }
        }
        PipelineData::Value(Value::String { val, .. }, _) => val.into_bytes(),
        // For structured data (records, lists, tables) — pipe through `table` command
        other => {
            pipe_through_table(other, engine)
        }
    }
}

fn pipe_through_table(data: PipelineData, engine: &mut NuShellEngine) -> Vec<u8> {
    // Find the `table` command declaration
    if let Some(decl_id) = engine.engine_state.table_decl_id {
        let command = engine.engine_state.get_decl(decl_id);
        if command.block_id().is_none() {
            let call = nu_protocol::ast::Call::new(nu_protocol::Span::new(0, 0));
            match command.run(
                &engine.engine_state,
                &mut engine.stack,
                &(&call).into(),
                data,
            ) {
                Ok(table_output) => {
                    // Collect the table output as a string
                    let config = (*engine.engine_state.get_config()).as_ref().clone();
                    match table_output.collect_string("\n", &config) {
                        Ok(s) => return s.into_bytes(),
                        Err(e) => return format!("Table error: {}", e).into_bytes(),
                    }
                }
                Err(e) => return format!("Table error: {}", e).into_bytes(),
            }
        }
    }

    // Fallback: use to_expanded_string
    let config = engine.engine_state.get_config();
    let mut output = Vec::new();
    for item in data {
        let s = item.to_expanded_string("\n", &config);
        output.extend_from_slice(s.as_bytes());
        output.push(b'\n');
    }
    output
}

// --- Evaluate prompt closures ---

fn evaluate_prompt(engine: &mut NuShellEngine) -> Vec<u8> {
    let mut prompt_bytes = Vec::new();

    // Evaluate $env.PROMPT_COMMAND
    if let Some(prompt_cmd) = get_env_closure(&engine.engine_state, &engine.stack, "PROMPT_COMMAND") {
        match ClosureEvalOnce::new(&engine.engine_state, &engine.stack, prompt_cmd)
            .run_with_input(PipelineData::empty())
        {
            Ok(data) => {
                let config = (*engine.engine_state.get_config()).clone();
                if let Ok(s) = data.collect_string("", &config) {
                    prompt_bytes.extend_from_slice(s.as_bytes());
                }
            }
            Err(e) => {
                warn!("Prompt command error: {:?}", e);
                prompt_bytes.extend_from_slice(b"> ");
            }
        }
    } else {
        prompt_bytes.extend_from_slice(b"> ");
    }

    // Evaluate $env.PROMPT_INDICATOR
    if let Some(indicator) = get_env_closure(&engine.engine_state, &engine.stack, "PROMPT_INDICATOR") {
        match ClosureEvalOnce::new(&engine.engine_state, &engine.stack, indicator)
            .run_with_input(PipelineData::empty())
        {
            Ok(data) => {
                let config = (*engine.engine_state.get_config()).clone();
                if let Ok(s) = data.collect_string("", &config) {
                    prompt_bytes.extend_from_slice(s.as_bytes());
                }
            }
            Err(_) => {}
        }
    }

    prompt_bytes
}

fn get_env_closure(engine_state: &EngineState, stack: &Stack, var_name: &str) -> Option<Closure> {
    let val = stack.get_env_var(engine_state, var_name)
        .or_else(|| engine_state.get_env_var(var_name))?;

    match val {
        Value::Closure { val, .. } => Some(*val.clone()),
        _ => None,
    }
}

// --- Feed bytes into alacritty Term ---

fn feed_term(term: &Arc<FairMutex<Term<BevyEventProxy>>>, processor: &mut Processor, bytes: &[u8]) {
    let mut t = term.lock();
    processor.advance(&mut *t, bytes);
}

// --- Setup ---

fn setup_terminal(world: &mut World) {
    // If terminal state already exists, restore rendering and return (persist state)
    if world.get_non_send_resource::<TerminalNonSendState>().is_some() {
        // Handle resize if window changed while terminal was inactive
        check_resize(world);

        // Get logical window size for sprite
        let logical_size: Option<(f32, f32)> = world
            .query_filtered::<Entity, With<PrimaryWindow>>()
            .single(world)
            .ok()
            .map(|e| {
                let w = world.get::<Window>(e).unwrap();
                (w.width(), w.height())
            });

        // Read current state for spawning entities
        let image_handle = {
            let state = world.get_non_send_resource_mut::<TerminalNonSendState>().unwrap().into_inner();
            state.force_full_render = true;
            state.image_handle.clone()
        };

        let (lw, lh) = logical_size.unwrap_or((1280.0, 800.0));

        // Re-spawn camera + sprite for this world
        world.spawn((
            TerminalMarker,
            Camera2d,
            Camera {
                clear_color: ClearColorConfig::Custom(bevy::color::Color::BLACK),
                ..default()
            },
            Tonemapping::None,
        ));
        world.spawn((
            TerminalMarker,
            Sprite {
                image: image_handle,
                custom_size: Some(Vec2::new(lw, lh)),
                ..default()
            },
        ));
        info!("Terminal resumed (state persisted)");
        return;
    }

    let primary_entity = world
        .query_filtered::<Entity, With<PrimaryWindow>>()
        .single(world);
    let Ok(entity) = primary_entity else { return };

    let (win_w, win_h, logical_w, logical_h, scale_factor) = {
        let window = world.get::<Window>(entity).unwrap();
        (
            window.physical_width(),
            window.physical_height(),
            window.width(),
            window.height(),
            window.scale_factor(),
        )
    };

    // Get Bevy's GPU resources (cloned into main world by GpuBridgePlugin)
    let device = world.resource::<bevy::render::renderer::RenderDevice>().wgpu_device().clone();
    let queue: sugarloaf::wgpu::Queue = {
        let rq = world.resource::<bevy::render::renderer::RenderQueue>();
        let inner: &sugarloaf::wgpu::Queue = &**rq;
        inner.clone()
    };

    // Create Sugarloaf with Bevy's device/queue (no separate GPU stack)
    let surface_format = sugarloaf::wgpu::TextureFormat::Bgra8UnormSrgb;
    let ctx = SugarloafContext::new_external(
        device,
        queue,
        surface_format,
        SugarloafWindowSize {
            width: win_w as f32,
            height: win_h as f32,
        },
        scale_factor,
    );

    let (font_library, _font_errors) = FontLibrary::new(Default::default());
    let layout = RootStyle::new(scale_factor, FONT_SIZE, 1.0);

    let mut sugarloaf = match Sugarloaf::new_with_context(ctx, &font_library, layout) {
        Ok(s) => s,
        Err(e) => {
            warn!("Failed to create Sugarloaf: {:?}", e);
            return;
        }
    };

    // Enable offscreen rendering (creates offscreen texture + readback buffer)
    sugarloaf.ctx.enable_offscreen();

    let rich_text_id = sugarloaf.create_rich_text();
    sugarloaf.set_background_color(Some(sugarloaf::wgpu::Color {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    }));

    let dims = sugarloaf.get_rich_text_dimensions(&rich_text_id);
    let cell_w = if dims.width > 0.0 { dims.width } else { 9.0 };
    let cell_h = if dims.height > 0.0 { dims.height } else { 18.0 };

    let cols = (win_w as f32 / cell_w).floor().max(2.0) as usize;
    let rows = (win_h as f32 / cell_h).floor().max(1.0) as usize;

    // Create alacritty terminal grid (NO PTY)
    let config = Config::default();
    let term_dims = TermDimensions { cols, lines: rows };
    let term = Arc::new(FairMutex::new(Term::new(config, &term_dims, BevyEventProxy)));
    let mut processor = Processor::new();

    // Initialize nushell engine
    let ctrlc_flag = Arc::new(AtomicBool::new(false));
    let mut nu_engine = init_nushell_engine();
    wire_ctrlc_signal(&mut nu_engine, ctrlc_flag.clone());

    // Render initial prompt
    let prompt_bytes = evaluate_prompt(&mut nu_engine);
    {
        let mut t = term.lock();
        processor.advance(&mut *t, &prompt_bytes);
    }

    // Create Bevy Image (Bgra8UnormSrgb, matching sugarloaf output)
    let image = Image::new_fill(
        bevy::render::render_resource::Extent3d {
            width: win_w,
            height: win_h,
            depth_or_array_layers: 1,
        },
        bevy::render::render_resource::TextureDimension::D2,
        &[0, 0, 0, 255], // BGRA dark background
        bevy::render::render_resource::TextureFormat::Bgra8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    let image_handle = world.resource_mut::<Assets<Image>>().add(image);

    // Spawn Camera2d + fullscreen Sprite (tonemapping disabled for accurate terminal colors)
    world.spawn((
        TerminalMarker,
        Camera2d,
        Camera {
            clear_color: ClearColorConfig::Custom(bevy::color::Color::BLACK),
            ..default()
        },
        Tonemapping::None,
    ));
    world.spawn((
        TerminalMarker,
        Sprite {
            image: image_handle.clone(),
            custom_size: Some(Vec2::new(logical_w, logical_h)),
            ..default()
        },
    ));

    world.insert_non_send_resource(TerminalNonSendState {
        nu_engine: Some(nu_engine),
        term,
        processor,
        line_buffer: LineBuffer::new(),
        cols,
        rows,
        rich_text_id,
        sugarloaf,
        image_handle,
        eval_rx: None,
        eval_in_progress: false,
        key_cursor: Default::default(),
        last_width: win_w,
        last_height: win_h,
        ctrlc_flag,
        force_full_render: true,
    });

    info!(
        "Terminal created ({}x{}) cell={:.0}x{:.0} (embedded nushell + sugarloaf → Bevy Sprite)",
        cols, rows, cell_w, cell_h
    );
}

// --- Single exclusive update system (handles input, poll, render) ---

fn terminal_update(world: &mut World) {
    // Deferred init: if OnEnter fired before window was ready, retry here
    if world.get_non_send_resource::<TerminalNonSendState>().is_none() {
        setup_terminal(world);
        return; // skip this frame, render next frame
    }

    // Check for window resize
    check_resize(world);

    // Process keyboard input
    process_keyboard_input(world);

    // Process mouse wheel scroll
    process_scroll_input(world);

    // Poll eval results
    poll_eval_results(world);

    // Render
    render_terminal(world);
}

fn process_scroll_input(world: &mut World) {
    let scroll_delta = {
        let accumulated = world.resource::<AccumulatedMouseScroll>();
        if accumulated.delta.y == 0.0 {
            return;
        }
        let lines = match accumulated.unit {
            MouseScrollUnit::Line => accumulated.delta.y as i32,
            MouseScrollUnit::Pixel => (accumulated.delta.y / 20.0) as i32,
        };
        lines
    };

    let Some(state) = world.get_non_send_resource::<TerminalNonSendState>() else { return };
    let mut term = state.term.lock();
    term.scroll_display(Scroll::Delta(scroll_delta));
}

fn process_keyboard_input(world: &mut World) {
    // Clone cursor from state, read new messages, update cursor back
    let Some(state_ref) = world.get_non_send_resource::<TerminalNonSendState>() else { return };
    let mut cursor = state_ref.key_cursor.clone();
    drop(state_ref);

    let events: Vec<KeyboardInput> = {
        let messages = world.resource::<bevy::ecs::message::Messages<KeyboardInput>>();
        cursor.read(messages).cloned().collect()
    };

    // Check modifier keys
    let (cmd_held, ctrl_held) = {
        let keys = world.resource::<ButtonInput<KeyCode>>();
        (
            keys.pressed(KeyCode::SuperLeft) || keys.pressed(KeyCode::SuperRight),
            keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight),
        )
    };

    let Some(state) = world.get_non_send_resource_mut::<TerminalNonSendState>() else { return };
    let state = state.into_inner();
    state.key_cursor = cursor;

    for event in &events {
        if !event.state.is_pressed() {
            continue;
        }

        // Skip when Cmd is held — these are hotkey combos, not terminal input
        if cmd_held {
            continue;
        }

        if state.eval_in_progress {
            // Only handle Ctrl+C during eval — interrupt the running command
            if ctrl_held {
                if let Key::Character(c) = &event.logical_key {
                    if c.as_str() == "c" {
                        state.ctrlc_flag.store(true, Ordering::Relaxed);
                        feed_term(&state.term, &mut state.processor, b"^C\r\n");
                        info!("Ctrl+C: signaling interrupt to nushell eval");
                    }
                }
            }
            continue;
        }

        match &event.logical_key {
            Key::Character(c) => {
                // Auto-scroll to bottom when typing
                {
                    let mut term = state.term.lock();
                    term.scroll_display(Scroll::Bottom);
                }
                let s = c.as_str();
                for ch in s.chars() {
                    state.line_buffer.insert_char(ch);
                    let bytes = ch.to_string().into_bytes();
                    feed_term(&state.term, &mut state.processor, &bytes);
                }
            }
            Key::Enter => {
                // Auto-scroll to bottom on Enter
                {
                    let mut term = state.term.lock();
                    term.scroll_display(Scroll::Bottom);
                }
                feed_term(&state.term, &mut state.processor, b"\r\n");

                let input = state.line_buffer.take_line();
                if input.trim().is_empty() {
                    if let Some(ref mut engine) = state.nu_engine {
                        let prompt = evaluate_prompt(engine);
                        feed_term(&state.term, &mut state.processor, &prompt);
                    }
                } else {
                    dispatch_eval(state, input);
                }
            }
            Key::Backspace => {
                if state.line_buffer.backspace() {
                    feed_term(&state.term, &mut state.processor, b"\x08 \x08");
                }
            }
            Key::ArrowUp => {
                if state.line_buffer.history_up() {
                    redraw_line_buffer(state);
                }
            }
            Key::ArrowDown => {
                if state.line_buffer.history_down() {
                    redraw_line_buffer(state);
                }
            }
            Key::ArrowLeft => {
                if state.line_buffer.cursor_pos > 0 {
                    state.line_buffer.cursor_pos -= 1;
                    feed_term(&state.term, &mut state.processor, b"\x1b[D");
                }
            }
            Key::ArrowRight => {
                if state.line_buffer.cursor_pos < state.line_buffer.buffer.len() {
                    state.line_buffer.cursor_pos += 1;
                    feed_term(&state.term, &mut state.processor, b"\x1b[C");
                }
            }
            Key::Tab => {
                state.line_buffer.insert_char('\t');
                feed_term(&state.term, &mut state.processor, b"\t");
            }
            Key::Escape => {
                state.line_buffer.buffer.clear();
                state.line_buffer.cursor_pos = 0;
                feed_term(&state.term, &mut state.processor, b"\r\x1b[K");
                if let Some(ref mut engine) = state.nu_engine {
                    let prompt = evaluate_prompt(engine);
                    feed_term(&state.term, &mut state.processor, &prompt);
                }
            }
            Key::Space => {
                state.line_buffer.insert_char(' ');
                feed_term(&state.term, &mut state.processor, b" ");
            }
            Key::PageUp => {
                let mut term = state.term.lock();
                term.scroll_display(Scroll::PageUp);
            }
            Key::PageDown => {
                let mut term = state.term.lock();
                term.scroll_display(Scroll::PageDown);
            }
            Key::Home => {
                let mut term = state.term.lock();
                term.scroll_display(Scroll::Top);
            }
            Key::End => {
                let mut term = state.term.lock();
                term.scroll_display(Scroll::Bottom);
            }
            _ => {}
        }
    }
}

fn redraw_line_buffer(state: &mut TerminalNonSendState) {
    feed_term(&state.term, &mut state.processor, b"\r\x1b[K");
    if let Some(ref mut engine) = state.nu_engine {
        let prompt = evaluate_prompt(engine);
        feed_term(&state.term, &mut state.processor, &prompt);
    }
    let buf = state.line_buffer.buffer.clone();
    if !buf.is_empty() {
        feed_term(&state.term, &mut state.processor, buf.as_bytes());
    }
}

fn dispatch_eval(state: &mut TerminalNonSendState, input: String) {
    let Some(engine) = state.nu_engine.take() else {
        warn!("No nushell engine available for eval");
        return;
    };

    state.eval_in_progress = true;

    let (tx, rx) = std::sync::mpsc::channel();
    state.eval_rx = Some(rx);

    std::thread::spawn(move || {
        let mut engine = engine;
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            evaluate_and_capture(&mut engine, &input)
        })) {
            Ok(result) => {
                let _ = tx.send((result, engine));
            }
            Err(panic_info) => {
                let panic_msg = if let Some(s) = panic_info.downcast_ref::<&str>() {
                    s.to_string()
                } else if let Some(s) = panic_info.downcast_ref::<String>() {
                    s.clone()
                } else {
                    "Unknown panic".to_string()
                };
                let result = EvalResult {
                    output: Vec::new(),
                    error: Some(format!("Internal panic: {}", panic_msg)),
                };
                let _ = tx.send((result, engine));
            }
        }
    });
}

fn poll_eval_results(world: &mut World) {
    let Some(state) = world.get_non_send_resource_mut::<TerminalNonSendState>() else { return };
    let state = state.into_inner();

    if !state.eval_in_progress {
        return;
    }

    let result: (EvalResult, NuShellEngine) = {
        let Some(ref rx) = state.eval_rx else { return };
        match rx.try_recv() {
            Ok(result) => result,
            Err(std::sync::mpsc::TryRecvError::Empty) => return,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                warn!("Eval thread lost — rebuilding nushell engine");
                state.eval_in_progress = false;
                state.eval_rx = None;
                state.ctrlc_flag.store(false, Ordering::Relaxed);

                feed_term(
                    &state.term,
                    &mut state.processor,
                    b"\x1b[31mError: command execution failed, engine restarted\x1b[0m\r\n",
                );

                let mut engine = init_nushell_engine();
                wire_ctrlc_signal(&mut engine, state.ctrlc_flag.clone());
                let prompt = evaluate_prompt(&mut engine);
                feed_term(&state.term, &mut state.processor, &prompt);
                state.nu_engine = Some(engine);
                return;
            }
        }
    };

    let (eval_result, mut engine) = result;
    state.eval_rx = None;
    state.eval_in_progress = false;
    state.ctrlc_flag.store(false, Ordering::Relaxed);
    engine.engine_state.reset_signals();

    // Feed output into terminal
    if !eval_result.output.is_empty() {
        let output = convert_lf_to_crlf(&eval_result.output);
        feed_term(&state.term, &mut state.processor, &output);
        if !eval_result.output.ends_with(b"\n") {
            feed_term(&state.term, &mut state.processor, b"\r\n");
        }
    }

    // Show error if any
    if let Some(ref err) = eval_result.error {
        let err_msg = format!("\x1b[31mError: {}\x1b[0m\r\n", err);
        feed_term(&state.term, &mut state.processor, err_msg.as_bytes());
    }

    // Render prompt
    let prompt = evaluate_prompt(&mut engine);
    feed_term(&state.term, &mut state.processor, &prompt);

    state.nu_engine = Some(engine);
}


fn render_terminal(world: &mut World) {
    // Phase 1: build sugarloaf content + offscreen render + readback (borrows NonSend)
    let (image_handle, pixels) = {
        let Some(state) = world.get_non_send_resource_mut::<TerminalNonSendState>() else { return };
        let state = state.into_inner();

        // Check if terminal content has changed (dirty tracking)
        let needs_rebuild = {
            let mut term = state.term.lock();
            let needs = match term.damage() {
                TermDamage::Full => true,
                TermDamage::Partial(mut iter) => iter.next().is_some(),
            };
            term.reset_damage();
            needs || state.force_full_render
        };

        if !needs_rebuild {
            return; // Bevy handles presentation — nothing to do
        }

        state.force_full_render = false;

        render_terminal_content(state);

        // Render sugarloaf to offscreen texture + CPU readback
        let pixels = {
            let view = state.sugarloaf.ctx.offscreen_view();
            if let Some(ref view) = view {
                state.sugarloaf.render_to_view(view);
            }
            state.sugarloaf.ctx.read_offscreen_pixels()
        };

        (state.image_handle.clone(), pixels)
    }; // NonSend borrow released here

    // Phase 2: update Bevy Image data (borrows Assets<Image>)
    if let Some(pixels) = pixels {
        let mut images = world.resource_mut::<Assets<Image>>();
        if let Some(image) = images.get_mut(&image_handle) {
            if let Some(ref mut data) = image.data {
                if data.len() == pixels.len() {
                    data.copy_from_slice(&pixels);
                }
            }
        }
    }
}

/// Build sugarloaf content from terminal grid (called within NonSend borrow scope)
fn render_terminal_content(state: &mut TerminalNonSendState) {
    let sugarloaf = &mut state.sugarloaf;
    let rt_id = state.rich_text_id;

    let mut cursor_col: usize = 0;
    let mut cursor_row: i32 = -1;
    let mut cursor_shape = CursorShape::Block;

    {
        let term = state.term.lock();
        let content = term.renderable_content();

        sugarloaf.content().sel(rt_id).clear();

        let display_offset = content.display_offset as i32;
        let mut current_line: i32 = -1;
        for indexed in content.display_iter {
            let col = indexed.point.column.0;
            let term_row = indexed.point.line.0;
            let viewport_row = term_row + display_offset;

            if viewport_row < 0 || col >= state.cols || viewport_row as usize >= state.rows {
                continue;
            }

            if term_row != current_line {
                sugarloaf.content().sel(rt_id).new_line();
                current_line = term_row;
            }

            let cell = &*indexed;
            let mut fg_color = cell.fg;
            let mut bg_color = cell.bg;

            if cell.flags.contains(Flags::INVERSE) {
                std::mem::swap(&mut fg_color, &mut bg_color);
            }

            let fg = resolve_color(fg_color, content.colors);
            let bg = resolve_color(bg_color, content.colors);

            let fg = if cell.flags.contains(Flags::BOLD) {
                match fg_color {
                    Color::Named(NamedColor::Black) => default_ansi_rgb(NamedColor::BrightBlack),
                    Color::Named(NamedColor::Red) => default_ansi_rgb(NamedColor::BrightRed),
                    Color::Named(NamedColor::Green) => default_ansi_rgb(NamedColor::BrightGreen),
                    Color::Named(NamedColor::Yellow) => default_ansi_rgb(NamedColor::BrightYellow),
                    Color::Named(NamedColor::Blue) => default_ansi_rgb(NamedColor::BrightBlue),
                    Color::Named(NamedColor::Magenta) => default_ansi_rgb(NamedColor::BrightMagenta),
                    Color::Named(NamedColor::Cyan) => default_ansi_rgb(NamedColor::BrightCyan),
                    Color::Named(NamedColor::White) => default_ansi_rgb(NamedColor::BrightWhite),
                    _ => fg,
                }
            } else {
                fg
            };

            let mut style = FragmentStyle {
                color: rgb_to_f32(fg),
                background_color: Some(rgb_to_f32(bg)),
                ..Default::default()
            };

            if cell.flags.contains(Flags::BOLD) {
                style.font_attrs = sugarloaf::font_introspector::Attributes::new(
                    sugarloaf::Stretch::NORMAL,
                    sugarloaf::Weight(700),
                    sugarloaf::Style::Normal,
                );
            }

            if cell.flags.contains(Flags::ITALIC) {
                style.font_attrs = sugarloaf::font_introspector::Attributes::new(
                    sugarloaf::Stretch::NORMAL,
                    if cell.flags.contains(Flags::BOLD) {
                        sugarloaf::Weight(700)
                    } else {
                        sugarloaf::Weight(400)
                    },
                    sugarloaf::Style::Italic,
                );
            }

            if cell.flags.contains(Flags::UNDERLINE) {
                style.decoration = Some(FragmentStyleDecoration::Underline(UnderlineInfo {
                    is_doubled: false,
                    shape: UnderlineShape::Regular,
                }));
            }

            if cell.flags.contains(Flags::STRIKEOUT) {
                style.decoration = Some(FragmentStyleDecoration::Strikethrough);
            }

            let ch = cell.c;
            if let Some(drawable) = sugarloaf::drawable_character(ch) {
                style.drawable_char = Some(drawable);
            }

            let text_owned;
            let text_str = if ch == '\0' || ch == ' ' {
                " "
            } else {
                text_owned = ch.to_string();
                text_owned.as_str()
            };

            sugarloaf.content().sel(rt_id).add_text(text_str, style);
        }

        sugarloaf.content().build();

        cursor_col = content.cursor.point.column.0;
        cursor_row = content.cursor.point.line.0 + display_offset;
        cursor_shape = content.cursor.shape;
    }

    if cursor_row >= 0 && (cursor_row as usize) < state.rows && cursor_col < state.cols {
        let dims = sugarloaf.get_rich_text_dimensions(&rt_id);
        let cell_w = if dims.width > 0.0 { dims.width } else { 9.0 };
        let cell_h = if dims.height > 0.0 { dims.height } else { 18.0 };

        let cursor_color = [0.9, 0.9, 0.9, 0.7];
        let cx = cursor_col as f32 * cell_w;
        let cy = cursor_row as f32 * cell_h;

        let cursor_quad = match cursor_shape {
            CursorShape::Block => sugarloaf::Quad {
                position: [cx, cy],
                size: [cell_w, cell_h],
                color: cursor_color,
                border_color: [0.0; 4],
                border_radius: [0.0; 4],
                border_width: 0.0,
                shadow_color: [0.0; 4],
                shadow_offset: [0.0; 2],
                shadow_blur_radius: 0.0,
            },
            CursorShape::Beam => sugarloaf::Quad {
                position: [cx, cy],
                size: [2.0, cell_h],
                color: cursor_color,
                border_color: [0.0; 4],
                border_radius: [0.0; 4],
                border_width: 0.0,
                shadow_color: [0.0; 4],
                shadow_offset: [0.0; 2],
                shadow_blur_radius: 0.0,
            },
            CursorShape::Underline => sugarloaf::Quad {
                position: [cx, cy + cell_h - 2.0],
                size: [cell_w, 2.0],
                color: cursor_color,
                border_color: [0.0; 4],
                border_radius: [0.0; 4],
                border_width: 0.0,
                shadow_color: [0.0; 4],
                shadow_offset: [0.0; 2],
                shadow_blur_radius: 0.0,
            },
            _ => sugarloaf::Quad {
                position: [cx, cy],
                size: [cell_w, cell_h],
                color: [0.0; 4],
                border_color: cursor_color,
                border_radius: [0.0; 4],
                border_width: 1.0,
                shadow_color: [0.0; 4],
                shadow_offset: [0.0; 2],
                shadow_blur_radius: 0.0,
            },
        };

        sugarloaf.set_objects(vec![
            Object::RichText(RichText {
                id: rt_id,
                position: [0.0, 0.0],
                lines: None,
            }),
            Object::Quad(cursor_quad),
        ]);
    } else {
        sugarloaf.set_objects(vec![Object::RichText(RichText {
            id: rt_id,
            position: [0.0, 0.0],
            lines: None,
        })]);
    }
}

fn convert_lf_to_crlf(input: &[u8]) -> Vec<u8> {
    let mut output = Vec::with_capacity(input.len() + input.len() / 10);
    for i in 0..input.len() {
        if input[i] == b'\n' && (i == 0 || input[i - 1] != b'\r') {
            output.push(b'\r');
        }
        output.push(input[i]);
    }
    output
}

// --- Destroy ---

fn destroy_terminal(world: &mut World) {
    // Despawn camera + sprite (Bevy naturally shows other worlds)
    let entities: Vec<Entity> = world
        .query_filtered::<Entity, With<TerminalMarker>>()
        .iter(world)
        .collect();
    for entity in entities {
        world.despawn(entity);
    }

    // Keep TerminalNonSendState alive — state persists between world switches
    info!("Terminal paused (state persisted)");
}
