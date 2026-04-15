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

const el = (id) => document.getElementById(id);

let elevenVoices = [];

async function loadSettings() {
  const settings = await invoke("get_settings");
  const sayVoices = await invoke("list_voices");
  const hookCmd = await invoke("hook_command");

  el("enabled").checked = settings.enabled;
  el("show-popup").checked = settings.show_popup;
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
    [{ id: settings.elevenlabs_voice_id, name: settings.elevenlabs_voice_id }],
    settings.elevenlabs_voice_id,
  );

  el("summarize").checked = settings.summarize;
  el("anthropic-key").value = settings.anthropic_api_key;
  el("summary-model").value = settings.summary_model;
  el("summary-threshold").value = settings.summary_threshold_chars;
  el("threshold-value").textContent = settings.summary_threshold_chars;

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
    enabled: el("enabled").checked,
    port: parseInt(el("port").value, 10),
    show_popup: el("show-popup").checked,
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
  };
}

async function save() {
  try {
    await invoke("set_settings", { new: collect() });
    setStatus("Saved.");
    el("hook-cmd").textContent = await invoke("hook_command");
  } catch (e) {
    setStatus(`Error: ${e}`);
  }
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
    populateElevenVoices(voices, current);
    setStatus(`Loaded ${voices.length} voices.`);
  } catch (e) {
    setStatus(`Error: ${e}`, true);
  }
}

window.addEventListener("DOMContentLoaded", async () => {
  await loadSettings();

  el("backend").addEventListener("change", updateBackendVisibility);
  el("rate").addEventListener("input", (e) => {
    el("rate-value").textContent = e.target.value;
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
