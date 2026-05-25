# agent-research.md

Research on 20 agent / sandbox / runtime projects (May 2026).
Two questions answered:

1. Which parts of which projects can we lift into the cyb robot?
2. What architectural ideas are common, cool, and necessary for a modern agent?

Cyb context: Rust workspace (`bevy/`, `react/`, `leptos/`, `nu/`), macOS / Linux / iOS / Android, immortal robot on a content-addressed graph. We already have a desktop shell, web UI, scripting layer, and a knowledge graph. What we are missing is the agent runtime layer: sandboxed execution, durable loops, tool registry, skill memory, multi-agent coordination, desktop perception.

---

## Per-repo digest

Format: language · core idea · what cyb takes.

### Sandboxing / isolation

#### [microsandbox](https://github.com/superradcompany/microsandbox)
Rust + Go + TS + Python · KVM microVMs that boot in <100ms, OCI-image compatible, embeddable SDK (no daemon required), MCP server for Claude integration.
Cyb takes: the `Sandbox::builder` fluent API shape; the "embeddable SDK, no daemon" pattern; MCP server adapter for exposing cyb tools to other agents.

#### [CubeSandbox](https://github.com/TencentCloud/CubeSandbox)
Rust + Go + C · KVM microVMs with sub-60ms cold start, <5MB per instance, eBPF egress filtering, E2B-compatible REST gateway, snapshot cloning from pre-warmed pools.
Cyb takes: the `CubeVS` eBPF network-policy pattern as inspiration for cyb's egress firewall; pre-warmed sandbox pools for instant tool execution; E2B-compatible API surface so existing agent code maps cleanly.

#### [worktrunk](https://github.com/max-sixty/worktrunk)
Rust CLI · git worktree manager addressed by branch name, with per-worktree port assignment, build-cache sharing, post-create hooks, LLM-generated commit messages.
Cyb takes: the whole tool, almost as-is. Cyb spawns N parallel sub-agents — each one needs an isolated worktree with shared `target/`. This is exactly that. Port-template filters fit cyb's dev-server pattern.

#### [agentfs](https://github.com/tursodatabase/agentfs)
Rust + SQLite (Turso) · filesystem-in-SQLite mounted via FUSE/NFS, COW overlays, full audit trail of every syscall, time-travel forking via WAL.
Cyb takes: the SQLite-as-filesystem model for agent workspaces (snapshot = `cp db`, fork = WAL replay). The "queryable history of agent actions" is a near-perfect fit for cyb's audit / karma layer — every tool call becomes a row, and the graph can ask the agent "why did you do X" by SQL.

### Rust agent frameworks

#### [rig](https://github.com/0xPlaygrounds/rig)
Pure Rust, Tokio · 20+ LLM providers + 10+ vector stores under unified traits, GenAI Semantic Convention compatible, full WASM build.
Cyb takes: `rig-core` as cyb's LLM provider layer — this is the closest thing to "the React of LLM clients" in Rust. Vector-store traits map cleanly onto cyb's IPFS+graph backend. WASM build matters because cyb's leptos UI runs in the browser.

#### [chidori](https://github.com/ThousandBirdsInc/chidori)
Rust + Starlark · deterministic agent runtime; every side effect goes through logged host functions, enabling pause / resume / replay with cached LLM responses.
Cyb takes: the replay-as-foundation pattern. Cyb's "immortal" framing demands this — an agent that survives a process crash needs deterministic resume. The Starlark-as-DSL idea is interesting but cyb has nushell for the same job; the value is in `call_log.rs`-style checkpointing.

#### [adk-rust](https://github.com/zavora-ai/adk-rust)
Rust 2024, Tokio · Google ADK port, layered crates (`adk-core`, `adk-agent`, `adk-model`, `adk-tool`, `adk-runner`), realtime voice, A2A v1.0, graph-based workflows, `cargo adk` scaffolding.
Cyb takes: the crate layering itself — `adk-core` trait shape is the cleanest agent-framework decomposition in the Rust ecosystem. A2A protocol compliance for cyb-to-cyb agent talk.

