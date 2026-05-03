const { event, core, window: tauriWindow } = window.__TAURI__;
const popupWindow = tauriWindow.getCurrentWindow();
const { LogicalSize, LogicalPosition } = tauriWindow;

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
const codeBlocksEl = document.getElementById("code-blocks");
const historyBtn = document.getElementById("history-btn");
const historyPanel = document.getElementById("history-panel");
const historyList = document.getElementById("history-list");
const historyEmpty = document.getElementById("history-empty");
const historyClear = document.getElementById("history-clear");
const originalBtn = document.getElementById("original-btn");
const minimizeBtn = document.getElementById("minimize-btn");
const orb = document.getElementById("orb");
const orbCore = document.getElementById("orb-core");
const orbLabel = document.getElementById("orb-label");

// Orb-mode window size. The window is invisibly click-blocking everywhere
// inside its rect, so this should be just big enough to contain the halo
// glow and the peak ripple animation — see @keyframes orb-ripple in
// popup.css. Bumping the ripple `scale` ceiling means bumping this too.
const SIZE_ORB = { w: 140, h: 140 };

const MAX_CHIPS = 5;
let historyOpen = true;
let historyViewMode = "messages";
let historyQuery = "";
let cachedHistory = [];
let showingOriginal = false;
let minimized = false;
let expandedSize = { ...SIZE_COLLAPSED };
let currentSpokenText = "";
let currentOriginalText = "";
let currentSessionId = null;
let currentEntryId = null;
let selectedEntry = null;
const replayBtn = document.getElementById("replay-btn");

let paused = false;
let pinned = false;
let isPlaying = false;
let dismissDelayMs = 1500;
let dismissTimer = null;
let dismissFadeTimer = null;
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

