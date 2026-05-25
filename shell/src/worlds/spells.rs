use bevy::prelude::*;
use bip39::Mnemonic;
use hmac::{Hmac, Mac};
use sha2::{Sha256, Sha512, Digest as _};
use k256::SecretKey;
use bech32::{ToBase32, Variant};

use super::WorldState;
use crate::prysm::{
    theme,
    atoms::{GlassDepth, glass_bg},
    molecules::{spawn_commander, spawn_button, spawn_input, TextInput},
};

type HmacSha512 = Hmac<Sha512>;

pub struct SpellsWorldPlugin;

#[derive(Component)]
struct SpellsMarker;

#[derive(Component)]
struct WordPill(usize);

#[derive(Component)]
struct GenerateButton;

#[derive(Component)]
struct ImportInput;

#[derive(Component)]
struct AddressDisplay;

#[derive(Resource, Default)]
struct NeuronMnemonic {
    pub words:   Vec<String>,
    pub active:  bool,
    pub address: String,
}

impl Plugin for SpellsWorldPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NeuronMnemonic>()
            .add_systems(OnEnter(WorldState::Spells), show_spells)
            .add_systems(OnExit(WorldState::Spells), hide_spells)
            .add_systems(
                Update,
                (handle_generate_click, handle_import_enter, update_word_pills, update_address_display)
                    .run_if(in_state(WorldState::Spells)),
            );
    }
}

