#!/bin/bash
# Test all .cyb format models — load, run, check output
set -o pipefail
LLM_DIR="$HOME/llm"
BIN="target/release/cyb-llm"
TIMEOUT=120

# Build first
echo "Building..."
cargo build --release -p cyb-llm --bin cyb-llm 2>/dev/null

echo ""
echo "=== CYB Format Smoke Test ==="
echo ""

PASS=0
FAIL=0
SKIP=0
RESULTS=""

test_decoder() {
    local name="$1"
    local cyb=$(ls "$LLM_DIR/$name/"*.cyb 2>/dev/null | head -1)
    [ -z "$cyb" ] && { echo "SKIP  $name (no .cyb file)"; SKIP=$((SKIP+1)); return; }

    local tmpf=$(mktemp)
    $BIN run "$cyb" -p "The capital of France is" -n 10 -t 0 > "$tmpf" 2>&1 || true
    local output=$(cat "$tmpf")
    rm -f "$tmpf"

    local speed=$(echo "$output" | grep -o "[0-9.]* tok/s" | tail -1)
    local err=$(echo "$output" | grep -o "Model load failed:.*\|panicked.*" | head -1)

    if [ -n "$err" ]; then
        local short_err=$(echo "$err" | cut -c1-60)
        echo "FAIL  $name — $short_err"
        FAIL=$((FAIL+1))
        RESULTS="$RESULTS\n| $name | FAIL | — | $short_err |"
    elif [ -n "$speed" ]; then
        echo "OK    $name — $speed"
        PASS=$((PASS+1))
        RESULTS="$RESULTS\n| $name | OK | $speed | |"
    else
        echo "FAIL  $name — no output"
        FAIL=$((FAIL+1))
        RESULTS="$RESULTS\n| $name | FAIL | — | no output |"
    fi
}

test_encoder() {
    local name="$1"
    local cyb=$(ls "$LLM_DIR/$name/"*.cyb 2>/dev/null | head -1)
    [ -z "$cyb" ] && { echo "SKIP  $name (no .cyb file)"; SKIP=$((SKIP+1)); return; }

    # Try embed command with weights file
    local weights=""
    [ -f "$LLM_DIR/$name/weights.gguf" ] && weights="$LLM_DIR/$name/weights.gguf"
    [ -f "$LLM_DIR/$name/weights.onnx" ] && weights="$LLM_DIR/$name/weights.onnx"
    [ -z "$weights" ] && weights="$cyb"

    local tmpf=$(mktemp)
    $BIN embed "$weights" -t "hello world" > "$tmpf" 2>&1 || true
    local output=$(cat "$tmpf")
    rm -f "$tmpf"

    local err=$(echo "$output" | grep -o "Model load failed:.*\|panicked.*\|Cannot read config.*" | head -1)

    if [ -n "$err" ]; then
        local short_err=$(echo "$err" | cut -c1-60)
        echo "FAIL  $name — $short_err"
        FAIL=$((FAIL+1))
        RESULTS="$RESULTS\n| $name | FAIL | — | $short_err |"
    elif echo "$output" | grep -q "Pooler output\|Classification\|output.*dims\|computed in"; then
        echo "OK    $name — encoder works"
        PASS=$((PASS+1))
        RESULTS="$RESULTS\n| $name | OK | encoder | |"
    else
        echo "FAIL  $name — no encoder output"
        FAIL=$((FAIL+1))
        RESULTS="$RESULTS\n| $name | FAIL | — | no encoder output |"
    fi
}

# ── Decoder models ──
echo "── Decoders ──"
test_decoder "qwen3-0.6b-abl"
test_decoder "qwen2.5-0.5b-abl"
test_decoder "qwen2.5-coder-1.5b-abl"
test_decoder "smollm2-360m"
test_decoder "bitnet-2b"
test_decoder "nuextract-1.5"
test_decoder "deepseek-r1-8b-abl"
test_decoder "qwen3.5-4b-abl"
test_decoder "qwen3.5-9b-abl"
test_decoder "qwen2.5-coder-14b-abl"
test_decoder "mimo-7b-rl"

echo ""
echo "── Encoders ──"
test_encoder "granite-hap-125m"
test_encoder "granite-hap-38m"
test_encoder "deberta-zeroshot"
test_encoder "modernbert"
test_encoder "jina-v5-nano"

echo ""
echo "── Non-text (skipped) ──"
for m in beats glotlid piper-tts wan22-video whisper-small xtts-v2 yolo11n moondream2 qwen2.5-vl-7b-abl; do
    echo "SKIP  $m"
    SKIP=$((SKIP+1))
done

echo ""
echo "=== Results: $PASS passed, $FAIL failed, $SKIP skipped ==="
echo ""
echo "| Model | Status | Speed | Error |"
echo "|-------|--------|-------|-------|"
echo -e "$RESULTS"