#### [autoagents](https://github.com/liquidos-ai/autoagents)
Rust, Tokio · multi-agent with typed pub/sub messaging, WASM tool sandbox, guardrails crate, structured outputs via proc-macro (`AgentOutputT`).
Cyb takes: `AgentOutputT`-style structured-output macros (this is what makes Rust agents ergonomic); WASM tool sandbox pattern; guardrails crate as a separable layer cyb can wrap around any tool.

#### [stakpak/agent](https://github.com/stakpak/agent)
Pure Rust · DevOps autopilot with TUI + autopilot modes, Secret Substitution (LLM never sees credentials), Warden Guardrails (network-level kill rules), mTLS MCP, reversible file ops with backups.
Cyb takes: Secret Substitution is critical — cyb runs on user devices with keychains and seed phrases, the LLM should never see them. Reversible-file-ops with auto-backup matches the immortal-robot promise: nothing the agent does is unrecoverable.

### Distributed / agent OS

#### [golem](https://github.com/golemcloud/golem)
Rust + MoonBit + Scala · WASM Component Model + WIT interfaces, durable execution (workers survive crashes), `golem-worker-executor`, `golem-shard-manager`.
Cyb takes: WIT interface definitions as cyb's agent ABI — a tool defined in WIT can be implemented in any language and called from Rust, Nu, React, or Leptos. Durable-execution pattern (state journaling) for long-running agent jobs.

#### [agent-os](https://github.com/rivet-dev/agent-os) (rivet-dev)
Rust + TypeScript · V8 isolates as agent containers (~6ms cold start), JS kernel mounting WASM (POSIX utils) + V8 (agent code) + virtual FS, ACP session protocol, multiplayer (multiple humans watching one agent), agent-to-agent delegation.
Cyb takes: the in-process kernel pattern (no host process spawn) for cyb's tool layer; multiplayer agent sessions for cyb's collective use cases; ACP for editor integration.

#### [zerostack](https://github.com/gi-dellav/zerostack)
Pure Rust, single binary · "Ralph Wiggum loops" for long-horizon iteration, prompt-switching (code/plan/review/debug) as an alternative to skills, MCP + ACP, integrated Exa search, four permission modes with glob rules.
Cyb takes: prompt-switching as a first-class UX primitive; the four-mode permission model (`safe`/`balanced`/`permissive`/`yolo`); Ralph Wiggum loops as a named retry pattern.

#### [spacebot](https://github.com/spacedriveapp/spacebot)
Rust + TS · 5-process model: Channels (user-facing), Branches (concurrent thinking), Workers (focused executors), Compactor (non-LLM context manager), Cortex (cross-channel knowledge graph). Discord / Slack / Telegram / Email gateways. Spacedrive P2P for multi-device single-agent.
Cyb takes: the 5-process decomposition is the most thoughtful agent topology in the list — especially the Compactor (non-LLM context management) and Cortex (graph-level memory across channels). Channel adapters for cyb's multi-platform reach. P2P-single-agent maps onto cyb's "one robot, many devices" vision.

#### [herdr](https://github.com/ogulcancelik/herdr)
Pure Rust, single binary · tmux-alternative with agent state detection (blocked / working / done / idle), Unix-socket API, SSH remote attach, agents can self-orchestrate via socket.
Cyb takes: the agent-state detection model (state machine for "is this agent blocked / done"); the Unix-socket control plane so cyb agents can orchestrate each other; SSH-attach for cyb-on-remote-server.

### Embodied / desktop / coding agents

#### [iii](https://github.com/iii-hq/iii) (iii-hq)
Rust + multi-language SDKs · backend composition with three primitives: Workers (processes), Triggers (declarative events), Functions (units of work). Agents register and discover capabilities at runtime; unified trace surface.
Cyb takes: the Workers/Triggers/Functions triplet as cyb's task-graph primitive. Runtime extensibility (agents adding workers while the system is alive) is exactly the immortal-robot promise. Single trace surface across all workers.

