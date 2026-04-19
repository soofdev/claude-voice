const enabledEl = document.getElementById("enabled");
const debugEl = document.getElementById("debug");
const finalOnlyEl = document.getElementById("finalOnly");
const hostEl = document.getElementById("host");
const portEl = document.getElementById("port");
const statusEl = document.getElementById("status");
const saveBtn = document.getElementById("save");
const testBtn = document.getElementById("test");

const DEFAULTS = {
  enabled: true,
  debug: false,
  finalOnly: false,
  host: "127.0.0.1",
  port: 8765,
};

async function load() {
  const v = await chrome.storage.sync.get(DEFAULTS);
  enabledEl.checked = v.enabled;
  debugEl.checked = v.debug;
  finalOnlyEl.checked = v.finalOnly;
  hostEl.value = v.host;
  portEl.value = v.port;
  await refreshStatus();
}

async function save() {
  const v = {
    enabled: enabledEl.checked,
    debug: debugEl.checked,
    finalOnly: finalOnlyEl.checked,
    host: hostEl.value.trim() || DEFAULTS.host,
    port: parseInt(portEl.value, 10) || DEFAULTS.port,
  };
  await chrome.storage.sync.set(v);
  await refreshStatus();
}

async function refreshStatus() {
  statusEl.textContent = "Checking…";
  statusEl.className = "status";
  try {
    const resp = await chrome.runtime.sendMessage({ type: "bridge:ping" });
    if (resp?.ok) {
      const d = resp.data || {};
      statusEl.className = "status ok";
      statusEl.textContent = `Connected · voice: ${d.voice || "—"} · ${
        d.speaking ? "speaking" : "idle"
      }`;
    } else {
      statusEl.className = "status err";
      statusEl.textContent = `Offline — is Claude Voice running?`;
    }
  } catch (e) {
    statusEl.className = "status err";
    statusEl.textContent = `Error: ${e.message || e}`;
  }
}

async function sendTest() {
  const resp = await chrome.runtime.sendMessage({
    type: "bridge:speak",
    payload: {
      sessionId: "bridge:test",
      sessionLabel: "Bridge Test",
      text: "Hello from the Claude Voice bridge. If you hear this, the extension is wired up correctly.",
      cwd: "bridge-test",
    },
  });
  if (resp?.ok) {
    statusEl.className = "status ok";
    statusEl.textContent = "Test sent.";
  } else {
    statusEl.className = "status err";
    statusEl.textContent = `Send failed: ${resp?.error || resp?.status || "unknown"}`;
  }
}

saveBtn.addEventListener("click", save);
testBtn.addEventListener("click", sendTest);
[enabledEl, debugEl, finalOnlyEl].forEach((el) =>
  el.addEventListener("change", save),
);

load();