function renderCodeBlocks(blocks) {
  codeBlocksEl.innerHTML = "";
  if (!blocks || blocks.length === 0) {
    codeBlocksEl.hidden = true;
    return;
  }
  codeBlocksEl.hidden = false;
  for (const b of blocks) {
    const wrap = document.createElement("div");
    wrap.className = "code-block";

    const head = document.createElement("div");
    head.className = "code-block-head";

    const lang = document.createElement("span");
    lang.className = "code-block-lang";
    lang.textContent = b.language || "code";

    const copy = document.createElement("button");
    copy.type = "button";
    copy.className = "code-block-copy";
    copy.textContent = "Copy";
    copy.addEventListener("click", async () => {
      try {
        await navigator.clipboard.writeText(b.code);
        copy.textContent = "Copied";
        copy.classList.add("copied");
        setTimeout(() => {
          copy.textContent = "Copy";
          copy.classList.remove("copied");
        }, 1200);
      } catch (e) {
        console.error("clipboard write failed", e);
      }
    });

    head.appendChild(lang);
    head.appendChild(copy);

    const body = document.createElement("pre");
    body.className = "code-block-body";
    const codeEl = document.createElement("code");
    codeEl.textContent = b.code;
    body.appendChild(codeEl);

    wrap.appendChild(head);
    wrap.appendChild(body);
    codeBlocksEl.appendChild(wrap);
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

event.listen("voice:waking", (e) => {
  const session = e.payload?.session ?? null;
  errorShowing = false;
  if (dismissTimer) {
    clearTimeout(dismissTimer);
    dismissTimer = null;
  }
  if (dismissFadeTimer) {
    clearTimeout(dismissFadeTimer);
    dismissFadeTimer = null;
  }
  document.body.classList.remove("fading-out", "speaking");
  document.body.classList.add("waking");
  applySessionTheme(session);
  currentSessionId = session && session.id ? session.id : null;
  if (!minimized) {
    titleEl.textContent = (session && session.label ? session.label : "Claude") + " — preparing…";
  }
});

event.listen("voice:start", (e) => {
  const text = e.payload?.text ?? "";
  const original = e.payload?.original ?? text;
  const list = e.payload?.words ?? [];
  const links = e.payload?.links ?? [];
  const session = e.payload?.session ?? null;
  errorShowing = false;
  isPlaying = true;
  if (dismissTimer) {
    clearTimeout(dismissTimer);
    dismissTimer = null;
  }
  if (dismissFadeTimer) {
    clearTimeout(dismissFadeTimer);
    dismissFadeTimer = null;
  }
  stopBrowserSpeech();
  document.body.classList.remove("fading-out", "waking");
  document.body.classList.add("speaking");
  textEl.style.color = "";
  applySessionTheme(session);
  setPaused(false);
  currentSpokenText = text;
  currentOriginalText = original;
  currentSessionId = session && session.id ? session.id : null;
  currentEntryId = e.payload?.id ?? null;
  selectedEntry = null;
  replayBtn.hidden = true;
  applyCurrentHighlight();
  setOriginalMode(false);

  const browserSpeech = e.payload?.browser_speech ?? false;
  if (browserSpeech) {
    const bVoice = e.payload?.browser_voice ?? "";
    const bRate = e.payload?.browser_rate ?? 1.0;
    startBrowserSpeech(text, bVoice, bRate);
  } else {
    startHighlight(text, list);
  }
  renderLinks(links);
  renderCodeBlocks(e.payload?.code_blocks ?? []);
});

function setOriginalMode(on) {
  showingOriginal = on;
  originalBtn.classList.toggle("active", on);
  originalBtn.title = on ? "Show spoken version (T)" : "Show full original (T)";
  if (on) {
    if (rafId) {
      cancelAnimationFrame(rafId);
      rafId = null;
    }
    textEl.classList.add("original-mode");
    textEl.textContent = currentOriginalText || currentSpokenText;
    textEl.scrollTop = 0;
  } else {
    textEl.classList.remove("original-mode");
    if (words && words.length > 0) {
      renderWords(currentSpokenText, words);
      activeIndex = -1;
      if (!rafId) rafId = requestAnimationFrame(tick);
    } else {
      textEl.textContent = currentSpokenText;
    }
  }
}

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
  orbLabel.textContent = session && session.label ? session.label : "Claude";
}

function hexToRgba(hex, a) {
  const m = hex.replace("#", "");
  const r = parseInt(m.slice(0, 2), 16);
  const g = parseInt(m.slice(2, 4), 16);
  const b = parseInt(m.slice(4, 6), 16);
  return `rgba(${r}, ${g}, ${b}, ${a})`;
}

event.listen("voice:end", async () => {
  isPlaying = false;
  // Reset local paused state eagerly. The Rust side already cleared its
  // paused flag on stop/end, so the UI shouldn't linger thinking we're
  // paused (otherwise the next play-button click tries to toggle_pause
  // on nothing instead of falling back to replay).
  setPaused(false);
  if (errorShowing) return;
  if (rafId) { cancelAnimationFrame(rafId); rafId = null; }
  document.body.classList.remove("speaking", "waking");
  if (minimized) {
    return;
  }
  if (pinned) {
    titleEl.textContent = "Done (pinned)";
    return;
  }
  // Two nested timers — outer waits the user-visible dismiss delay, inner
  // runs the fade animation. Both must be cancellable from voice:start so
  // a queued next message doesn't race with the hide.
  if (dismissTimer) clearTimeout(dismissTimer);
  if (dismissFadeTimer) clearTimeout(dismissFadeTimer);
  const FADE_MS = 350;
  dismissTimer = setTimeout(async () => {
    dismissTimer = null;
    document.body.classList.add("fading-out");
    dismissFadeTimer = setTimeout(async () => {
      dismissFadeTimer = null;
      document.body.classList.remove("fading-out");
      textEl.textContent = "";
      wordSpans = [];
      words = [];
      renderLinks([]);
      renderCodeBlocks([]);
      setPaused(false);
      try { await popupWindow.hide(); } catch {}
    }, FADE_MS);
  }, dismissDelayMs);
});

event.listen("voice:paused", () => setPaused(true));
event.listen("voice:resumed", () => setPaused(false));

event.listen("voice:error", async (e) => {
  const msg = e.payload?.message ?? "Unknown error";
  errorShowing = true;
  document.body.classList.remove("speaking", "waking");
  if (rafId) { cancelAnimationFrame(rafId); rafId = null; }
  titleEl.textContent = "Error";
  textEl.textContent = msg;
  textEl.style.color = "#ff8a8a";
  try { await popupWindow.show(); } catch {}
  setTimeout(async () => {
    if (!errorShowing) return;
    errorShowing = false;
    textEl.textContent = "";
    textEl.style.color = "";
    titleEl.textContent = activeTitle || "Claude speaking";
    if (!pinned) {
      try { await popupWindow.hide(); } catch {}
    }
  }, 6000);
});

function handlePauseResume() {
  if (browserSpeechActive) {
    if (speechSynthesis.paused) {
      speechSynthesis.resume();
      setPaused(false);
    } else {
      speechSynthesis.pause();
      setPaused(true);
    }
    return;
  }
  if (isPlaying || paused) {
    // Active playback: toggle pause/resume at the TTS engine.
    core.invoke("toggle_pause");
    return;
  }
  // Nothing is playing — the process ended (naturally or otherwise) while
  // the user wasn't looking. Replay the currently-highlighted message so
  // the play button always does something useful.
  if (currentEntryId) {
    core.invoke("replay_history", { id: currentEntryId });
  }
}

pauseBtn.addEventListener("click", handlePauseResume);
stopBtn.addEventListener("click", () => {
  stopBrowserSpeech();
  core.invoke("stop_speaking");
});
pinBtn.addEventListener("click", () => core.invoke("toggle_pin_popup"));
historyBtn.addEventListener("click", () => toggleHistory());
historyBtn.classList.add("active");
loadHistory();
originalBtn.addEventListener("click", () => setOriginalMode(!showingOriginal));

const copyBtn = document.getElementById("copy-btn");
async function copyCurrentMessage() {
  const text = showingOriginal
    ? currentOriginalText || currentSpokenText
    : currentSpokenText || currentOriginalText;
  if (!text) return;
  try {
    await navigator.clipboard.writeText(text);
    copyBtn.classList.add("copied");
    setTimeout(() => copyBtn.classList.remove("copied"), 900);
  } catch (e) {
    console.error("clipboard write failed", e);
  }
}
copyBtn.addEventListener("click", copyCurrentMessage);

const replyInput = document.getElementById("reply-input");
const replySend = document.getElementById("reply-send");
const replyStatus = document.getElementById("reply-status");
let replyStatusTimer = null;

function setReplyStatus(msg, kind) {
  replyStatus.textContent = msg;
  replyStatus.className = "reply-status" + (kind ? ` ${kind}` : "");
  if (replyStatusTimer) clearTimeout(replyStatusTimer);
  if (msg) {
    replyStatusTimer = setTimeout(() => {
      replyStatus.textContent = "";
      replyStatus.className = "reply-status";
    }, 3000);
  }
}

replyInput.addEventListener("focus", async () => {
  try { await popupWindow.setFocus(); } catch {}
});

replyInput.addEventListener("input", () => {
  replyInput.style.height = "auto";
  replyInput.style.height = Math.min(replyInput.scrollHeight, 100) + "px";
});

async function sendReply() {
  const text = replyInput.value.trim();
  if (!text) return;
  replySend.disabled = true;
  setReplyStatus("Sending…");
  try {
    const target = await core.invoke("send_to_terminal", {
      text,
      sessionId: currentSessionId,
    });
    setReplyStatus(`Sent to ${target}`, "ok");
    replyInput.value = "";
    replyInput.style.height = "auto";
  } catch (e) {
    setReplyStatus(String(e), "err");
  } finally {
    replySend.disabled = false;
  }
}

replySend.addEventListener("click", sendReply);

replyInput.addEventListener("keydown", (e) => {
  if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) {
    e.preventDefault();
    sendReply();
  } else if (e.key === "Escape") {
    e.preventDefault();
    e.stopPropagation();
    replyInput.blur();
  }
});

