# AGENTS.md — rules for every coding agent in this repository

This file is read by Codex, Kimi Code, Grok Build, Cursor and any tool that
honours AGENTS.md. Claude reads CLAUDE.md in the same directory; both say the
same thing, and the doctrine behind both is `~/cyber/cyberia/dev.md`
(published as the [[cyberia/dev]] page). Read that page first when in doubt.

## release train (cyb, cyber, soft3)

- Truth is origin. A release candidate is built only from committed, pushed
  inputs on the default branch of every repository it closes over. A working
  tree is never an input; a dirty or unpushed input makes a candidate local,
  and a local candidate can never become a release.
- One candidate every friday 12:00 UTC, cut by an agent from origin:
  `candidate-YYYYMMDD.N`, a draft pre-release on GitHub with binaries per
  platform (macOS arm64/x64, Linux arm64/x64; cyb adds the Android apk; no
  Windows, no iOS), `SHA256SUMS`, `sources.json`, `candidate.json`,
  `release-validation.json`.
- Gates are executable and named in each repository's CLAUDE.md / AGENTS.md.
  A red gate ships as a red candidate with its evidence; never hide it, never
  fix the artifact by hand.
- Versions are semver. One bump is one pull request `chore: <component>
  <version>` touching `Cargo.toml`, `CHANGELOG.md` and the sibling pins in the
  other two repositories, nothing else. Tags are `v<version>` on the default
  branch. `soft3/release/phase1.toml` pins the sibling revisions; drift from a
  pin is a red gate and is fixed by a bump PR, never by editing a path.
- Freeze from the cut until the verdict: nothing merges into the three default
  branches; candidate fixes go to `release/<date>` as `fix:` commits.
- Roles: agents cut candidates, run gates, write receipts to
  `audit/release-<date>/` and open bump PRs. Only the owner merges a bump,
  promotes a candidate to a release, publishes to crates.io, pushes a tag or
  runs `make ship`. An agent never does any of those.

## git

- Commit after every logical unit of work, one change per commit, conventional
  prefixes (`feat:`, `fix:`, `refactor:`, `docs:`, `test:`, `chore:`), message
  says why.
- Stage by explicit paths. Never `git add -A`. Never amend, rebase, reset,
  stash, checkout files or force-push on shared branches; never rewrite
  history. No AI co-authorship trailers.
- Zero warnings and green tests before a commit; if red, prefix `wip:` and
  quote the first error in the message.
- Other people's working trees under `~/cyber/<repo>` may be dirty with
  their own work: never edit, stage or discard it; work in a worktree.

## moving a page out of cyber

Nothing is thrown away by default. Settled and implemented → the owner's spec
or an existing explainer, with measured numbers. A projection with a named
assumption and an open question → the owner's `roadmap/`, one file per
question, with the registry's status words. A cross-component argument →
`soft3/docs/` as an explainer with each projection marked measured, refuted
or open. Delete only duplicates and refuted-and-rewritten claims. Remove the
ledger row of a deleted page; keep old wiki-links resolving via `alias:`.

## reports

Audits, benchmarks and release validation go to `<repo>/audit/`; contracts
to `specs/`; observed results never in a spec. Every number in a report
names the command and the revision it came from.

## vocabulary

Root terms of the cybergraph: file (the thing), particle (its 32-byte hemera
identity), cyberlink, neuron, token, focus. Never CID, user, edge. Cyber
graph pages: no bold, no definitions by negation, wiki-links for concepts.

## platform

macOS dev machine: BSD `sed` has no `\b` (use `[[:<:]]w[[:>:]]`); zsh does
not word-split unquoted `$var`; `~/.cargo/bin` and `/opt/homebrew/bin` are
on PATH; sibling repositories resolve by relative path from `~/cyber`.
