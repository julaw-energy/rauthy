#!/usr/bin/env python3
"""
End to end test of the LDAP server of an image, with the checks of this fork.

  ldap_smoke.py [--image IMG]       default rauthy-custom:0.37.2-custom.9

It starts the image on a fresh Hiqlite database with LDAP on (plain, behind `proxy_mode`, as a reverse
proxy would do it), the bootstrap data of the repo (client `bootstrap_client_1` with the `password` flow
and `restrict_group_prefix = bs_group_`, user `alfred@batcave`), and checks:

  1. the port is closed on an image started WITHOUT `[ldap]` (it is off by default)
  2. RootDSE answers
  3. a user without a group of the client's prefix is refused
  4. the same user in a group of the prefix gets in, a wrong password is refused
  5. an access rule the user does not meet refuses the bind (our rules apply to LDAP binds),
     removing the rule lets the user in again
  6. a user bind can search its own entry
(a public client is used: user binds need no service bind first)

Needs docker, python 3.8+. Port 18081 (HTTP) and 16360 (LDAP) are used. Exit code 1 on any failure.
"""
import argparse
import base64
import json
import os
import shutil
import socket
import subprocess
import sys
import tempfile
import time
import urllib.error
import urllib.request
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent.parent
HTTP = 18081
LDAP = 16360
NAME = "rauthy-ldap-smoke"
SECRET = "ldapsmokeldapsmokeldapsmokeldapsmokeldapsmokeldapsmokeldapsmoke00"
BASE_DN = "dc=example,dc=com"
CLIENT = "bootstrap_client_1"
SERVICE_DN = f"cn={CLIENT},ou=clients,{BASE_DN}"
USER_DN = f"uid=alfred_p,ou=people,cn={CLIENT},ou=clients,{BASE_DN}"
FAILED = []


def sh(*a, check=True):
    r = subprocess.run(a, capture_output=True, text=True)
    if check and r.returncode != 0:
        raise RuntimeError(" ".join(a) + "\n" + r.stdout + r.stderr)
    return r


def api(method, path, body=None):
    data = json.dumps(body).encode() if body is not None else None
    req = urllib.request.Request(f"http://localhost:{HTTP}/auth/v1{path}", data=data, method=method)
    req.add_header("Authorization", f"API-Key smoke${SECRET}")
    req.add_header("X-Forwarded-For", "127.0.0.1")
    req.add_header("User-Agent", "ldap-smoke")
    if data is not None:
        req.add_header("Content-Type", "application/json")
    try:
        with urllib.request.urlopen(req, timeout=20) as r:
            raw = r.read()
            return r.status, (json.loads(raw) if raw else None)
    except urllib.error.HTTPError as e:
        return e.code, e.read().decode(errors="replace")[:300]


def start(tmp: Path, ldap: bool):
    sh("docker", "rm", "-f", NAME, check=False)
    key = {
        "name": "smoke", "exp": 4102444800, "secret": {"Plain": SECRET},
        "access": [{"group": g, "access_rights": ["read", "create", "update", "delete"]}
                   for g in ("Clients", "Users", "Pam", "Secrets", "Groups", "Roles")],
    }
    env = {
        "HQL_DATA_DIR": "/app/data", "HQL_NODES": "1 127.0.0.1:18110 127.0.0.1:18210",
        "METRICS_PORT": "19291", "LISTEN_PORT_HTTP": str(HTTP),
        "PUB_URL": f"localhost:{HTTP}", "RP_ORIGIN": f"http://localhost:{HTTP}",
        "PROXY_MODE": "true", "TRUSTED_PROXIES": "127.0.0.0/8",
        "BOOTSTRAP_API_KEY": base64.b64encode(json.dumps(key).encode()).decode(),
        "BOOTSTRAP_API_KEY_SECRET": SECRET,
    }
    if ldap:
        env.update({"LDAP_ENABLE": "true", "LDAP_PORT": str(LDAP), "LDAP_BASE_DN": BASE_DN,
                    "LDAP_PLAIN_TEXT": "true", "LDAP_LISTEN_ADDRESS": "127.0.0.1"})
    args = []
    for k, v in env.items():
        args += ["-e", f"{k}={v}"]
    data = tmp / ("data_ldap" if ldap else "data_off")
    data.mkdir()
    sh("docker", "run", "-d", "--name", NAME, "--network", "host", "--user", f"{os.getuid()}:{os.getgid()}",
       "-v", f"{data}:/app/data", "-v", f"{tmp / 'config.toml'}:/app/config.toml:ro",
       "-v", f"{REPO / 'bootstrap'}:/app/bootstrap:ro", *args, IMAGE, "serve", "-c", "/app/config.toml")
    end = time.time() + 120
    while time.time() < end:
        try:
            with urllib.request.urlopen(f"http://localhost:{HTTP}/auth/v1/health", timeout=3) as r:
                if b'"db_healthy":true' in r.read():
                    return
        except Exception:
            pass
        time.sleep(1.5)
    raise RuntimeError("not healthy: " + sh("docker", "logs", "--tail", "20", NAME, check=False).stderr[-600:])


def check(name, ok, detail=""):
    print(f"  {'ok    ' if ok else 'FAILED'}  {name}" + (f"   ({detail})" if detail and not ok else ""), flush=True)
    if not ok:
        FAILED.append(name)


