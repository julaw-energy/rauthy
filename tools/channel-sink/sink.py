#!/usr/bin/env python3
"""
Channel sink: a stand-in notification agent for testing Rauthy.

It receives what Rauthy would send to a real SMS / WhatsApp / email provider, checks it, stores the
last messages in memory and shows them on a web page. NOTHING is delivered to anyone.

  * POST /hook         Rauthy's generic `webhook` driver (HMAC checked if SIGNING_SECRET is set)
  * POST /v1/send      the `notifyd` driver (X-Api-Key checked if API_KEY is set)
  * GET  /v1/health    what the notifyd "Test the agent" button calls
  * SMTP (optional)    plain, unencrypted SMTP so Rauthy's built-in mailer can be tested too
  * GET  /             the received messages (protect it with VIEW_TOKEN, codes are visible there!)

Python 3.8+, standard library only.

Environment variables (all optional):
  PORT=3400             HTTP port            BIND=0.0.0.0
  SIGNING_SECRET=...    the `api_key` of a webhook channel in Rauthy
  API_KEY=...           the `api_key` of a notifyd channel in Rauthy
  VIEW_TOKEN=...        needed to read messages: open /?token=... or send `Authorization: Bearer ...`
  SMTP_PORT=2525        also start a plain SMTP sink on this port (0 = off, default)
  MAX_MESSAGES=200
"""
import asyncio
import email
import hashlib
import hmac
import html
import json
import os
import threading
import time
import uuid
from collections import deque
from email import policy
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from urllib.parse import parse_qs, urlparse

PORT = int(os.environ.get("PORT", "3400"))
BIND = os.environ.get("BIND", "0.0.0.0")
SIGNING_SECRET = os.environ.get("SIGNING_SECRET", "")
API_KEY = os.environ.get("API_KEY", "")
VIEW_TOKEN = os.environ.get("VIEW_TOKEN", "")
SMTP_PORT = int(os.environ.get("SMTP_PORT", "0"))
MAX_MESSAGES = int(os.environ.get("MAX_MESSAGES", "200"))

MESSAGES = deque(maxlen=MAX_MESSAGES)
LOCK = threading.Lock()


def record(source, **fields):
    entry = {"id": str(uuid.uuid4()), "time": time.strftime("%Y-%m-%d %H:%M:%S"), "source": source, **fields}
    with LOCK:
        MESSAGES.appendleft(entry)
    print(f"[{entry['time']}] {source}: {fields.get('to', '')} {str(fields.get('text', ''))[:80]!r}", flush=True)
    return entry


