use std::fs::File;
use std::io::{Read, Write};
use std::os::unix::io::{AsRawFd, FromRawFd};
use std::sync::Arc;
use std::thread;

use alacritty_terminal::event::{Event, EventListener, WindowSize};
use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::sync::FairMutex;
use alacritty_terminal::term::cell::Flags;
use alacritty_terminal::term::Config;
use alacritty_terminal::tty;
use alacritty_terminal::vte::ansi::{Color, CursorShape, NamedColor, Processor, Rgb};
use alacritty_terminal::Term;

use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy::winit::WINIT_WINDOWS;

use sugarloaf::{
    FragmentStyle, FragmentStyleDecoration, Object, RichText, Sugarloaf, SugarloafRenderer,
    SugarloafWindow, SugarloafWindowSize, UnderlineInfo, UnderlineShape,
};
use sugarloaf::font::FontLibrary;
use sugarloaf::layout::RootStyle;

use super::WorldState;

const FONT_SIZE: f32 = 16.0;

pub struct TerminalWorldPlugin;

#[derive(Component)]
struct TerminalMarker;

impl Plugin for TerminalWorldPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TerminalStateRes>()
            .add_systems(OnEnter(WorldState::Terminal), setup_terminal)
            .add_systems(OnExit(WorldState::Terminal), destroy_terminal)
            .add_systems(
                Update,
                render_terminal.run_if(in_state(WorldState::Terminal)),
            )
            .add_systems(
                Update,
                forward_keyboard_input.run_if(in_state(WorldState::Terminal)),
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
        NamedColor::Background => Rgb { r: 18, g: 18, b: 18 },
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

// --- Bevy resources ---

struct TerminalState {
    term: Arc<FairMutex<Term<BevyEventProxy>>>,
    pty_writer: Arc<std::sync::Mutex<File>>,
    cols: usize,
    rows: usize,
    rich_text_id: usize,
    _reader_handle: thread::JoinHandle<()>,
    _pty: tty::Pty,
}

#[derive(Resource, Default)]
struct TerminalStateRes {
    state: Option<TerminalState>,
}

// Sugarloaf must be NonSend (contains wgpu resources, not Send)
struct SugarloafState {
    sugarloaf: Sugarloaf<'static>,
}

// --- Setup ---

fn setup_terminal(world: &mut World) {
    let primary_entity = world
        .query_filtered::<Entity, With<PrimaryWindow>>()
        .single(world);
    let Ok(entity) = primary_entity else { return };

    // Get window dimensions
    let (win_w, win_h, scale_factor) = {
        let window = world.get::<Window>(entity).unwrap();
        (window.physical_width(), window.physical_height(), window.scale_factor())
    };

    // Get raw window handle from winit and create sugarloaf
    let sugar_result = WINIT_WINDOWS.with(|ww| {
        let ww = ww.borrow();
        let Some(window_wrapper) = ww.get_window(entity) else {
            return None;
        };

        use sugarloaf::wgpu::rwh::{HasDisplayHandle, HasWindowHandle};
        let raw_wh = window_wrapper.window_handle().ok()?.as_raw();
        let raw_dh = window_wrapper.display_handle().ok()?.as_raw();

        let sugar_window = SugarloafWindow {
            handle: raw_wh,
            display: raw_dh,
            size: SugarloafWindowSize {
                width: win_w as f32,
                height: win_h as f32,
            },
            scale: scale_factor,
        };

        let renderer = SugarloafRenderer::default();
        let (font_library, _font_errors) = FontLibrary::new(Default::default());
        let layout = RootStyle::new(scale_factor, FONT_SIZE, 1.0);

        match Sugarloaf::new(sugar_window, renderer, &font_library, layout) {
            Ok(mut sugarloaf) => {
                let rich_text_id = sugarloaf.create_rich_text();

                sugarloaf.set_background_color(Some(sugarloaf::wgpu::Color {
                    r: 0.07,
                    g: 0.07,
                    b: 0.07,
                    a: 1.0,
                }));

                Some((sugarloaf, rich_text_id))
            }
            Err(e) => {
                warn!("Failed to create Sugarloaf: {:?}", e);
                None
            }
        }
    });

    let Some((mut sugarloaf, rich_text_id)) = sugar_result else {
        warn!("Failed to initialize sugarloaf for terminal");
        return;
    };

    // Calculate terminal grid size from sugarloaf font dimensions
    let dims = sugarloaf.get_rich_text_dimensions(&rich_text_id);
    let cell_w = if dims.width > 0.0 { dims.width } else { 9.0 };
    let cell_h = if dims.height > 0.0 { dims.height } else { 18.0 };

    let cols = (win_w as f32 / cell_w).floor().max(2.0) as usize;
    let rows = (win_h as f32 / cell_h).floor().max(1.0) as usize;

    // Setup PTY
    tty::setup_env();

    let window_size = WindowSize {
        num_lines: rows as u16,
        num_cols: cols as u16,
        cell_width: cell_w as u16,
        cell_height: cell_h as u16,
    };

    let opts = tty::Options::default();
    let pty = match tty::new(&opts, window_size, 0) {
        Ok(pty) => pty,
        Err(e) => {
            warn!("Failed to create PTY: {}", e);
            return;
        }
    };

    // Create alacritty terminal grid
    let config = Config::default();
    let term_dims = TermDimensions { cols, lines: rows };
    let term = Arc::new(FairMutex::new(Term::new(config, &term_dims, BevyEventProxy)));

    // Dup PTY fd for reader/writer
    let pty_fd = pty.file().as_raw_fd();
    let reader_fd = unsafe { libc::dup(pty_fd) };
    let writer_fd = unsafe { libc::dup(pty_fd) };

    // Make reader blocking
    unsafe {
        let flags = libc::fcntl(reader_fd, libc::F_GETFL);
        libc::fcntl(reader_fd, libc::F_SETFL, flags & !libc::O_NONBLOCK);
    }

    let reader_file = unsafe { File::from_raw_fd(reader_fd) };
    let writer_file = unsafe { File::from_raw_fd(writer_fd) };
    let pty_writer = Arc::new(std::sync::Mutex::new(writer_file));

    // Spawn PTY reader thread
    let term_clone = Arc::clone(&term);
    let reader_handle = thread::spawn(move || {
        let mut reader = reader_file;
        let mut parser: Processor = Processor::new();
        let mut buf = [0u8; 4096];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    let mut term = term_clone.lock();
                    parser.advance(&mut *term, &buf[..n]);
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(std::time::Duration::from_millis(10));
                    continue;
                }
                Err(_) => break,
            }
        }
        info!("Terminal reader thread exited");
    });

    // No Bevy camera/sprite — sugarloaf renders directly to the window surface.
    // Tag a marker entity so we can clean up on exit.
    world.spawn(TerminalMarker);

    // Store state
    world.resource_mut::<TerminalStateRes>().state = Some(TerminalState {
        term,
        pty_writer,
        cols,
        rows,
        rich_text_id,
        _reader_handle: reader_handle,
        _pty: pty,
    });

    world.insert_non_send_resource(SugarloafState { sugarloaf });

    info!(
        "Terminal created ({}x{}) cell={:.0}x{:.0} (sugarloaf direct surface)",
        cols, rows, cell_w, cell_h
    );
}

