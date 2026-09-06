#!/usr/bin/env python3
"""mockchain — a deterministic stand-in chain for the fleet harness.

Serves the soft3 /status form (the same one cyb.ai/spacepussy-test speaks):
height advances every TICK seconds from process start, bbg-root is
sha256(height) — deterministic, so every body watching this chain computes
the same beacon and the harness can assert on it.

Also serves /hits: how many /status requests it has answered, which is the
harness's proof that the bodies actually talked to the chain.

Usage: mockchain.py <port> [tick_seconds]
"""

import hashlib
import sys
import time
from http.server import BaseHTTPRequestHandler, HTTPServer
from threading import Lock

PORT = int(sys.argv[1]) if len(sys.argv) > 1 else 9911
TICK = float(sys.argv[2]) if len(sys.argv) > 2 else 5.0
START = time.time()
BASE_HEIGHT = 100

hits = 0
lock = Lock()


def height() -> int:
    return BASE_HEIGHT + int((time.time() - START) / TICK)


def root(h: int) -> str:
    return hashlib.sha256(f"mockchain-{h}".encode()).hexdigest()


class Handler(BaseHTTPRequestHandler):
    def do_GET(self):
        global hits
        if self.path == "/status":
            with lock:
                hits += 1
            h = height()
            body = (
                "particle: status\n"
                "chain: mockchain\n"
                "protocol: soft3/mockchain/v2\n"
                f"height: {h}\n"
                f"bbg-root: {root(h)}\n"
                "denom: testpussy\n"
                "prefix: pussy\n"
            ).encode()
            self.send_response(200)
            self.send_header("Content-Type", "text/plain")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)
        elif self.path == "/hits":
            body = f"{hits}\n".encode()
            self.send_response(200)
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)
        else:
            self.send_response(404)
            self.end_headers()

    def log_message(self, *args):  # quiet: the harness owns the narrative
        pass


if __name__ == "__main__":
    print(f"mockchain on 127.0.0.1:{PORT} tick={TICK}s", flush=True)
    HTTPServer(("127.0.0.1", PORT), Handler).serve_forever()