#### [pi_agent_rust](https://github.com/Dicklesworthstone/pi_agent_rust)
Rust 2024, single binary (~21MB) · custom async runtime `asupersync`, QuickJS extensions with capability profiles, command-level shell mediation (AST-blocks `rm -rf`, reverse shells), CUSUM+BOCPD regime detection, conformal prediction envelopes, process-tree management for orphan cleanup.
Cyb takes: this is the most security-mature agent in the list. The capability-gated QuickJS extension model fits cyb's existing JS-extension surface. Command-level mediation (catching dangerous shells before spawn) is something cyb needs. The trust lifecycle (`pending → acknowledged → trusted → killed`) is a clean state machine for extension governance.

#### [glass-hands](https://github.com/sachaarbonel/glass-hands)
Rust · OpenAI Computer-Use-Agent client driving live Chromium via CDP, isolated user-data-dir per run, disk-backed snapshot store of agent decisions.
Cyb takes: the `ChromiumComputer` + `CuaClient` + `CuaReasoner` split as cyb's browser-control layer. Disk-snapshot store for agent decisions is replay-friendly.

#### [opencode](https://github.com/anomalyco/opencode) (anomalyco fork)
TypeScript + Bun · multi-mode agents (Build / Plan / General) with Tab-key switching, permission-based command exec in Plan mode, desktop app across all platforms.
Cyb takes: Tab-to-switch-mode is a great UX pattern for the cyb shell. Multi-agent monorepo via Turbo (less relevant for cyb's Cargo workspace).

#### [openfang](https://github.com/RightNow-AI/openfang)
Rust, ~32MB binary · 14-crate decomposition (137k LoC), 27 LLM providers, WASM dual-metered sandbox with watchdog, 40 channel adapters, Merkle hash-chain audit, taint tracking, 60 skills + 7 autonomous "Hands". Multi-phase 500+ word system prompts as playbooks.
Cyb takes: this is the most complete reference architecture for a Rust agent OS in the list. The crate decomposition is the model to study (kernel / runtime / channels / api / skills / hands). Merkle hash-chain audit + taint tracking match cyb's content-addressed-everything ethos. Multi-phase playbooks as system prompts is a documented pattern worth adopting.

Note: this is not the same as cyberia's own [OpenFang](../cyberia/midao/agents.md). Naming collision — different project, same vibe.

### Skill-learning / multi-platform

#### [hermes-agent](https://github.com/nousresearch/hermes-agent) (Nous Research)
Python (88%) + TS · self-improving agent with built-in learning loop: skills auto-curated from execution, self-improve during use, agent nudges itself to persist knowledge, FTS5 cross-session search, Honcho user modeling. Multi-gateway (Telegram / Discord / Slack / Signal / WhatsApp / Email). Seven terminal backends (local / Docker / SSH / Singularity / Modal / Daytona / Vercel). agentskills.io standard. Built-in cron with platform-aware delivery.
Cyb takes: the learning loop is the most interesting idea here — agent auto-extracts skills, then auto-improves them. This maps perfectly onto cyb's "neuron" abstraction. agentskills.io as a portable skill format. Cron-with-delivery-channel as a first-class primitive (cyb's robot should be able to schedule itself).

Hermes vs LangChain: LangChain abstracts tools; Hermes abstracts continuity — sessions, platforms, user models, scheduled work. CrewAI is multi-agent within one chat; Hermes is one agent across many channels. Cyb's instinct is much closer to Hermes than to LangChain.

---

## Question 1: what cyb can lift

Ordered by leverage (highest first). Each is a concrete next move, not a vague suggestion.

### tier 1: ship now

`rig` as the LLM provider layer.
Drop `rig-core` into cyb's workspace, replace any ad-hoc Anthropic/OpenAI calls. WASM build covers leptos. 20+ providers solved overnight. This is the single highest-leverage move in the list.

`worktrunk` for parallel sub-agents.
Cyb is going to spawn parallel agents. Each needs an isolated worktree with shared `target/` cache. `worktrunk` is exactly this and is pure Rust. Either vendor or call as a binary.

`agentfs` SQLite-as-FS for agent scratch space.
Every sub-agent gets a fresh `agent-<id>.db`. Snapshot = `cp`. Fork = WAL replay. Audit trail comes free. Maps onto cyb's content-addressed model: each db is a particle.

