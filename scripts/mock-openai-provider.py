#!/usr/bin/env python3
"""A loopback OpenAI-compatible provider stub for tests and browser evidence.

It never contacts the internet and never records a credential: the request body
is read only to decide which canned reply to return, and the default
`log_message` is silenced so no header or path is written to a log.

    python3 scripts/mock-openai-provider.py 8799

Paths select the reply, which lets one stub cover the happy path, an
untraceable value and a provider failure:

    /ok/v1/chat/completions         grounded proposal
    /ambiguous/v1/chat/completions  a budget the sentence does not contain
    /fail/v1/chat/completions       HTTP 500
"""

import json
import sys
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

GROUNDED = {
    "budgetMinor": {"value": 200000, "excerpt": "$2000"},
    "useCase": {"value": "gaming", "excerpt": "gaming"},
    "inventoryOnly": {"value": True, "excerpt": "from my stock"},
    "lockedSkus": {"value": ["FIX-R-CPU-A"], "excerpt": "FIX-R-CPU-A"},
    "excludedSkus": {"value": [], "excerpt": ""},
}

AMBIGUOUS = {
    "budgetMinor": {"value": 900000, "excerpt": "$9000"},
    "useCase": {"value": "gaming", "excerpt": "gaming"},
    "inventoryOnly": {"value": True, "excerpt": "from my stock"},
    "lockedSkus": {"value": ["FIX-R-CPU-A"], "excerpt": "FIX-R-CPU-A"},
    "excludedSkus": {"value": [], "excerpt": ""},
}


class Handler(BaseHTTPRequestHandler):
    def do_POST(self):
        length = int(self.headers.get("content-length", 0))
        self.rfile.read(length)
        if self.path.startswith("/fail/"):
            body = b'{"error":"stub failure that must not be echoed"}'
            self.send_response(500)
        else:
            proposal = AMBIGUOUS if self.path.startswith("/ambiguous/") else GROUNDED
            body = json.dumps(
                {"choices": [{"message": {"role": "assistant", "content": json.dumps(proposal)}}]}
            ).encode()
            self.send_response(200)
        self.send_header("content-type", "application/json")
        self.send_header("content-length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, *args):
        # Never log a path or a header; the stub holds no credential.
        return


if __name__ == "__main__":
    port = int(sys.argv[1]) if len(sys.argv) > 1 else 8799
    ThreadingHTTPServer(("127.0.0.1", port), Handler).serve_forever()
