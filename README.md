# Claude Voice

A macOS menu-bar companion that gives your AI coding agents a voice — and a face. When Claude Code or the Replit agent finishes a response, Claude Voice reads it aloud through a natural-sounding voice, surfaces a floating popup with synchronized word highlighting, and lets you reply back by text or voice without leaving your editor or your browser.

Built with Tauri 2 (Rust + WebView). Lives in your menu bar. Stays out of your way.

## Why

Agents produce a steady stream of text — often long, often dense, often arriving while you're focused elsewhere. Reading every response interrupts you. Glancing at the terminal misses the nuance. Running several sessions in parallel — or a CLI agent *plus* a web agent — makes it worse.

Claude Voice makes the output ambient. Start a long task, walk away, and hear "Wodworx says: done — tests pass, ready for review." Click a history entry to replay for free. Hit the microphone, say a follow-up, watch it land in the correct terminal tab or browser chat.

## Features

### With Claude Code

- **One-click hook install.** Settings → *Install / Update hook* writes the Stop hook into `~/.claude/settings.json`. Every Stop event flows to the local HTTP server.
- **Multi-session aware.** Each Claude Code session gets a color, label, and optional voice override. Mute the noisy ones. Messages are prefixed with the session label ("Wodworx says: …") so you know who's talking without looking.
- **Reply in place.** Type or dictate a follow-up in the popup. The prompt is routed to the session's specific terminal tab via TTY matching (iTerm2 and Terminal.app). Voice input uses ElevenLabs Scribe.

### With web-based agents (Chrome extension)

An MV3 extension in [`chrome-extension/`](./chrome-extension) bridges browser-based agent platforms into the same pipeline. Replit today; Cursor / Lovable / v0 adapters fit the same shape.

- **Hands-off capture.** The content script watches the agent timeline for new response bubbles, waits for streaming to settle, and POSTs to `127.0.0.1:8765/hook/stop` with a synthetic `replit:<repl-slug>` session id. All the existing machinery — color theme, voice override, history, replay — works automatically.
- **Smart turn detection.** Skips input-box placeholders, toolbar text, and user messages; keys off stable anchors (`.rendered-markdown` outside user-message wrappers) instead of Replit's hashed CSS classes.
- **"Only read final message" mode.** Opt-in toggle in the extension popup: during a long multi-step task, intermediate thought-process messages are silently absorbed and only the final summary (after the "Working" indicator clears) gets read.
- **Refresh-safe.** A warmup phase at page load absorbs any historical turns that hydrate post-refresh, so a reload never replays the whole conversation.
- **Session name.** Reads the repl's project name from the header, so playback says "Super Demo Builder says: …" instead of a generic "Replit".
- **Reply-back.** Replies you type in the Claude Voice popup get injected into the Replit agent chat as a synthetic paste event (CodeMirror-compatible) and submitted — no focus grab required.

### Voice

- **Three backends.** Pick your tradeoff per message:
  - **System voice** (`say`) — free, always works, no keys.
  - **Browser voice** (Web Speech API) — free, offline, real-time word-boundary highlighting.
  - **ElevenLabs** — premium quality, character-level timing, cached MP3 per message so replays never re-bill the API.
- **Automatic fallback.** If ElevenLabs fails (quota, rate limit, network), the pipeline speaks the message with `say` and surfaces a visible warning. You always hear something.
- **Speech pipeline.** Extracts links and code blocks, strips markdown, optionally rephrases long responses into spoken-word form using Claude Haiku (hardened against prompt injection), and routes the result to the selected backend.

### Popup & history

- **Live popup.** Semi-transparent, always-on-top window with word-by-word highlighting, auto-scroll, pin toggle, draggable sidebar, and a state machine of visual states (waking → speaking → fading). Four orb styles for the minimized ambient mode: Glass, Plasma, Iridescent, Energy.
- **Smooth queueing.** New messages that arrive while one is still playing appear in history immediately with a pulsing *Queued…* indicator. Playback stays on the current message until it finishes, then the queue advances with no blank-popup flash.
- **Sticky selection.** The currently-playing (or last-played) message stays highlighted in the sidebar until a new message starts or you pick a different entry.
- **Pause / resume / stop.** Space toggles pause (SIGSTOP preserves position); Esc stops and clears the queue; the play button falls back to replaying the current message when playback has ended.
- **History.** Every message saved locally (up to 200). Browse as a flat list or grouped by session. Search by text. Click to reload into the main view with the same color theme. Replay plays the cached audio — no API call, instant.
- **Code block extraction.** Fenced code is stripped from speech but kept in the popup as copyable blocks below the text.
- **Global hotkey.** `Cmd+Shift+V` from anywhere to show/hide the popup; auto-recenters if it ended up off-screen.

