pub mod nushell_to_stream;
use nushell_to_stream::{pipeline_to_chunks, StreamMsg};

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use bevy::ecs::system::SystemState;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;

use nu_cli::{gather_parent_env_vars, eval_source};
use nu_cmd_lang::create_default_context;
use nu_command::add_shell_command_context;
use nu_engine::env::convert_env_values;
use nu_parser::parse;
use nu_protocol::engine::{EngineState, Redirection, Stack, StateWorkingSet};
use nu_protocol::debugger::WithoutDebug;
use nu_protocol::{OutDest, PipelineData, Signals};
use nu_std::load_standard_library;

use tape::Chunk;
use prysm::{StreamScrollback, dispatch, theme};

use super::WorldState;
use crate::shell::chrome::{CHROME_TOP_H, CHROME_BOTTOM_H};

const CONTENT_RATIO: f32 = 0.62;
const G: f32 = theme::G;

const NU_ENV_SOURCE: &str = include_str!("../../../assets/nu-config/env.nu");
const NU_CONFIG_SOURCE: &str = include_str!("../../../assets/nu-config/config.nu");

// ── Plugin ────────────────────────────────────────────────────────────────────

pub struct ComWorldPlugin;

#[derive(Component)]
struct PromptLabel;

#[derive(Component)]
struct LineBufferDisplay;

impl Plugin for ComWorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(WorldState::Com), setup_terminal)
            .add_systems(OnExit(WorldState::Com), destroy_terminal)
            .add_systems(
                Update,
                terminal_update.run_if(in_state(WorldState::Com)),
            );
    }
}

// ── Nushell engine ────────────────────────────────────────────────────────────

struct NuShellEngine {
    engine_state: EngineState,
    stack: Stack,
}

// ── Line buffer ───────────────────────────────────────────────────────────────

struct LineBuffer {
    buffer: String,
    cursor_pos: usize,
    history: Vec<String>,
    history_index: Option<usize>,
}

impl LineBuffer {
    fn new() -> Self {
        Self { buffer: String::new(), cursor_pos: 0, history: Vec::new(), history_index: None }
    }

    fn insert_char(&mut self, ch: char) {
        self.buffer.insert(self.cursor_pos, ch);
        self.cursor_pos += ch.len_utf8();
    }

    fn backspace(&mut self) -> bool {
        if self.cursor_pos > 0 {
            let prev = self.buffer[..self.cursor_pos]
                .chars().last().map(|c| c.len_utf8()).unwrap_or(0);
            self.cursor_pos -= prev;
            self.buffer.remove(self.cursor_pos);
            true
        } else {
            false
        }
    }

    fn take_line(&mut self) -> String {
        let line = self.buffer.clone();
        if !line.trim().is_empty() { self.history.push(line.clone()); }
        self.buffer.clear();
        self.cursor_pos = 0;
        self.history_index = None;
        line
    }

