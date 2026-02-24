# Убрать egui, переписать терминал на bevy_ascii_terminal

## Context

Текущий терминал в `cyb-shell` использует `bevy_egui` для рендера текста в `egui::TextEdit` — монохром, без цветов, без курсора. egui больше нигде не используется (Game мир — чистый Bevy). Нужно:
1. Полностью убрать `bevy_egui` из проекта
2. Заменить терминальный рендер на `bevy_ascii_terminal` (оптимизированный mesh-based ASCII рендер для Bevy 0.18)
3. Сохранить `vte` + `teletypewriter` для VTE парсинга и PTY

### Почему НЕ rio-backend

`rio-backend` тянет `sugarloaf` → `wgpu 23` как хард-зависимость. Bevy 0.18 использует `wgpu 27`. Два мажорных wgpu не совместимы → конфликт компиляции. Терминальная модель rio-backend (Crosswords/Square) привязана к sugarloaf типам через re-export.

### Почему bevy_ascii_terminal

- Версия 0.18.2 совместима с Bevy 0.18
- Mesh-based рендер (один entity, кастомный шейдер) — быстрее чем 4800 Text2d entities
- Per-character fg/bg цвета из коробки
- Готовый grid API: `terminal.put_char([col, row], 'A'.fg(Color::GREEN).bg(Color::BLACK))`
- 10K+ символов stress test при 600+ fps

---

## Изменения

### 1. Cargo.toml — зависимости

**Файл**: `cyb/cyb-shell/Cargo.toml`

- Убрать: `bevy_egui = "0.39"`
- Добавить: `bevy_ascii_terminal = "0.18"`
- Оставить: `vte = "0.13"`, `teletypewriter = "2.0.1"`, `libc = "0.2"`

### 2. main.rs — убрать EguiPlugin

**Файл**: `cyb/cyb-shell/src/main.rs`

- Убрать: `use bevy_egui::EguiPlugin;`
- Убрать: `.add_plugins(EguiPlugin::default())`

### 3. terminal.rs — полная перезапись рендера

**Файл**: `cyb/cyb-shell/src/worlds/terminal.rs`

Сохраняем:
- PTY через `teletypewriter` (create_pty, reader/writer через dup fd)
- VTE парсер через `vte::Parser` + `vte::Perform`
- `TermGrid` структуру (cells, cursor) + reader thread + `Arc<Mutex<>>`
- Keyboard forwarding (Key → bytes → PTY writer)

Меняем:
- `Cell { c: char }` → `Cell { c: char, fg: Color, bg: Color }` (добавляем цвета)
- `GridPerformer::csi_dispatch` обрабатывает SGR ('m') — парсит ANSI цвета в `Color`
- Рендер: вместо egui `TextEdit` → `bevy_ascii_terminal::Terminal`
- OnEnter: спавним `Terminal::new([TERM_COLS, TERM_ROWS])` + камера
- Update: читаем `TermGrid` → пишем в `Terminal` через `terminal.put_char([col, row], styled_char)`
- OnExit: деспавним terminal entity

SGR парсинг (escape `\x1b[...m`):
- 0: reset (default fg/bg)
- 1: bold (яркие цвета)
- 30-37: fg standard colors
- 40-47: bg standard colors
- 38;5;N: fg 256-color
- 48;5;N: bg 256-color
- 38;2;R;G;B: fg truecolor
- 48;2;R;G;B: bg truecolor
- 90-97: fg bright colors
- 100-107: bg bright colors

### 4. game.rs — без изменений

Game мир уже чистый Bevy (Camera3d, Mesh3d, DirectionalLight). egui там не используется.

---

## Файлы для изменения

| Файл | Действие |
|---|---|
| `cyb/cyb-shell/Cargo.toml` | Убрать bevy_egui, добавить bevy_ascii_terminal |
| `cyb/cyb-shell/src/main.rs` | Убрать EguiPlugin |
| `cyb/cyb-shell/src/worlds/terminal.rs` | Переписать рендер |

---

## Verify

```bash
cd cyb && cargo check -p cyb-shell
# Должно скомпилироваться без ошибок

cargo run -p cyb-shell
# Cmd+3 → Terminal: ASCII терминал с цветами, работающий shell
# Cmd+2 → Game: зелёный куб вращается (без изменений)
# ls --color → цветной вывод в терминале
```
