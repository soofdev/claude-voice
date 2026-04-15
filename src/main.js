const { invoke } = window.__TAURI__.core;
const { event } = window.__TAURI__;

event.listen("voice:error", (e) => {
  const msg = e.payload?.message ?? "Unknown error";
  const s = document.getElementById("status");
  if (s) {
    s.textContent = msg;
    s.style.color = "#d25b5b";
  }
});

event.listen("settings:changed", (e) => {
  if (e.payload) currentSettings = e.payload;
});

event.listen("sessions:changed", () => loadSessions());

event.listen("session:focus", async (e) => {
  await loadSessions();
  const id = e.payload;
  const list = el("sessions-list");
  if (!list) return;
  const target = Array.from(list.children).find(
    (li) => li.dataset.sessionId === id,
  );
  if (target) {
    target.scrollIntoView({ behavior: "smooth", block: "center" });
    target.classList.add("highlight");
    setTimeout(() => target.classList.remove("highlight"), 1800);
  }
});

function formatAgo(ms) {
  const d = Date.now() - Number(ms);
  const min = Math.floor(d / 60000);
  if (min < 1) return "active now";
  if (min < 60) return `${min}m ago`;
  const hr = Math.floor(min / 60);
  if (hr < 24) return `${hr}h ago`;
  const day = Math.floor(hr / 24);
  return `${day}d ago`;
}

function renderSession(s) {
  const li = document.createElement("li");
  li.className = "session" + (s.enabled ? "" : " muted");
  li.dataset.sessionId = s.session_id;
  if (s.color) {
    li.style.borderLeft = `4px solid ${s.color}`;
  }

  const info = document.createElement("div");
  info.className = "info";

  const labelRow = document.createElement("div");
  labelRow.className = "label-row";
  const swatch = document.createElement("input");
  swatch.type = "color";
  swatch.className = "session-swatch";
  swatch.value = s.color || "#888888";
  swatch.title = "Session color";
  swatch.addEventListener("change", async () => {
    try {
      await invoke("set_session_color", {
        id: s.session_id,
        color: swatch.value,
      });
    } catch (e) {
      console.error("set_session_color failed", e);
    }
  });
  const label = document.createElement("input");
  label.className = "label";
  label.value = s.label;
  label.title = "Click to rename";
  label.addEventListener("change", async () => {
    const next = label.value.trim() || s.label;
    try {
      await invoke("rename_session", { id: s.session_id, label: next });
    } catch (e) {
      console.error("rename_session failed", e);
    }
  });
  labelRow.appendChild(swatch);
  labelRow.appendChild(label);

  const voice = document.createElement("select");
  voice.className = "session-voice";
  voice.title = "Voice for this session";
  const defOpt = document.createElement("option");
  defOpt.value = "";
  defOpt.textContent = "Default voice";
  voice.appendChild(defOpt);
  const seen = new Set();
  for (const v of [...elevenVoices, ...PRESET_ELEVEN_VOICES]) {
    if (!v || !v.id || seen.has(v.id)) continue;
    seen.add(v.id);
    const o = document.createElement("option");
    o.value = v.id;
    o.textContent = v.name ? `${v.name} (${v.id.slice(0, 6)}…)` : v.id;
    voice.appendChild(o);
  }
  voice.value = s.voice_override || "";
  voice.addEventListener("change", async () => {
    try {
      await invoke("set_session_voice", {
        id: s.session_id,
        voiceId: voice.value || null,
      });
    } catch (e) {
      console.error("set_session_voice failed", e);
    }
  });

  const meta = document.createElement("div");
  meta.className = "meta";
  const cwdBit = s.cwd ? `${s.cwd} · ` : "";
  meta.textContent = `${cwdBit}${formatAgo(s.last_seen_ms)}`;

  info.appendChild(labelRow);
  info.appendChild(voice);
  info.appendChild(meta);

  const toggle = document.createElement("button");
  toggle.type = "button";
  toggle.className = "toggle " + (s.enabled ? "on" : "off");
  toggle.textContent = s.enabled ? "On" : "Off";
  toggle.addEventListener("click", async () => {
    try {
      await invoke("set_session_enabled", {
        id: s.session_id,
        enabled: !s.enabled,
      });
    } catch (e) {
      console.error("set_session_enabled failed", e);
    }
  });

  const remove = document.createElement("button");
  remove.type = "button";
  remove.className = "remove";
  remove.textContent = "Remove";
  remove.title = "Forget this session";
  remove.addEventListener("click", async () => {
    try {
      await invoke("remove_session", { id: s.session_id });
    } catch (e) {
      console.error("remove_session failed", e);
    }
  });

  li.appendChild(info);
  li.appendChild(toggle);
  li.appendChild(remove);
  return li;
}

