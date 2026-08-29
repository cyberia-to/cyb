pub mod nushell_to_stream;
use nushell_to_stream::{pipeline_to_chunks, StreamMsg};

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use bevy::ecs::system::SystemState;
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

use super::{ComInbox, ComSay, Notice, Speaker, WorldState};
use crate::shell::chrome::{ContentRoot, CHROME_TOP_H, CHROME_BOTTOM_H};

const G: f32 = theme::G;

const NU_ENV_SOURCE: &str = include_str!("../../../assets/nu-config/env.nu");
const NU_CONFIG_SOURCE: &str = include_str!("../../../assets/nu-config/config.nu");

// ── Plugin ────────────────────────────────────────────────────────────────────

pub struct ComWorldPlugin;

impl Plugin for ComWorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(WorldState::Com), setup_terminal)
            .add_systems(OnExit(WorldState::Com), destroy_terminal)
            // Not gated on being in com: the other worlds write here while you
            // are looking at them, and the record has to be waiting when you
            // arrive. com's tree is hidden between visits, never torn down.
            .add_systems(Update, drain_com_inbox)
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

// ── NonSend state ─────────────────────────────────────────────────────────────

struct TerminalNonSendState {
    nu_engine: Option<NuShellEngine>,
    eval_rx: Option<std::sync::mpsc::Receiver<StreamMsg>>,
    engine_rx: Option<std::sync::mpsc::Receiver<NuShellEngine>>,
    eval_in_progress: bool,
    ctrlc_flag: Arc<AtomicBool>,
    wheel_cursor: bevy::ecs::message::MessageCursor<MouseWheel>,
    // UI entity IDs (the tree is built once and hidden between visits)
    root_entity: Entity,
    scrollback_entity: Entity,
    scroll_area_entity: Entity,
    scroll_offset: f32,
    /// Follow the tail: new output scrolls into view until the reader
    /// scrolls up, and resumes when they scroll back down to the end.
    stick_to_bottom: bool,
    /// The command currently running, so its completion can be announced by
    /// name from wherever you happen to be looking.
    last_cmd: String,
    /// The text entity of a reply currently streaming in, if one is open.
    stream_row: Option<Entity>,
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
            return format!("~{} > ", &cwd[home.len()..]);
        }
    }
    format!("{} > ", cwd)
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
    state.stick_to_bottom = true;
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
    state.stick_to_bottom = true;
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
        state.stick_to_bottom = true;
        (true, eng)
    } else {
        (false, None)
    };

    // Add status chunk at end-of-command
    let finished = engine_back.then(|| state.last_cmd.clone());
    if engine_back {
        chunks.push(Chunk::status(0));
    }

    // Say so under the address bar. The output itself stays here in com; this
    // is only the fact that it is over, for whoever is looking elsewhere.
    if let Some(cmd) = finished {
        if !cmd.is_empty() {
            world.resource_mut::<Notice>().show(format!("done: {cmd}"));
        }
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
        publish_prompt(world);
    }
}

