#!/usr/bin/env python3
"""mockchain — a deterministic stand-in chain for the fleet harness.

Serves the chain-wire §7 surface a cyb body uses (soft3/specs/chain-wire.md):

  GET  /status        cybermark; height advances every TICK seconds from
                      process start, bbg-root is sha256(height) — every body
                      watching computes the same beacon; `genesis` is the
                      chain id every signed tx must bind to
  GET  /nonce/<hex>   the next nonce of a neuron (JSON)
  GET  /balance/<hex> a balance (cybermark)
  POST /tx            a rule-set-1 transaction: decoded by kind and length,
                      exact nonce enforced per neuron, counted; the NSIG1
                      signature is NOT verified here (no secp256k1 in the
                      harness — true-cyber's mock node does that in Rust)
  GET  /tx/<hex>      included at the current height once anything was
                      submitted, else 404
  POST /v1/link,/v1/pay   410 Gone — the unsigned ingress has no place on a
                      chain; a body still using it is a harness failure
  GET  /hits, /links  how many /status probes and kind-2 txs it saw —
                      the harness's proof that the bodies talked to the chain

Usage: mockchain.py <port> [tick_seconds]
"""

import hashlib
import json
import sys
import time
from http.server import BaseHTTPRequestHandler, HTTPServer
from threading import Lock

PORT = int(sys.argv[1]) if len(sys.argv) > 1 else 9911
TICK = float(sys.argv[2]) if len(sys.argv) > 2 else 5.0
START = time.time()
BASE_HEIGHT = 100
CHAIN_ID = hashlib.sha256(b"mockchain-genesis").hexdigest()

# kind -> (name, body bytes); every signed kind carries 102 sig bytes
KINDS = {1: ("transfer", 120), 2: ("cyberlink", 153), 3: ("claim", 36)}

hits = 0
links = 0
txs = 0
rejected = 0
nonces = {}
lock = Lock()


def height() -> int:
    # Time ticks AND received txs both advance the chain — the soft3
    # canon (one signal, one block) on top of a heartbeat.
    return BASE_HEIGHT + int((time.time() - START) / TICK) + txs


def root(h: int) -> str:
    return hashlib.sha256(f"mockchain-{h}".encode()).hexdigest()


def tip(h: int) -> str:
    return hashlib.sha256(f"mockchain-tip-{h}".encode()).hexdigest()


def decode(raw: bytes):
    """(from_hex, nonce, kind_name) or an error string."""
    if len(raw) < 2 or raw[0] != 1:
        return "tx version must be 1"
    kind = raw[1]
    if kind not in KINDS:
        return f"unknown tx kind {kind}"
    name, body = KINDS[kind]
    if len(raw) != 2 + body + 102:
        return f"{name} tx is {2 + body + 102} bytes, got {len(raw)}"
    if raw[2 + body : 2 + body + 5] != b"NSIG1":
        return "signature envelope is not NSIG1"
    if kind == 3:
        signer = raw[2 + body + 5 : 2 + body + 5 + 33]  # pubkey; neuron = hemera(pubkey), unknown here
        frm = "claim:" + signer.hex()
        nonce = int.from_bytes(raw[22:30], "little")
    else:
        frm = raw[2:34].hex()
        nonce = int.from_bytes(raw[34:42], "little")
    return (frm, nonce, name)


class Handler(BaseHTTPRequestHandler):
    def reply(self, code: int, body: bytes, ctype: str = "text/plain"):
        self.send_response(code)
        self.send_header("Content-Type", ctype)
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def do_GET(self):
        global hits
        path = self.path.split("?")[0].rstrip("/")
        if path == "/status":
            with lock:
                hits += 1
            h = height()
            body = (
                "---\n"
                "particle: status\n"
                "chain: mockchain\n"
                "protocol: soft3/chain/1\n"
                "rule-set: 1\n"
                f"height: {h}\n"
                f"tip: {tip(h)}\n"
                f"genesis: {CHAIN_ID}\n"
                f"bbg-root: {root(h)}\n"
                "denom: testpussy\n"
                "prefix: pussy\n"
                "catching-up: false\n"
                "---\n"
            ).encode()
            self.reply(200, body)
        elif path.startswith("/nonce/"):
            with lock:
                n = nonces.get(path[7:], 0)
            self.reply(200, json.dumps({"nonce": n}).encode(), "application/json")
        elif path.startswith("/balance/"):
            h = height()
            self.reply(200, f"---\nparticle: balance\nbalance: 1000\nheight: {h}\nsupply: 1000000\n---\n".encode())
        elif path.startswith("/tx/"):
            if txs > 0:
                h = height()
                self.reply(200, json.dumps({"height": h, "block": tip(h)}).encode(), "application/json")
            else:
                self.reply(404, b'{"error":"unknown txid"}', "application/json")
        elif path == "/hits":
            self.reply(200, f"{hits}\n".encode())
        elif path == "/links":
            self.reply(200, f"{links}\n".encode())
        elif path == "/rejected":
            self.reply(200, f"{rejected}\n".encode())
        else:
            self.reply(404, b"")

    def do_POST(self):
        global links, txs, rejected
        length = int(self.headers.get("Content-Length", 0))
        raw = self.rfile.read(length)
        path = self.path.split("?")[0].rstrip("/")
        if path == "/tx":
            decoded = decode(raw)
            if isinstance(decoded, str):
                with lock:
                    rejected += 1
                self.reply(400, json.dumps({"error": decoded}).encode(), "application/json")
                return
            frm, nonce, name = decoded
            with lock:
                expected = nonces.get(frm, 0)
                if nonce != expected:
                    rejected += 1
                    self.reply(400, json.dumps({"error": f"bad nonce {nonce}: expected {expected}"}).encode(), "application/json")
                    return
                nonces[frm] = expected + 1
                txs += 1
                if name == "cyberlink":
                    links += 1
            # No txid: the harness has no hemera. The client keeps its own.
            self.reply(200, json.dumps({"ok": True, "kind": name}).encode(), "application/json")
        elif path in ("/v1/link", "/v1/pay", "/v1/frame", "/v2/frame"):
            with lock:
                rejected += 1
            self.reply(410, b'{"error":"unsigned ingress is gone; POST /tx"}', "application/json")
        else:
            self.reply(404, b"")

    def log_message(self, *args):  # quiet: the harness owns the narrative
        pass


if __name__ == "__main__":
    print(f"mockchain on 127.0.0.1:{PORT} tick={TICK}s chain={CHAIN_ID[:8]}", flush=True)
    HTTPServer(("127.0.0.1", PORT), Handler).serve_forever()
