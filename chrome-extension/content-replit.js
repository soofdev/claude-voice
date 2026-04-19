// Claude Voice Bridge — Replit adapter
//
// Replit wraps every agent response in a `.rendered-markdown` element, and
// user messages sit inside a `data-event-type="user-message"` wrapper (plus
// a `userMessage` class). Those stable anchors let us avoid generic "prose"
// heuristics that would otherwise pick up input placeholders, toolbars,
// etc. We watch for mutations inside any agent `.rendered-markdown`, wait
// for streaming to settle, then read its innerText and send it to Claude
// Voice.

const CONFIG = {
  debug: false,
  // Wait this long after the last agent-message mutation before sending.
  // Long enough to cover normal pauses between streamed chunks, short
  // enough that replies feel prompt once the agent actually stops.
  settleMs: 3000,
  // Minimum characters to bother sending.
  minChars: 8,
  // Don't fire two turn-ends within this window (defensive — shouldn't
  // happen with the settle-debounce model, but cheap insurance).
  cooldownMs: 4000,
  // Absorb any historical agent messages that hydrate while warming up.
  warmupIdleMs: 3000,
  warmupMaxMs: 60000,
  // How often to ask Claude Voice for queued replies for this repl.
  replyPollMs: 1500,
  // finalOnly: safety cap — don't postpone forever if "Working" stays
  // visible for pathological reasons. Fire after this many postpones.
  maxPostponedMs: 90000,
};

function log(...args) {
  if (!CONFIG.debug) return;
  console.log("[claude-voice-bridge]", ...args);
}

