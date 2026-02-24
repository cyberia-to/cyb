# Plan: Rio-powered Terminal via Forked Sugarloaf

## Context

Текущий терминал работает, но на CPU-блитинге через swash — это костыль. Нужен полноценный GPU-рендерер Rio (sugarloaf). wgpu версии совпадают: sugarloaf 0.2.37 = wgpu 27, Bevy 0.18 = wgpu 27. Конфликта нет.

Проблема: sugarloaf::Context создаёт свой wgpu Instance/Device/Surface. Для Bevy нужно рендерить в offscreen текстуру через Device/Queue от Bevy.

## Подход: Vendor sugarloaf + rio-backend + copa

Копируем исходники sugarloaf/rio-backend/copa в `cyb/vendor/`, модифицируем:
1. `sugarloaf/context` — добавляем `Context::from_device_queue()` (принимает внешний device/queue)
2. `sugarloaf/sugarloaf.rs` — добавляем `render_to_texture()` (рендерит в wgpu::TextureView вместо surface)
3. `terminal.rs` — полная интеграция: rio-backend Crosswords + sugarloaf рендер в Bevy Image

## Шаги

### Step 1: Vendor Rio crates

Копируем из `~/.cargo/registry/src/` в `cyb/vendor/`:
- `sugarloaf-0.2.37/` → `cyb/vendor/sugarloaf/`
- `rio-backend-0.2.37/` → `cyb/vendor/rio-backend/`
- `copa-0.2.37/` → `cyb/vendor/copa/`

Обновляем `cyb/Cargo.toml` workspace members + path deps.
Обновляем `cyb/cyb-shell/Cargo.toml`:
```toml
sugarloaf = { path = "../vendor/sugarloaf" }
rio-backend = { path = "../vendor/rio-backend" }
copa = { path = "../vendor/copa" }
```

Убираем `rio-backend` из crates.io deps (был добавлен для скачивания).

**Файлы**: `cyb/Cargo.toml`, `cyb/cyb-shell/Cargo.toml`, `cyb/vendor/`

### Step 2: Patch sugarloaf Context

В `vendor/sugarloaf/src/context/mod.rs` добавляем:

```rust
impl Context<'_> {
    pub fn from_device_queue(
        device: wgpu::Device,
        queue: wgpu::Queue,
        format: wgpu::TextureFormat,
        width: u32,
        height: u32,
        scale: f32,
    ) -> Context<'static> {
        // Создаём offscreen текстуру вместо surface
        // Возвращаем Context без surface (Surface заменяем на offscreen texture)
    }
}
```

Основная модификация: Context хранит `Option<Surface>` + offscreen `Texture` для headless рендеринга.

**Файл**: `cyb/vendor/sugarloaf/src/context/mod.rs`

### Step 3: Patch sugarloaf render

В `vendor/sugarloaf/src/sugarloaf.rs` добавляем `render_to_texture()`:
- Вместо `surface.get_current_texture()` используем offscreen texture view
- Вместо `frame.present()` — `queue.submit(encoder)`
- Возвращаем `&wgpu::Texture` для чтения Bevy

**Файл**: `cyb/vendor/sugarloaf/src/sugarloaf.rs`

### Step 4: Rewrite terminal.rs

Новая архитектура:
```
PTY (zsh) ←→ rio-backend::Crosswords (grid + ANSI parser via copa)
                    ↓ Grid<Square> cells
              sugarloaf::Sugarloaf (GPU render → offscreen wgpu::Texture)
                    ↓ texture
              Bevy Image (Handle<Image> → Sprite fullscreen)
```

**OnEnter(Terminal)**:
1. Создать PTY через rio-backend (или teletypewriter)
2. Создать `Crosswords<BevyEventProxy>` (rio-backend grid)
3. Получить `RenderDevice`/`RenderQueue` от Bevy
4. Создать `Sugarloaf` с `Context::from_device_queue()`
5. Создать Bevy `Image` + `Sprite`

**Update** (Terminal state):
1. Read PTY → feed `copa` parser → update `Crosswords` grid
2. Convert grid to sugarloaf `Content` (fragments с цветами/стилями)
3. `sugarloaf.render_to_texture()` → offscreen wgpu::Texture
4. Copy texture data → Bevy Image (или share handle)

**OnExit(Terminal)**: cleanup всё

**Файл**: `cyb/cyb-shell/src/worlds/terminal.rs`

### Step 5: Build and test

```bash
cd cyb && make dmg
```

## Файлы

| Файл | Действие |
|---|---|
| `cyb/Cargo.toml` | workspace members += vendor/* |
| `cyb/cyb-shell/Cargo.toml` | path deps на vendor |
| `cyb/vendor/sugarloaf/` | vendor + patch Context + render |
| `cyb/vendor/rio-backend/` | vendor (path dep на local sugarloaf) |
| `cyb/vendor/copa/` | vendor (без изменений) |
| `cyb/cyb-shell/src/worlds/terminal.rs` | полный переписать на rio stack |
| `cyb/cyb-shell/src/main.rs` | без изменений |

## Риски

- sugarloaf зависит от ~40 crates (skrifa, half, png, etc.) — vendor только sugarloaf/rio-backend/copa, остальное из crates.io
- Патч Context требует менять internal fields — при обновлении Rio надо мержить
- RenderDevice/RenderQueue доступны только в render world — может потребоваться render-to-texture через Bevy camera pipeline

## Проверка

1. `cargo check --workspace` — компилируется
2. `cd cyb && make dmg` → DMG
3. Запустить → Cmd+3 → Terminal:
   - Шрифт рендерится GPU (не CPU-блитинг)
   - Цвета, bold, italic, underline
   - Unicode, emoji, powerline символы
   - `vim`, `htop`, `ls --color` работают
   - Ресайз окна → терминал адаптируется
4. Cmd+4 → Cmd+3 → без крашей