/// Run what the commander sent.
///
/// com has no input of its own: the chrome commander is the one line in cyb,
/// on every screen, and this world is where its history lives. A submitted
/// line arrives as `PendingShellCmd`, gets echoed into the scrollback, and
/// runs — the same path whichever world it was typed from.
fn run_pending_command(world: &mut World) {
    let pending = world
        .get_resource_mut::<crate::worlds::PendingShellCmd>()
        .and_then(|mut p| p.0.take());
    let Some(cmd) = pending else { return };
    debug!("com: pending {cmd:?}");
    if cmd.trim().is_empty() {
        return;
    }

    // A question outranks a command: `? ...` and `ask ...` go to soma
    // explicitly, and a line whose first word is nothing this shell can run
    // goes there too. Typing `privet` into a box whose placeholder says
    // "ask, search, transact" should get an answer, not
    // `External command failed` in a red bar.
    let to_soma = crate::worlds::soma_bridge::parse_ask(&cmd)
        .map(str::to_string)
        .or_else(|| {
            let state = world.get_non_send_resource::<TerminalNonSendState>()?;
            let engine = state.nu_engine.as_ref()?;
            (!resolves_in_shell(&engine.engine_state, &cmd)).then(|| cmd.clone())
        });
    if let Some(q) = to_soma {
        crate::worlds::soma_bridge::ask(world, &q);
        return;
    }

    let (scrollback_entity, busy) = {
        let Some(state) = world.get_non_send_resource::<TerminalNonSendState>() else { return };
        (state.scrollback_entity, state.eval_in_progress)
    };
    if busy {
        warn!("com: dropped {cmd:?} — eval in progress");
        return;
    }

    let prompt = {
        let state = world.get_non_send_resource::<TerminalNonSendState>().unwrap();
        state.nu_engine.as_ref().map(prompt_text).unwrap_or_default()
    };
    world.spawn((
        Text::new(format!("{prompt}{cmd}")),
        TextFont { font_size: theme::BODY, ..default() },
        TextColor(theme::TEXT_DIM),
        Node { margin: UiRect::vertical(Val::Px(2.0)), ..default() },
        ChildOf(scrollback_entity),
    ));
    // The line of record: what was asked of the shell, prompt and all.
    // Output stays session-local — it is re-runnable — but the asking is
    // part of the story com tells after a restart.
    crate::worlds::persist_com_line("cmd", &format!("{prompt}{cmd}"));

    let state = world.get_non_send_resource_mut::<TerminalNonSendState>().unwrap().into_inner();
    state.last_cmd = cmd.clone();
    dispatch_eval(state, cmd);
}

/// Write what the other worlds have said into the scrollback.
///
/// Side carries the speaker: what you asked for on the left, what came back on
/// the right. Nothing else distinguishes them, which is the point — a wall of
/// events with no shape to it is what the sigma page had, and nobody reads it.
fn drain_com_inbox(world: &mut World) {
    let lines = {
        let Some(mut inbox) = world.get_resource_mut::<ComInbox>() else { return };
        if inbox.0.is_empty() { return }
        std::mem::take(&mut inbox.0)
    };

    // com may not have been opened yet, in which case there is nowhere to put
    // these. Hold them rather than drop them.
    let Some(state) = world.get_non_send_resource::<TerminalNonSendState>() else {
        world.resource_mut::<ComInbox>().0 = lines;
        return;
    };
    let scrollback = state.scrollback_entity;

    for say in lines {
        match say {
            ComSay::Line(who, text) => {
                spawn_said_row(world, scrollback, who, text);
            }
            // A streamed reply is one system row whose text grows as the
            // model writes. The row exists from the first instant, so the
            // reply visibly *starts* — the difference between a mind at work
            // and a frozen app.
            ComSay::StreamStart => {
                let entity = spawn_said_row(world, scrollback, Speaker::System, String::new());
                let state = world
                    .get_non_send_resource_mut::<TerminalNonSendState>()
                    .unwrap()
                    .into_inner();
                state.stream_row = entity.into();
            }
            ComSay::StreamDelta(delta) => {
                let row = world
                    .get_non_send_resource::<TerminalNonSendState>()
                    .and_then(|s| s.stream_row);
                match row.and_then(|e| world.get_mut::<Text>(e)) {
                    Some(mut text) => text.0.push_str(&delta),
                    // A delta with no open row (com opened mid-answer):
                    // better a plain line than a lost piece.
                    None => {
                        spawn_said_row(world, scrollback, Speaker::System, delta);
                    }
                }
            }
            ComSay::StreamEnd(fin) => {
                let row = world
                    .get_non_send_resource_mut::<TerminalNonSendState>()
                    .unwrap()
                    .into_inner()
                    .stream_row
                    .take();
                if let Some(mut text) = row.and_then(|e| world.get_mut::<Text>(e)) {
                    text.0 = fin;
                }
            }
        }
    }

    // A line arriving is news; follow it.
    if let Some(state) = world.get_non_send_resource_mut::<TerminalNonSendState>() {
        state.into_inner().stick_to_bottom = true;
    }
}

