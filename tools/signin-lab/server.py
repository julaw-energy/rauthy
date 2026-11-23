#!/usr/bin/env python3
"""
Sign-in lab: a tiny relying party (RP) to compare ways of signing a user in with Rauthy.

  ISSUER     https://accounts.example.com/auth/v1     (required)
  CLIENT_ID  public client registered in Rauthy      (required)
  PORT       8099
  JWKS_URL   optional, default <ISSUER>/oidc/certs

Serves the page, its config, and one backend endpoint, POST /verify, which checks an ID token
(signature through the JWKS of the issuer, `iss`, `aud`, `exp`, `nonce`) and sets a session cookie of
its own. That is what any of your backends would do after a token reaches them.
"""
import json
import os
import secrets
import time
import urllib.request
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

import jwt  # PyJWT, with `cryptography` for EdDSA / RSA

ISSUER = os.environ["ISSUER"].rstrip("/")
CLIENT_ID = os.environ["CLIENT_ID"]
PORT = int(os.environ.get("PORT", "8099"))
# where to fetch the keys from, if the public issuer URL is not reachable from the container
JWKS_URL = os.environ.get("JWKS_URL", f"{ISSUER}/oidc/certs")
HERE = Path(__file__).resolve().parent
SESSIONS = {}  # lab session id -> claims
_jwks = {"at": 0, "client": None}


def jwks_client():
    # one cache, refreshed every 10 min, so a rotated key does not need a restart
    if time.time() - _jwks["at"] > 600 or _jwks["client"] is None:
        _jwks["client"] = jwt.PyJWKClient(JWKS_URL, cache_keys=True)
        _jwks["at"] = time.time()
    return _jwks["client"]


def verify(id_token: str, nonce: str | None):
    key = jwks_client().get_signing_key_from_jwt(id_token)
    claims = jwt.decode(
        id_token, key.key, algorithms=["EdDSA", "RS256", "RS384", "RS512", "ES256"],
        audience=CLIENT_ID, issuer=[ISSUER, ISSUER + "/"],  # Rauthy's `iss` ends with a slash
        options={"require": ["exp", "iss", "aud"]},
    )
    if nonce is not None and claims.get("nonce") != nonce:
        raise jwt.InvalidTokenError("nonce does not match")
    return claims


class H(BaseHTTPRequestHandler):
    def log_message(self, fmt, *a):
        print("%s %s" % (self.address_string(), fmt % a), flush=True)

    def send(self, code, body, ctype="application/json", cookie=None):
        raw = body if isinstance(body, bytes) else (body if isinstance(body, str) else json.dumps(body)).encode()
        self.send_response(code)
        self.send_header("Content-Type", ctype)
        self.send_header("Content-Length", str(len(raw)))
        self.send_header("Cache-Control", "no-store")
        if cookie:
            self.send_header("Set-Cookie", cookie)
        self.end_headers()
        self.wfile.write(raw)

    def sid(self):
        for part in self.headers.get("Cookie", "").split(";"):
            k, _, v = part.strip().partition("=")
            if k == "lab_session":
                return v

    def do_GET(self):
        path = self.path.split("?")[0]
        if path in ("/", "/index.html"):
            return self.send(200, (HERE / "index.html").read_bytes(), "text/html; charset=utf-8")
        if path == "/config.json":
            return self.send(200, {"issuer": ISSUER, "client_id": CLIENT_ID})
        if path == "/me":
            claims = SESSIONS.get(self.sid())
            return self.send(200 if claims else 401, {"signed_in": bool(claims), "claims": claims})
        self.send(404, {"error": "not found"})

    def do_POST(self):
        path = self.path.split("?")[0]
        n = int(self.headers.get("Content-Length") or 0)
        try:
            data = json.loads(self.rfile.read(n) or b"{}")
        except ValueError:
            return self.send(400, {"error": "bad json"})
        if path == "/verify":
            t0 = time.perf_counter()
            try:
                claims = verify(data.get("id_token", ""), data.get("nonce"))
            except Exception as e:  # any failure = not signed in
                return self.send(401, {"error": f"{type(e).__name__}: {e}"})
            sid = secrets.token_urlsafe(24)
            SESSIONS[sid] = {k: claims.get(k) for k in ("sub", "email", "name", "given_name", "family_name", "iss", "aud", "exp")}
            return self.send(
                200, {"ok": True, "claims": SESSIONS[sid], "verify_ms": round((time.perf_counter() - t0) * 1000, 1)},
                cookie=f"lab_session={sid}; Path=/; HttpOnly; SameSite=Lax")
        if path == "/logout":
            SESSIONS.pop(self.sid(), None)
            return self.send(200, {"ok": True}, cookie="lab_session=; Path=/; Max-Age=0")
        self.send(404, {"error": "not found"})


if __name__ == "__main__":
    print(f"sign-in lab on :{PORT}, issuer {ISSUER}, client {CLIENT_ID}", flush=True)
    ThreadingHTTPServer(("0.0.0.0", PORT), H).serve_forever()