class Handler(BaseHTTPRequestHandler):
    server_version = "channel-sink"

    def log_message(self, *args):  # quiet, we print our own lines
        pass

    # ---- helpers ----
    def send_json(self, status, obj):
        body = json.dumps(obj).encode()
        self.send_response(status)
        self.send_header("content-type", "application/json")
        self.send_header("content-length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def read_body(self):
        return self.rfile.read(int(self.headers.get("content-length") or 0))

    def can_view(self):
        if not VIEW_TOKEN:
            return True
        token = parse_qs(urlparse(self.path).query).get("token", [""])[0]
        bearer = (self.headers.get("authorization") or "").removeprefix("Bearer ").strip()
        return hmac.compare_digest(token, VIEW_TOKEN) or hmac.compare_digest(bearer, VIEW_TOKEN)

    # ---- routes ----
    def do_GET(self):
        path = urlparse(self.path).path
        if path == "/v1/health":
            return self.send_json(200, {"status": "ok", "service": "channel-sink"})
        if path == "/api/messages":
            if not self.can_view():
                return self.send_json(401, {"error": "token required"})
            with LOCK:
                return self.send_json(200, list(MESSAGES))
        if path == "/":
            if not self.can_view():
                self.send_response(401)
                self.send_header("content-type", "text/plain")
                self.end_headers()
                return self.wfile.write(b"401 - open /?token=<VIEW_TOKEN>")
            page = render_page().encode()
            self.send_response(200)
            self.send_header("content-type", "text/html; charset=utf-8")
            self.send_header("content-length", str(len(page)))
            self.end_headers()
            return self.wfile.write(page)
        self.send_json(404, {"error": "not found"})

    def do_DELETE(self):
        if urlparse(self.path).path == "/api/messages" and self.can_view():
            with LOCK:
                MESSAGES.clear()
            return self.send_json(200, {"cleared": True})
        self.send_json(404, {"error": "not found"})

    def do_POST(self):
        path = urlparse(self.path).path
        raw = self.read_body()

        if path == "/hook":  # Rauthy's generic webhook driver
            if SIGNING_SECRET:
                expected = "sha256=" + hmac.new(SIGNING_SECRET.encode(), raw, hashlib.sha256).hexdigest()
                got = self.headers.get("x-rauthy-signature", "")
                if not hmac.compare_digest(expected, got):
                    record("webhook", error="bad or missing X-Rauthy-Signature", to="", text="REJECTED")
                    return self.send_json(401, {"error": "bad signature"})
            try:
                data = json.loads(raw)
            except ValueError:
                return self.send_json(400, {"error": "invalid json"})
            if data.get("type") == "ping":
                record("webhook", to="", text="ping from Rauthy (Test the agent)")
                return self.send_json(200, {"pong": True})
            rec = data.get("recipient", {})
            e = record(
                "webhook", channel=data.get("channel"), purpose=data.get("purpose"), to=rec.get("to", ""),
                name=rec.get("name", ""), subject=data.get("subject"), text=data.get("text", ""),
                signature="verified" if SIGNING_SECRET else "not checked",
            )
            return self.send_json(200, {"job_id": e["id"]})

        if path == "/v1/send":  # notifyd
            if API_KEY and not hmac.compare_digest(self.headers.get("x-api-key", ""), API_KEY):
                record("notifyd", error="bad or missing X-Api-Key", to="", text="REJECTED")
                return self.send_json(401, {"error": "bad api key"})
            try:
                data = json.loads(raw)
            except ValueError:
                return self.send_json(400, {"error": "invalid json"})
            e = record(
                "notifyd", channel=data.get("channel") or data.get("channels"), to=data.get("to", ""),
                subject=data.get("subject"), text=data.get("body", ""), priority=data.get("priority"),
                transactional=data.get("transactional"), idempotency_key=data.get("idempotency_key"),
            )
            return self.send_json(200, {"success": True, "job_ids": [e["id"]], "skipped": []})

        self.send_json(404, {"error": "not found"})


def render_page():
    with LOCK:
        items = list(MESSAGES)
    rows = []
    for m in items:
        meta = " · ".join(f"{html.escape(str(k))}: <b>{html.escape(str(v))}</b>"
                          for k, v in m.items() if k not in ("id", "time", "text", "source", "to") and v not in (None, ""))
        rows.append(
            f"<article><header><span class='tag'>{html.escape(m['source'])}</span> "
            f"<b>{html.escape(str(m.get('to', '')))}</b><time>{m['time']}</time></header>"
            f"<pre>{html.escape(str(m.get('text', '')))}</pre><small>{meta}</small></article>"
        )
    empty = "<p class='empty'>Nothing received yet.</p>" if not rows else ""
    return f"""<!doctype html><meta charset=utf-8><meta name=viewport content="width=device-width,initial-scale=1">
<meta http-equiv=refresh content=5><title>Channel sink</title>
<style>
body{{font:15px system-ui,sans-serif;margin:0;background:#0f1417;color:#e6e9ea}}
main{{max-width:760px;margin:0 auto;padding:1rem}} h1{{font-size:1.2rem}}
article{{background:#182025;border:1px solid #2a3439;border-radius:12px;padding:.8rem 1rem;margin:.7rem 0}}
header{{display:flex;gap:.6rem;align-items:center}} time{{margin-left:auto;color:#8a979c;font-size:.8rem}}
.tag{{background:#2a3439;border-radius:99px;padding:.1rem .6rem;font-size:.75rem}}
pre{{white-space:pre-wrap;margin:.6rem 0;font:inherit}} small{{color:#8a979c}} .empty{{color:#8a979c}}
</style><main><h1>Channel sink — {len(items)} message(s) <small>(auto-refresh, nothing is delivered)</small></h1>
{empty}{''.join(rows)}</main>"""


# ---- optional plain SMTP sink ----
async def smtp_session(reader, writer):
    async def say(line):
        writer.write((line + "\r\n").encode())
        await writer.drain()

    await say("220 channel-sink ESMTP")
    mail_from, rcpts = "", []
    try:
        while True:
            line = (await reader.readline()).decode(errors="replace").rstrip("\r\n")
            if not line:
                break
            cmd = line.upper()
            if cmd.startswith(("EHLO", "HELO")):
                await say("250-channel-sink")
                await say("250 8BITMIME")
            elif cmd.startswith("MAIL FROM"):
                mail_from, rcpts = line[10:].strip(), []
                await say("250 OK")
            elif cmd.startswith("RCPT TO"):
                rcpts.append(line[8:].strip())
                await say("250 OK")
            elif cmd == "DATA":
                await say("354 End data with <CR><LF>.<CR><LF>")
                chunks = []
                while True:
                    l = await reader.readline()
                    if l in (b".\r\n", b""):
                        break
                    chunks.append(l[1:] if l.startswith(b"..") else l)
                msg = email.message_from_bytes(b"".join(chunks), policy=policy.default)
                body = msg.get_body(preferencelist=("plain", "html"))
                record("smtp", to=", ".join(rcpts), subject=msg.get("subject"), sender=mail_from,
                       text=body.get_content() if body else "")
                await say("250 OK")
            elif cmd == "RSET":
                mail_from, rcpts = "", []
                await say("250 OK")
            elif cmd == "NOOP":
                await say("250 OK")
            elif cmd == "QUIT":
                await say("221 Bye")
                break
            else:
                await say("502 Command not implemented")
    finally:
        writer.close()


def run_smtp():
    async def main():
        server = await asyncio.start_server(smtp_session, BIND, SMTP_PORT)
        print(f"SMTP sink listening on {BIND}:{SMTP_PORT} (plain, no auth)", flush=True)
        async with server:
            await server.serve_forever()

    asyncio.run(main())


if __name__ == "__main__":
    if SMTP_PORT:
        threading.Thread(target=run_smtp, daemon=True).start()
    print(f"Channel sink on http://{BIND}:{PORT}  "
          f"(webhook: /hook, notifyd: /v1/send, view: /{'?token=...' if VIEW_TOKEN else ''})", flush=True)
    if not VIEW_TOKEN:
        print("WARNING: VIEW_TOKEN is not set - anyone who can reach this port can read the messages", flush=True)
    ThreadingHTTPServer((BIND, PORT), Handler).serve_forever()
