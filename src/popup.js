const { event, core, window: tauriWindow } = window.__TAURI__;
const popupWindow = tauriWindow.getCurrentWindow();
const { LogicalSize } = tauriWindow;

const SIZE_COLLAPSED = { w: 380, h: 360 };
const SIZE_EXPANDED = { w: 380, h: 720 };

const textEl = document.getElementById("text");
const titleEl = document.getElementById("title");
const dotEl = document.getElementById("dot");
const pauseBtn = document.getElementById("pause-btn");
const stopBtn = document.getElementById("stop-btn");
const pinBtn = document.getElementById("pin-btn");
const pauseIcon = document.getElementById("pause-icon");
const playIcon = document.getElementById("play-icon");
const linksEl = document.getElementById("links");
const historyBtn = document.getElementById("history-btn");
const historyPanel = document.getElementById("history-panel");
const historyList = document.getElementById("history-list");
const historyEmpty = document.getElementById("history-empty");
const historyClear = document.getElementById("history-clear");

const MAX_CHIPS = 5;
let historyOpen = false;

let paused = false;
let pinned = false;
let dismissDelayMs = 1500;
let dismissTimer = null;
let errorShowing = false;
let rafId = null;
let wordSpans = [];
let words = [];
let activeIndex = -1;
let startPerf = 0;
let pausedAt = 0;
let pauseAccum = 0;

let activeTitle = "Claude speaking";

function setPaused(p) {
  paused = p;
  pauseIcon.style.display = p ? "none" : "";
  playIcon.style.display = p ? "" : "none";
  titleEl.textContent = p ? "Paused" : activeTitle;
  dotEl.classList.toggle("paused", p);
  if (p) {
    pausedAt = performance.now();
  } else if (pausedAt) {
    pauseAccum += performance.now() - pausedAt;
    pausedAt = 0;
  }
}

function setPinned(p) {
  pinned = p;
  pinBtn.classList.toggle("active", p);
  pinBtn.title = p ? "Unpin popup" : "Pin popup";
}

function renderLinks(list) {
  linksEl.innerHTML = "";
  if (!list || list.length === 0) {
    linksEl.hidden = true;
    return;
  }
  linksEl.hidden = false;
  const shown = list.slice(0, MAX_CHIPS);
  for (const l of shown) {
    const chip = document.createElement("button");
    chip.type = "button";
    chip.className = "link-chip";
    chip.title = l.url;
    chip.textContent = l.label || l.url;
    chip.addEventListener("click", async () => {
      try {
        await core.invoke("plugin:opener|open_url", { url: l.url });
      } catch (e) {
        console.error("open_url failed", e);
      }
    });
    linksEl.appendChild(chip);
  }
  if (list.length > MAX_CHIPS) {
    const more = document.createElement("span");
    more.className = "link-more";
    more.textContent = `+${list.length - MAX_CHIPS} more`;
    linksEl.appendChild(more);
  }
}

function renderWords(text, list) {
  textEl.innerHTML = "";
  wordSpans = [];
  if (!list || list.length === 0) {
    textEl.textContent = text;
    return;
  }
  for (let i = 0; i < list.length; i++) {
    const span = document.createElement("span");
    span.className = "word";
    span.textContent = list[i].text;
    textEl.appendChild(span);
    wordSpans.push(span);
    if (i < list.length - 1) {
      textEl.appendChild(document.createTextNode(" "));
    }
  }
}