fn show_spells(mut commands: Commands, mut mnemonic: ResMut<NeuronMnemonic>) {
    if mnemonic.words.is_empty() {
        let words = fresh_words();
        let address = derive_address_from_words(&words);
        mnemonic.words   = words;
        mnemonic.address = address;
        mnemonic.active  = true;
    }
    let words = mnemonic.words.clone();

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
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(theme::G),
                    width: Val::Percent(100.0),
                    max_width: Val::Px(480.0),
                    ..default()
                },
                BackgroundColor(Color::NONE),
            ))
            .with_children(|col| {
                col.spawn((
                    Node {
                        display: Display::Grid,
                        grid_template_columns: vec![
                            RepeatedGridTrack::flex(4, 1.0),
                        ],
                        column_gap: Val::Px(theme::G),
                        row_gap: Val::Px(theme::G),
                        width: Val::Percent(100.0),
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

                let addr_text = if mnemonic.address.is_empty() {
                    "generating address…".to_string()
                } else {
                    mnemonic.address.clone()
                };
                col.spawn((
                    Node {
                        padding: UiRect::axes(Val::Px(theme::G), Val::Px(theme::G * 0.5)),
                        margin: UiRect::top(Val::Px(theme::G * 0.5)),
                        ..default()
                    },
                    glass_bg(GlassDepth::Subtle),
                ))
                .with_children(|addr_row| {
                    addr_row.spawn((
                        AddressDisplay,
                        Text::new(addr_text),
                        TextFont { font_size: theme::MICRO, ..default() },
                        TextColor(theme::ACID_BLUE),
                    ));
                });
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
            let words = fresh_words();
            let address = derive_address_from_words(&words);
            mnemonic.words   = words;
            mnemonic.active  = true;
            mnemonic.address = address;
        }
    }
}

fn handle_import_enter(
    keys:         Res<ButtonInput<KeyCode>>,
    input_q:      Query<&TextInput, With<ImportInput>>,
    mut mnemonic: ResMut<NeuronMnemonic>,
) {
    if !keys.just_pressed(KeyCode::Enter) { return; }
    let Ok(input) = input_q.single() else { return };
    let phrase = input.value.trim().to_string();
    if phrase.is_empty() { return; }
    if let Ok(m) = bip39::Mnemonic::parse(&phrase) {
        let address = derive_address(&m, "bostrom");
        mnemonic.words   = m.words().map(|w| w.to_string()).collect();
        mnemonic.active  = true;
        mnemonic.address = address;
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

fn update_address_display(
    mnemonic: Res<NeuronMnemonic>,
    mut addr_q: Query<&mut Text, With<AddressDisplay>>,
) {
    if !mnemonic.is_changed() { return; }
    for mut text in &mut addr_q {
        **text = if mnemonic.address.is_empty() {
            "…".to_string()
        } else {
            mnemonic.address.clone()
        };
    }
}

fn fresh_words() -> Vec<String> {
    Mnemonic::generate(12)
        .map(|m| m.words().map(|w| w.to_string()).collect())
        .unwrap_or_default()
}

fn derive_address_from_words(words: &[String]) -> String {
    let phrase = words.join(" ");
    if let Ok(m) = Mnemonic::parse(&phrase) {
        derive_address(&m, "bostrom")
    } else {
        String::new()
    }
}

fn derive_address(mnemonic: &Mnemonic, prefix: &str) -> String {
    let seed = mnemonic.to_seed("");
    let mut mac = HmacSha512::new_from_slice(b"Bitcoin seed").unwrap();
    mac.update(&seed);
    let result = mac.finalize().into_bytes();
    let mut master_key = [0u8; 32];
    let mut master_chain = [0u8; 32];
    master_key.copy_from_slice(&result[..32]);
    master_chain.copy_from_slice(&result[32..]);

    let path = [44u32 | 0x80000000, 118 | 0x80000000, 0 | 0x80000000, 0, 0];
    let mut key = master_key;
    let mut chain = master_chain;
    for &index in &path {
        let (k2, c2) = ckd_priv(&key, &chain, index);
        key = k2;
        chain = c2;
    }

    let sk = SecretKey::from_bytes((&key).into()).unwrap();
    let pk = sk.public_key();
    let pk_bytes = pk.to_sec1_bytes();

    let sha = Sha256::digest(&pk_bytes);
    let addr_bytes = ripemd160(&sha);

    bech32::encode(prefix, addr_bytes.to_base32(), Variant::Bech32).unwrap_or_default()
}

fn ckd_priv(key: &[u8; 32], chain: &[u8; 32], index: u32) -> ([u8; 32], [u8; 32]) {
    use k256::elliptic_curve::ff::PrimeField;

    let mut mac = HmacSha512::new_from_slice(chain).unwrap();
    if index >= 0x80000000 {
        mac.update(&[0u8]);
        mac.update(key);
    } else {
        let sk = SecretKey::from_bytes(key.into()).unwrap();
        let pk = sk.public_key().to_sec1_bytes();
        mac.update(&pk);
    }
    mac.update(&index.to_be_bytes());
    let result = mac.finalize().into_bytes();

    let mut il_bytes = [0u8; 32];
    il_bytes.copy_from_slice(&result[..32]);
    let il = k256::Scalar::from_repr(il_bytes.into()).unwrap();

    let parent = k256::Scalar::from_repr((*key).into()).unwrap();
    let child = il + parent;

    let child_repr = child.to_repr();
    let mut k = [0u8; 32];
    let mut c = [0u8; 32];
    k.copy_from_slice(&child_repr);
    c.copy_from_slice(&result[32..]);
    (k, c)
}

fn ripemd160(input: &[u8]) -> [u8; 20] {
    const K1: [u32; 5] = [0x00000000, 0x5A827999, 0x6ED9EBA1, 0x8F1BBCDC, 0xA953FD4E];
    const K2: [u32; 5] = [0x50A28BE6, 0x5C4DD124, 0x6D703EF3, 0x7A6D76E9, 0x00000000];
    const RL: [usize; 80] = [
        0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,
        7,4,13,1,10,6,15,3,12,0,9,5,2,14,11,8,
        3,10,14,4,9,15,8,1,2,7,0,6,13,11,5,12,
        1,9,11,10,0,8,12,4,13,3,7,15,14,5,6,2,
        4,0,5,9,7,12,2,10,14,1,3,8,11,6,15,13,
    ];
    const RR: [usize; 80] = [
        5,14,7,0,9,2,11,4,13,6,15,8,1,10,3,12,
        6,11,3,7,0,13,5,10,14,15,8,12,4,9,1,2,
        15,5,1,3,7,14,6,9,11,8,12,2,10,0,4,13,
        8,6,4,1,3,11,15,0,5,12,2,13,9,7,10,14,
        12,15,10,4,1,5,8,7,6,2,13,14,0,3,9,11,
    ];
    const SL: [u32; 80] = [
        11,14,15,12,5,8,7,9,11,13,14,15,6,7,9,8,
        7,6,8,13,11,9,7,15,7,12,15,9,11,7,13,12,
        11,13,6,7,14,9,13,15,14,8,13,6,5,12,7,5,
        11,12,14,15,14,15,9,8,9,14,5,6,8,6,5,12,
        9,15,5,11,6,8,13,12,5,12,13,14,11,8,5,6,
    ];
    const SR: [u32; 80] = [
        8,9,9,11,13,15,15,5,7,7,8,11,14,14,12,6,
        9,13,15,7,12,8,9,11,7,7,12,7,6,15,13,11,
        9,7,15,11,8,6,6,14,12,13,5,14,13,13,7,5,
        15,5,8,11,14,14,6,14,6,9,12,9,12,5,15,8,
        8,5,12,9,12,5,14,6,8,13,6,5,15,13,11,11,
    ];

    fn f0(x: u32, y: u32, z: u32) -> u32 { x ^ y ^ z }
    fn f1(x: u32, y: u32, z: u32) -> u32 { (x & y) | (!x & z) }
    fn f2(x: u32, y: u32, z: u32) -> u32 { (x | !y) ^ z }
    fn f3(x: u32, y: u32, z: u32) -> u32 { (x & z) | (y & !z) }
    fn f4(x: u32, y: u32, z: u32) -> u32 { x ^ (y | !z) }
    fn rol(x: u32, n: u32) -> u32 { x.rotate_left(n) }

    let msg_len = input.len();
    let mut msg = input.to_vec();
    msg.push(0x80);
    while msg.len() % 64 != 56 { msg.push(0); }
    let bit_len = (msg_len as u64) * 8;
    msg.extend_from_slice(&bit_len.to_le_bytes());

    let mut h = [0x67452301u32, 0xEFCDAB89, 0x98BADCFE, 0x10325476, 0xC3D2E1F0];

    for chunk in msg.chunks(64) {
        let mut x = [0u32; 16];
        for i in 0..16 {
            x[i] = u32::from_le_bytes(chunk[i*4..i*4+4].try_into().unwrap());
        }
        let (mut al, mut bl, mut cl, mut dl, mut el) = (h[0], h[1], h[2], h[3], h[4]);
        let (mut ar, mut br, mut cr, mut dr, mut er) = (h[0], h[1], h[2], h[3], h[4]);

        for j in 0..80 {
            let (fl, kl) = match j {
                 0..=15 => (f0(bl, cl, dl), K1[0]),
                16..=31 => (f1(bl, cl, dl), K1[1]),
                32..=47 => (f2(bl, cl, dl), K1[2]),
                48..=63 => (f3(bl, cl, dl), K1[3]),
                _       => (f4(bl, cl, dl), K1[4]),
            };
            let t = rol(al.wrapping_add(fl).wrapping_add(x[RL[j]]).wrapping_add(kl), SL[j]).wrapping_add(el);
            al = el; el = dl; dl = rol(cl, 10); cl = bl; bl = t;

            let (fr, kr) = match j {
                 0..=15 => (f4(br, cr, dr), K2[0]),
                16..=31 => (f3(br, cr, dr), K2[1]),
                32..=47 => (f2(br, cr, dr), K2[2]),
                48..=63 => (f1(br, cr, dr), K2[3]),
                _       => (f0(br, cr, dr), K2[4]),
            };
            let t = rol(ar.wrapping_add(fr).wrapping_add(x[RR[j]]).wrapping_add(kr), SR[j]).wrapping_add(er);
            ar = er; er = dr; dr = rol(cr, 10); cr = br; br = t;
        }

        let t = h[1].wrapping_add(cl).wrapping_add(dr);
        h[1] = h[2].wrapping_add(dl).wrapping_add(er);
        h[2] = h[3].wrapping_add(el).wrapping_add(ar);
        h[3] = h[4].wrapping_add(al).wrapping_add(br);
        h[4] = h[0].wrapping_add(bl).wrapping_add(cr);
        h[0] = t;
    }

    let mut out = [0u8; 20];
    for i in 0..5 {
        out[i*4..i*4+4].copy_from_slice(&h[i].to_le_bytes());
    }
    out
}
