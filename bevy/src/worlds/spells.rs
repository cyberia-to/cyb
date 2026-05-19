use bevy::prelude::*;
use bip39::Mnemonic;

use super::WorldState;
use crate::prysm::{
    theme,
    atoms::{GlassDepth, glass_bg},
    molecules::{spawn_commander, spawn_button, spawn_input},
};

pub struct SpellsWorldPlugin;

#[derive(Component)]
struct SpellsMarker;

#[derive(Component)]
struct WordPill(usize);

#[derive(Component)]
struct GenerateButton;

#[derive(Component)]
struct ImportInput;

#[derive(Resource, Default)]
struct NeuronMnemonic {
    pub words:  Vec<String>,
    pub active: bool,
}

impl Plugin for SpellsWorldPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NeuronMnemonic>()
            .add_systems(OnEnter(WorldState::Spells), show_spells)
            .add_systems(OnExit(WorldState::Spells), hide_spells)
            .add_systems(
                Update,
                (handle_generate_click, update_word_pills)
                    .run_if(in_state(WorldState::Spells)),
            );
    }
}

fn show_spells(mut commands: Commands, mnemonic: Res<NeuronMnemonic>) {
    let words = if mnemonic.words.is_empty() {
        fresh_words()
    } else {
        mnemonic.words.clone()
    };

    commands
        .spawn((
            SpellsMarker,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                padding: UiRect::all(Val::Px(theme::G * 3.0)),
                ..default()
            },
            BackgroundColor(Color::BLACK),
        ))
        .with_children(|root| {
            root.spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(theme::G * 0.5),
                    margin: UiRect::bottom(Val::Px(theme::G * 2.0)),
                    ..default()
                },
                BackgroundColor(Color::NONE),
            ))
            .with_children(|header| {
                header.spawn((
                    Text::new("spells"),
                    TextFont { font_size: theme::H2, ..default() },
                    TextColor(theme::TEXT_PRIMARY),
                ));
                header.spawn((
                    Text::new("neuron identity"),
                    TextFont { font_size: theme::CAPTION, ..default() },
                    TextColor(theme::TEXT_DIM),
                ));
            });

            root.spawn((
                Node {
                    display: Display::Grid,
                    grid_template_columns: vec![
                        RepeatedGridTrack::flex(4, 1.0),
                    ],
                    column_gap: Val::Px(theme::G),
                    row_gap: Val::Px(theme::G),
                    width: Val::Percent(100.0),
                    max_width: Val::Px(480.0),
                    ..default()
                },
                BackgroundColor(Color::NONE),
            ))
            .with_children(|grid| {
                for (i, word) in words.iter().enumerate() {
                    grid.spawn((
                        WordPill(i),
                        Node {
                            flex_direction: FlexDirection::Row,
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            padding: UiRect::all(Val::Px(theme::G)),
                            column_gap: Val::Px(theme::G * 0.5),
                            ..default()
                        },
                        glass_bg(GlassDepth::Midground),
                    ))
                    .with_children(|pill| {
                        pill.spawn((
                            Text::new(format!("{}", i + 1)),
                            TextFont { font_size: theme::MICRO, ..default() },
                            TextColor(theme::TEXT_DIM),
                        ));
                        pill.spawn((
                            Text::new(word.clone()),
                            TextFont { font_size: theme::CAPTION, ..default() },
                            TextColor(theme::TEXT_PRIMARY),
                        ));
                    });
                }
            });

            root.spawn((
                Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(theme::G),
                    width: Val::Percent(100.0),
                    max_width: Val::Px(480.0),
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::NONE),
            ))
            .with_children(|bar| {
                let btn = spawn_button(bar, "generate");
                bar.commands().entity(btn).insert(GenerateButton);

                let inp = spawn_input(bar, "import phrase");
                bar.commands().entity(inp).insert(ImportInput);
                bar.commands().entity(inp).insert(Node {
                    flex_grow: 1.0,
                    ..default()
                });
            });

            root.spawn((
                Node {
                    width: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    ..default()
                },
                BackgroundColor(Color::NONE),
            ))
            .with_children(|footer| {
                spawn_commander(footer, 0, &["spells", "graph", "sense"]);
            });
        });
}

fn hide_spells(mut commands: Commands, q: Query<Entity, With<SpellsMarker>>) {
    for e in &q {
        commands.entity(e).despawn();
    }
}

fn handle_generate_click(
    interaction_q: Query<&Interaction, (Changed<Interaction>, With<GenerateButton>)>,
    mut mnemonic:  ResMut<NeuronMnemonic>,
) {
    for interaction in &interaction_q {
        if *interaction == Interaction::Pressed {
            mnemonic.words  = fresh_words();
            mnemonic.active = true;
        }
    }
}

fn update_word_pills(
    mnemonic: Res<NeuronMnemonic>,
    mut pill_q: Query<(&WordPill, &Children)>,
    mut text_q: Query<&mut Text>,
) {
    if !mnemonic.is_changed() || mnemonic.words.is_empty() {
        return;
    }
    for (pill, children) in &mut pill_q {
        if let Some(word) = mnemonic.words.get(pill.0) {
            for child in children.iter() {
                if let Ok(mut t) = text_q.get_mut(child) {
                    let s = t.0.clone();
                    if !s.chars().next().map_or(false, |c| c.is_ascii_digit()) {
                        **t = word.clone();
                    }
                }
            }
        }
    }
}

fn fresh_words() -> Vec<String> {
    Mnemonic::generate(12)
        .map(|m| m.words().map(|w| w.to_string()).collect())
        .unwrap_or_default()
}