Secret Substitution (from `stakpak/agent`).
Cyb runs next to keychain, seed phrase, IPFS keys. The LLM must not see them. Implement a redact-on-egress / restore-on-tool-call layer before cyb gets any agent work.

### tier 2: design now, build next

WIT-based tool ABI (from `golem`).
Define cyb's tool surface in WIT. Implementations in Rust, nushell wrappers, TS for leptos — all callable from one place. This is also what lets cyb's nu/react/leptos/bevy crates share a tool registry.

Spacebot 5-process topology.
Channels (UI surfaces) / Branches (concurrent thinking) / Workers (focused tools) / Compactor (non-LLM context manager) / Cortex (graph-level cross-session memory). This is the cleanest agent topology in the survey and it fits cyb's existing graph layer for the Cortex role.

ACP + MCP + A2A protocol support.
Three standards: editor integration (ACP), tool federation (MCP), agent federation (A2A). cyb should speak all three. `adk-rust` and `stakpak/agent` show the Rust patterns.

Capability-gated extensions (from `pi_agent_rust`).
QuickJS-style sandbox with safe / balanced / permissive profiles. Trust lifecycle: pending → acknowledged → trusted → killed, audited and persisted.

### tier 3: study, copy patterns later

OpenFang's 14-crate decomposition — read it for naming and module split when cyb's agent crates grow past 3-4.

Chidori's `call_log.rs` checkpoint pattern — copy when cyb implements pause/resume.

CubeSandbox eBPF egress filtering — copy when cyb needs to harden a single tool against network exfiltration.

Hermes learning loop — copy when cyb is ready for skill auto-curation (this is a step-2 feature; needs the runtime first).

iii Workers/Triggers/Functions — copy as cyb's cron / event scheduler API shape.

---

## Question 2: architectural patterns common across the modern agent

Sixteen ideas show up over and over. Roughly grouped:

### runtime shape

1. Single static binary, no runtime deps. Almost every Rust project ships this way (`zerostack`, `stakpak/agent`, `openfang`, `herdr`, `pi_agent_rust`). The robot installs in one move and runs.

2. Async-first on Tokio. Universal. No exceptions among the Rust projects.

3. Embeddable SDK over standalone daemon. `microsandbox`, `rig`, `adk-rust` all explicitly target embedding. The agent runtime is a crate, not a service.

4. WASM as the universal tool/extension format. `autoagents`, `openfang`, `golem`, `agent-os` all use WASM as the isolation boundary for untrusted code. WIT (`golem`) is the emerging interface standard.

5. V8/QuickJS isolates as a lighter alternative to WASM. `agent-os` and `pi_agent_rust` use JS isolates for ~6ms cold starts vs ~100ms for microVMs.

### sandboxing layers (pick a depth)

6. KVM microVMs (`microsandbox`, `CubeSandbox`) — strongest isolation, ~60-100ms cold start, ~5MB overhead per instance.

7. WASM components (`autoagents`, `golem`, `openfang`) — language-agnostic, fast, sandboxed by default. WIT for typed interfaces.

8. JS isolates (`agent-os`, `pi_agent_rust`) — fastest cold start (~6ms), good enough for most tool calls.

9. OS sandboxing (`spacebot`) — bubblewrap (Linux) / sandbox-exec (macOS) for local filesystem containment.

10. Capability profiles + command mediation (`pi_agent_rust`, `zerostack`) — AST-level inspection of shell commands before exec. Catches the long tail microVMs don't.

### state / memory / durability

11. Replay / checkpoint / resume as primitives. `chidori` and `golem` make this the foundation. Modern agents are expected to survive a crash without losing or duplicating work.

12. Structured storage over markdown files. `spacebot` explicit: "state belongs in structured storage, not markdown files the LLM manages." `agentfs` puts the whole filesystem in SQL.

13. Multi-tier memory. Procedural (skills) + semantic (vector + FTS) + user model. `hermes-agent` is the cleanest example.