async function loadSessions() {
  try {
    const list = el("sessions-list");
    const empty = el("sessions-empty");
    const sessions = await invoke("get_sessions");
    list.innerHTML = "";
    if (!sessions || sessions.length === 0) {
      empty.hidden = false;
      return;
    }
    empty.hidden = true;
    for (const s of sessions) {
      list.appendChild(renderSession(s));
    }
  } catch (e) {
    console.error("get_sessions failed", e);
  }
}

const el = (id) => document.getElementById(id);

let elevenVoices = [];
let currentSettings = {};

const PRESET_ELEVEN_VOICES = [
  { id: "fvVBPXuE7f1iX3dZLKFy", name: "Preset A" },
  { id: "lOg4rs9vOIKiYNmAA4C5", name: "Preset B" },
  { id: "pNInz6obpgDQGcFmaJgB", name: "Adam (free)" },
];

function mergeWithPresets(list, selectedId) {
  const seen = new Set();
  const merged = [];
  for (const v of PRESET_ELEVEN_VOICES) {
    if (seen.has(v.id)) continue;
    seen.add(v.id);
    merged.push(v);
  }
  for (const v of list || []) {
    if (!v || !v.id || seen.has(v.id)) continue;
    seen.add(v.id);
    merged.push(v);
  }
  if (selectedId && !seen.has(selectedId)) {
    merged.unshift({ id: selectedId, name: "Saved voice" });
  }
  return merged;
}

async function loadSettings() {
  const settings = await invoke("get_settings");
  currentSettings = settings;
  const sayVoices = await invoke("list_voices");
  const hookCmd = await invoke("hook_command");

  el("enabled").checked = settings.enabled;
  el("show-popup").checked = settings.show_popup;
  el("dismiss-delay").value = settings.popup_dismiss_delay_ms ?? 1500;
  el("dismiss-delay-value").textContent = (
    (settings.popup_dismiss_delay_ms ?? 1500) / 1000
  ).toFixed(1);
  el("speak-prefix").checked = settings.speak_session_prefix !== false;
  el("prefix-skip").value = settings.prefix_skip_window_ms ?? 30000;
  el("prefix-skip-value").textContent = Math.round(
    (settings.prefix_skip_window_ms ?? 30000) / 1000,
  );
  el("backend").value = settings.backend;

  const voiceSel = el("voice");
  voiceSel.innerHTML = "";
  for (const v of sayVoices) {
    const opt = document.createElement("option");
    opt.value = v;
    opt.textContent = v;
    if (v === settings.voice) opt.selected = true;
    voiceSel.appendChild(opt);
  }

  el("rate").value = settings.rate;
  el("rate-value").textContent = settings.rate;

  el("eleven-key").value = settings.elevenlabs_api_key;
  el("eleven-model").value = settings.elevenlabs_model_id;
  el("eleven-speed").value = settings.elevenlabs_speed;
  el("eleven-speed-value").textContent = Number(settings.elevenlabs_speed).toFixed(2);
  populateElevenVoices(
    mergeWithPresets([], settings.elevenlabs_voice_id),
    settings.elevenlabs_voice_id,
  );

  el("summarize").checked = settings.summarize;
  el("anthropic-key").value = settings.anthropic_api_key;
  el("summary-model").value = settings.summary_model;
  el("summary-threshold").value = settings.summary_threshold_chars;
  el("threshold-value").textContent = settings.summary_threshold_chars;
  el("summary-brevity").value = settings.summary_brevity || "balanced";

  el("port").value = settings.port;
  el("hook-cmd").textContent = hookCmd;

  updateBackendVisibility();
}

function populateElevenVoices(voices, selectedId) {
  elevenVoices = voices;
  const sel = el("eleven-voice");
  sel.innerHTML = "";
  for (const v of voices) {
    const opt = document.createElement("option");
    opt.value = v.id;
    opt.textContent = v.name ? `${v.name} (${v.id.slice(0, 6)}…)` : v.id;
    if (v.id === selectedId) opt.selected = true;
    sel.appendChild(opt);
  }
}