function tick() {
  if (paused) {
    rafId = requestAnimationFrame(tick);
    return;
  }
  const t = (performance.now() - startPerf - pauseAccum) / 1000;
  let next = activeIndex;
  for (let i = activeIndex < 0 ? 0 : activeIndex; i < words.length; i++) {
    if (t >= words[i].start && t <= words[i].end) {
      next = i;
      break;
    }
    if (t > words[i].end) {
      next = i;
    }
  }
  if (next !== activeIndex) {
    if (activeIndex >= 0 && wordSpans[activeIndex]) {
      wordSpans[activeIndex].classList.remove("active");
      wordSpans[activeIndex].classList.add("past");
    }
    if (next >= 0 && wordSpans[next]) {
      wordSpans[next].classList.add("active");
      wordSpans[next].classList.remove("past");
      const span = wordSpans[next];
      const parent = textEl;
      const sTop = span.offsetTop - parent.offsetTop;
      const sBot = sTop + span.offsetHeight;
      if (sBot > parent.scrollTop + parent.clientHeight - 8) {
        parent.scrollTop = sBot - parent.clientHeight + 8;
      } else if (sTop < parent.scrollTop) {
        parent.scrollTop = sTop;
      }
    }
    activeIndex = next;
  }
  if (activeIndex < words.length - 1 || t <= words[words.length - 1]?.end) {
    rafId = requestAnimationFrame(tick);
  } else {
    rafId = null;
  }
}

function startHighlight(text, list) {
  if (rafId) cancelAnimationFrame(rafId);
  words = list || [];
  activeIndex = -1;
  startPerf = performance.now();
  pauseAccum = 0;
  pausedAt = 0;
  renderWords(text, words);
  textEl.scrollTop = 0;
  if (words.length > 0) {
    rafId = requestAnimationFrame(tick);
  }
}

event.listen("voice:start", (e) => {
  const text = e.payload?.text ?? "";
  const list = e.payload?.words ?? [];
  const links = e.payload?.links ?? [];
  const session = e.payload?.session ?? null;
  errorShowing = false;
  if (dismissTimer) {
    clearTimeout(dismissTimer);
    dismissTimer = null;
  }
  document.body.classList.remove("fading-out");
  textEl.style.color = "";
  applySessionTheme(session);
  setPaused(false);
  startHighlight(text, list);
  renderLinks(links);
});

function applySessionTheme(session) {
  const root = document.documentElement;
  if (session && session.color) {
    root.style.setProperty("--session-color", session.color);
    root.style.setProperty("--session-color-glow", hexToRgba(session.color, 0.55));
    root.style.setProperty("--session-color-faint", hexToRgba(session.color, 0.32));
  } else {
    root.style.removeProperty("--session-color");
    root.style.removeProperty("--session-color-glow");
    root.style.removeProperty("--session-color-faint");
  }
  activeTitle = session && session.label ? session.label : "Claude speaking";
  titleEl.textContent = activeTitle;
}

function hexToRgba(hex, a) {
  const m = hex.replace("#", "");
  const r = parseInt(m.slice(0, 2), 16);
  const g = parseInt(m.slice(2, 4), 16);
  const b = parseInt(m.slice(4, 6), 16);
  return `rgba(${r}, ${g}, ${b}, ${a})`;
}

event.listen("voice:end", async () => {
  if (errorShowing) return;
  if (rafId) { cancelAnimationFrame(rafId); rafId = null; }
  if (pinned) {
    titleEl.textContent = "Done (pinned)";
    return;
  }
  if (dismissTimer) clearTimeout(dismissTimer);
  const FADE_MS = 350;
  dismissTimer = setTimeout(async () => {
    document.body.classList.add("fading-out");
    setTimeout(async () => {
      document.body.classList.remove("fading-out");
      textEl.textContent = "";
      wordSpans = [];
      words = [];
      renderLinks([]);
      setPaused(false);
      try { await popupWindow.hide(); } catch {}
      dismissTimer = null;
    }, FADE_MS);
  }, dismissDelayMs);
});

event.listen("voice:paused", () => setPaused(true));
event.listen("voice:resumed", () => setPaused(false));

