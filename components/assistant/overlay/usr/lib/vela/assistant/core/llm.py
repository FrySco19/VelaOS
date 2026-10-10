"""OpenAI-compatible chat client with streaming and tool calls (works with Ollama /v1 and any compatible API).

Pure standard library (http.client), so it runs in a worker thread and can be tested against a mock server.
"""
import http.client
import json
import re
import socket
import ssl
import threading
import urllib.error
import urllib.parse
import urllib.request

from . import i18n

T = i18n.T


class LLMError(Exception):
    """Error with a plain, user-facing message (Italian/English via i18n) and a short technical detail."""

    def __init__(self, kind, message, detail=""):
        super().__init__(message)
        self.kind = kind
        self.message = message
        self.detail = detail


class ToolsUnsupported(Exception):
    pass


class Cancelled(Exception):
    pass


# ----------------------------------------------------------------------------------------------- readiness
def _get_json(url, timeout=2.0, headers=None):
    req = urllib.request.Request(url, headers=headers or {})
    with urllib.request.urlopen(req, timeout=timeout) as r:
        return json.loads(r.read().decode("utf-8") or "{}")


def ollama_models(native_url, timeout=2.0):
    """Return the list of installed model tags, or raise OSError when Ollama does not answer."""
    data = _get_json(native_url.rstrip("/") + "/api/tags", timeout=timeout)
    return [m.get("name") or m.get("model") for m in data.get("models", []) if (m.get("name") or m.get("model"))]


def ollama_capabilities(native_url, model, timeout=3.0):
    """Capabilities reported by Ollama >= 0.6 (e.g. ["completion","tools","vision"]); [] if unknown."""
    try:
        body = json.dumps({"model": model}).encode()
        req = urllib.request.Request(native_url.rstrip("/") + "/api/show", data=body,
                                     headers={"Content-Type": "application/json"})
        with urllib.request.urlopen(req, timeout=timeout) as r:
            return json.loads(r.read().decode("utf-8")).get("capabilities") or []
    except (OSError, ValueError):
        return []


def check_ready(cfg, ep, ollama_installed=None):
    """Return dict(state=..., message=..., model=...). States:
    disabled, cloud_blocked, no_key, ollama_missing, ollama_down, no_model, ready."""
    if not cfg.get("enabled", True):
        return {"state": "disabled", "message": T("state_disabled"), "model": ""}
    if ep.get("base_url") == "http://127.0.0.1:11435/v1":
        try:
            _get_json("http://127.0.0.1:11435/health", timeout=2,
                      headers={"Authorization": "Bearer " + ep.get("api_key", "")})
        except (OSError, ValueError):
            return {"state": "local_loading", "message": "Modello locale in caricamento. Riprova tra pochi secondi.",
                    "model": ep["model"]}
    if ep["provider"] == "ollama":
        try:
            models = ollama_models(ep["native_url"])
        except (OSError, ValueError):
            if ollama_installed is None:
                import shutil, os
                ollama_installed = bool(shutil.which("ollama")) or os.path.exists(
                    os.path.expanduser("~/.local/share/vela/ollama/bin/ollama"))
            if ollama_installed:
                return {"state": "ollama_down", "message": T("state_ollama_down"), "model": ep["model"]}
            return {"state": "ollama_missing", "message": T("state_ollama_missing"), "model": ep["model"]}
        if not models:
            return {"state": "no_model", "message": T("state_no_model"), "model": ep["model"]}
        want = ep.get("model") or ""
        if want and want not in models and not any(m.split(":")[0] == want and m.endswith(":latest") for m in models):
            # configured model not installed: use the first installed one, say so in the header
            return {"state": "ready", "message": T("state_model_fallback", want=want, got=models[0]),
                    "model": models[0], "models": models}
        return {"state": "ready", "message": "", "model": want or models[0], "models": models}
    # OpenAI-compatible
    if ep["cloud"] and not cfg.get("privacy", {}).get("allow_cloud", False):
        return {"state": "cloud_blocked", "message": T("state_cloud_blocked"), "model": ep["model"]}
    if ep["cloud"] and not ep.get("api_key"):
        return {"state": "no_key", "message": T("state_no_key"), "model": ep["model"]}
    if not ep.get("model"):
        return {"state": "no_key", "message": T("state_no_model_api"), "model": ""}
    return {"state": "ready", "message": "", "model": ep["model"]}