/// Would nushell recognise this line's head word as something it can run?
///
/// Declared commands (builtins, aliases, custom defs) count, and so do
/// executables on PATH or by explicit path. Expression-looking lines —
/// digits, operators, variables, subexpressions — count as shell too, so
/// `1 + 2` and `$env.HOME | print` stay maths and pipes. What is left over —
/// a bare word the shell has never heard of — is somebody talking, and
/// talking is soma's job.
fn resolves_in_shell(engine_state: &EngineState, line: &str) -> bool {
    let Some(head) = line.split_whitespace().next() else { return true };

    // Expressions and syntax that only make sense in the shell.
    let first = head.chars().next().unwrap_or(' ');
    if first.is_ascii_digit() || "$([{\"'-.~/^".contains(first) {
        return true;
    }
    // Multi-word command heads (`str join`, `into int`) resolve as the first
    // two words; a declared name wins at any length.
    let two: String = line.split_whitespace().take(2).collect::<Vec<_>>().join(" ");
    if engine_state.find_decl(two.as_bytes(), &[]).is_some()
        || engine_state.find_decl(head.as_bytes(), &[]).is_some()
    {
        return true;
    }
    // A real binary on PATH.
    if let Ok(path) = std::env::var("PATH") {
        for dir in path.split(':') {
            if !dir.is_empty() && std::path::Path::new(dir).join(head).is_file() {
                return true;
            }
        }
    }
    false
}

/// Publish the shell's prompt so the commander can wear it. The commander is
/// com's prompt line wherever it is drawn.
fn publish_prompt(world: &mut World) {
    let prompt = {
        let Some(state) = world.get_non_send_resource::<TerminalNonSendState>() else { return };
        state.nu_engine.as_ref().map(prompt_text)
    };
    let Some(prompt) = prompt else { return };
    if let Some(mut res) = world.get_resource_mut::<crate::shell::chrome::ComPrompt>() {
        if res.0 != prompt {
            res.0 = prompt;
        }
    }
}

// ── Setup ─────────────────────────────────────────────────────────────────────

fn setup_terminal(world: &mut World) {
    // Already built on an earlier visit — unhide the same tree.
    if let Some(state) = world.get_non_send_resource::<TerminalNonSendState>() {
        let root_entity = state.root_entity;
        set_terminal_visible(world, root_entity, true);
        publish_prompt(world);
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
        // Top-aligned and free to grow. Bottom-aligning it (flex-end plus a
        // full-height minimum) pushed overflow off the top of the clip, where
        // no scroll position can reach it — which is why long output only
        // ever showed its first screen.
        Node {
            flex_direction: FlexDirection::Column,
            width: Val::Percent(100.0),
            // A flex child shrinks to its container by default, so the
            // scrollback was squeezed to exactly the viewport: it never
            // overflowed, max_scroll stayed zero, and the tail was
            // unreachable. It must keep its content height.
            flex_shrink: 0.0,
            row_gap: Val::Px(1.0),
            ..default()
        },
    )).id();
    // Build the UI tree
    let (root_entity, scroll_area_entity) = spawn_terminal_ui(world, scrollback_entity);
    replay_transcript(world, scrollback_entity);

    world.insert_non_send_resource(TerminalNonSendState {
        nu_engine: Some(nu_engine),
        eval_rx: None,
        engine_rx: None,
        eval_in_progress: false,
        wheel_cursor: Default::default(),
        ctrlc_flag,
        root_entity,
        scrollback_entity,
        scroll_area_entity,
        scroll_offset: 0.0,
        stick_to_bottom: true,
        last_cmd: String::new(),
        stream_row: None,
    });

    info!("Com world initialized");
}

