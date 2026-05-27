//! stream_demo — visual sanity check for prysm stream molecules.
//!
//! Runs a Bevy window, spawns a StreamScrollback, and feeds every v0 chunk type
//! into the channel from a Startup system. Run with:
//!
//!   cargo run -p prysm --example stream_demo

use bevy::prelude::*;
use prysm::{
    theme, atoms::{glass_bg, GlassDepth},
    stream::{
        StreamPlugin, StreamChannel, StreamScrollback, MoleculeRegistry,
        register_v0_molecules,
    },
    cyb_stream::{Chunk, sigil, render, encode_nested, table_chunk},
};
use prysm::cyb_stream::bytes::Bytes;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "cyb-stream molecules demo".into(),
                resolution: (900u32, 700u32).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(StreamPlugin)
        .add_systems(Startup, (setup, send_demo_chunks).chain())
        .run();
}

fn setup(
    mut commands: Commands,
    mut registry: ResMut<MoleculeRegistry>,
) {
    commands.spawn(Camera2d);

    // Register all v0 molecules
    register_v0_molecules(&mut registry);

    // Root layout
    commands.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            ..default()
        },
        BackgroundColor(theme::DARK_BASE),
    ))
    .with_children(|root| {
        // Title bar
        root.spawn((
            Node {
                width: Val::Percent(100.0),
                padding: UiRect::all(Val::Px(theme::G)),
                ..default()
            },
            glass_bg(GlassDepth::Background),
        ))
        .with_children(|bar| {
            bar.spawn((
                Text::new("cyb-stream molecules · v0 demo"),
                TextFont { font_size: theme::CAPTION, ..default() },
                TextColor(theme::TEXT_DIM),
            ));
        });

        // Scrollback area
        root.spawn((
            StreamScrollback::default(),
            Node {
                flex_direction: FlexDirection::Column,
                flex_grow: 1.0,
                padding: UiRect::all(Val::Px(theme::G * 2.0)),
                row_gap: Val::Px(2.0),
                overflow: Overflow::clip_y(),
                ..default()
            },
        ));
    });
}

fn send_demo_chunks(channel: Res<StreamChannel>) {
    let tx = &channel.tx;

    // ── plain text ────────────────────────────────────────────────────────────
    tx.send(Chunk::text("Hello from cyb-stream. Every chunk type below is a native prysm molecule.")).ok();

    // ── annotation ────────────────────────────────────────────────────────────
    tx.send(Chunk::annotation("~/cyber/cyb  ·  main")).ok();

    // ── neuron reference ──────────────────────────────────────────────────────
    tx.send(Chunk::new(
        sigil::PAT, render::TEXT,
        Bytes::from_static(b"@alice"),
    )).ok();

    // ── table ─────────────────────────────────────────────────────────────────
    tx.send(table_chunk(
        &["name", "size", "modified"],
        vec![
            vec![Chunk::text("Cargo.toml"), Chunk::text("512 B"),  Chunk::text("2m ago")],
            vec![Chunk::text("src/lib.rs"), Chunk::text("18.3 KB"), Chunk::text("4m ago")],
            vec![Chunk::text("Makefile"),   Chunk::text("2.1 KB"),  Chunk::text("1h ago")],
        ],
    )).ok();

    // ── log lines ─────────────────────────────────────────────────────────────
    tx.send(Chunk::log("info",  "compiler", "Compiling cyb-stream v0.1.0")).ok();
    tx.send(Chunk::log("warn",  "linker",   "unused variable `x` in src/lib.rs:42")).ok();
    tx.send(Chunk::log("error", "compiler", "mismatched types: expected `u8`, found `u32`")).ok();

    // ── progress bar ──────────────────────────────────────────────────────────
    let mut p = Chunk::progress(1, "Building cyb-shell", 72, 100);
    tx.send(p.clone()).ok();
    p = Chunk::progress(1, "Building cyb-shell", 100, 100);
    tx.send(p).ok();

    // ── error panel ───────────────────────────────────────────────────────────
    tx.send(Chunk::error("cannot borrow `engine` as mutable more than once")).ok();

    // ── action button ─────────────────────────────────────────────────────────
    let label_chunk  = Chunk::annotation("retry build");
    let ref_chunk    = Chunk::text("cmd:cargo_build");
    let action_payload = encode_nested(&[label_chunk, ref_chunk]);
    tx.send(Chunk::new(sigil::ZAP, render::COMPONENT, action_payload)).ok();

    // ── composition container ─────────────────────────────────────────────────
    let inner = encode_nested(&[
        Chunk::text("composed:"),
        Chunk::annotation("three chunks inside one container"),
        Chunk::new(sigil::PAT, render::TEXT, Bytes::from_static(b"@builder")),
    ]);
    tx.send(Chunk::compose(inner)).ok();

    // ── scope / breadcrumb ────────────────────────────────────────────────────
    let breadcrumb = encode_nested(&[
        Chunk::annotation("~/cyber/cyb"),
        Chunk::annotation("shell"),
        Chunk::annotation("worlds"),
    ]);
    tx.send(Chunk::scope(breadcrumb)).ok();

    // ── status (end of command) ───────────────────────────────────────────────
    tx.send(Chunk::status(0)).ok();

    // Second command — non-zero exit
    tx.send(Chunk::text("cargo build --release")).ok();
    tx.send(Chunk::log("error", "cargo", "could not compile `cyb-shell`")).ok();
    tx.send(Chunk::status(101)).ok();
}
