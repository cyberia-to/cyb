#!/bin/bash
# fleet — the multi-environment proof that a cyb build actually works.
#
# Spins FLEET_N isolated bodies (each with its own HOME: fresh identity,
# fresh graph, fresh vault) against one deterministic mockchain, drives
# them with the scripted hands (CYB_SHOT / CYB_RUN / CYB_TOUR), and
# asserts on the artifacts every subsystem leaves behind:
#
#   boot      the body renders (self-shot exists, non-trivial size)
#   identity  each body minted its own mnemonic; all distinct
#   graph     attention casting grew a real graph.log
#   networks  every body reached the chain; netstate holds its height
#   beacon    bodies agree on the chain (same root observed)
#   prover    verified tickets accumulated in ~/cyb/proofs
#   vault     a sealed secret exists NOWHERE in plaintext
#   orphans   nothing we spawned outlives the fleet
#
# This is the per-commit gate: `make fleet`. It uses the debug binary and
# rebuilds only what changed. ~60-90s on a warm target dir.
#
# Env knobs: FLEET_N (bodies, default 3), FLEET_SECS (run time, default 30),
# FLEET_SKIP_BUILD=1 (trust the existing binary).

set -u
cd "$(dirname "$0")/.."
ROOT="$(pwd)"
T="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin"
BIN="$ROOT/target/debug/cyb"
N="${FLEET_N:-3}"
RUN_SECS="${FLEET_SECS:-30}"
PORT=$((20000 + RANDOM % 10000))
WORK="$(mktemp -d /tmp/cyb-fleet.XXXXXX)"
PASS=0; FAIL=0
PIDS=()

say()  { printf '%s\n' "$*"; }
ok()   { PASS=$((PASS+1)); say "  ok    $*"; }
bad()  { FAIL=$((FAIL+1)); say "  FAIL  $*"; }

cleanup() {
  for pid in "${PIDS[@]:-}"; do kill "$pid" 2>/dev/null; done
  sleep 1
  for pid in "${PIDS[@]:-}"; do kill -9 "$pid" 2>/dev/null; done
  # The fleet must leave no machinery running, ours or spawned-by-ours.
  pkill -f "cyb-fleet" 2>/dev/null
  [ "${FLEET_KEEP:-0}" = "1" ] || rm -rf "$WORK"
}
trap cleanup EXIT INT TERM

say "fleet: $N bodies, ${RUN_SECS}s, work dir $WORK"

# ── build ───────────────────────────────────────────────────────────────
if [ "${FLEET_SKIP_BUILD:-0}" != "1" ]; then
  say "fleet: building debug binary..."
  if ! RUSTC="$T/rustc" "$T/cargo" build -p cyb 2>"$WORK/build.log"; then
    tail -20 "$WORK/build.log"
    say "fleet: BUILD FAILED"; exit 1
  fi
fi
[ -x "$BIN" ] || { say "fleet: no binary at $BIN"; exit 1; }

# ── mockchain ───────────────────────────────────────────────────────────
python3 harness/mockchain.py "$PORT" 3 >"$WORK/mockchain.log" 2>&1 &
PIDS+=($!)
for _ in $(seq 1 20); do
  curl -s --max-time 1 "http://127.0.0.1:$PORT/status" >/dev/null 2>&1 && break
  sleep 0.5
done
curl -s "http://127.0.0.1:$PORT/status" | grep -q "height:" \
  || { say "fleet: mockchain did not come up"; exit 1; }

# ── the fault: a blackhole "chain" that accepts and never answers ───────
# Every body gets it as a second network. Mission rule under test: a dead
# chain may only lose its own row, never freeze pussy or the body.
VOID_PORT=$((PORT + 1))
python3 -c "
import socket, time
s = socket.socket(); s.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
s.bind(('127.0.0.1', $VOID_PORT)); s.listen(16)
conns = []
while True:
    c, _ = s.accept(); conns.append(c)  # hold forever, say nothing
" >"$WORK/void.log" 2>&1 &
PIDS+=($!)

# ── launch the bodies ───────────────────────────────────────────────────
for i in $(seq 1 "$N"); do
  ENV="$WORK/env-$i"
  mkdir -p "$ENV/cyb" "$ENV/llm"
  # Every body follows the same mockchain — the shared world.
  printf '[[network]]\nname = "pussy"\nurl = "http://127.0.0.1:%s"\n\n[[network]]\nname = "void"\nurl = "http://127.0.0.1:%s"\n' "$PORT" "$VOID_PORT" \
    > "$ENV/cyb/networks.toml"
  printf 'on' > "$ENV/cyb/proving"

  EXTRA_RUN=""
  if [ "$i" = "1" ]; then
    # Body 1 also seals a secret; the leak check later hunts for it.
    EXTRA_RUN="vault add fleet-check password s3cr3t-fleet-canary"
  fi

  HOME="$ENV" \
  CYB_WORLD=log \
  CYB_RUN="$EXTRA_RUN" \
  CYB_TOUR="body:3,brain:3,sigma:2,body:99" \
  CYB_SHOT="$ENV/shot.png" CYB_SHOT_AT=12 \
  RUST_LOG=warn \
  "$BIN" >"$ENV/stdout.log" 2>&1 &
  PIDS+=($!)
done

say "fleet: bodies up, letting them live ${RUN_SECS}s..."
sleep "$RUN_SECS"

# The chain's view of the fleet, captured while it still answers.
HITS=$(curl -s --max-time 2 "http://127.0.0.1:$PORT/hits" | tr -d '[:space:]')
LINKS=$(curl -s --max-time 2 "http://127.0.0.1:$PORT/links" | tr -d '[:space:]')