const micBtn = document.getElementById("reply-mic");
let mediaRecorder = null;
let recordedChunks = [];
let recording = false;
let activeStream = null;

async function toggleRecord() {
  if (recording) {
    try { mediaRecorder?.stop(); } catch {}
    return;
  }
  let stream;
  try {
    stream = await navigator.mediaDevices.getUserMedia({ audio: true });
  } catch (e) {
    setReplyStatus(`Mic error: ${e.message || e}`, "err");
    return;
  }
  activeStream = stream;
  const mime = pickMime();
  if (!mime) {
    setReplyStatus("No supported audio recorder format", "err");
    stream.getTracks().forEach((t) => t.stop());
    return;
  }
  recordedChunks = [];
  mediaRecorder = new MediaRecorder(stream, { mimeType: mime });
  mediaRecorder.ondataavailable = (e) => {
    if (e.data && e.data.size > 0) recordedChunks.push(e.data);
  };
  mediaRecorder.onstop = async () => {
    try {
      activeStream?.getTracks().forEach((t) => t.stop());
    } catch {}
    activeStream = null;
    recording = false;
    micBtn.classList.remove("recording");
    micBtn.classList.add("uploading");
    setReplyStatus("Transcribing…");
    const blob = new Blob(recordedChunks, { type: mime });
    if (blob.size < 800) {
      micBtn.classList.remove("uploading");
      setReplyStatus("Recording too short", "err");
      return;
    }
    try {
      const buf = await blob.arrayBuffer();
      const text = await core.invoke("transcribe_audio", {
        audio: Array.from(new Uint8Array(buf)),
        mime,
      });
      if (text) {
        const sep = replyInput.value && !replyInput.value.endsWith(" ") ? " " : "";
        replyInput.value = replyInput.value + sep + text;
        replyInput.dispatchEvent(new Event("input"));
        await popupWindow.setFocus().catch(() => {});
        replyInput.focus();
        setReplyStatus("Transcribed", "ok");
      } else {
        setReplyStatus("No speech detected", "err");
      }
    } catch (e) {
      setReplyStatus(String(e), "err");
    } finally {
      micBtn.classList.remove("uploading");
    }
  };
  mediaRecorder.start();
  recording = true;
  micBtn.classList.add("recording");
  setReplyStatus("Recording… click mic to stop");
}

