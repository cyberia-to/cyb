# hermes-learning-loop.md

How Hermes (Nous Research) actually learns. Technicalities, not slogans.

The loop has four cycles operating at different time scales:

```
turn-level    →  background review       (after every turn)
session-level →  trajectory recording    (every completion)
week-level    →  curator consolidation   (every 7 days)
forever       →  agentskills.io export   (portable knowledge)
```

The agent improves on three substrates: memory (facts about the user), skills (procedural knowledge), trajectories (training data for next-gen models).

---

## Cycle 1: background review — after every turn

File: `agent/background_review.py`. Fires after every `AIAgent.run_conversation` turn.

### Mechanics

1. Parent spawns a daemon thread via `spawn_background_review_thread(agent, messages_snapshot, review_memory, review_skills)`.
2. Thread runs a forked `AIAgent` inheriting the parent's runtime: same provider, model, credentials, cached system prompt. Cheap fork — shares the loaded state.
3. The fork has a tool whitelist limited to memory and skill management tools. Everything else is denied at runtime, so the review pass cannot exec, write files arbitrarily, or call the network.
4. Receives the conversation snapshot plus a review prompt.
5. Runs with `max_iterations=16`.
6. Parent scans returned messages for successful tool actions and surfaces a one-line summary in the UI (`💾 Self-improvement review: Memory updated`).
7. Memory providers shut down, fork closes, stale messages are filtered to avoid re-acting on actions already taken in earlier turns of the same conversation.

### The three review prompts

`_MEMORY_REVIEW_PROMPT` — asks one question:
> Has the user revealed things about themselves — their persona, desires, preferences, or personal details worth remembering?

If nothing, response is literally `Nothing to save.` — the fork is allowed to no-op, which is the cheap path.

`_SKILL_REVIEW_PROMPT` — biased aggressively toward action:
> Be ACTIVE — most sessions produce at least one skill update, even if small. A pass that does nothing is a missed learning opportunity.

Watches for specific signals:
- User corrected style / tone / format / approach.
- Frustration signals like "stop doing X" — explicitly called out as FIRST-CLASS skill signals.
- Non-trivial techniques or debugging paths surfaced.
- Loaded skills turned out wrong or outdated.

Preference order for skill writes:
1. Patch currently-loaded skills (smallest change).
2. Update existing umbrella skills.
3. Add support files (`references/`, `templates/`, `scripts/`).
4. Create a new class-level umbrella skill (most disruptive — last resort).

`_COMBINED_REVIEW_PROMPT` — fires when both memory and skill triggers hit.

### What gets written

- Memory entries via memory tool calls, with provenance metadata: `write_origin="background_review"`, session ID, platform info.
- Skill patches via `skill_manage` tool: `action=patch | create | write_file | delete`.
- Protection layer: bundled or hub-installed skills are read-only. Pinned skills can be improved but not deleted by the curator (the long cycle).

Cost: one forked LLM call per turn, restricted scope, short iteration cap. Hidden from the user unless something is written.

---

## Cycle 2: trajectory recording — every completion

File: `agent/trajectory.py`. Captures every conversation as training data.

### Format

ShareGPT JSONL — the de-facto standard for tool-using conversation corpora. Each entry:

```json
{
  "conversations": [/* exchanges */],
  "timestamp": "2026-05-24T...",
  "model": "claude-opus-4-7",
  "completed": true
}
```

Two streams:
- `trajectory_samples.jsonl` — successful completions.
- `failed_trajectories.jsonl` — incomplete attempts.

The split matters: the failed corpus is as valuable as the successful one for RL-style training (negative examples + recovery patterns).

### Reasoning preservation

`<REASONING_SCRATCHPAD>` tags are normalized to `<think>` tags, preserving the model's internal reasoning alongside the tool calls and dialogue. This means trajectories carry not just *what* the agent did, but *why* — directly trainable.

### Downstream

`_convert_to_trajectory_format` is called by `batch_runner.py`. This is the bridge from Hermes-as-product to Hermes-as-training-data-pipeline. Nous trains its next model on its own agent's execution. Closed loop at the model layer.