    fn history_up(&mut self) -> bool {
        if self.history.is_empty() { return false; }
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

// ── NonSend state ─────────────────────────────────────────────────────────────

struct TerminalNonSendState {
    nu_engine: Option<NuShellEngine>,
    line_buffer: LineBuffer,
    eval_rx: Option<std::sync::mpsc::Receiver<StreamMsg>>,
    engine_rx: Option<std::sync::mpsc::Receiver<NuShellEngine>>,
    eval_in_progress: bool,
    ctrlc_flag: Arc<AtomicBool>,
    key_cursor: bevy::ecs::message::MessageCursor<KeyboardInput>,
    wheel_cursor: bevy::ecs::message::MessageCursor<MouseWheel>,
    // UI entity IDs (the tree is built once and hidden between visits)
    root_entity: Entity,
    scrollback_entity: Entity,
    scroll_area_entity: Entity,
    prompt_entity: Entity,
    input_entity: Entity,
    scroll_offset: f32,
}

// ── Nushell init ──────────────────────────────────────────────────────────────

fn init_nushell_engine() -> NuShellEngine {
    let engine_state = create_default_context();
    let mut engine_state = add_shell_command_context(engine_state);

    let home = std::env::var("HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("/"));
    let _ = std::env::set_current_dir(&home);

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
            if !paths.contains(&p) { paths.push(p); }
        }
        unsafe { std::env::set_var("PATH", paths.join(":")); }
    }

    gather_parent_env_vars(&mut engine_state, &home);

    if let Err(e) = load_standard_library(&mut engine_state) {
        warn!("Failed to load nu standard library: {:?}", e);
    }

    let mut stack = Stack::new();

    eval_source(&mut engine_state, &mut stack,
        NU_ENV_SOURCE.as_bytes(), "env.nu", PipelineData::empty(), false);
    eval_source(&mut engine_state, &mut stack,
        NU_CONFIG_SOURCE.as_bytes(), "config.nu", PipelineData::empty(), false);

    {
        let mut config: nu_protocol::Config = (*engine_state.get_config()).as_ref().clone();
        config.use_ansi_coloring = nu_protocol::config::UseAnsiColoring::False;
        engine_state.set_config(config);
    }

    if let Err(e) = convert_env_values(&mut engine_state, &mut stack) {
        warn!("Failed to convert env values: {:?}", e);
    }

    info!("Nushell engine initialized");
    NuShellEngine { engine_state, stack }
}

fn wire_ctrlc_signal(engine: &mut NuShellEngine, flag: Arc<AtomicBool>) {
    engine.engine_state.set_signals(Signals::new(flag));
}

/// Build the prompt text: cwd display (no ANSI, just text).
fn prompt_text(_engine: &NuShellEngine) -> String {
    let cwd = std::env::current_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "~".to_string());
    if let Ok(home) = std::env::var("HOME") {
        if cwd.starts_with(&home) {
            return format!("~{} ▸ ", &cwd[home.len()..]);
        }
    }
    format!("{} ▸ ", cwd)
}

// ── Eval thread ───────────────────────────────────────────────────────────────

fn evaluate_and_capture(
    engine: &mut NuShellEngine,
    input: &str,
    tx: &std::sync::mpsc::Sender<StreamMsg>,
) -> Option<String> {
    let input_bytes = input.as_bytes();

    let mut working_set = StateWorkingSet::new(&engine.engine_state);
    let block = parse(&mut working_set, Some("input"), input_bytes, false);

    if let Some(err) = working_set.parse_errors.first() {
        return Some(format!("Parse error: {:?}", err));
    }

    let delta = working_set.render();
    if let Err(e) = engine.engine_state.merge_delta(delta) {
        return Some(format!("Merge error: {:?}", e));
    }

    let pipeline_data = {
        let mut guard = engine.stack.push_redirection(
            Some(Redirection::Pipe(OutDest::Pipe)),
            Some(Redirection::Pipe(OutDest::Pipe)),
        );

        let result = nu_engine::eval_block::<WithoutDebug>(
            &engine.engine_state,
            &mut guard,
            &block,
            PipelineData::empty(),
        );

        match result {
            Ok(exec_data) => exec_data.body,
            Err(e) => {
                let _ = tx.send(StreamMsg::Chunk(Chunk::error(&e.to_string())));
                return None;
            }
        }
    };

    pipeline_to_chunks(pipeline_data, &mut engine.engine_state, &mut engine.stack, tx);
    None
}