function pickMime() {
  const candidates = [
    "audio/webm;codecs=opus",
    "audio/webm",
    "audio/mp4",
    "audio/ogg;codecs=opus",
    "audio/ogg",
  ];
  for (const m of candidates) {
    if (window.MediaRecorder && MediaRecorder.isTypeSupported(m)) return m;
  }
  return null;
}

micBtn.addEventListener("click", toggleRecord);

let browserSpeechActive = false;
let browserUtterance = null;
let browserWordMap = [];

function buildWordMap(text) {
  const map = [];
  let pos = 0;
  for (const w of text.split(/(\s+)/)) {
    if (w.trim()) {
      map.push({ text: w, start: pos, end: pos + w.length });
    }
    pos += w.length;
  }
  return map;
}

function startBrowserSpeech(text, voiceName, rate) {
  stopBrowserSpeech();
  browserSpeechActive = true;

  browserWordMap = buildWordMap(text);
  renderWords(
    text,
    browserWordMap.map((w) => ({ text: w.text, start: 0, end: 0 })),
  );

  const utt = new SpeechSynthesisUtterance(text);
  utt.rate = rate || 1.0;

  if (voiceName) {
    const voices = speechSynthesis.getVoices();
    const match = voices.find((v) => v.name === voiceName);
    if (match) utt.voice = match;
  }

  utt.onboundary = (ev) => {
    if (ev.name !== "word") return;
    const ci = ev.charIndex;
    for (let i = 0; i < browserWordMap.length; i++) {
      if (ci >= browserWordMap[i].start && ci < browserWordMap[i].end) {
        if (i !== activeIndex) {
          if (activeIndex >= 0 && wordSpans[activeIndex]) {
            wordSpans[activeIndex].classList.remove("active");
            wordSpans[activeIndex].classList.add("past");
          }
          if (wordSpans[i]) {
            wordSpans[i].classList.add("active");
            wordSpans[i].classList.remove("past");
            const span = wordSpans[i];
            const parent = textEl;
            const sTop = span.offsetTop - parent.offsetTop;
            const sBot = sTop + span.offsetHeight;
            if (sBot > parent.scrollTop + parent.clientHeight - 8) {
              parent.scrollTop = sBot - parent.clientHeight + 8;
            } else if (sTop < parent.scrollTop) {
              parent.scrollTop = sTop;
            }
          }
          activeIndex = i;
        }
        break;
      }
    }
  };

  utt.onend = () => {
    browserSpeechActive = false;
    browserUtterance = null;
    document.body.classList.remove("speaking");
    if (minimized) return;
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
      renderCodeBlocks([]);
        setPaused(false);
        try { await popupWindow.hide(); } catch {}
        dismissTimer = null;
      }, FADE_MS);
    }, dismissDelayMs);
  };

  utt.onerror = () => {
    browserSpeechActive = false;
    browserUtterance = null;
    document.body.classList.remove("speaking");
  };

  browserUtterance = utt;
  speechSynthesis.speak(utt);
}