event.listen("voice:error", async (e) => {
  const msg = e.payload?.message ?? "Unknown error";
  errorShowing = true;
  if (rafId) { cancelAnimationFrame(rafId); rafId = null; }
  titleEl.textContent = "Error";
  textEl.textContent = msg;
  textEl.style.color = "#ff8a8a";
  try { await popupWindow.show(); } catch {}
  setTimeout(async () => {
    errorShowing = false;
    textEl.textContent = "";
    textEl.style.color = "";
    titleEl.textContent = "Claude speaking";
    if (!pinned) {
      try { await popupWindow.hide(); } catch {}
    }
  }, 6000);
});

pauseBtn.addEventListener("click", () => core.invoke("toggle_pause"));
stopBtn.addEventListener("click", () => core.invoke("stop_speaking"));
pinBtn.addEventListener("click", () => core.invoke("toggle_pin_popup"));
historyBtn.addEventListener("click", () => toggleHistory());
historyClear.addEventListener("click", async () => {
  try {
    await core.invoke("clear_history");
    await loadHistory();
  } catch (e) {
    console.error("clear_history failed", e);
  }
});

function formatTime(ms) {
  const d = new Date(Number(ms));
  const diff = Date.now() - d.getTime();
  const min = Math.floor(diff / 60000);
  if (min < 1) return "just now";
  if (min < 60) return `${min}m ago`;
  const hr = Math.floor(min / 60);
  if (hr < 24) return `${hr}h ago`;
  const day = Math.floor(hr / 24);
  if (day < 7) return `${day}d ago`;
  return d.toLocaleDateString();
}

function renderHistoryEntry(entry) {
  const li = document.createElement("li");
  li.className = "history-entry";

  const row = document.createElement("div");
  row.className = "row";
  const time = document.createElement("span");
  time.className = "time";
  time.textContent = formatTime(entry.timestamp_ms);
  const replay = document.createElement("button");
  replay.className = "replay";
  replay.textContent = "Replay";
  replay.addEventListener("click", async () => {
    try {
      await core.invoke("replay_history", { id: entry.id });
    } catch (e) {
      console.error("replay_history failed", e);
    }
  });
  row.appendChild(time);
  row.appendChild(replay);

  const preview = document.createElement("div");
  preview.className = "preview";
  preview.textContent = (entry.spoken || entry.original || "").trim().replace(/\s+/g, " ");

  li.appendChild(row);
  li.appendChild(preview);
  return li;
}

async function loadHistory() {
  try {
    const entries = await core.invoke("get_history");
    historyList.innerHTML = "";
    if (!entries || entries.length === 0) {
      historyEmpty.hidden = false;
      return;
    }
    historyEmpty.hidden = true;
    for (const e of entries) {
      historyList.appendChild(renderHistoryEntry(e));
    }
  } catch (e) {
    console.error("get_history failed", e);
  }
}

async function toggleHistory(force) {
  const next = typeof force === "boolean" ? force : !historyOpen;
  historyOpen = next;
  historyPanel.hidden = !next;
  historyBtn.classList.toggle("active", next);
  if (next) await loadHistory();
}

event.listen("history:changed", () => {
  if (historyOpen) loadHistory();
});

event.listen("settings:changed", (e) => {
  applySettings(e.payload);
});

function applySettings(cfg) {
  if (!cfg) return;
  if (typeof cfg.pin_popup === "boolean") setPinned(cfg.pin_popup);
  if (typeof cfg.popup_dismiss_delay_ms === "number") {
    dismissDelayMs = cfg.popup_dismiss_delay_ms;
  }
}

(async () => {
  try {
    const cfg = await core.invoke("get_settings");
    applySettings(cfg);
  } catch {}
})();

document.addEventListener("keydown", (e) => {
  if (e.code === "Space") {
    e.preventDefault();
    core.invoke("toggle_pause");
  } else if (e.code === "Escape") {
    e.preventDefault();
    if (pinned) {
      popupWindow.hide().catch(() => {});
    } else {
      core.invoke("stop_speaking");
    }
  } else if (e.code === "KeyP") {
    core.invoke("toggle_pin_popup");
  }
});
