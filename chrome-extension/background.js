const DEFAULT_PORT = 8765;
const DEFAULT_HOST = "127.0.0.1";

async function getServer() {
  const { host, port } = await chrome.storage.sync.get({
    host: DEFAULT_HOST,
    port: DEFAULT_PORT,
  });
  return `http://${host}:${port}`;
}

async function speak(payload) {
  const base = await getServer();
  const body = {
    session_id: payload.sessionId,
    cwd: payload.cwd || "",
    tty: null,
    last_assistant_message: payload.text,
    stop_hook_active: false,
  };
  const resp = await fetch(`${base}/hook/stop`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });
  const text = await resp.text().catch(() => "");
  return { ok: resp.ok, status: resp.status, body: text };
}

async function pingServer() {
  try {
    const base = await getServer();
    const resp = await fetch(`${base}/status`, { method: "GET" });
    if (!resp.ok) return { ok: false, status: resp.status };
    const data = await resp.json();
    return { ok: true, data };
  } catch (e) {
    return { ok: false, error: String(e) };
  }
}

async function pollPending(sessionId) {
  try {
    const base = await getServer();
    const resp = await fetch(
      `${base}/bridge/pending/${encodeURIComponent(sessionId)}`,
    );
    if (!resp.ok) return { ok: false, status: resp.status, items: [] };
    const items = await resp.json();
    return { ok: true, items: Array.isArray(items) ? items : [] };
  } catch (e) {
    return { ok: false, error: String(e), items: [] };
  }
}

chrome.runtime.onMessage.addListener((msg, _sender, sendResponse) => {
  if (msg?.type === "bridge:speak") {
    speak(msg.payload)
      .then(sendResponse)
      .catch((e) => sendResponse({ ok: false, error: String(e) }));
    return true;
  }
  if (msg?.type === "bridge:ping") {
    pingServer()
      .then(sendResponse)
      .catch((e) => sendResponse({ ok: false, error: String(e) }));
    return true;
  }
  if (msg?.type === "bridge:pending-poll") {
    pollPending(msg.sessionId)
      .then(sendResponse)
      .catch((e) =>
        sendResponse({ ok: false, error: String(e), items: [] }),
      );
    return true;
  }
});