fn dispatch_eval(state: &mut TerminalNonSendState, input: String) {
    // `rune <expr>` routes to the rune interpreter instead of nushell. Its
    // result noun decodes to the SAME tape chunks nushell emits, so the rest
    // of the pipeline (poll → dispatch → prysm) is identical. This is the
    // rune↔prysm seam, live in the terminal.
    if let Some(expr) = input.strip_prefix("rune ") {
        dispatch_rune(state, expr.to_string());
        return;
    }

    let Some(engine) = state.nu_engine.take() else {
        warn!("No nushell engine for eval");
        return;
    };

    state.eval_in_progress = true;
    state.scroll_offset = 100_000.0; // auto-scroll to bottom on new command
    let (tx, rx) = std::sync::mpsc::channel::<StreamMsg>();
    let (engine_tx, engine_rx) = std::sync::mpsc::channel::<NuShellEngine>();
    state.eval_rx = Some(rx);
    state.engine_rx = Some(engine_rx);

    std::thread::spawn(move || {
        let mut engine = engine;
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            evaluate_and_capture(&mut engine, &input, &tx)
        }));
        match outcome {
            Ok(error) => {
                // Send engine back before Done so it's available when Done arrives
                let _ = engine_tx.send(engine);
                let _ = tx.send(StreamMsg::Done { error });
            }
            Err(panic) => {
                let msg = if let Some(s) = panic.downcast_ref::<&str>() { s.to_string() }
                    else if let Some(s) = panic.downcast_ref::<String>() { s.clone() }
                    else { "unknown panic".to_string() };
                // Don't return engine on panic — it may be corrupt
                let _ = tx.send(StreamMsg::Done { error: Some(format!("panic: {msg}")) });
            }
        }
    });
}

// ── rune eval ───────────────────────────────────────────────────────────────

/// Evaluate a `rune <expr>` command. rune is instant-start (no compile phase),
/// so this runs synchronously on the main thread and pushes its chunks into the
/// same `StreamMsg` channel nushell uses. The engine is handed straight back
/// untouched so `poll_eval_results` restores it without reinitializing.
fn dispatch_rune(state: &mut TerminalNonSendState, expr: String) {
    let Some(engine) = state.nu_engine.take() else {
        warn!("No engine slot for rune eval");
        return;
    };
    state.eval_in_progress = true;
    state.scroll_offset = 100_000.0;
    let (tx, rx) = std::sync::mpsc::channel::<StreamMsg>();
    let (engine_tx, engine_rx) = std::sync::mpsc::channel::<NuShellEngine>();
    state.eval_rx = Some(rx);
    state.engine_rx = Some(engine_rx);
    let _ = engine_tx.send(engine); // rune never touches the nu engine

    let error = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        rune_eval_to_chunks(&expr, &tx)
    }))
    .unwrap_or_else(|_| Some("rune: panic during eval".to_string()));
    let _ = tx.send(StreamMsg::Done { error });
}

/// The terminal's rune `Host`: performs the `emit` act by decoding the chunk
/// noun and pushing it straight into the stream — so a program emits chunks *as
/// it runs*, not only at the end. Today emit is granted unconditionally (your
/// terminal = full trust); the ward will gate it on `~caps`. Other acts stub.
struct TerminalHost<'a> {
    tx: &'a std::sync::mpsc::Sender<StreamMsg>,
    emitted: usize,
}

impl rune_interp::Host for TerminalHost<'_> {
    fn perform(
        &mut self,
        act: u64,
        args: &rune_ast::Noun,
        _caps: &rune_ast::Noun,
    ) -> Result<rune_ast::Noun, rune_interp::InterpError> {
        if act == rune_ast::act::EMIT {
            for c in rune_prysm::noun_to_chunks(args) {
                self.emitted += 1;
                let _ = self.tx.send(StreamMsg::Chunk(c));
            }
        }
        // query/link/seal/host: stubbed until the ward + cybergraph land.
        Ok(rune_ast::Noun::Atom(0))
    }
}