function stopBrowserSpeech() {
  if (browserSpeechActive) {
    speechSynthesis.cancel();
    browserSpeechActive = false;
    browserUtterance = null;
  }
}

function quadrant(cx, cy, monitor, scale) {
  if (!monitor) return { onRightHalf: true, onBottomHalf: false };
  const monX = monitor.position.x / scale;
  const monY = monitor.position.y / scale;
  const monW = monitor.size.width / scale;
  const monH = monitor.size.height / scale;
  return {
    onRightHalf: cx > monX + monW / 2,
    onBottomHalf: cy > monY + monH / 2,
  };
}

function applyAnchorAttr(onRightHalf, onBottomHalf) {
  document.body.dataset.anchor = `${onBottomHalf ? "bottom" : "top"}-${onRightHalf ? "right" : "left"}`;
}

async function refreshAnchor() {
  try {
    const scale = await popupWindow.scaleFactor();
    const pos = await popupWindow.outerPosition();
    const sz = await popupWindow.innerSize();
    const monitor = await tauriWindow.currentMonitor();
    const cx = pos.x / scale + sz.width / scale / 2;
    const cy = pos.y / scale + sz.height / scale / 2;
    const { onRightHalf, onBottomHalf } = quadrant(cx, cy, monitor, scale);
    applyAnchorAttr(onRightHalf, onBottomHalf);
  } catch (e) {}
}

