#!/usr/bin/env python3
"""
Dump HuggingFace reference logits for a model.

For each prompt in the test set, loads the HF model in f32, runs forward
for N tokens, dumps logits to JSON. Our mr runtime must reproduce these
within ε tolerance (per reference/runtime/test.md Tier 3).

Requires: pip install transformers torch

Usage:
    python3 mr/scripts/dump_hf_golden.py \\
        --model Qwen/Qwen3-0.6B \\
        --output mr/goldens/qwen3-0.6b.json

The repo name must match what HF hosts. For abliterated variants, use
the source repo (e.g. huihui-ai/Qwen3-0.6B-abliterated).
"""

import argparse
import json
import sys
from pathlib import Path


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--model", required=True, help="HF repo id, e.g. Qwen/Qwen3-0.6B")
    ap.add_argument("--output", required=True, help="output golden JSON path")
    ap.add_argument(
        "--prompts",
        nargs="+",
        default=[
            # Single-token probe: model should react to BOS-like token meaningfully.
            # We pick neutral prompts; the exact output text is not the test —
            # matching of logits within eps is the test.
            "Hello",
            "The capital of France is",
            "2 + 2 =",
        ],
        help="prompts to run",
    )
    ap.add_argument("--num-tokens", type=int, default=3, help="tokens per prompt")
    args = ap.parse_args()

    try:
        import torch
        from transformers import AutoModelForCausalLM, AutoTokenizer
    except ImportError as e:
        print(f"ERROR: {e}", file=sys.stderr)
        print("Install: pip install transformers torch", file=sys.stderr)
        sys.exit(2)

    print(f"Loading {args.model} in f32...")
    tok = AutoTokenizer.from_pretrained(args.model)
    model = AutoModelForCausalLM.from_pretrained(
        args.model,
        torch_dtype=torch.float32,
        trust_remote_code=True,
    )
    model.eval()

    records = []
    for prompt in args.prompts:
        print(f"  prompt: {prompt!r}")
        # Encode without chat template — test raw forward.
        ids = tok(prompt, return_tensors="pt").input_ids
        print(f"    input tokens: {ids.tolist()[0]}")

        with torch.no_grad():
            out = model(input_ids=ids, use_cache=True)
            # Logits after last prompt token — what generation would sample.
            last_logits = out.logits[0, -1, :].float().tolist()
            kv = out.past_key_values

            # Top-5 just for human-readable sanity check in the golden.
            probs_sorted = sorted(
                enumerate(last_logits), key=lambda kv: kv[1], reverse=True
            )[:5]

            # One generation step with greedy.
            next_id = int(torch.argmax(out.logits[0, -1, :]).item())

        records.append(
            {
                "prompt": prompt,
                "input_tokens": ids.tolist()[0],
                "vocab_size": len(last_logits),
                "last_logits": last_logits,  # full vocab, f32 list
                "top5": probs_sorted,
                "greedy_next_token": next_id,
                "greedy_next_decoded": tok.decode([next_id]),
            }
        )

    # Config fingerprint so Rust side can verify model identity.
    config = model.config.to_dict()
    relevant = {
        k: config.get(k)
        for k in [
            "model_type",
            "hidden_size",
            "num_attention_heads",
            "num_key_value_heads",
            "num_hidden_layers",
            "vocab_size",
            "rope_theta",
            "rms_norm_eps",
            "tie_word_embeddings",
            "head_dim",
            "intermediate_size",
        ]
        if k in config
    }

    payload = {
        "model_repo": args.model,
        "config": relevant,
        "records": records,
    }

    out_path = Path(args.output)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    with out_path.open("w") as f:
        json.dump(payload, f, indent=2)
    print(f"wrote {out_path}  ({len(records)} records, {len(records[0]['last_logits'])} logits each)")


if __name__ == "__main__":
    main()