/// Parse → lower → interpret a rune expression against a minimal subject,
/// performing `emit` acts live via `TerminalHost`. After evaluation, any chunks
/// the result *itself* decodes to (the `col(text(..))` style) are rendered too.
/// Returns `Some(msg)` on error.
fn rune_eval_to_chunks(
    expr: &str,
    tx: &std::sync::mpsc::Sender<StreamMsg>,
) -> Option<String> {
    let ast = match rune_parse::parse(expr) {
        Ok(a) => a,
        Err(e) => return Some(format!("rune parse error: {}", e.message)),
    };
    let formula = match rune_lower::lower(ast) {
        Ok(n) => n,
        Err(e) => return Some(format!("rune lower error: {}", e.message)),
    };
    let subject = rune_subject::Subject::minimal().to_noun();

    let mut host = TerminalHost { tx, emitted: 0 };
    let result = match rune_interp::eval_with_host(&subject, &formula, &mut host) {
        Ok(n) => n,
        Err(e) => return Some(format!("rune eval error: {}", e.message)),
    };

    let final_chunks = rune_prysm::noun_to_chunks(&result);
    for c in &final_chunks {
        let _ = tx.send(StreamMsg::Chunk(c.clone()));
    }
    // Nothing emitted and nothing returned to render → show the raw noun
    // (e.g. `rune add(2, 3)` → 5).
    if host.emitted == 0 && final_chunks.is_empty() {
        let _ = tx.send(StreamMsg::Chunk(Chunk::text(&noun_text(&result))));
    }
    None
}

/// Render a noun in `[head tail]` display form (atoms as decimals).
fn noun_text(n: &rune_ast::Noun) -> String {
    match n {
        rune_ast::Noun::Atom(a) => a.to_string(),
        rune_ast::Noun::Cell(h, t) => format!("[{} {}]", noun_text(h), noun_text(t)),
    }
}

// ── poll_eval_results ─────────────────────────────────────────────────────────

fn poll_eval_results(world: &mut World) {
    let Some(state) = world.get_non_send_resource_mut::<TerminalNonSendState>() else { return };
    let state = state.into_inner();
    if !state.eval_in_progress { return; }

    // Drain all pending messages into local vecs
    let mut chunks: Vec<Chunk> = Vec::new();
    let mut done_error: Option<Option<String>> = None;

    loop {
        let Some(ref rx) = state.eval_rx else { break };
        match rx.try_recv() {
            Ok(StreamMsg::Chunk(chunk)) => chunks.push(chunk),
            Ok(StreamMsg::Done { error }) => {
                done_error = Some(error);
                break;
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => break,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                warn!("Eval thread disconnected — restarting engine");
                state.eval_in_progress = false;
                state.eval_rx = None;
                state.ctrlc_flag.store(false, Ordering::Relaxed);
                let mut engine = init_nushell_engine();
                wire_ctrlc_signal(&mut engine, state.ctrlc_flag.clone());
                state.nu_engine = Some(engine);
                return;
            }
        }
    }

    let scrollback_entity = state.scrollback_entity;
    let (engine_back, returned_engine) = if let Some(_err) = done_error {
        // Retrieve engine from the return channel (sent before Done by the eval thread)
        let eng = state.engine_rx.as_ref().and_then(|rx| rx.try_recv().ok());
        state.eval_rx = None;
        state.engine_rx = None;
        state.eval_in_progress = false;
        state.ctrlc_flag.store(false, Ordering::Relaxed);
        state.scroll_offset = 100_000.0; // auto-scroll to bottom after command finishes
        (true, eng)
    } else {
        (false, None)
    };

    // Add status chunk at end-of-command
    if engine_back {
        chunks.push(Chunk::status(0));
    }

    // Dispatch all chunks to Bevy widgets via the typed molecule match
    if !chunks.is_empty() {
        let mut ss: SystemState<Commands> = SystemState::new(world);
        let mut commands = ss.get_mut(world);
        for chunk in &chunks {
            dispatch(&mut commands, scrollback_entity, chunk);
        }
        ss.apply(world);
    }

    if engine_back {
        let mut engine = returned_engine.unwrap_or_else(|| {
            warn!("Engine not returned from eval thread (panic?), reinitializing");
            init_nushell_engine()
        });
        let state = world.get_non_send_resource_mut::<TerminalNonSendState>().unwrap().into_inner();
        wire_ctrlc_signal(&mut engine, state.ctrlc_flag.clone());
        state.nu_engine = Some(engine);
        update_prompt(world);
    }
}