# ---- a minimal LDAP client: simple bind, enough for this test --------------------------------------
def tlv(tag, content: bytes):
    n = len(content)
    if n < 128:
        ln = bytes([n])
    else:
        b = n.to_bytes((n.bit_length() + 7) // 8, "big")
        ln = bytes([0x80 | len(b)]) + b
    return bytes([tag]) + ln + content


def bind_request(msg_id, dn, pw):
    body = tlv(0x02, b"\x03") + tlv(0x04, dn.encode()) + tlv(0x80, pw.encode())
    return tlv(0x30, tlv(0x02, bytes([msg_id])) + tlv(0x60, body))


def read_tlv(buf, i):
    tag = buf[i]
    ln = buf[i + 1]
    i += 2
    if ln & 0x80:
        k = ln & 0x7F
        ln = int.from_bytes(buf[i:i + k], "big")
        i += k
    return tag, buf[i:i + ln], i + ln


def recv_msg(sock):
    data = b""
    sock.settimeout(10)
    while True:
        data += sock.recv(4096)
        try:
            tag, content, end = read_tlv(data, 0)
            if len(data) >= end:
                return content
        except IndexError:
            pass


def bind(sock, msg_id, dn, pw):
    """Returns (resultCode, diagnosticMessage)."""
    sock.sendall(bind_request(msg_id, dn, pw))
    content = recv_msg(sock)
    _, _, i = read_tlv(content, 0)          # messageID
    _, resp, _ = read_tlv(content, i)        # BindResponse
    _, code, j = read_tlv(resp, 0)           # resultCode
    _, _, j = read_tlv(resp, j)              # matchedDN
    _, diag, _ = read_tlv(resp, j)           # diagnosticMessage
    return int.from_bytes(code, "big"), diag.decode(errors="replace")


def connect():
    return socket.create_connection(("127.0.0.1", LDAP), timeout=10)


def main():
    global IMAGE
    ap = argparse.ArgumentParser()
    ap.add_argument("--image", default="rauthy-custom:0.37.2-custom.9")
    IMAGE = ap.parse_args().image

    tmp = Path(tempfile.mkdtemp(prefix="rauthy-ldap-"))
    (tmp / "config.toml").write_text(
        "\n".join(l for l in (REPO / "config-test.toml").read_text().splitlines()
                  if not l.strip().startswith("internal_hosts")) + "\n")
    try:
        print("1. LDAP is off by default", flush=True)
        start(tmp, ldap=False)
        s = socket.socket()
        s.settimeout(2)
        closed = s.connect_ex(("127.0.0.1", LDAP)) != 0
        s.close()
        check("the LDAP port is closed without [ldap]", closed)

        print("2. LDAP on", flush=True)
        start(tmp, ldap=True)
        code, users = api("GET", "/users")
        alfred = next((u for u in users if u["email"] == "alfred@batcave"), None) if code == 200 else None
        check("bootstrap user alfred@batcave exists", alfred is not None, f"{code}")
        code, body = api("POST", "/pam/users", {"username": "alfred_p", "email": "alfred@batcave"})
        check("a PAM username for alfred (the LDAP uid)", code in (200, 201), f"{code} {body}")
        # `bootstrap_client_1` is a public client: user binds work right away, no service bind needed
        if not alfred:
            raise RuntimeError("setup failed")

        r = sh("ldapsearch", "-x", "-H", f"ldap://127.0.0.1:{LDAP}", "-b", "", "-s", "base",
               "-LLL", "objectClass=*", check=False)
        check("RootDSE answers", r.returncode == 0, (r.stdout + r.stderr)[:120])

        c = connect()
        code_, msg = bind(c, 1, USER_DN, "123SuperSafe")
        check("a user without a group of the prefix is refused", code_ != 0, f"code {code_}")
        print(f"        server says: {msg!r}", flush=True)
        c.close()

        code, body = api("PUT", f"/users/{alfred['id']}/groups/bs_group_1")
        check("alfred joins group bs_group_1", code in (200, 201, 204), f"{code} {body}")
        c = connect()
        code_, _ = bind(c, 1, USER_DN, "123SuperSafe")
        check("the same user in a group of the prefix gets in", code_ == 0, f"code {code_}")
        c.close()

        c = connect()
        code_, _ = bind(c, 1, USER_DN, "not-the-password")
        check("a wrong password is refused", code_ == 49, f"code {code_}")
        c.close()

        code, body = api("PUT", f"/clients/{CLIENT}/access_rules", {"rules": [
            {"field": "country", "op": "in", "value": ["UG"], "message": "only for Uganda"}]})
        check("an access rule on the country is saved", code == 200, f"{code} {body}")
        c = connect()
        code_, msg = bind(c, 1, USER_DN, "123SuperSafe")
        check("a user who does not meet the rule is refused (our rules apply to LDAP)", code_ != 0, f"code {code_}")
        print(f"        server says: {msg!r}", flush=True)
        c.close()

        code, body = api("PUT", f"/clients/{CLIENT}/access_rules", {"rules": []})
        c = connect()
        code_, _ = bind(c, 1, USER_DN, "123SuperSafe")
        check("without the rule the user gets in again", code_ == 0, f"code {code_}")
        c.close()

        r = sh("ldapsearch", "-x", "-H", f"ldap://127.0.0.1:{LDAP}", "-D", USER_DN, "-w", "123SuperSafe",
               "-b", USER_DN, "-LLL", "(objectClass=*)", "uid", "mail", check=False)
        check("a search of the user's own entry returns it",
              "uid: alfred_p" in r.stdout and "alfred@batcave" in r.stdout, (r.stdout + r.stderr)[:160])
    except Exception as e:
        FAILED.append(f"crashed: {e}")
        print("CRASH:", e, flush=True)
    finally:
        sh("docker", "rm", "-f", NAME, check=False)
        shutil.rmtree(tmp, ignore_errors=True)

    print()
    print("ALL OK" if not FAILED else "FAILED: " + "; ".join(FAILED))
    sys.exit(1 if FAILED else 0)


if __name__ == "__main__":
    main()