14. Snapshot-and-fork as a first-class op. `agentfs` (WAL replay), `glass-hands` (disk snapshot store), `CubeSandbox` (snapshot cloning).

### agent topology

15. Multi-process decomposition. The frontier shape is not "one agent loop" — it's many specialized processes:
    - User-facing (Channels)
    - Concurrent reasoning (Branches)
    - Focused executors (Workers)
    - Non-LLM context manager (Compactor)
    - Cross-session graph memory (Cortex)
   `spacebot` named this best, but it shows up in pieces across `openfang`, `hermes-agent`, `agent-os`.

16. Agent-state machine (blocked / working / done / idle). `herdr` is built on this. `agent-os` uses it for session resumption. Cyb needs this for any UI that shows "what is my robot doing."

### identity / trust / safety

17. Secret substitution. The LLM should never see credentials. Substitute, exec, restore. `stakpak/agent` formalized this.

18. Reversible / auditable everything. Hash-chain audit (`openfang`), file backups (`stakpak`), SQL audit trail (`agentfs`), disk decision snapshots (`glass-hands`). The immortal-robot premise demands undo.

19. Trust lifecycle for extensions. pending → acknowledged → trusted → killed, with operator provenance and audit log. `pi_agent_rust` is the model.

20. Permission modes as UX. Four modes (safe/balanced/permissive/yolo) is the emerging standard (`zerostack`, `pi_agent_rust`). One slider, not 50 toggles.

### connectivity / multi-channel

21. Multi-gateway as a first-class layer. The modern agent talks across Telegram / Discord / Slack / Email / Signal / web / CLI / TUI from a single core. `openfang` (40 adapters), `hermes-agent` (8+), `spacebot` (8+). Per-platform message coalescing.

22. Multiplayer / shared-agent sessions. Multiple humans watch and collaborate with one agent. `agent-os` and `spacebot` both implement it.

23. Cron / scheduled-self-invocation. The agent must be able to schedule itself with platform-aware delivery. `hermes-agent` cron module is the reference; `stakpak/agent` does it for DevOps.

### protocols (the emerging stack)

24. MCP (Model Context Protocol) — tool federation. Universal across the survey.
25. ACP (Agent Client Protocol) — editor integration. `stakpak`, `zerostack`, `agent-os`.
26. A2A (Agent-to-Agent) — agent federation. `adk-rust` v1.0 compliance.
27. WIT (WASM Interface Types) — language-agnostic tool ABI. `golem`.

Pick at least three. Cyb should speak MCP, ACP, A2A from day one.

### learning / self-improvement

28. Autonomous skill extraction. After completing a complex task, the agent writes a reusable skill. `hermes-agent` and `spacebot` both implement this. agentskills.io is the emerging portable format.

29. Self-nudged reflection. The agent schedules its own post-conversation reflection to update user model and skill library. `hermes-agent` and `spacebot`.

---

## The shape of the cyb robot, derived

Pulling everything together, the modern Rust agent looks like this:

```
cyb-kernel              orchestration, scheduling, RBAC
  ├─ cyb-runtime        agent loop, tool registry, WASM/JS sandbox
  ├─ cyb-providers      rig-based LLM layer
  ├─ cyb-memory         procedural (skills) + semantic (graph) + user model
  ├─ cyb-channels       Telegram / Discord / Web / TUI / Bevy adapters
  ├─ cyb-tools          MCP-exposed tool catalog; WIT interfaces
  ├─ cyb-skills         agentskills.io portable skills, auto-curated
  ├─ cyb-fs             SQLite-backed agent workspace (agentfs pattern)
  ├─ cyb-trust          secret substitution, capability profiles, audit
  ├─ cyb-api            MCP server + ACP + A2A endpoints
  └─ cyb-hands          autonomous always-on capabilities (cron-driven)
```

This crate split is roughly openfang's, refined by spacebot's process topology and hermes's continuity layer. None of the 20 projects has all of it. Cyb can be the one that does, because cyb already has the graph (Cortex), the desktop shell (Bevy), the web UI (React), the WASM UI (Leptos), and the scripting layer (Nu) that the others bolt on after the fact.

---

discover all [[concepts]]