fn spawn_terminal_ui(world: &mut World, scrollback_entity: Entity) -> (Entity, Entity) {
    // Root container: the band between the chrome bars (ContentRoot keeps
    // top/bottom tracking the bars' true heights, safe areas included).
    let root = world.spawn((
        ContentRoot,
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(CHROME_TOP_H),
            bottom: Val::Px(CHROME_BOTTOM_H),
            left: Val::Px(0.0),
            right: Val::Px(0.0),
            flex_direction: FlexDirection::Column,
            // The root takes the whole viewport and the column inside it
            // carries the measure, so a narrow screen keeps every pixel.
            align_items: AlignItems::Center,
            overflow: Overflow::clip_y(),
            ..default()
        },
        BackgroundColor(theme::DARK_BASE),
    )).id();

    // Scrollback area (flex-grow, scrolled via ScrollPosition)
    let scroll_area = world.spawn((
        Node {
            flex_grow: 1.0,
            width: Val::Percent(100.0),
            max_width: Val::Px(theme::MEASURE),
            flex_direction: FlexDirection::Column,
            // Scroll, not clip: bevy_ui ignores ScrollPosition unless an axis
            // is actually declared scrollable, so a clipping node stays fixed
            // at the top however the position is set.
            overflow: Overflow::scroll_y(),
            padding: UiRect::all(Val::Px(G)),
            ..default()
        },
        ScrollPosition::default(),
        ChildOf(root),
    )).id();

    // Attach scrollback entity into scroll area
    world.entity_mut(scrollback_entity).insert(ChildOf(scroll_area));

    (root, scroll_area)
}

/// Restarting cyb must not cost the record: com replays its transcript into
/// the scrollback before the session's first line, so the history is simply
/// *there*, the way it is in anything that deserves to be called a log.
///
/// Only the most recent lines are replayed — the file is the full archive,
/// the screen is the recent past.
const REPLAY_LINES: usize = 300;

fn replay_transcript(world: &mut World, scrollback: Entity) {
    let path = crate::worlds::com_transcript_path();
    let Ok(body) = std::fs::read_to_string(&path) else { return };
    let lines: Vec<&str> = body.lines().collect();
    let start = lines.len().saturating_sub(REPLAY_LINES);
    let mut replayed = 0usize;

    for line in &lines[start..] {
        let Some(kind) = json_field(line, "kind") else { continue };
        let Some(text) = json_field(line, "text") else { continue };
        match kind.as_str() {
            "cmd" => {
                world.spawn((
                    Text::new(text),
                    TextFont { font_size: theme::BODY, ..default() },
                    TextColor(theme::TEXT_DIM),
                    Node { margin: UiRect::vertical(Val::Px(2.0)), ..default() },
                    ChildOf(scrollback),
                ));
            }
            "user" => { spawn_said_row(world, scrollback, Speaker::User, text); }
            "system" => { spawn_said_row(world, scrollback, Speaker::System, text); }
            _ => continue,
        }
        replayed += 1;
    }

    if replayed > 0 {
        // A quiet seam between then and now.
        world.spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(1.0),
                margin: UiRect::vertical(Val::Px(theme::G)),
                ..default()
            },
            BackgroundColor(theme::BORDER),
            ChildOf(scrollback),
        ));
        info!("com: replayed {replayed} lines of transcript");
    }
}

/// One attributed row in the record: yours on the left, the machine's on the
/// right. The single place this layout is decided, for live lines and
/// replayed ones alike.
fn spawn_said_row(world: &mut World, scrollback: Entity, who: Speaker, text: String) -> Entity {
    let (justify, colour) = match who {
        Speaker::User   => (JustifyContent::FlexStart, theme::TEXT_PRIMARY),
        Speaker::System => (JustifyContent::FlexEnd,   theme::ACID_GREEN),
    };
    let row = world.spawn((
        Node {
            width: Val::Percent(100.0),
            justify_content: justify,
            margin: UiRect::vertical(Val::Px(2.0)),
            ..default()
        },
        ChildOf(scrollback),
    )).id();
    world.spawn((
        Text::new(text),
        TextFont { font_size: theme::BODY, ..default() },
        TextColor(colour),
        ChildOf(row),
    )).id()
}

/// Pull one string field out of a hand-written JSON line — the mirror of the
/// writer in worlds::persist_com_line, and of soma's sidecar.
fn json_field(line: &str, name: &str) -> Option<String> {
    let key = format!("\"{name}\":\"");
    let start = line.find(&key)? + key.len();
    let mut out = String::new();
    let mut chars = line[start..].chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' => match chars.next() {
                Some('n') => out.push('\n'),
                Some(other) => out.push(other),
                None => return None,
            },
            '"' => return Some(out),
            _ => out.push(c),
        }
    }
    None
}

// ── Scroll ────────────────────────────────────────────────────────────────────