async function setMinimized(on) {
  if (on === minimized) return;
  minimized = on;
  document.body.classList.toggle("minimized", on);
  try {
    const scale = await popupWindow.scaleFactor();
    const pos = await popupWindow.outerPosition();
    const sz = await popupWindow.innerSize();
    // currentMonitor is a top-level function on the window module, NOT a
    // method on the Window instance. The instance has no such method, so
    // calling popupWindow.currentMonitor() silently throws and aborts the
    // resize/reposition logic that follows.
    const monitor = await tauriWindow.currentMonitor();
    const curX = pos.x / scale;
    const curY = pos.y / scale;
    const curW = sz.width / scale;
    const curH = sz.height / scale;
    const { onRightHalf, onBottomHalf } = quadrant(
      curX + curW / 2,
      curY + curH / 2,
      monitor,
      scale,
    );

    let newW;
    let newH;
    if (on) {
      expandedSize = { w: curW, h: curH };
      newW = SIZE_ORB.w;
      newH = SIZE_ORB.h;
    } else {
      newW = expandedSize.w || SIZE_COLLAPSED.w;
      newH = expandedSize.h || SIZE_COLLAPSED.h;
    }

    let nx = onRightHalf ? curX + curW - newW : curX;
    let ny = onBottomHalf ? curY + curH - newH : curY;
    if (monitor) {
      const monX = monitor.position.x / scale;
      const monY = monitor.position.y / scale;
      const monW = monitor.size.width / scale;
      const monH = monitor.size.height / scale;
      nx = Math.max(monX, Math.min(nx, monX + monW - newW));
      ny = Math.max(monY, Math.min(ny, monY + monH - newH));
    }

    // Lower the OS-level min size BEFORE shrinking, otherwise wry/macOS
    // silently clamps setSize to the previous min and the window stays
    // (and continues to block clicks at) its old dimensions. When
    // expanding back we relax the min after the resize so the user can
    // still drag-resize the expanded popup down to a sane minimum.
    if (on) {
      try {
        await popupWindow.setMinSize(new LogicalSize(newW, newH));
      } catch (e) {}
    }
    await popupWindow.setSize(new LogicalSize(newW, newH));
    await popupWindow.setPosition(new LogicalPosition(nx, ny));
    if (!on) {
      try {
        await popupWindow.setMinSize(new LogicalSize(240, 240));
      } catch (e) {}
    }
    applyAnchorAttr(onRightHalf, onBottomHalf);
  } catch (e) {
    console.error("setSize failed", e);
  }
}

refreshAnchor();
popupWindow.onMoved(() => refreshAnchor());
popupWindow.onResized(() => refreshAnchor());

minimizeBtn.addEventListener("click", () => setMinimized(true));

let orbPressedAt = 0;
orbCore.addEventListener("mousedown", () => {
  orbPressedAt = performance.now();
});
orbCore.addEventListener("mouseup", () => {
  const elapsed = performance.now() - orbPressedAt;
  if (elapsed > 0 && elapsed < 180) {
    setMinimized(false);
  }
});
orbCore.addEventListener("dblclick", () => setMinimized(false));
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
  li.dataset.entryId = entry.id;
  if (entry.id === currentEntryId) li.classList.add("selected");
  if (entry.pending) li.classList.add("pending");
  li.style.cursor = "pointer";
  li.addEventListener("click", (e) => {
    if (e.target.closest(".delete-entry")) return;
    if (entry.pending) return; // queued — not playable yet
    selectHistoryEntry(entry, li);
  });

  const row = document.createElement("div");
  row.className = "row";
  const time = document.createElement("span");
  time.className = "time";
  time.textContent = entry.pending ? "Queued…" : formatTime(entry.timestamp_ms);
  const del = document.createElement("button");
  del.type = "button";
  del.className = "delete-entry";
  del.title = "Delete";
  del.textContent = "\u00D7";
  del.addEventListener("click", async (e) => {
    e.stopPropagation();
    try {
      await core.invoke("delete_history_entry", { id: entry.id });
      if (selectedEntry && selectedEntry.id === entry.id) {
        selectedEntry = null;
        replayBtn.hidden = true;
        textEl.textContent = "";
        renderLinks([]);
      renderCodeBlocks([]);
      }
    } catch (err) {
      console.error("delete_history_entry failed", err);
    }
  });
  row.appendChild(time);
  row.appendChild(del);

  const preview = document.createElement("div");
  preview.className = "preview";
  preview.textContent = (entry.spoken || entry.original || "").trim().replace(/\s+/g, " ");

  li.appendChild(row);
  li.appendChild(preview);
  return li;
}

function selectHistoryEntry(entry, el) {
  selectedEntry = entry;
  currentEntryId = entry.id;
  currentSpokenText = entry.spoken || "";
  currentOriginalText = entry.original || "";
  currentSessionId = entry.session?.id || null;
  applySessionTheme(entry.session || null);
  setOriginalMode(false);
  textEl.textContent = currentSpokenText;
  renderLinks(entry.links || []);
  renderCodeBlocks(entry.code_blocks || []);
  replayBtn.hidden = false;
  applyCurrentHighlight();
}