fn update_prompt(world: &mut World) {
    let (prompt_entity, prompt_str) = {
        let state = world.get_non_send_resource::<TerminalNonSendState>().unwrap();
        let engine = state.nu_engine.as_ref();
        let text = engine.map(prompt_text).unwrap_or_else(|| "▸ ".to_string());
        (state.prompt_entity, text)
    };
    if let Some(mut t) = world.get_mut::<Text>(prompt_entity) {
        **t = prompt_str;
    }
}

fn update_input_display(world: &mut World) {
    let (input_entity, display) = {
        let state = world.get_non_send_resource::<TerminalNonSendState>().unwrap();
        let text = format!("{}█", state.line_buffer.buffer);
        (state.input_entity, text)
    };
    if let Some(mut t) = world.get_mut::<Text>(input_entity) {
        **t = display;
    }
}

// ── Keyboard input ────────────────────────────────────────────────────────────

fn process_keyboard_input(world: &mut World) {
    let events: Vec<KeyboardInput> = {
        let mut cursor = {
            let Some(state_ref) = world.get_non_send_resource::<TerminalNonSendState>() else { return };
            state_ref.key_cursor.clone()
        };
        let messages = world.resource::<bevy::ecs::message::Messages<KeyboardInput>>();
        let evts: Vec<KeyboardInput> = cursor.read(messages).cloned().collect();
        // Store cursor back
        let state = world.get_non_send_resource_mut::<TerminalNonSendState>().unwrap().into_inner();
        state.key_cursor = cursor;
        evts
    };

    let mut dispatch: Option<String> = None;
    let mut changed = false;

    {
        let state = world.get_non_send_resource_mut::<TerminalNonSendState>().unwrap().into_inner();
        for ev in &events {
            if !ev.state.is_pressed() { continue; }
            if state.eval_in_progress { continue; }

            match (&ev.logical_key, &ev.text) {
                (Key::Enter, _) => {
                    let line = state.line_buffer.take_line();
                    if !line.is_empty() {
                        dispatch = Some(line.clone());

                        // Echo the command as a text chunk — we'll spawn it after releasing borrow
                    }
                    changed = true;
                }
                (Key::Backspace, _) => {
                    changed = state.line_buffer.backspace();
                }
                (Key::ArrowUp, _) => {
                    changed = state.line_buffer.history_up();
                }
                (Key::ArrowDown, _) => {
                    changed = state.line_buffer.history_down();
                }
                (Key::Character(c), _) if c == "c" && ev.state.is_pressed() => {
                    // Ctrl+C
                    if ev.state.is_pressed() {
                        state.ctrlc_flag.store(true, Ordering::Relaxed);
                    }
                    changed = true;
                }
                (_, Some(text)) => {
                    for ch in text.chars() {
                        if !ch.is_ascii_control() {
                            state.line_buffer.insert_char(ch);
                            changed = true;
                        }
                    }
                }
                _ => {}
            }
        }
    }

    // Spawn echo chunk for the command
    if let Some(ref cmd) = dispatch {
        let (scrollback_entity, input_entity) = {
            let state = world.get_non_send_resource::<TerminalNonSendState>().unwrap();
            (state.scrollback_entity, state.input_entity)
        };
        // Echo command text
        let echo = format!("> {}", cmd);
        world.spawn((
            Text::new(echo),
            TextFont { font_size: theme::BODY, ..default() },
            TextColor(theme::TEXT_DIM),
            Node { margin: UiRect::vertical(Val::Px(2.0)), ..default() },
            ChildOf(scrollback_entity),
        ));
        // Clear input display
        if let Some(mut t) = world.get_mut::<Text>(input_entity) {
            **t = "█".to_string();
        }

        // Dispatch eval
        let state = world.get_non_send_resource_mut::<TerminalNonSendState>().unwrap().into_inner();
        dispatch_eval(state, cmd.clone());
    } else if changed {
        update_input_display(world);
    }

    // Handle commander forwarded commands
    let pending = world
        .get_resource_mut::<crate::worlds::PendingShellCmd>()
        .and_then(|mut p| p.0.take());
    if let Some(cmd) = pending {
        let state = world.get_non_send_resource_mut::<TerminalNonSendState>().unwrap().into_inner();
        if !state.eval_in_progress {
            dispatch_eval(state, cmd);
        }
    }
}

