pub mod nushell_to_stream;
use nushell_to_stream::{pipeline_to_chunks, StreamMsg};

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use bevy::ecs::system::SystemState;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;

use nu_cli::{gather_parent_env_vars, eval_source};
use nu_cmd_lang::create_default_context;
use nu_command::add_shell_command_context;
use nu_engine::env::convert_env_values;
use nu_engine::ClosureEvalOnce;
use nu_parser::parse;
use nu_protocol::engine::{EngineState, Redirection, Stack, StateWorkingSet, Closure};
use nu_protocol::debugger::WithoutDebug;
use nu_protocol::{OutDest, PipelineData, Signals, Value};
use nu_std::load_standard_library;

use cyb_stream::Chunk;
use prysm::stream::{StreamScrollback, MoleculeRegistry, register_v0_molecules};
use prysm::{theme, atoms::glass_bg, atoms::GlassDepth};

use super::WorldState;
use crate::shell::chrome::{CHROME_TOP_H, CHROME_BOTTOM_H};

const CONTENT_RATIO: f32 = 0.62;
const G: f32 = theme::G;

const NU_ENV_SOURCE: &str = include_str!("../../../assets/nu-config/env.nu");
const NU_CONFIG_SOURCE: &str = include_str!("../../../assets/nu-config/config.nu");

// ── Plugin ────────────────────────────────────────────────────────────────────

pub struct TerminalWorldPlugin;

#[derive(Component)]
struct TerminalMarker;

#[derive(Component)]
struct PromptLabel;

#[derive(Component)]
struct LineBufferDisplay;

impl Plugin for TerminalWorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(WorldState::Terminal), setup_terminal)
            .add_systems(OnExit(WorldState::Terminal), destroy_terminal)
            .add_systems(
                Update,
                terminal_update.run_if(in_state(WorldState::Terminal)),
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
    eval_in_progress: bool,
    ctrlc_flag: Arc<AtomicBool>,
    key_cursor: bevy::ecs::message::MessageCursor<KeyboardInput>,
    // UI entity IDs (persisted across world switches)
    scrollback_entity: Entity,
    prompt_entity: Entity,
    input_entity: Entity,
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

fn get_env_closure(engine_state: &EngineState, stack: &Stack, var_name: &str) -> Option<Closure> {
    let val = stack.get_env_var(engine_state, var_name)
        .or_else(|| engine_state.get_env_var(var_name))?;
    match val {
        Value::Closure { val, .. } => Some(*val.clone()),
        _ => None,
    }
}

/// Build the prompt text: cwd display (no ANSI, just text).
fn prompt_text(engine: &NuShellEngine) -> String {
    // Try PROMPT_COMMAND closure
    if let Some(prompt_cmd) = get_env_closure(&engine.engine_state, &engine.stack, "PROMPT_COMMAND") {
        if let Ok(data) = ClosureEvalOnce::new(&engine.engine_state, &engine.stack, prompt_cmd)
            .run_with_input(PipelineData::empty())
        {
            let config = (*engine.engine_state.get_config()).clone();
            if let Ok(s) = data.collect_string("", &config) {
                let clean: String = s.chars().filter(|c| c.is_ascii_graphic() || *c == ' ' || *c == '/').collect();
                if !clean.is_empty() {
                    return format!("{} ▸ ", clean.trim());
                }
            }
        }
    }
    // Fallback: cwd
    let cwd = std::env::current_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "~".to_string());
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
    let Some(engine) = state.nu_engine.take() else {
        warn!("No nushell engine for eval");
        return;
    };

    state.eval_in_progress = true;
    let (tx, rx) = std::sync::mpsc::channel::<StreamMsg>();
    state.eval_rx = Some(rx);

    std::thread::spawn(move || {
        let mut engine = engine;
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            evaluate_and_capture(&mut engine, &input, &tx)
        }));
        let error = match outcome {
            Ok(e) => e,
            Err(panic) => {
                let msg = if let Some(s) = panic.downcast_ref::<&str>() { s.to_string() }
                    else if let Some(s) = panic.downcast_ref::<String>() { s.clone() }
                    else { "unknown panic".to_string() };
                Some(format!("panic: {msg}"))
            }
        };
        let _ = tx.send(StreamMsg::Done { error });
    });
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
    let (engine_back, error_str) = if let Some(err) = done_error {
        state.eval_rx = None;
        state.eval_in_progress = false;
        state.ctrlc_flag.store(false, Ordering::Relaxed);
        // Engine is returned via StreamMsg::Done — but it's in the spawned thread
        // (we don't get it back here since we use StreamMsg, not EvalMsg).
        // Re-init a fresh engine — TODO: thread returns engine once we adopt a channel that carries it.
        (true, err)
    } else {
        (false, None)
    };

    // Add status chunk at end-of-command
    if engine_back {
        chunks.push(Chunk::status(0));
    }

    // Spawn molecules for all chunks using SystemState<Commands>
    if !chunks.is_empty() {
        let mut ss: SystemState<(Commands, Res<MoleculeRegistry>)> = SystemState::new(world);
        let (mut commands, registry) = ss.get_mut(world);
        for chunk in &chunks {
            if let Some(mol) = registry.get(chunk.sigil, chunk.render) {
                mol.spawn(&mut commands, scrollback_entity, chunk);
            } else {
                // Fallback: plain text
                commands.spawn((
                    Text::new(String::from_utf8_lossy(&chunk.payload).into_owned()),
                    TextFont { font_size: theme::BODY, ..default() },
                    TextColor(theme::TEXT_PRIMARY),
                    ChildOf(scrollback_entity),
                ));
            }
        }
        ss.apply(world);
    }

    if engine_back {
        // Re-initialize engine (v0: fresh engine; v1: return engine from thread)
        let mut engine = init_nushell_engine();
        let state = world.get_non_send_resource_mut::<TerminalNonSendState>().unwrap().into_inner();
        if let Some(err) = error_str {
            // Error chunk already spawned above
            let _ = err;
        }
        wire_ctrlc_signal(&mut engine, state.ctrlc_flag.clone());
        state.nu_engine = Some(engine);
        // Refresh prompt
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
        let Some(state_ref) = world.get_non_send_resource::<TerminalNonSendState>() else { return };
        let mut cursor = state_ref.key_cursor.clone();
        drop(state_ref);
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
    // Register molecules on first entry
    {
        let mut registry = world.resource_mut::<MoleculeRegistry>();
        if registry.get(cyb_stream::sigil::HAX, cyb_stream::render::TEXT).is_none() {
            register_v0_molecules(&mut registry);
        }
    }

    // If state persists, re-attach scrollback to a new root and show
    if world.get_non_send_resource::<TerminalNonSendState>().is_some() {
        let (scrollback_entity, prompt_entity, input_entity) = {
            let state = world.get_non_send_resource::<TerminalNonSendState>().unwrap();
            (state.scrollback_entity, state.prompt_entity, state.input_entity)
        };
        spawn_terminal_ui(world, scrollback_entity, prompt_entity, input_entity);
        update_prompt(world);
        update_input_display(world);
        info!("Terminal resumed");
        return;
    }

    // First entry: create entities and state
    let ctrlc_flag = Arc::new(AtomicBool::new(false));
    let mut nu_engine = init_nushell_engine();
    wire_ctrlc_signal(&mut nu_engine, ctrlc_flag.clone());

    // Create persistent entities (not children of root yet — will be attached below)
    let scrollback_entity = world.spawn(StreamScrollback::default()).id();
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
    spawn_terminal_ui(world, scrollback_entity, prompt_entity, input_entity);

    world.insert_non_send_resource(TerminalNonSendState {
        nu_engine: Some(nu_engine),
        line_buffer: LineBuffer::new(),
        eval_rx: None,
        eval_in_progress: false,
        key_cursor: Default::default(),
        ctrlc_flag,
        scrollback_entity,
        prompt_entity,
        input_entity,
    });

    info!("Terminal world initialized");
}