// --- Render terminal directly to window surface via sugarloaf ---

fn render_terminal(
    state_res: Res<TerminalStateRes>,
    sugar_state: Option<NonSendMut<SugarloafState>>,
) {
    let Some(ref state) = state_res.state else { return };
    let Some(mut sugar_state) = sugar_state else { return };

    let sugarloaf = &mut sugar_state.sugarloaf;
    let rt_id = state.rich_text_id;

    // Cursor info extracted from term lock scope
    let mut cursor_col: usize = 0;
    let mut cursor_row: i32 = -1;
    let mut cursor_shape = CursorShape::Block;

    // Scope the term lock — drop before GPU work
    {
        let term = state.term.lock();
        let content = term.renderable_content();

        // Clear previous content
        sugarloaf.content().sel(rt_id).clear();

        // Build content from alacritty terminal grid
        let mut current_line: i32 = -1;
        for indexed in content.display_iter {
            let col = indexed.point.column.0;
            let row = indexed.point.line.0;

            if row < 0 || col >= state.cols || row as usize >= state.rows {
                continue;
            }

            if row != current_line {
                sugarloaf.content().sel(rt_id).new_line();
                current_line = row;
            }

            let cell = &*indexed;
            let mut fg_color = cell.fg;
            let mut bg_color = cell.bg;

            if cell.flags.contains(Flags::INVERSE) {
                std::mem::swap(&mut fg_color, &mut bg_color);
            }

            let fg = resolve_color(fg_color, content.colors);
            let bg = resolve_color(bg_color, content.colors);

            // Bold → bright color mapping
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

            // Bold
            if cell.flags.contains(Flags::BOLD) {
                style.font_attrs = sugarloaf::font_introspector::Attributes::new(
                    sugarloaf::Stretch::NORMAL,
                    sugarloaf::Weight(700),
                    sugarloaf::Style::Normal,
                );
            }

            // Italic
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

            // Underline
            if cell.flags.contains(Flags::UNDERLINE) {
                style.decoration = Some(FragmentStyleDecoration::Underline(UnderlineInfo {
                    is_doubled: false,
                    shape: UnderlineShape::Regular,
                }));
            }

            // Strikethrough
            if cell.flags.contains(Flags::STRIKEOUT) {
                style.decoration = Some(FragmentStyleDecoration::Strikethrough);
            }

            // Drawable chars (box drawing, powerline, etc.)
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

        // Build the text layout
        sugarloaf.content().build();

        // Extract cursor info
        cursor_col = content.cursor.point.column.0;
        cursor_row = content.cursor.point.line.0;
        cursor_shape = content.cursor.shape;
    } // term lock dropped here

    // Handle cursor as quad overlay
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

    // Render directly to window surface via sugarloaf's own wgpu swapchain
    sugarloaf.render();
}

// --- Keyboard input forwarding ---

fn forward_keyboard_input(
    state_res: Res<TerminalStateRes>,
    mut key_events: MessageReader<KeyboardInput>,
) {
    let Some(ref state) = state_res.state else { return };

    for event in key_events.read() {
        if !event.state.is_pressed() {
            continue;
        }

        let bytes: Option<Vec<u8>> = match &event.logical_key {
            Key::Character(c) => {
                let s = c.as_str();
                Some(s.as_bytes().to_vec())
            }
            Key::Enter => Some(b"\r".to_vec()),
            Key::Backspace => Some(b"\x7f".to_vec()),
            Key::Tab => Some(b"\t".to_vec()),
            Key::Escape => Some(b"\x1b".to_vec()),
            Key::Space => Some(b" ".to_vec()),
            Key::ArrowUp => Some(b"\x1b[A".to_vec()),
            Key::ArrowDown => Some(b"\x1b[B".to_vec()),
            Key::ArrowRight => Some(b"\x1b[C".to_vec()),
            Key::ArrowLeft => Some(b"\x1b[D".to_vec()),
            Key::Home => Some(b"\x1b[H".to_vec()),
            Key::End => Some(b"\x1b[F".to_vec()),
            Key::Delete => Some(b"\x1b[3~".to_vec()),
            Key::PageUp => Some(b"\x1b[5~".to_vec()),
            Key::PageDown => Some(b"\x1b[6~".to_vec()),
            _ => None,
        };

        if let Some(bytes) = bytes {
            if let Ok(mut writer) = state.pty_writer.lock() {
                let _ = writer.write_all(&bytes);
                let _ = writer.flush();
            }
        }
    }
}

// --- Destroy ---

fn destroy_terminal(world: &mut World) {
    let entities: Vec<Entity> = world
        .query_filtered::<Entity, With<TerminalMarker>>()
        .iter(world)
        .collect();
    for entity in entities {
        world.despawn(entity);
    }

    world.resource_mut::<TerminalStateRes>().state = None;
    world.remove_non_send_resource::<SugarloafState>();

    info!("Terminal world destroyed");
}