// ── Setup ─────────────────────────────────────────────────────────────────────

fn setup_terminal(world: &mut World) {
    // Already built on an earlier visit — unhide the same tree.
    if let Some(state) = world.get_non_send_resource::<TerminalNonSendState>() {
        let root_entity = state.root_entity;
        set_terminal_visible(world, root_entity, true);
        update_prompt(world);
        update_input_display(world);
        info!("Com resumed");
        return;
    }

    // First entry: create entities and state
    let ctrlc_flag = Arc::new(AtomicBool::new(false));
    let mut nu_engine = init_nushell_engine();
    wire_ctrlc_signal(&mut nu_engine, ctrlc_flag.clone());

    // Create persistent entities (not children of root yet — will be attached below)
    let scrollback_entity = world.spawn((
        StreamScrollback::default(),
        Node {
            flex_direction: FlexDirection::Column,
            width: Val::Percent(100.0),
            min_height: Val::Percent(100.0),
            justify_content: JustifyContent::FlexEnd,
            row_gap: Val::Px(1.0),
            ..default()
        },
    )).id();
    let prompt_text_init = prompt_text(&nu_engine);
    let prompt_entity = world.spawn((
        PromptLabel,
        Text::new(prompt_text_init),
        TextFont { font_size: theme::BODY, ..default() },
        TextColor(theme::ACID_BLUE),
    )).id();
    let input_entity = world.spawn((
        LineBufferDisplay,
        Text::new("█".to_string()),
        TextFont { font_size: theme::BODY, ..default() },
        TextColor(theme::TEXT_PRIMARY),
    )).id();

    // Build the UI tree
    let (root_entity, scroll_area_entity) =
        spawn_terminal_ui(world, scrollback_entity, prompt_entity, input_entity);

    world.insert_non_send_resource(TerminalNonSendState {
        nu_engine: Some(nu_engine),
        line_buffer: LineBuffer::new(),
        eval_rx: None,
        engine_rx: None,
        eval_in_progress: false,
        key_cursor: Default::default(),
        wheel_cursor: Default::default(),
        ctrlc_flag,
        root_entity,
        scrollback_entity,
        scroll_area_entity,
        prompt_entity,
        input_entity,
        scroll_offset: 0.0,
    });

    info!("Com world initialized");
}

fn spawn_terminal_ui(
    world: &mut World,
    scrollback_entity: Entity,
    prompt_entity: Entity,
    input_entity: Entity,
) -> (Entity, Entity) {
    // Root container: full screen, column flex, padding for chrome bars
    let root = world.spawn((
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(CHROME_TOP_H),
            bottom: Val::Px(CHROME_BOTTOM_H),
            left: Val::Percent((1.0 - CONTENT_RATIO) / 2.0 * 100.0),
            width: Val::Percent(CONTENT_RATIO * 100.0),
            flex_direction: FlexDirection::Column,
            overflow: Overflow::clip_y(),
            ..default()
        },
        BackgroundColor(theme::DARK_BASE),
    )).id();

    // Scrollback area (flex-grow, scrolled via ScrollPosition)
    let scroll_area = world.spawn((
        Node {
            flex_grow: 1.0,
            flex_direction: FlexDirection::Column,
            overflow: Overflow::clip_y(),
            padding: UiRect::all(Val::Px(G)),
            ..default()
        },
        ScrollPosition::default(),
        ChildOf(root),
    )).id();

    // Attach scrollback entity into scroll area
    world.entity_mut(scrollback_entity).insert(ChildOf(scroll_area));

    // Prompt row (fixed height at bottom)
    let prompt_row = world.spawn((
        Node {
            flex_direction: FlexDirection::Row,
            padding: UiRect::axes(Val::Px(G), Val::Px(G * 0.5)),
            column_gap: Val::Px(4.0),
            border: UiRect::top(Val::Px(1.0)),
            ..default()
        },
        BackgroundColor(theme::DARK_BASE),
        BorderColor::all(theme::BORDER),
        ChildOf(root),
    )).id();

    // Attach prompt and input entities into the prompt row
    world.entity_mut(prompt_entity).insert(ChildOf(prompt_row));
    world.entity_mut(input_entity).insert(ChildOf(prompt_row));

    (root, scroll_area)
}