# ----------------------------------------------------------------------------------------------- redaction
_SECRET_PATTERNS = [
    re.compile(r"\bsk-[A-Za-z0-9_\-]{16,}"),
    re.compile(r"\bgh[pousr]_[A-Za-z0-9]{20,}"),
    re.compile(r"\bAKIA[0-9A-Z]{16}\b"),
    re.compile(r"\bxox[abprs]-[A-Za-z0-9\-]{10,}"),
    re.compile(r"\beyJ[A-Za-z0-9_\-]{10,}\.[A-Za-z0-9_\-]{10,}\.[A-Za-z0-9_\-]{10,}"),
    re.compile(r"-----BEGIN [A-Z ]*PRIVATE KEY-----[\s\S]*?-----END [A-Z ]*PRIVATE KEY-----"),
    re.compile(r"(?i)\b(password|passwd|pwd|secret|token|api[_-]?key)\s*[:=]\s*\S+"),
]


def redact(text):
    if not isinstance(text, str):
        return text
    for p in _SECRET_PATTERNS:
        text = p.sub(lambda m: (m.group(1) + "=[nascosto]") if m.re.groups else "[nascosto]", text)
    return text


def redact_messages(messages):
    out = []
    for m in messages:
        m = dict(m)
        c = m.get("content")
        if isinstance(c, str):
            m["content"] = redact(c)
        elif isinstance(c, list):
            m["content"] = [dict(p, text=redact(p["text"])) if p.get("type") == "text" else p for p in c]
        out.append(m)
    return out


# ----------------------------------------------------------------------------------------------- streaming
def _connect(url, timeout):
    u = urllib.parse.urlsplit(url)
    if u.scheme == "https":
        conn = http.client.HTTPSConnection(u.hostname, u.port or 443, timeout=timeout,
                                           context=ssl.create_default_context())
    else:
        conn = http.client.HTTPConnection(u.hostname, u.port or 80, timeout=timeout)
    path = u.path or "/"
    if u.query:
        path += "?" + u.query
    return conn, path


def _http_error(status, body, model):
    detail = body[:400]
    try:
        j = json.loads(body)
        err = j.get("error")
        detail = (err.get("message") if isinstance(err, dict) else str(err or j))[:400]
    except ValueError:
        pass
    low = detail.lower()
    if status == 401 or status == 403:
        return LLMError("auth", T("err_auth"), detail)
    if status == 404 or "not found" in low and "model" in low:
        return LLMError("model", T("err_model", model=model), detail)
    if status == 429:
        return LLMError("rate", T("err_rate"), detail)
    if status == 400 and ("tool" in low or "function" in low):
        raise ToolsUnsupported(detail)
    if status >= 500:
        return LLMError("server", T("err_server"), detail)
    return LLMError("http", T("err_http", code=status), detail)