fn spawn_terminal_ui(
    world: &mut World,
    scrollback_entity: Entity,
    prompt_entity: Entity,
    input_entity: Entity,
) {
    // Root container: full screen, column flex, padding for chrome bars
    let root = world.spawn((
        TerminalMarker,
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

    // Scrollback area (flex-grow)
    let scroll_area = world.spawn((
        TerminalMarker,
        Node {
            flex_grow: 1.0,
            flex_direction: FlexDirection::Column,
            overflow: Overflow::clip_y(),
            padding: UiRect::all(Val::Px(G)),
            row_gap: Val::Px(1.0),
            ..default()
        },
        ChildOf(root),
    )).id();

    // Attach scrollback entity into scroll area
    world.entity_mut(scrollback_entity).insert(ChildOf(scroll_area));

    // Prompt row (fixed height at bottom)
    let prompt_row = world.spawn((
        TerminalMarker,
        Node {
            flex_direction: FlexDirection::Row,
            padding: UiRect::axes(Val::Px(G), Val::Px(G * 0.5)),
            column_gap: Val::Px(4.0),
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5)),
        ChildOf(root),
    )).id();

    // Attach prompt and input entities into the prompt row
    world.entity_mut(prompt_entity).insert(ChildOf(prompt_row));
    world.entity_mut(input_entity).insert(ChildOf(prompt_row));
}

// ── Update ────────────────────────────────────────────────────────────────────

fn terminal_update(world: &mut World) {
    if world.get_non_send_resource::<TerminalNonSendState>().is_none() {
        setup_terminal(world);
        return;
    }

    process_keyboard_input(world);
    poll_eval_results(world);
}

// ── Teardown ──────────────────────────────────────────────────────────────────

fn destroy_terminal(world: &mut World) {
    // Detach scrollback/prompt/input from their parents (so they survive despawn)
    let (scrollback_entity, prompt_entity, input_entity) = {
        let Some(state) = world.get_non_send_resource::<TerminalNonSendState>() else { return };
        (state.scrollback_entity, state.prompt_entity, state.input_entity)
    };
    world.entity_mut(scrollback_entity).remove::<ChildOf>();
    world.entity_mut(prompt_entity).remove::<ChildOf>();
    world.entity_mut(input_entity).remove::<ChildOf>();

    // Despawn TerminalMarker entities (root + scroll area + prompt row)
    let entities: Vec<Entity> = world
        .query_filtered::<Entity, With<TerminalMarker>>()
        .iter(world)
        .collect();
    for e in entities {
        world.despawn(e);
    }

    info!("Terminal paused (state persisted)");
}
