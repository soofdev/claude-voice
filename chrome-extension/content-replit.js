// Claude Voice Bridge — Replit adapter
//
// Watches the Replit agent timeline and, at each turn boundary (the
// "Checkpoint made" marker), collects the agent's prose messages from
// that turn and posts them to the local Claude Voice app.
//
// Replit's DOM classes change; the script avoids brittle class selectors
// and works off stable textual signals:
//   - "Checkpoint made …"   → end-of-turn marker
//   - "Worked for …"        → status row (skip)
//   - "N actions"           → action summary (skip)
//   - "Show less / more"    → action toggle (skip)
//
// Messages are *everything else* — prose the agent wrote — between the
// previous checkpoint and this one.

const CONFIG = {
  debug: false,
  // How long after a terminal marker appears to wait before harvesting.
  // Gives the DOM a beat to finish mounting trailing rows.
  harvestDelayMs: 400,
  // Minimum characters for a node to be considered prose.
  minProseChars: 8,
  // Don't fire two end-of-turn triggers within this window (checkpoint
  // row and "Worked for…" row usually both appear at the end of a turn).
  turnCooldownMs: 4000,
};

const patterns = {
  checkpoint: /^\s*checkpoint\s+made/i,
  worked: /^\s*worked\s+for\s+\d+/i,
  actions: /^\s*\d+\s+actions?\s*$/i,
  showToggle: /^\s*show\s+(less|more)\s*$/i,
  published: /^\s*published\s+your\s+app/i,
  started: /\sstarted:\s/i,
};

function log(...args) {
  if (!CONFIG.debug) return;
  console.log("[claude-voice-bridge]", ...args);
}

function deriveSessionId() {
  const m = location.pathname.match(/\/repls\/([^/]+)/i);
  const replId = m ? m[1] : location.pathname.replace(/^\/+|\/+$/g, "") || "root";
  return `replit:${replId}`;
}

function deriveSessionLabel() {
  const m = location.pathname.match(/\/repls\/([^/]+)/i);
  return m ? `Replit · ${m[1]}` : "Replit";
}

let enabled = true;
let seenTerminals = new WeakSet();
let lastFiredAt = 0;

chrome.storage.sync
  .get({ enabled: true, debug: false })
  .then(({ enabled: e, debug }) => {
    enabled = e;
    CONFIG.debug = debug;
    log("boot", { enabled, debug, sessionId: deriveSessionId() });
  });

chrome.storage.onChanged.addListener((changes) => {
  if (changes.enabled) enabled = changes.enabled.newValue;
  if (changes.debug) CONFIG.debug = changes.debug.newValue;
});

function classifyRow(el) {
  const text = (el.innerText || el.textContent || "").trim();
  if (!text) return "empty";
  // Terminal markers are short rows whose *entire* text matches. If the
  // text is long, the pattern is probably matching prose that happens to
  // mention "worked for" inside a sentence.
  if (text.length < 80) {
    if (patterns.checkpoint.test(text)) return "terminal";
    if (patterns.published.test(text)) return "terminal";
    if (patterns.worked.test(text)) return "terminal";
    if (patterns.started.test(text)) return "meta";
  }
  if (patterns.actions.test(text)) return "action";
  if (patterns.showToggle.test(text)) return "action";
  return "prose";
}

function isUserMessage(el) {
  // User bubbles on Replit are typically a distinct background color
  // block that sits inline with agent rows. Heuristic: a block whose
  // computed bg is noticeably saturated (blue), OR one that contains only
  // the user's written prose with no agent icons adjacent.
  // Use role or author markers if Replit exposes them.
  const role = el.getAttribute?.("data-author") || el.getAttribute?.("data-role");
  if (role && /user|me|author/i.test(role)) return true;
  // Final fallback: bubble style. Keep conservative — false negatives are
  // better than reading the user's own prompt aloud.
  return false;
}

function findTimelineRoot() {
  // Pick the scroll container whose subtree contains a terminal marker.
  const candidates = Array.from(
    document.querySelectorAll(
      "main, [class*='chat'], [class*='timeline'], [class*='agent'], [class*='conversation']",
    ),
  );
  for (const el of candidates) {
    const text = el.textContent || "";
    if (
      patterns.checkpoint.test(text) ||
      patterns.published.test(text) ||
      patterns.worked.test(text)
    ) {
      return el;
    }
  }
  return document.body;
}