---

## Cycle 3: curator — every 7 days

File: `agent/curator.py`. The long, slow consolidation pass.

### Gating

```python
def should_run_now(now: Optional[datetime] = None) -> bool
```

Four conditions, all required:
1. `curator.enabled == True` (default).
2. Not paused.
3. `last_run_at` is older than `interval_hours` (default: 168 = 7 days).
4. First-run defers by one full interval.

Plus a caller-side idle check: the curator spawns only when the agent has been inactive for `min_idle_hours` (default: 2 hours). Compute and attention budget protection.

### Phase A: automatic transitions (no LLM)

```python
def apply_automatic_transitions(now) -> Dict[str, int]
```

Walks every agent-created skill:
- Inactive ≥ `stale_after_days` (default 30) → mark `STATE_STALE`.
- Inactive ≥ `archive_after_days` (default 90) → move to `.archive/`.
- Stale skill with new activity → reactivate.
- Pinned skills → skip entirely.

This is pure bookkeeping. No LLM cost.

### Phase B: forked review LLM

Spawns an auxiliary LLM with the `CURATOR_REVIEW_PROMPT`. The prompt mandates "umbrella-building consolidation" — merge narrow sibling skills into broader class-level umbrella skills with labeled subsections, support files (`references/`, `templates/`, `scripts/`).

Tool actions used:
- `action=patch` — add sections to an existing umbrella.
- `action=create` — create a new umbrella skill.
- `action=write_file` — add support files under an existing skill.
- `action=delete` — archive a skill, taking `absorbed_into=<target>` for forward links.

### Classification: consolidation vs pruning

When a skill is deleted, the curator needs to know: was it absorbed into a bigger skill (consolidation) or dropped (pruning)? Three signals in priority order:

1. Model-declared `absorbed_into` parameter at delete time (authoritative).
2. Substring matching in subsequent tool calls (heuristic — was the absorbed name referenced in a later patch?).
3. Structured YAML block parsed from the model's final response.

Contradiction between these falls back to heuristic evidence or explicit pruning. The forward-pointer (`absorbed_into`) is what lets old skill references heal into new ones.

### Output and state

State file `.curator_state`:

```python
{
  "last_run_at": None,
  "last_run_duration_seconds": None,
  "last_run_summary": None,
  "last_run_summary_shown_at": None,
  "last_report_path": None,
  "paused": False,
  "run_count": 0,
}
```

Reports under `~/.hermes/logs/curator/{YYYYMMDD-HHMMSS}/`:
- `run.json` — structured metadata (model calls, counts, classification breakdown).
- `REPORT.md` — human-readable summary with consolidated / pruned skill lists.

Plus a "rename summary" (up to 10 entries) appended to the agent's user-facing log, telling the user where skills were archived and suggesting `hermes skill pin <name>` commands for ones the user might want to keep.

---

## Cycle 4: agentskills.io — the portable substrate

Skills are not Hermes-internal. They are folders in the open `agentskills.io` format (originated by Anthropic, adopted by ~40+ agents per the showcase: Claude Code, Cursor, Goose, OpenCode, OpenHands, GitHub Copilot, Codex, Letta, etc.).

### Format

```
my-skill/
├── SKILL.md          # required: YAML frontmatter + markdown body
├── scripts/          # optional: executable code
├── references/       # optional: extra docs, loaded on demand
├── assets/           # optional: templates, images, schemas
```

`SKILL.md` frontmatter:

| field | required | constraint |
|---|---|---|
| `name` | yes | ≤64 chars, `[a-z0-9-]`, no leading/trailing/consecutive hyphens, must match parent dir |
| `description` | yes | ≤1024 chars, what + when to use |
| `license` | no | string or filename |
| `compatibility` | no | ≤500 chars, env requirements |
| `metadata` | no | arbitrary string→string map (author, version, etc.) |
| `allowed-tools` | no | space-separated pre-approved tools (experimental) |

### Progressive disclosure — the key idea

Skills load in three stages by token cost:

