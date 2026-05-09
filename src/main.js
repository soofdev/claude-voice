const { invoke } = window.__TAURI__.core;
const { event } = window.__TAURI__;

event.listen("voice:error", (e) => {
  const msg = e.payload?.message ?? "Unknown error";
  setStatus(msg, true);
});

event.listen("settings:changed", (e) => {
  if (!e.payload) return;
  currentSettings = e.payload;
  // Re-sync controls whose state can be flipped from another window
  // (popup bottom-bar toggle, tray menu, etc.) so this view doesn't
  // drift. Set isLoading so the change handlers don't bounce the
  // update back through autosave.
  const wasLoading = isLoading;
  isLoading = true;
  const toggle = el("enabled");
  if (toggle && typeof e.payload.enabled === "boolean") {
    toggle.checked = e.payload.enabled;
  }
  isLoading = wasLoading;
});

event.listen("sessions:changed", () => loadSessions());

event.listen("session:focus", async (e) => {
  switchTab("sessions");
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
let saveTimer = null;
let isLoading = true;

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
  el("orb-style").value = settings.orb_style || "glass";
  el("browser-voice").value = settings.browser_voice || "";
  el("browser-rate").value = settings.browser_rate ?? 1.0;
  el("browser-rate-value").textContent = Number(settings.browser_rate ?? 1.0).toFixed(1);
  el("backend").value = settings.backend;
  setBackendSegment(settings.backend);

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
  el("avoid-repetition").checked = !!settings.avoid_repetition;

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

function setBackendSegment(value) {
  for (const btn of document.querySelectorAll("#backend-seg .seg")) {
    btn.classList.toggle("active", btn.dataset.value === value);
    btn.setAttribute("aria-checked", btn.dataset.value === value ? "true" : "false");
  }
}

function updateBackendVisibility() {
  const backend = el("backend").value;
  el("say-settings").hidden = backend !== "say";
  el("browser-settings").hidden = backend !== "browser";
  el("eleven-settings").hidden = backend !== "elevenlabs";
  if (backend === "browser") loadBrowserVoices();
}

function loadBrowserVoices() {
  const sel = el("browser-voice");
  const populate = () => {
    const voices = speechSynthesis.getVoices();
    if (!voices.length) return;
    sel.innerHTML = "";
    for (const v of voices) {
      const opt = document.createElement("option");
      opt.value = v.name;
      opt.textContent = `${v.name} (${v.lang})`;
      sel.appendChild(opt);
    }
    const saved = currentSettings.browser_voice;
    if (saved) sel.value = saved;
  };
  populate();
  speechSynthesis.onvoiceschanged = populate;
}

function collect() {
  return {
    ...currentSettings,
    enabled: el("enabled").checked,
    port: parseInt(el("port").value, 10),
    show_popup: el("show-popup").checked,
    popup_dismiss_delay_ms: parseInt(el("dismiss-delay").value, 10),
    browser_voice: el("browser-voice").value || "",
    browser_rate: parseFloat(el("browser-rate").value),
    speak_session_prefix: el("speak-prefix").checked,
    prefix_skip_window_ms: parseInt(el("prefix-skip").value, 10),
    orb_style: el("orb-style").value || "glass",
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
    avoid_repetition: el("avoid-repetition").checked,
  };
}

function scheduleAutosave() {
  if (isLoading) return;
  if (saveTimer) clearTimeout(saveTimer);
  saveTimer = setTimeout(autosave, 300);
}

async function autosave() {
  saveTimer = null;
  try {
    const next = collect();
    await invoke("set_settings", { new: next });
    currentSettings = next;
    el("hook-cmd").textContent = await invoke("hook_command");
    setStatus("Saved", false, 1000);
  } catch (e) {
    setStatus(`Error: ${e}`, true);
  }
}

function setStatus(msg, isError = false, timeoutMs = 3500) {
  const s = el("status");
  s.textContent = msg;
  s.classList.toggle("error", !!isError);
  s.classList.toggle("ok", !isError && msg === "Saved");
  if (timeoutMs > 0) {
    setTimeout(() => {
      if (s.textContent === msg) {
        s.textContent = "";
        s.classList.remove("error", "ok");
      }
    }, timeoutMs);
  }
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

function switchTab(name) {
  for (const btn of document.querySelectorAll(".tab")) {
    btn.classList.toggle("active", btn.dataset.tab === name);
  }
  for (const panel of document.querySelectorAll(".panel")) {
    const match = panel.dataset.panel === name;
    panel.classList.toggle("active", match);
    panel.hidden = !match;
  }
}

async function refreshHookBanner() {
  try {
    const installed = await invoke("hook_installed");
    el("setup-banner").hidden = !!installed;
  } catch (e) {
    console.error("hook_installed failed", e);
  }
}

window.addEventListener("DOMContentLoaded", async () => {
  isLoading = true;
  await loadSettings();
  await loadSessions();
  await refreshHookBanner();
  isLoading = false;

  for (const btn of document.querySelectorAll(".tab")) {
    btn.addEventListener("click", () => switchTab(btn.dataset.tab));
  }

  for (const seg of document.querySelectorAll("#backend-seg .seg")) {
    seg.addEventListener("click", () => {
      el("backend").value = seg.dataset.value;
      setBackendSegment(seg.dataset.value);
      updateBackendVisibility();
      scheduleAutosave();
    });
  }

  el("rate").addEventListener("input", (e) => {
    el("rate-value").textContent = e.target.value;
  });
  el("dismiss-delay").addEventListener("input", (e) => {
    el("dismiss-delay-value").textContent = (Number(e.target.value) / 1000).toFixed(1);
  });
  el("browser-rate").addEventListener("input", (e) => {
    el("browser-rate-value").textContent = Number(e.target.value).toFixed(1);
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

  // Wire autosave to every input/change-emitting control inside .panels.
  // The master toggle in the topbar gets the same treatment.
  const watch = [
    "enabled",
    "show-popup",
    "dismiss-delay",
    "orb-style",
    "voice",
    "rate",
    "browser-voice",
    "browser-rate",
    "eleven-key",
    "eleven-voice",
    "eleven-model",
    "eleven-speed",
    "summarize",
    "anthropic-key",
    "summary-model",
    "summary-threshold",
    "summary-brevity",
    "avoid-repetition",
    "speak-prefix",
    "prefix-skip",
    "port",
  ];
  for (const id of watch) {
    const node = el(id);
    if (!node) continue;
    const evt =
      node.tagName === "SELECT" ||
      (node.type === "checkbox" || node.type === "color")
        ? "change"
        : "input";
    node.addEventListener(evt, scheduleAutosave);
    if (node.type === "range") {
      node.addEventListener("change", scheduleAutosave);
    }
  }

  el("refresh-voices").addEventListener("click", refreshElevenVoices);

  el("test").addEventListener("click", async () => {
    if (saveTimer) {
      clearTimeout(saveTimer);
      await autosave();
    }
    await invoke("test_speak", {
      text: "Hello. This is Claude speaking through the configured voice engine.",
    });
  });

  el("stop").addEventListener("click", () => invoke("stop_speaking"));

  el("copy-hook").addEventListener("click", async () => {
    await navigator.clipboard.writeText(el("hook-cmd").textContent);
    setStatus("Copied.");
  });

  const installHook = async (sourceBtn) => {
    sourceBtn.disabled = true;
    try {
      const result = await invoke("install_hook");
      setStatus(result);
      await refreshHookBanner();
    } catch (e) {
      setStatus(`Install failed: ${e}`, true);
    } finally {
      sourceBtn.disabled = false;
    }
  };
  el("install-hook").addEventListener("click", (e) => installHook(e.currentTarget));
  el("install-hook-banner").addEventListener("click", (e) => installHook(e.currentTarget));
});