function deriveReplSlug() {
  // Newer Replit URLs: /@user/project-name
  const m1 = location.pathname.match(/\/@[^/]+\/([^/?#]+)/);
  if (m1) return m1[1];
  // Legacy: /repls/<id>
  const m2 = location.pathname.match(/\/repls\/([^/]+)/i);
  if (m2) return m2[1];
  return location.pathname.replace(/^\/+|\/+$/g, "") || "root";
}

function deriveSessionId() {
  return `replit:${deriveReplSlug()}`;
}

function deriveSessionLabel() {
  // Project name straight from Replit's header button (stable test id,
  // unlike the hashed CSS-module classes on everything else).
  const headerBtn = document.querySelector(
    '[data-cy="repl-info-header-settings"]',
  );
  if (headerBtn) {
    const name = (headerBtn.innerText || "").trim();
    if (name) return name;
  }
  // URL fallback: turn "Super-Demo-Builder" into "Super Demo Builder".
  const slug = deriveReplSlug();
  try {
    return decodeURIComponent(slug).replace(/[-_]+/g, " ");
  } catch {
    return slug;
  }
}

let enabled = true;
let finalOnly = false;
let seenResponses = new WeakSet();
let settleTimer = null;
let lastFiredAt = 0;
let firstPostponedAt = 0;
let warmingUp = true;
let warmupIdleTimer = null;

function scheduleWarmupEnd() {
  if (!warmingUp) return;
  if (warmupIdleTimer) clearTimeout(warmupIdleTimer);
  warmupIdleTimer = setTimeout(() => {
    if (!warmingUp) return;
    warmingUp = false;
    log("warmup ended — live from here on");
  }, CONFIG.warmupIdleMs);
}

chrome.storage.sync
  .get({ enabled: true, debug: false, finalOnly: false })
  .then(({ enabled: e, debug, finalOnly: f }) => {
    enabled = e;
    CONFIG.debug = debug;
    finalOnly = f;
    log("boot", { enabled, debug, finalOnly, sessionId: deriveSessionId() });
  });

chrome.storage.onChanged.addListener((changes) => {
  if (changes.enabled) enabled = changes.enabled.newValue;
  if (changes.debug) CONFIG.debug = changes.debug.newValue;
  if (changes.finalOnly) finalOnly = changes.finalOnly.newValue;
});

// ---------------------------------------------------------------------------
// Agent-message detection

function isInUserMessage(el) {
  let cur = el;
  while (cur && cur !== document.body) {
    if (
      cur.getAttribute &&
      cur.getAttribute("data-event-type") === "user-message"
    ) {
      return true;
    }
    const cls = typeof cur.className === "string" ? cur.className : "";
    if (/userMessage/i.test(cls)) return true;
    cur = cur.parentElement;
  }
  return false;
}

function isAgentResponseRoot(el) {
  return (
    el.classList &&
    el.classList.contains("rendered-markdown") &&
    !isInUserMessage(el)
  );
}

function findAncestorAgentResponse(el) {
  let cur = el;
  while (cur && cur !== document.body) {
    if (cur.classList && cur.classList.contains("rendered-markdown")) {
      return isInUserMessage(cur) ? null : cur;
    }
    cur = cur.parentElement;
  }
  return null;
}

function findAllAgentResponses() {
  return Array.from(document.querySelectorAll(".rendered-markdown")).filter(
    (el) => !isInUserMessage(el),
  );
}

function isAgentActivelyWorking() {
  // Replit shows a short "Working" / "Working..." / "Running ..." label
  // (often with a canvas spinner) inside a tool-use button while the agent
  // is mid-step. Completed steps switch to counts like "4 actions" or
  // descriptors like "Ran grep". No live label = settled.
  const spans = document.querySelectorAll(
    ".ToolUseGroup-module__sZ38ga__toolUseGroupButton span, .ToolUse-module__ZltkIG__toolUse span, span",
  );
  for (const span of spans) {
    const text = (span.innerText || "").trim();
    if (!text || text.length > 20) continue;
    if (/^working\.{0,3}$/i.test(text)) return true;
    if (/^running\s+\S/i.test(text) && text.endsWith("...")) return true;
    if (/^thinking\.{0,3}$/i.test(text)) return true;
  }
  return false;
}

// ---------------------------------------------------------------------------
// Fire logic

async function fireUnseenResponses() {
  settleTimer = null;
  if (!enabled) return;

  // "Only read final message" mode: skip intermediate thought-process
  // messages. Postpone firing while the agent is still mid-step; once
  // working stops we'll speak just the most recent agent response. Cap
  // the postponement so we never get stuck if the indicator stalls.
  if (finalOnly && isAgentActivelyWorking()) {
    const postponedFor = firstPostponedAt ? Date.now() - firstPostponedAt : 0;
    if (postponedFor < CONFIG.maxPostponedMs) {
      if (!firstPostponedAt) firstPostponedAt = Date.now();
      log("finalOnly: agent still working; postponing fire", { postponedFor });
      scheduleSettle();
      return;
    }
    log("finalOnly: postpone cap reached — firing anyway");
  }
  firstPostponedAt = 0;

  const now = Date.now();
  if (now - lastFiredAt < CONFIG.cooldownMs) {
    log("debounce within cooldown; skipping", {
      sinceLast: now - lastFiredAt,
    });
    return;
  }

  const all = findAllAgentResponses();
  const candidates = all.filter(
    (el) => el.isConnected && !seenResponses.has(el),
  );
  log("fire check", {
    total: all.length,
    unseen: candidates.length,
    finalOnly,
    working: isAgentActivelyWorking(),
  });
  if (candidates.length === 0) return;

  // In finalOnly mode, mark every intermediate unseen response as seen
  // (so they're never spoken) and fire only the last one — that's the
  // turn's conclusion.
  const toFire = finalOnly ? [candidates[candidates.length - 1]] : candidates;
  for (const el of candidates) seenResponses.add(el);

  const parts = [];
  for (const el of toFire) {
    const text = (el.innerText || "").trim();
    if (text.length >= CONFIG.minChars) parts.push(text);
  }

  const combined = parts.join("\n\n").trim();
  if (!combined) return;
  lastFiredAt = now;

  const payload = {
    sessionId: deriveSessionId(),
    sessionLabel: deriveSessionLabel(),
    cwd: deriveSessionLabel(),
    text: combined,
  };
  log("firing agent response", {
    chars: combined.length,
    count: candidates.length,
  });
  try {
    const resp = await chrome.runtime.sendMessage({
      type: "bridge:speak",
      payload,
    });
    log("server response", resp);
  } catch (e) {
    log("sendMessage failed", e);
  }
}

function scheduleSettle() {
  if (settleTimer) clearTimeout(settleTimer);
  settleTimer = setTimeout(fireUnseenResponses, CONFIG.settleMs);
}

// ---------------------------------------------------------------------------
// DOM observation

const observer = new MutationObserver((mutations) => {
  let agentActivity = false;

  for (const m of mutations) {
    // New nodes added — either a fresh agent message, or a child appended
    // inside an existing one as content streams in.
    for (const n of m.addedNodes) {
      if (n.nodeType !== Node.ELEMENT_NODE) continue;
      if (isAgentResponseRoot(n)) {
        agentActivity = true;
        break;
      }
      if (findAncestorAgentResponse(n)) {
        agentActivity = true;
        break;
      }
      // A wrapper added that contains a new agent message inside it.
      if (n.querySelector && n.querySelector(".rendered-markdown")) {
        const found = Array.from(n.querySelectorAll(".rendered-markdown")).some(
          (el) => !isInUserMessage(el),
        );
        if (found) {
          agentActivity = true;
          break;
        }
      }
    }
    if (agentActivity) break;

    // Character-level changes inside an existing agent message.
    if (m.target && m.target.nodeType !== undefined) {
      const target =
        m.target.nodeType === Node.ELEMENT_NODE ? m.target : m.target.parentElement;
      if (target && findAncestorAgentResponse(target)) {
        agentActivity = true;
      }
    }
  }

  if (!agentActivity) return;

  if (warmingUp) {
    // Historical hydration — absorb current agent messages silently and
    // keep the quiet deadline extended.
    for (const el of findAllAgentResponses()) seenResponses.add(el);
    scheduleWarmupEnd();
    return;
  }

  scheduleSettle();
});

function start() {
  // Anything already in the DOM is historical — mark it seen so we don't
  // replay it.
  for (const el of findAllAgentResponses()) seenResponses.add(el);

  scheduleWarmupEnd();
  setTimeout(() => {
    if (warmingUp) {
      warmingUp = false;
      log("warmup hard cap reached — going live");
    }
  }, CONFIG.warmupMaxMs);

  observer.observe(document.body, {
    childList: true,
    subtree: true,
    characterData: true,
  });
  log("started; warming up", {
    existingResponses: findAllAgentResponses().length,
  });

  startReplyPoller();
}

// ---------------------------------------------------------------------------
// Reply injection — Claude Voice queues user replies per session; we poll,
// drain the queue, and type each reply into the Replit agent input.

function findAgentInput() {
  // Prefer a visible textarea whose placeholder hints at agent chat. Fall
  // back to the last visible textarea, then any contenteditable. Replit's
  // class names churn; these heuristics don't depend on them.
  const visible = (el) => {
    const r = el.getBoundingClientRect();
    return r.width > 0 && r.height > 0;
  };
  const textareas = Array.from(document.querySelectorAll("textarea")).filter(
    visible,
  );
  const agentPlaceholder =
    /ask|message|reply|prompt|chat|type|anything|what|describe|send|make|iterate|build/i;
  const hinted = textareas.find((el) =>
    agentPlaceholder.test(el.placeholder || el.getAttribute("aria-label") || ""),
  );
  if (hinted) return { el: hinted, kind: "textarea" };
  if (textareas.length > 0) {
    return { el: textareas[textareas.length - 1], kind: "textarea" };
  }
  const editables = Array.from(
    document.querySelectorAll('[contenteditable="true"]'),
  ).filter(visible);
  if (editables.length > 0) {
    return { el: editables[editables.length - 1], kind: "editable" };
  }
  return null;
}

function setNativeValue(el, value) {
  const proto = Object.getPrototypeOf(el);
  const desc = Object.getOwnPropertyDescriptor(proto, "value");
  if (desc && desc.set) desc.set.call(el, value);
  else el.value = value;
}

function dispatchEnter(el) {
  for (const type of ["keydown", "keypress", "keyup"]) {
    el.dispatchEvent(
      new KeyboardEvent(type, {
        key: "Enter",
        code: "Enter",
        keyCode: 13,
        which: 13,
        bubbles: true,
        cancelable: true,
      }),
    );
  }
}

function clickSendFallback(el) {
  const form = el.closest("form");
  const scope = form || el.parentElement?.parentElement || document.body;
  const buttons = Array.from(scope.querySelectorAll("button"));
  const match = buttons.find((b) => {
    const label = (
      b.innerText ||
      b.getAttribute("aria-label") ||
      b.title ||
      ""
    ).toLowerCase();
    return /send|submit|ask|go/.test(label);
  });
  if (match) {
    log("enter didn't submit, clicking fallback button");
    match.click();
  }
}

function injectReply(text) {
  const found = findAgentInput();
  if (!found) {
    log("no agent input found; skipping reply", { text: text.slice(0, 40) });
    return;
  }
  const { el, kind } = found;
  el.focus();
  if (kind === "textarea") {
    setNativeValue(el, text);
    el.dispatchEvent(new Event("input", { bubbles: true }));
  } else {
    document.execCommand("selectAll", false);
    document.execCommand("insertText", false, text);
  }
  log("injected reply", { chars: text.length, kind });

  setTimeout(() => {
    dispatchEnter(el);
    setTimeout(() => {
      const stillThere =
        kind === "textarea"
          ? el.value === text
          : (el.innerText || "").includes(text);
      if (stillThere) clickSendFallback(el);
    }, 200);
  }, 30);
}

let replyPollTimer = null;

async function pollOnce() {
  if (!enabled) return;
  const sessionId = deriveSessionId();
  try {
    const resp = await chrome.runtime.sendMessage({
      type: "bridge:pending-poll",
      sessionId,
    });
    if (!resp?.ok || !resp.items?.length) return;
    log("draining", { count: resp.items.length });
    for (const text of resp.items) {
      injectReply(text);
      await new Promise((r) => setTimeout(r, 500));
    }
  } catch (e) {
    log("poll failed", e);
  }
}

function startReplyPoller() {
  if (replyPollTimer) return;
  replyPollTimer = setInterval(pollOnce, CONFIG.replyPollMs);
  log("reply poller started", { everyMs: CONFIG.replyPollMs });
}

if (document.readyState === "loading") {
  document.addEventListener("DOMContentLoaded", start, { once: true });
} else {
  start();
}