### Local HTTP API

`127.0.0.1:8765` exposes:

- `POST /hook/stop` — entry point for Claude Code's Stop hook and the browser bridge
- `POST /speak`, `POST /stop`, `POST /pause`, `POST /resume`, `POST /toggle`
- `GET /status` — `{ enabled, speaking, paused, voice, rate }`
- `GET /bridge/pending/:sessionId` — drains queued replies for the Chrome extension
- `GET /` — liveness

## Install

Claude Voice is currently distributed as an unsigned build. You'll need to build from source or download a release and bypass Gatekeeper.

### Build the app

Requirements: macOS, Rust (stable), Node 18+, Xcode command-line tools.

```bash
git clone https://github.com/<your-fork>/claude-voice
cd claude-voice
npm install
npm run tauri dev   # development
npm run tauri build # production bundle (.app, .dmg)
```

The production bundle lands in `src-tauri/target/release/bundle/`.

### Wire up the Claude Code hook

Open Claude Voice → menu bar icon → **Settings…** → scroll to **Claude Code hook** → **Install / Update hook**. This adds a `Stop` hook to `~/.claude/settings.json` that POSTs to the local server.

If you'd rather edit manually, the hook is just a curl command the Settings window exposes for you to copy.

### Install the Chrome extension (optional)

For Replit / web-based agent support:

1. Open `chrome://extensions` and enable **Developer mode**.
2. Click **Load unpacked** and pick the [`chrome-extension/`](./chrome-extension) folder in this repo.
3. Click the extension's toolbar icon to open its popup:
   - Leave **Enabled** on.
   - Optionally enable **Only read final message** for long multi-step tasks.
   - Host/port should match the Claude Voice server (default `127.0.0.1:8765`).
4. Make sure the Claude Voice app is running, then open a Replit tab. New agent responses will flow through.

### API keys (optional)

- **ElevenLabs** (premium voice + speech-to-text): Settings → Voice backend → paste your key.
- **Anthropic** (summarizer): Settings → Summarizer → paste your key.

Both are optional. You can run Claude Voice with zero keys on the System or Browser voice backends.

## Shortcuts

Global:

- `Cmd+Shift+V` — show/hide popup (and recenter if off-screen)

In the popup:

- `Space` — pause / resume (or replay the current entry if playback ended)
- `Esc` — stop (or hide if pinned)
- `P` — toggle pin
- `T` — toggle spoken ↔ original
- `M` — minimize to orb
- `C` — copy current message

## Tech

- **Frontend:** vanilla HTML/CSS/JS in a Tauri WebView (no framework).
- **Backend:** Rust with Tokio, Axum for the HTTP server, `reqwest` for ElevenLabs / Anthropic, Tauri 2 plugins for global shortcut and `opener`.
- **TTS:** macOS `say`, browser SpeechSynthesis API, or ElevenLabs `/with-timestamps`.
- **STT:** ElevenLabs Scribe.
- **Summarization:** Claude Haiku (model configurable).
- **Terminal routing:** AppleScript against iTerm2 and Terminal.app, matched by TTY.
- **Chrome extension:** MV3, service worker + content script, DOM MutationObserver for agent-message detection, synthetic `ClipboardEvent` for CodeMirror 6 injection.
- **Persistence:** JSON + JSONL in `~/Library/Application Support/claude-voice/`.

## Privacy

All audio, history, session preferences, and voice recordings stay on your machine in `~/Library/Application Support/claude-voice/`. Outbound network traffic goes only to the voice, summarizer, and STT providers you configure — only when those features are enabled. No analytics. No telemetry.

The Chrome extension only has host permissions for `https://replit.com/*` (to observe the agent timeline) and `http://127.0.0.1/*` (to talk to your local Claude Voice server). It never sends data to third parties.

## Platform support

- **macOS** (primary target): everything works.
- **Windows / Linux:** the app builds, but reply-to-terminal and SIGSTOP-based pause (keeps audio position through a pause) are macOS/unix-only. On Windows, the play button replays from the start instead of resuming mid-audio. The browser voice backend, global hotkey, and Chrome extension bridge work cross-platform.

## Status

Early. Expect rough edges around unsigned-app distribution, first-run permissions, and anything outside the happy path. Issues and PRs welcome.

See [PRD.md](./PRD.md) for the full feature surface.

## License

MIT — see [LICENSE](./LICENSE).