function updateBackendVisibility() {
  const backend = el("backend").value;
  el("say-settings").hidden = backend !== "say";
  el("eleven-settings").hidden = backend !== "elevenlabs";
}

function collect() {
  return {
    ...currentSettings,
    enabled: el("enabled").checked,
    port: parseInt(el("port").value, 10),
    show_popup: el("show-popup").checked,
    popup_dismiss_delay_ms: parseInt(el("dismiss-delay").value, 10),
    speak_session_prefix: el("speak-prefix").checked,
    prefix_skip_window_ms: parseInt(el("prefix-skip").value, 10),
    backend: el("backend").value,
    voice: el("voice").value || "Samantha",
    rate: parseInt(el("rate").value, 10),
    elevenlabs_api_key: el("eleven-key").value,
    elevenlabs_voice_id: el("eleven-voice").value,
    elevenlabs_model_id: el("eleven-model").value,
    elevenlabs_speed: parseFloat(el("eleven-speed").value),
    summarize: el("summarize").checked,
    anthropic_api_key: el("anthropic-key").value,
    summary_model: el("summary-model").value,
    summary_threshold_chars: parseInt(el("summary-threshold").value, 10),
    summary_brevity: el("summary-brevity").value || "balanced",
  };
}

async function save() {
  const btn = el("save");
  try {
    const next = collect();
    await invoke("set_settings", { new: next });
    currentSettings = next;
    el("hook-cmd").textContent = await invoke("hook_command");
    flashSaveButton(btn, "Saved ✓", "saved");
  } catch (e) {
    setStatus(`Error: ${e}`, true);
    flashSaveButton(btn, "Error", "error");
  }
}

function flashSaveButton(btn, label, cls) {
  const original = btn.dataset.label || btn.textContent;
  btn.dataset.label = original;
  btn.disabled = true;
  btn.textContent = label;
  btn.classList.add(cls);
  setTimeout(() => {
    btn.classList.remove(cls);
    btn.textContent = original;
    btn.disabled = false;
  }, 1500);
}

function setStatus(msg, isError = false) {
  const s = el("status");
  s.textContent = msg;
  s.style.color = isError ? "#d25b5b" : "";
  setTimeout(() => (s.textContent = ""), 3500);
}

async function refreshElevenVoices() {
  const key = el("eleven-key").value;
  if (!key) {
    setStatus("Enter ElevenLabs API key first", true);
    return;
  }
  setStatus("Fetching voices…");
  try {
    const result = await invoke("list_elevenlabs_voices", { apiKey: key });
    const voices = result.map(([id, name]) => ({ id, name }));
    const current = el("eleven-voice").value;
    populateElevenVoices(mergeWithPresets(voices, current), current);
    setStatus(`Loaded ${voices.length} voices.`);
  } catch (e) {
    setStatus(`Error: ${e}`, true);
  }
}

window.addEventListener("DOMContentLoaded", async () => {
  await loadSettings();
  await loadSessions();

  el("backend").addEventListener("change", updateBackendVisibility);
  el("rate").addEventListener("input", (e) => {
    el("rate-value").textContent = e.target.value;
  });
  el("dismiss-delay").addEventListener("input", (e) => {
    el("dismiss-delay-value").textContent = (Number(e.target.value) / 1000).toFixed(1);
  });
  el("prefix-skip").addEventListener("input", (e) => {
    el("prefix-skip-value").textContent = Math.round(Number(e.target.value) / 1000);
  });
  el("summary-threshold").addEventListener("input", (e) => {
    el("threshold-value").textContent = e.target.value;
  });
  el("eleven-speed").addEventListener("input", (e) => {
    el("eleven-speed-value").textContent = Number(e.target.value).toFixed(2);
  });

  el("save").addEventListener("click", save);
  el("refresh-voices").addEventListener("click", refreshElevenVoices);

  el("test").addEventListener("click", async () => {
    await invoke("set_settings", { new: collect() });
    await invoke("test_speak", {
      text: "Hello. This is Claude speaking through the configured voice engine.",
    });
  });

  el("stop").addEventListener("click", () => invoke("stop_speaking"));

  el("copy-hook").addEventListener("click", async () => {
    await navigator.clipboard.writeText(el("hook-cmd").textContent);
    setStatus("Copied.");
  });
});