1. Discovery (~100 tokens): only `name` + `description`, loaded for all skills at startup.
2. Activation (<5000 tokens recommended): full `SKILL.md` body, loaded when the agent decides to use the skill.
3. Resources (as needed): files from `scripts/`, `references/`, `assets/` loaded on demand.

This is what makes "hundreds of skills" affordable. The agent sees a flat namespace of one-liner descriptions; full instructions only enter context when relevant.

### Validation

```bash
skills-ref validate ./my-skill
```

Reference library at `github.com/agentskills/agentskills`.

---

## Hermes' skill taxonomy

The repo ships ~25 skill bundles. Notable categories from `/skills/`:

```
autonomous-ai-agents   creative      data-science    devops
diagramming            email         github          inference-sh
mcp                    media         mlops           note-taking
productivity           red-teaming/godmode           research
smart-home             social-media  software-development
yuanbao  (Tencent)     apple         gaming          gifs
```

Two interesting ones:
- `red-teaming/godmode` — adversarial skill bundle, kept inside the agent for self-evaluation.
- `dogfood` — internal testing / self-hosting skills, meaning Hermes uses itself for its own QA.

User-created skills land in `~/.hermes/skills/<category>/<skill-name>/`, alongside protected hub-installed ones.

---

## Memory architecture

Three tiers:

1. Procedural — skills (covered above).
2. Semantic — FTS5 full-text search across saved session content, plus LLM-generated summarization for cross-session recall. The session itself is the corpus; no separate vector store needed.
3. User model — Honcho dialectic system. External service that builds a persistent psychological/preference model of the user across all conversations.

The `memory_manager.py` + `memory_provider.py` split: `memory_provider` is the read path used by the agent loop, `memory_manager` is the write path used by background review and curator. Read-write separation lets the agent stay fast while writes happen async.

`context_engine.py` + `context_references.py` build the prompt's context window from these three tiers, with `conversation_compression.py` summarizing old turns when token budget tightens.

`error_classifier.py` categorizes failures into reusable taxonomy — feeds back into skill creation triggers (a recurring error class becomes a candidate skill).

---

## What makes this loop different

Most "agent memory" systems are passive RAG: vectors of conversation, retrieved by similarity. Hermes is active on three counts:

1. The background review is a separate LLM call after every turn. Most frameworks would summarize-on-demand at the next session start. Hermes pays for the review upfront because the conversation is still hot in context.

2. The curator is its own LLM call on a 7-day cadence, with the only job of refactoring the skill library. Most frameworks accumulate skills monotonically and degrade as the library grows. Hermes garbage-collects.

3. The trajectory pipeline is not user-facing. It exists to feed model training. The agent is both product and data generator. This is the closed loop at the substrate level — Hermes the framework runs Hermes the model, which gets better because Hermes the framework records what worked.

---

## What cyb takes from this

Direct lifts:

1. The four-cycle topology (turn / session / week / forever) is the right shape. Cyb should implement at least the turn-cycle review and the week-cycle consolidation from day one.

2. agentskills.io as the on-disk format. No reason to invent another. Cyb's existing `cyber/cyber/root/cyberia/midao/agents.md` aligns naturally — every neuron is a skill folder.

3. The forked-agent-with-restricted-tools pattern for reviews. Same model, same context, but a tool whitelist that prevents the review pass from doing harm.

4. The frustration-signal-as-skill-trigger heuristic. Cyb users will swear at the robot; that swearing is training data. "Stop doing X" is the cheapest, highest-signal skill creation prompt.

5. Forward-pointer archival (`absorbed_into=<new-name>`). Old skill references survive consolidation by pointing at their new home. This is exactly how cyb's cybergraph treats particle merges — content-addressed forwards.

6. Two-stream trajectory recording (success / failure JSONL). Even before cyb trains its own model, this corpus is invaluable for debugging and for ranking which neurons earn karma.

Patterns to study but defer:

7. Honcho-style external user model. Cyb already has the cybergraph for this — the user's own particle history *is* the user model. Don't add another store.

8. FTS5 session search. SQLite-side feature, fits cleanly when cyb adopts the agentfs-style SQLite-as-FS layer.

---

discover all [[concepts]]