// ── Scroll ────────────────────────────────────────────────────────────────────

fn process_scroll(world: &mut World) {
    // Read mouse wheel events using the Messages API (same as KeyboardInput)
    let delta_y: f32 = {
        let mut cursor = {
            let Some(state_ref) = world.get_non_send_resource::<TerminalNonSendState>() else { return };
            state_ref.wheel_cursor.clone()
        };
        let messages = world.resource::<bevy::ecs::message::Messages<MouseWheel>>();
        let dy: f32 = cursor.read(messages).map(|e| -e.y).sum();
        let state = world.get_non_send_resource_mut::<TerminalNonSendState>().unwrap().into_inner();
        state.wheel_cursor = cursor;
        dy
    };

    if delta_y == 0.0 { return; }

    // Get content / viewport heights for clamping
    let (scrollback_h, area_h) = {
        let state = world.get_non_send_resource::<TerminalNonSendState>().unwrap();
        let sb_h = world.get::<ComputedNode>(state.scrollback_entity)
            .map(|cn| cn.size().y).unwrap_or(0.0);
        let sa_h = world.get::<ComputedNode>(state.scroll_area_entity)
            .map(|cn| cn.size().y).unwrap_or(0.0);
        (sb_h, sa_h)
    };

    let max_scroll = (scrollback_h - area_h).max(0.0);
    let state = world.get_non_send_resource_mut::<TerminalNonSendState>().unwrap().into_inner();
    state.scroll_offset = (state.scroll_offset + delta_y * 40.0).clamp(0.0, max_scroll);
}

fn apply_scroll_offset(world: &mut World) {
    let (offset, scroll_area_entity, scrollback_entity) = {
        let state = world.get_non_send_resource::<TerminalNonSendState>().unwrap();
        (state.scroll_offset, state.scroll_area_entity, state.scrollback_entity)
    };
    let sb_h = world.get::<ComputedNode>(scrollback_entity).map(|cn| cn.size().y).unwrap_or(0.0);
    let sa_h = world.get::<ComputedNode>(scroll_area_entity).map(|cn| cn.size().y).unwrap_or(0.0);
    let max_scroll = (sb_h - sa_h).max(0.0);
    if let Some(mut sp) = world.get_mut::<ScrollPosition>(scroll_area_entity) {
        sp.y = offset.clamp(0.0, max_scroll);
    }
}

// ── Update ────────────────────────────────────────────────────────────────────

fn terminal_update(world: &mut World) {
    if world.get_non_send_resource::<TerminalNonSendState>().is_none() {
        setup_terminal(world);
        return;
    }

    process_keyboard_input(world);
    poll_eval_results(world);
    process_scroll(world);
    apply_scroll_offset(world);
}

// ── Teardown ──────────────────────────────────────────────────────────────────

/// Leaving com hides its tree; it is never torn down.
///
/// The tree used to be despawned here while scrollback/prompt/input were
/// detached to survive — which left those three in `bevy_ui`'s taffy tree
/// pointing at nodes the despawn had just removed. The next layout pass then
/// panicked with `invalid SlotMap key used` and took the whole process down.
/// Hiding costs nothing (`Display::None` is skipped by layout) and keeps every
/// entity, and its taffy node, valid.
fn destroy_terminal(world: &mut World) {
    let Some(state) = world.get_non_send_resource::<TerminalNonSendState>() else { return };
    let root_entity = state.root_entity;
    set_terminal_visible(world, root_entity, false);
    info!("Com paused (state persisted)");
}

fn set_terminal_visible(world: &mut World, root: Entity, visible: bool) {
    if let Some(mut node) = world.get_mut::<Node>(root) {
        node.display = if visible { Display::Flex } else { Display::None };
    }
}