function applyCurrentHighlight() {
  document.querySelectorAll(".history-entry.selected").forEach(
    (el) => el.classList.remove("selected"),
  );
  if (!currentEntryId) return;
  const el = document.querySelector(
    `.history-entry[data-entry-id="${CSS.escape(currentEntryId)}"]`,
  );
  if (el) el.classList.add("selected");
}

replayBtn.addEventListener("click", async () => {
  if (!selectedEntry) return;
  try {
    await core.invoke("replay_history", { id: selectedEntry.id });
  } catch (e) {
    console.error("replay_history failed", e);
  }
});

function matchesQuery(entry, q) {
  if (!q) return true;
  const hay = (
    (entry.spoken || "") + "\n" +
    (entry.original || "") + "\n" +
    (entry.session?.label || "")
  ).toLowerCase();
  return hay.includes(q);
}

async function loadHistory() {
  try {
    cachedHistory = (await core.invoke("get_history")) || [];
    renderHistory();
  } catch (e) {
    console.error("get_history failed", e);
  }
}

// Popup windows are pre-rendered hidden on app start; the first invoke
// can race against Rust-side state readiness and return empty. Three
// safeguards: a one-time retry, a focus listener, and a visibility-change
// listener (the popup often appears without gaining focus when auto-shown
// on a new message).
popupWindow.onFocusChanged(({ payload: focused }) => {
  if (focused && historyOpen) loadHistory();
});

document.addEventListener("visibilitychange", () => {
  if (document.visibilityState === "visible" && historyOpen) loadHistory();
});

setTimeout(() => {
  if (historyOpen && cachedHistory.length === 0) loadHistory();
}, 1000);

function renderHistory() {
  historyList.innerHTML = "";
  const q = historyQuery.trim().toLowerCase();
  const filtered = q
    ? cachedHistory.filter((e) => matchesQuery(e, q))
    : cachedHistory;
  if (filtered.length === 0) {
    historyEmpty.hidden = false;
    historyEmpty.textContent = q ? "No matches." : "No messages yet.";
    return;
  }
  historyEmpty.hidden = true;
  if (historyViewMode === "conversations") {
    renderConversationView(filtered);
  } else {
    for (const e of filtered) {
      historyList.appendChild(renderHistoryEntry(e));
    }
  }
}

const historySearchInput = document.getElementById("history-search");
historySearchInput.addEventListener("input", () => {
  historyQuery = historySearchInput.value;
  renderHistory();
});
historySearchInput.addEventListener("keydown", (e) => {
  if (e.key === "Escape" && historySearchInput.value) {
    e.preventDefault();
    e.stopPropagation();
    historySearchInput.value = "";
    historyQuery = "";
    renderHistory();
  }
});

function groupBySession(entries) {
  const groups = new Map();
  for (const e of entries) {
    const sid = e.session?.id || "_none";
    if (!groups.has(sid)) {
      groups.set(sid, { session: e.session, entries: [] });
    }
    groups.get(sid).entries.push(e);
  }
  return Array.from(groups.values()).sort(
    (a, b) => Number(b.entries[0].timestamp_ms) - Number(a.entries[0].timestamp_ms),
  );
}

function renderConversationView(entries) {
  const groups = groupBySession(entries);
  for (const g of groups) {
    const li = document.createElement("li");
    li.className = "history-group";

    const header = document.createElement("div");
    header.className = "history-group-header";
    header.addEventListener("click", () => li.classList.toggle("collapsed"));

    const dot = document.createElement("span");
    dot.className = "group-dot";
    dot.style.background = g.session?.color || "#888";

    const label = document.createElement("span");
    label.className = "group-label";
    label.textContent = g.session?.label || "Unknown";

    const count = document.createElement("span");
    count.className = "group-count";
    count.textContent = `${g.entries.length}`;

    const chevron = document.createElement("span");
    chevron.className = "group-chevron";
    chevron.textContent = "\u25BE";

    header.appendChild(dot);
    header.appendChild(label);
    header.appendChild(count);
    header.appendChild(chevron);

    const messages = document.createElement("ul");
    messages.className = "history-group-messages";
    for (const e of g.entries) {
      messages.appendChild(renderHistoryEntry(e));
    }

    li.appendChild(header);
    li.appendChild(messages);
    historyList.appendChild(li);
  }
}

