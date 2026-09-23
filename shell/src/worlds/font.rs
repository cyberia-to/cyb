//! The UI face: JetBrains Mono, which actually contains Cyrillic.
//!
//! Bevy's built-in default is a Latin-only face. Every TextFont spawned
//! without an explicit handle wears that, so Russian (and anything outside
//! Latin-1) renders as tofu. We load one face into the asset store and
//! stamp it onto every new TextFont.

use bevy::prelude::*;
use bevy::text::Font;

const FACE: &[u8] = include_bytes!("../../assets/JetBrainsMono-Regular.ttf");

#[derive(Resource, Clone)]
struct AppFont(Handle<Font>);

pub struct FontPlugin;

impl Plugin for FontPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreStartup, load)
            .add_systems(PostUpdate, apply);
    }
}

fn load(mut fonts: ResMut<Assets<Font>>, mut commands: Commands) {
    match Font::try_from_bytes(FACE.to_vec()) {
        Ok(font) => {
            commands.insert_resource(AppFont(fonts.add(font)));
        }
        Err(e) => warn!("ui font: {e:?} — falling back to Bevy default"),
    }
}

fn apply(font: Option<Res<AppFont>>, mut q: Query<&mut TextFont, Added<TextFont>>) {
    let Some(font) = font else {
        return;
    };
    for mut tf in &mut q {
        tf.font = font.0.clone();
    }
}