function collectTurnProse(terminalEl) {
  // Walk the timeline in document order, resetting a "current turn"
  // accumulator at each terminal marker. When we hit the triggering
  // terminal element, return the accumulated prose of that turn.
  const root = findTimelineRoot();
  const walker = document.createTreeWalker(root, NodeFilter.SHOW_ELEMENT, null);
  const inTurn = [];
  let node = walker.nextNode();
  let reached = false;

  while (node) {
    if (node === terminalEl) {
      reached = true;
      break;
    }
    const rect = node.getBoundingClientRect();
    if (rect.height > 0 && rect.width > 0) {
      const kind = classifyRow(node);
      if (kind === "terminal") {
        inTurn.length = 0; // prior turn boundary — reset
      } else if (kind === "prose" && !isUserMessage(node)) {
        const text = (node.innerText || "").trim();
        if (text.length >= CONFIG.minProseChars) {
          inTurn.push({ el: node, text });
        }
      }
    }
    node = walker.nextNode();
  }

  if (!reached) {
    log("walker never reached terminal element");
    return "";
  }

  // Deduplicate nested matches — prefer the outermost element whose text
  // is not a substring of another we already collected.
  const pruned = [];
  for (let i = 0; i < inTurn.length; i++) {
    const a = inTurn[i].text;
    let redundant = false;
    for (let j = 0; j < inTurn.length; j++) {
      if (i === j) continue;
      const b = inTurn[j].text;
      if (b !== a && b.includes(a)) {
        redundant = true;
        break;
      }
    }
    if (!redundant) pruned.push(inTurn[i]);
  }

  const joined = pruned
    .map((p) => p.text)
    .filter(Boolean)
    .join("\n\n")
    .trim();

  log("collected turn prose", { count: pruned.length, chars: joined.length });
  return joined;
}

async function onTurnEnd(terminalEl) {
  if (!enabled) return;
  if (seenTerminals.has(terminalEl)) return;
  seenTerminals.add(terminalEl);

  // Suppress the second marker within a turn (e.g. "Checkpoint made" at
  // t and "Worked for …" at t+100ms both fire but describe one turn).
  const now = Date.now();
  if (now - lastFiredAt < CONFIG.turnCooldownMs) {
    log("within cooldown, skipping", {
      sinceLast: now - lastFiredAt,
      text: (terminalEl.innerText || "").slice(0, 60),
    });
    return;
  }
  lastFiredAt = now;

  await new Promise((r) => setTimeout(r, CONFIG.harvestDelayMs));

  const text = collectTurnProse(terminalEl);
  if (!text || text.length < CONFIG.minProseChars) {
    log("no prose to speak");
    return;
  }

  const payload = {
    sessionId: deriveSessionId(),
    sessionLabel: deriveSessionLabel(),
    cwd: deriveSessionLabel(),
    text,
  };
  log("sending to Claude Voice", { chars: text.length, sessionId: payload.sessionId });
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

function checkNode(node) {
  if (node.nodeType !== Node.ELEMENT_NODE) return;
  if (classifyRow(node) === "terminal" && !seenTerminals.has(node)) {
    const rect = node.getBoundingClientRect();
    if (rect.width > 0 && rect.height > 0) {
      onTurnEnd(node);
    }
  }
}

function scanSubtreeForTerminals(root, handler) {
  if (root.nodeType !== Node.ELEMENT_NODE) return;
  handler(root);
  const walker = document.createTreeWalker(root, NodeFilter.SHOW_ELEMENT, null);
  let node = walker.nextNode();
  while (node) {
    handler(node);
    node = walker.nextNode();
  }
}

const observer = new MutationObserver((mutations) => {
  for (const m of mutations) {
    for (const node of m.addedNodes) {
      scanSubtreeForTerminals(node, checkNode);
    }
  }
});

function start() {
  // Initial scan — mark every existing terminal as seen without firing,
  // so loading a page with prior conversation does not replay it.
  const root = findTimelineRoot();
  scanSubtreeForTerminals(root, (node) => {
    if (classifyRow(node) === "terminal") {
      seenTerminals.add(node);
    }
  });

  observer.observe(document.body, {
    childList: true,
    subtree: true,
  });
  log("started; only new turns from this point forward will be spoken");
}

if (document.readyState === "loading") {
  document.addEventListener("DOMContentLoaded", start, { once: true });
} else {
  start();
}
