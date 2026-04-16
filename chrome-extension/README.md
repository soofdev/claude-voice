# Claude Voice Bridge

A Chrome extension that reads agent responses from web-based agentic coding platforms (Replit today; Cursor / Lovable / v0 later) and speaks them through the Claude Voice desktop app.

## How it works

1. A content script runs on `replit.com/*` and watches the agent timeline.
2. When a turn ends (detected by the `Checkpoint made …` row Replit appends after each agent response), the script harvests the prose messages from that turn, skipping actions, the `Worked for …` timer, action summaries (`N actions`), and `Show less/more` toggles.
3. The harvested text is POSTed to the Claude Voice app's local HTTP server (`http://127.0.0.1:8765/hook/stop`) tagged with a synthetic session id like `replit:<replId>`, so the existing pipeline (summarize → color-coded popup → TTS → history) just works.

Each repl gets its own session color, voice override, and mute toggle inside the Claude Voice app.

## Install (dev mode)

1. Open `chrome://extensions` in Chrome.
2. Enable **Developer mode** (top right).
3. Click **Load unpacked** and pick this `chrome-extension/` folder.
4. Make sure the Claude Voice desktop app is running (menubar icon visible, listening on `127.0.0.1:8765`).
5. Open Replit. The extension auto-activates on any `replit.com/*` page.

Open the extension popup (puzzle-piece → Claude Voice Bridge) to:
- Toggle it on/off
- Toggle debug logging (Console output in DevTools)
- Change the host/port (if you moved Claude Voice off the default 8765)
- Click **Test** to send a one-shot sentence and verify the bridge

## Turn detection

The script treats the `Checkpoint made …` text row as an end-of-turn marker. On first load, every existing checkpoint is marked seen silently — prior turns in the conversation are not replayed as speech.

If Replit changes this marker or the surrounding DOM, adjust the `patterns` object in `content-replit.js`. The textual signals used today:

| Signal              | Classification                |
|---------------------|-------------------------------|
| `Checkpoint made …` | end-of-turn → trigger harvest |
| `Published your app` | end-of-turn → trigger harvest |
| `Worked for …`      | meta row → skip               |
| `N actions`         | action summary → skip         |
| `Show less / more`  | action toggle → skip          |
| `… started: …`      | action banner → skip          |
| anything else       | prose → speak                 |

## Known limitations

- **DOM-fragile.** Replit ships UI changes frequently. If speech stops or the wrong rows start being spoken, inspect the timeline DOM and tweak `patterns` / `findTimelineRoot` in `content-replit.js`.
- **User bubbles.** We try to skip user-authored messages via `data-author` / `data-role` attributes; if Replit doesn't expose those, add a selector-based check to `isUserMessage()`.
- **Streaming.** We wait for the end-of-turn marker before speaking, so partial/streaming output is ignored by design.
- **Reply-back.** Not wired yet. The Claude Voice popup can send text/voice to a terminal today; injecting replies into the Replit input is on the roadmap.

## Files

- `manifest.json` — MV3 manifest
- `background.js` — service worker, handles `fetch` to the Claude Voice server
- `content-replit.js` — Replit DOM adapter
- `popup.html` / `popup.js` / `popup.css` — extension popup (enable/disable, server config, test)
