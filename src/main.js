const { invoke } = window.__TAURI__.core;

const el = (id) => document.getElementById(id);

async function loadSettings() {
  const settings = await invoke("get_settings");
  const voices = await invoke("list_voices");
  const hookCmd = await invoke("hook_command");

  const voiceSel = el("voice");
  voiceSel.innerHTML = "";
  for (const v of voices) {
    const opt = document.createElement("option");
    opt.value = v;
    opt.textContent = v;
    if (v === settings.voice) opt.selected = true;
    voiceSel.appendChild(opt);
  }

  el("enabled").checked = settings.enabled;
  el("rate").value = settings.rate;
  el("rate-value").textContent = settings.rate;
  el("port").value = settings.port;
  el("hook-cmd").textContent = hookCmd;
}

async function save() {
  const settings = {
    enabled: el("enabled").checked,
    voice: el("voice").value,
    rate: parseInt(el("rate").value, 10),
    port: parseInt(el("port").value, 10),
  };
  try {
    await invoke("set_settings", { new: settings });
    setStatus("Saved. Restart app if port changed.");
    el("hook-cmd").textContent = await invoke("hook_command");
  } catch (e) {
    setStatus(`Error: ${e}`);
  }
}

function setStatus(msg) {
  el("status").textContent = msg;
  setTimeout(() => (el("status").textContent = ""), 3000);
}

window.addEventListener("DOMContentLoaded", async () => {
  await loadSettings();

  el("rate").addEventListener("input", (e) => {
    el("rate-value").textContent = e.target.value;
  });

  el("save").addEventListener("click", save);

  el("test").addEventListener("click", async () => {
    const voice = el("voice").value;
    const rate = parseInt(el("rate").value, 10);
    await invoke("set_settings", {
      new: {
        enabled: el("enabled").checked,
        voice,
        rate,
        port: parseInt(el("port").value, 10),
      },
    });
    await invoke("test_speak", {
      text: "Hello. This is Claude speaking through your Mac.",
    });
  });

  el("stop").addEventListener("click", () => invoke("stop_speaking"));

  el("copy-hook").addEventListener("click", async () => {
    await navigator.clipboard.writeText(el("hook-cmd").textContent);
    setStatus("Copied to clipboard.");
  });
});