# ── stop the fleet (graceful first: provers flush their counters) ──────
for pid in "${PIDS[@]}"; do kill "$pid" 2>/dev/null; done
sleep 2

# ── assertions ──────────────────────────────────────────────────────────
say "fleet: asserting..."

if [ "${HITS:-0}" -ge "$N" ] 2>/dev/null; then
  ok "fleet networks: chain answered $HITS status probes (>= $N bodies)"
else
  bad "fleet networks: chain saw only '${HITS:-0}' probes for $N bodies"
fi

# relay: living bodies cast attention, the relay carries it, one signal = one
# block — the chain must have RECEIVED links from the fleet.
if [ "${LINKS:-0}" -ge "$N" ] 2>/dev/null; then
  ok "fleet relay: chain received $LINKS links from living bodies"
else
  bad "fleet relay: chain received only '${LINKS:-0}' links for $N bodies"
fi

ROOTS="$WORK/roots.txt"; : > "$ROOTS"
MNEMOS="$WORK/mnemos.txt"; : > "$MNEMOS"

for i in $(seq 1 "$N"); do
  ENV="$WORK/env-$i"; C="$ENV/cyb"

  # boot: the self-shot exists and is a real image
  if [ -s "$ENV/shot.png" ] && [ "$(stat -f%z "$ENV/shot.png")" -gt 20000 ]; then
    ok "env-$i boot: self-shot rendered"
  else
    bad "env-$i boot: no usable self-shot"
  fi

  # identity: a mnemonic was minted
  if [ -s "$C/mnemonic" ]; then
    ok "env-$i identity: mnemonic minted"
    # mnemonic files carry no trailing newline; add one or they merge
    cat "$C/mnemonic" >> "$MNEMOS"; echo >> "$MNEMOS"
  else
    bad "env-$i identity: no mnemonic"
  fi

  # graph: attention casting produced a chain
  if [ -s "$C/graph.log" ]; then
    ok "env-$i graph: graph.log grew ($(stat -f%z "$C/graph.log") bytes)"
  else
    bad "env-$i graph: empty or missing graph.log"
  fi

  # networks: the body synced the chain and remembered it — WITH a
  # blackhole sibling wired in. Isolation is the assertion.
  if [ -s "$C/netstate" ] && grep -q "^pussy [0-9]" "$C/netstate"; then
    ok "env-$i networks: pussy synced despite the blackhole (h=$(grep '^pussy' "$C/netstate" | awk '{print $2}'))"
    grep '^pussy' "$C/netstate" | awk '{print $3}' | head -1 >> "$ROOTS"
  else
    bad "env-$i networks: no synced netstate"
  fi
  if grep -q "^void" "$C/netstate" 2>/dev/null; then
    bad "env-$i networks: the blackhole produced fake state"
  fi

  # prover: verified tickets accumulated
  P=$(cat "$C/proofs" 2>/dev/null || echo 0)
  if [ "${P:-0}" -gt 0 ] 2>/dev/null; then
    ok "env-$i prover: $P verified tickets"
  else
    bad "env-$i prover: no tickets counted"
  fi
done

# identities are distinct beings
if [ "$(sort -u "$MNEMOS" | wc -l | tr -d ' ')" = "$N" ]; then
  ok "fleet identity: $N distinct mnemonics"
else
  bad "fleet identity: mnemonics collide"
fi

# beacon truth: every recorded (height, root) must be the chain's own
# deterministic state — root == sha256("mockchain-<height>"). This is the
# multi-environment point: bodies provably read the same last block.
BEACON_BAD=0
for i in $(seq 1 "$N"); do
  ST="$WORK/env-$i/cyb/netstate"
  [ -s "$ST" ] || continue
  while read -r name h root; do
    WANT=$(python3 -c "import hashlib;print(hashlib.sha256(f'mockchain-$h'.encode()).hexdigest())")
    [ "$root" = "$WANT" ] || { BEACON_BAD=$((BEACON_BAD+1)); say "        env-$i: h=$h root mismatch"; }
  done < "$ST"
done
if [ "$BEACON_BAD" = "0" ] && [ -s "$ROOTS" ]; then
  ok "fleet beacon: every recorded root IS sha256(height) - true chain state"
else
  bad "fleet beacon: $BEACON_BAD bad roots"
fi

# vault: the canary secret exists nowhere in plaintext
LEAK=$(grep -rl "s3cr3t-fleet-canary" "$WORK"/env-1/cyb/graph.log "$WORK"/env-1/cyb/particles.jsonl "$WORK"/env-1/stdout.log 2>/dev/null | wc -l | tr -d ' ')
if [ -s "$WORK/env-1/cyb/vault.enc" ] && [ "$LEAK" = "0" ]; then
  ok "fleet vault: sealed, zero plaintext leaks"
else
  bad "fleet vault: enc=$([ -s "$WORK/env-1/cyb/vault.enc" ] && echo yes || echo no) leaks=$LEAK"
fi

# orphans: every pid we spawned must be dead
sleep 1
ORPHANS=0
for pid in "${PIDS[@]}"; do
  if kill -0 "$pid" 2>/dev/null; then ORPHANS=$((ORPHANS+1)); kill -9 "$pid" 2>/dev/null; fi
done
if [ "$ORPHANS" = "0" ]; then
  ok "fleet orphans: none"
else
  bad "fleet orphans: $ORPHANS spawned processes needed -9"
fi

say ""
say "fleet: $PASS ok, $FAIL failed"
[ "$FAIL" = "0" ] && { say "fleet: GREEN"; exit 0; } || { say "fleet: RED (artifacts kept in $WORK)"; FLEET_KEEP=1; exit 1; }