def stream_chat(ep, messages, tools=None, temperature=0.3, max_tokens=1024, timeout=60, cancel=None):
    """Yield events: ("delta", text) | ("tool_calls", [ {id,name,arguments(str)} ]) | ("done", finish_reason).

    Raises LLMError (user-facing), ToolsUnsupported (caller retries without tools) or Cancelled.
    """
    if cancel is not None and cancel.is_set():
        raise Cancelled()
    finished = threading.Event()
    url = ep["base_url"].rstrip("/") + "/chat/completions"
    payload = {"model": ep["model"], "messages": messages, "stream": True,
               "temperature": temperature, "max_tokens": max_tokens}
    if tools:
        payload["tools"] = tools
    headers = {"Content-Type": "application/json", "Accept": "text/event-stream"}
    if ep.get("api_key"):
        headers["Authorization"] = "Bearer " + ep["api_key"]
    try:
        conn, path = _connect(url, timeout)
        conn.request("POST", path, body=json.dumps(payload).encode("utf-8"), headers=headers)
        if cancel is not None:
            sock = conn.sock
            def interrupt_socket():
                while not finished.wait(0.1):
                    if cancel.is_set():
                        try:
                            sock.shutdown(socket.SHUT_RDWR)
                        except (OSError, AttributeError):
                            pass
                        return
            threading.Thread(target=interrupt_socket, daemon=True).start()
        resp = conn.getresponse()
    except (ConnectionRefusedError, socket.gaierror) as e:
        finished.set()
        raise LLMError("connect", T("err_connect_local") if not ep.get("cloud") else T("err_connect_cloud"), str(e))
    except (socket.timeout, TimeoutError) as e:
        finished.set()
        raise LLMError("timeout", T("err_timeout"), str(e))
    except ssl.SSLError as e:
        finished.set()
        raise LLMError("tls", T("err_tls"), str(e))
    except (OSError, http.client.HTTPException) as e:
        finished.set()
        if cancel is not None and cancel.is_set():
            raise Cancelled()
        raise LLMError("connect", T("err_connect_cloud") if ep.get("cloud") else T("err_connect_local"), str(e))

    if resp.status != 200:
        body = resp.read().decode("utf-8", "replace")
        conn.close()
        finished.set()
        raise _http_error(resp.status, body, ep["model"])

    ctype = resp.getheader("Content-Type", "")
    calls = {}
    finish = None
    try:
        if "text/event-stream" not in ctype and "ndjson" not in ctype:
            # server ignored stream=true: parse a normal JSON completion
            j = json.loads(resp.read().decode("utf-8"))
            ch = (j.get("choices") or [{}])[0]
            msg = ch.get("message", {})
            if msg.get("content"):
                yield ("delta", msg["content"])
            for i, tc in enumerate(msg.get("tool_calls") or []):
                calls[i] = {"id": tc.get("id") or f"call_{i}", "name": tc["function"]["name"],
                            "arguments": tc["function"].get("arguments") or "{}"}
            finish = ch.get("finish_reason") or "stop"
        else:
            while True:
                if cancel is not None and cancel.is_set():
                    raise Cancelled()
                line = resp.readline()
                if not line:
                    break
                line = line.decode("utf-8", "replace").strip()
                if not line or line.startswith(":"):
                    continue
                if line.startswith("data:"):
                    line = line[5:].strip()
                if line == "[DONE]":
                    break
                try:
                    j = json.loads(line)
                except ValueError:
                    continue
                if j.get("error"):
                    err = j["error"]
                    raise LLMError("server", T("err_server"), err.get("message") if isinstance(err, dict) else str(err))
                for ch in j.get("choices") or []:
                    d = ch.get("delta") or ch.get("message") or {}
                    if d.get("content"):
                        yield ("delta", d["content"])
                    for tc in d.get("tool_calls") or []:
                        idx = tc.get("index", len(calls))
                        cur = calls.setdefault(idx, {"id": "", "name": "", "arguments": ""})
                        if tc.get("id"):
                            cur["id"] = tc["id"]
                        fn = tc.get("function") or {}
                        if fn.get("name"):
                            cur["name"] += fn["name"]
                        if fn.get("arguments"):
                            a = fn["arguments"]
                            cur["arguments"] += a if isinstance(a, str) else json.dumps(a)
                    if ch.get("finish_reason"):
                        finish = ch["finish_reason"]
    except (socket.timeout, TimeoutError) as e:
        if cancel is not None and cancel.is_set():
            raise Cancelled()
        raise LLMError("timeout", T("err_timeout"), str(e))
    except (OSError, http.client.HTTPException) as e:
        if cancel is not None and cancel.is_set():
            raise Cancelled()
        raise LLMError("connect", T("err_connect_local") if not ep.get("cloud") else T("err_connect_cloud"), str(e))
    finally:
        finished.set()
        resp.close()
        try:
            conn.close()
        except Exception:
            pass
    if cancel is not None and cancel.is_set():
        raise Cancelled()
    if calls:
        out = []
        for i in sorted(calls):
            c = calls[i]
            if not c["id"]:
                c["id"] = f"call_{i}"
            out.append(c)
        yield ("tool_calls", out)
    yield ("done", finish or ("tool_calls" if calls else "stop"))
