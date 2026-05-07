---
tags: cyb, core
crystal-type: entity
crystal-domain: cyber
---
# particle handling

how [[cyb]] resolves, fetches, and renders [[particles]] from the [[cybergraph]]

## resolution

a [[particle]] is a content-addressed hash ([[hemera]] digest). when [[cyb/brain]] or [[cyb/oracle]] encounters a particle CID:

1. check local cache (SQLite)
2. check local [[radio]] blob store
3. fetch from P2P network via [[radio/blob]]
4. verify hash matches content

no HTTP, no URLs, no DNS. identity IS the address

## content type detection

every particle carries raw bytes. [[cyb]] infers the content type from magic bytes and structure:

| detected type | render pipeline | GPU primitive |
|---------------|----------------|--------------|
| [[text]] (markdown) | parse → glyph layout → GPU atlas | [[cyb/wgpu]] fragment shader |
| [[pixels]] (png, webp, jpg) | decode → texture upload | [[cyb/wgpu]] sampler |
| [[video]] (mp4, webm) | hardware decode → frame texture | [[cyb/wgpu]] temporal sampler |
| [[sound]] (wav, ogg, mp3) | decode → audio pipeline + waveform compute | [[cyb/wgpu]] compute shader |
| [[formula]] (latex) | parse → glyph + Vello paths | [[cyb/wgpu]] compute fill |
| [[vector]] (svg) | parse → Vello tiling | [[cyb/wgpu]] compute fill |
| [[table]] (csv) | parse → virtualized grid | [[cyb/wgpu]] text cells |
| [[struct]] (json, toml) | parse → collapsible tree | [[cyb/wgpu]] text render |
| [[component]] (wasm) | instantiate module → own render pass | [[cyb/wgpu]] composite |

if content type is ambiguous, [[cyb/onnx]] classification model resolves it

## render pipeline

all types flow through one pipeline:

```
raw bytes
  → content type detection
  → type-specific parser
  → layout (stream position, size)
  → GPU upload (texture / glyph atlas / path tiles)
  → fragment shader composite
  → screen
```

each particle renders independently. no cascading styles, no reflow between particles. a [[cyb/brain]] page is a stream of particles, each self-contained

## caching

- parsed content cached in SQLite with CID as key
- GPU resources (textures, glyph atlases) cached per session
- [[radio/bao]] verified streaming for large particles (progressive render while downloading)

## in cyb/brain

[[cyb/brain]] displays particles through tabs:

- graph: 3d spatial render
- list: table with analytics
- heap: 2d knowledge graph
- stack: vertical scroll (default)
- hike: single particle focus

each tab is a different layout mode over the same particle stream

see [[particle]] for the protocol concept. see [[cyb/features]] for the PureRender engine. see [[cyb/wgpu]] for GPU abstraction