fn process_scroll(world: &mut World) {
    // Wheel where there is one, finger where there is not: a phone sends no
    // MouseWheel at all, which is why com could not be scrolled by hand.
    let wheel: f32 = {
        let mut cursor = {
            let Some(state_ref) = world.get_non_send_resource::<TerminalNonSendState>() else { return };
            state_ref.wheel_cursor.clone()
        };
        let messages = world.resource::<bevy::ecs::message::Messages<MouseWheel>>();
        let dy: f32 = cursor.read(messages).map(|e| -e.y).sum();
        let state = world.get_non_send_resource_mut::<TerminalNonSendState>().unwrap().into_inner();
        state.wheel_cursor = cursor;
        dy * 40.0
    };

    // A single finger dragging up sends the text up: content follows the
    // finger, so the offset moves against it.
    let drag: f32 = {
        let touches = world.resource::<bevy::input::touch::Touches>();
        let live: Vec<&bevy::input::touch::Touch> = touches.iter().collect();
        if live.len() == 1 { -live[0].delta().y } else { 0.0 }
    };

    let delta_y = wheel + drag;
    if delta_y == 0.0 { return; }

    // Heights in the same units the scroll position is written in.
    //
    // ComputedNode measures in physical pixels; bevy_ui reads ScrollPosition as
    // *logical* ones and multiplies by the scale factor before clamping to the
    // physical overflow. Mixing the two silently pins the view: a logical
    // offset near a physical maximum comes back multiplied past it and clamps
    // to the end, every frame, however far the finger travelled. Touch and
    // wheel deltas are logical already, so logical is what everything here is.
    let (scrollback_h, area_h) = scroll_extent(world);

    let max_scroll = (scrollback_h - area_h).max(0.0);
    let state = world.get_non_send_resource_mut::<TerminalNonSendState>().unwrap().into_inner();
    state.scroll_offset = (state.scroll_offset + delta_y).clamp(0.0, max_scroll);
    // Reaching the end re-arms the follow; leaving it hands control back.
    state.stick_to_bottom = state.scroll_offset >= max_scroll - 1.0;
}

/// Content and viewport heights of the scrollback, in logical pixels.
fn scroll_extent(world: &mut World) -> (f32, f32) {
    let Some(state) = world.get_non_send_resource::<TerminalNonSendState>() else {
        return (0.0, 0.0);
    };
    let (sb, sa) = (state.scrollback_entity, state.scroll_area_entity);
    let logical = |e: Entity| -> f32 {
        world
            .get::<ComputedNode>(e)
            .map(|cn| cn.size().y * cn.inverse_scale_factor())
            .unwrap_or(0.0)
    };
    (logical(sb), logical(sa))
}

fn apply_scroll_offset(world: &mut World) {
    let (offset, stick, scroll_area_entity) = {
        let state = world.get_non_send_resource::<TerminalNonSendState>().unwrap();
        (state.scroll_offset, state.stick_to_bottom, state.scroll_area_entity)
    };
    let (sb_h, sa_h) = scroll_extent(world);
    let max_scroll = (sb_h - sa_h).max(0.0);

    // Layout runs after this, so max_scroll is one frame behind the newest
    // line; following the tail every frame catches up as the output streams.
    {
        static LAST: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let c = sb_h as u32;
        if LAST.swap(c, std::sync::atomic::Ordering::Relaxed) != c {
            debug!("com: content {sb_h:.0} view {sa_h:.0} max_scroll {max_scroll:.0} stick {stick}");
        }
    }
    let target = if stick { max_scroll } else { offset.clamp(0.0, max_scroll) };
    {
        let state = world.get_non_send_resource_mut::<TerminalNonSendState>().unwrap().into_inner();
        state.scroll_offset = target;
    }
    if let Some(mut sp) = world.get_mut::<ScrollPosition>(scroll_area_entity) {
        sp.y = target;
    }
}

// ── Update ────────────────────────────────────────────────────────────────────

fn terminal_update(world: &mut World) {
    if world.get_non_send_resource::<TerminalNonSendState>().is_none() {
        setup_terminal(world);
        return;
    }

    run_pending_command(world);
    publish_prompt(world);
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