const viewToggleBtn = document.getElementById("history-view-toggle");
const viewIconMessages = document.getElementById("view-icon-messages");
const viewIconConversations = document.getElementById("view-icon-conversations");

function updateViewToggleIcon() {
  // Show the icon of the *other* mode — the one the button would switch to.
  viewIconMessages.style.display = historyViewMode === "messages" ? "none" : "";
  viewIconConversations.style.display =
    historyViewMode === "conversations" ? "none" : "";
}

viewToggleBtn.addEventListener("click", () => {
  historyViewMode = historyViewMode === "messages" ? "conversations" : "messages";
  updateViewToggleIcon();
  loadHistory();
});
updateViewToggleIcon();

async function toggleHistory(force) {
  const next = typeof force === "boolean" ? force : !historyOpen;
  historyOpen = next;
  historyPanel.hidden = !next;
  const resizerEl = document.getElementById("history-resizer");
  if (resizerEl) resizerEl.hidden = !next;
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
  if (typeof cfg.orb_style === "string") {
    document.body.dataset.orbStyle = cfg.orb_style;
  }
  if (typeof cfg.history_panel_width === "number" && cfg.history_panel_width > 0) {
    applyHistoryPanelWidth(cfg.history_panel_width);
  }
}

function applyHistoryPanelWidth(w) {
  const clamped = Math.max(120, Math.min(360, w));
  const panel = document.getElementById("history-panel");
  panel.style.flex = `0 0 ${clamped}px`;
  panel.style.width = `${clamped}px`;
}

const resizer = document.getElementById("history-resizer");
let dragState = null;
resizer.addEventListener("mousedown", (e) => {
  const panel = document.getElementById("history-panel");
  dragState = { startX: e.screenX, startWidth: panel.offsetWidth };
  resizer.classList.add("dragging");
  document.body.style.cursor = "col-resize";
  e.preventDefault();
});
document.addEventListener("mousemove", (e) => {
  if (!dragState) return;
  const delta = e.screenX - dragState.startX;
  applyHistoryPanelWidth(dragState.startWidth + delta);
});
document.addEventListener("mouseup", async () => {
  if (!dragState) return;
  dragState = null;
  resizer.classList.remove("dragging");
  document.body.style.cursor = "";
  const panel = document.getElementById("history-panel");
  try {
    await core.invoke("set_history_panel_width", { width: panel.offsetWidth });
  } catch (e) {
    console.error("set_history_panel_width failed", e);
  }
});

(async () => {
  try {
    const cfg = await core.invoke("get_settings");
    applySettings(cfg);
  } catch {}
})();

document.addEventListener("keydown", (e) => {
  const inInput =
    e.target instanceof HTMLTextAreaElement ||
    e.target instanceof HTMLInputElement;
  if (inInput && e.code !== "Escape") return;
  if (e.code === "Space") {
    e.preventDefault();
    handlePauseResume();
  } else if (e.code === "Escape") {
    e.preventDefault();
    if (pinned) {
      popupWindow.hide().catch(() => {});
    } else {
      core.invoke("stop_speaking");
    }
  } else if (e.code === "KeyP") {
    core.invoke("toggle_pin_popup");
  } else if (e.code === "KeyT") {
    e.preventDefault();
    setOriginalMode(!showingOriginal);
  } else if (e.code === "KeyM") {
    e.preventDefault();
    setMinimized(!minimized);
  } else if (e.code === "KeyC" && !e.metaKey && !e.ctrlKey) {
    e.preventDefault();
    copyCurrentMessage();
  }
});
