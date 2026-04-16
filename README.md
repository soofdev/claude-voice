# Claude Voice

A macOS menu-bar companion that gives [Claude Code](https://docs.anthropic.com/claude/docs/claude-code) a voice — and a face. When Claude finishes a response, Claude Voice reads it aloud through a natural-sounding voice, surfaces a floating popup with synchronized word highlighting, and lets you reply back by text or voice without leaving your editor.

Built with Tauri 2 (Rust + WebView). Lives in your menu bar. Stays out of your way.

## Why

Claude Code produces a steady stream of text — often long, often dense, often arriving while you're focused elsewhere. Reading every response interrupts you. Glancing at the terminal misses the nuance. Running several sessions in parallel makes it worse.

Claude Voice makes Claude's output ambient. Start a long task, walk away, and hear "Wodworx says: done — tests pass, ready for review." Click a history entry to replay for free. Hit the microphone, say a follow-up, watch it land in the correct terminal tab.

## Features

**Three voice backends.** Pick your tradeoff per message:
- **System voice** (macOS `say`) — free, always works, no keys.
- **Browser voice** (Web Speech API) — free, offline, real-time word-boundary highlighting.
- **ElevenLabs** — premium quality, character-level timing, cached MP3 per message so replays never re-bill the API.

Automatic fallback: if ElevenLabs fails (quota, rate limit, network), the pipeline speaks the message with `say` and surfaces a visible warning. You always hear something.

**Speech pipeline.** Extracts links and code blocks, strips markdown, optionally rephrases long responses into spoken-word form using Claude Haiku, and routes the result to the selected backend. The summarizer is hardened against prompt injection.

**Multi-session aware.** Each Claude Code session gets a color, label, and optional voice override. Mute the noisy ones. Prefix each message with the session's name ("Wodworx says: …") so you know who's talking without looking.

**Live popup.** Semi-transparent, always-on-top window with word-by-word highlighting, auto-scroll, pin toggle, draggable sidebar, and a state-machine of visual states (waking → speaking → fading). Four orb styles for the minimized ambient mode: Glass, Plasma, Iridescent, Energy.

**History.** Every message saved locally (up to 200). Browse as a flat list or grouped by session. Search by text. Click to reload into the main view with the same color theme. Replay plays the cached audio — no API call, instant.

**Reply in place.** Type or dictate a follow-up in the popup. The prompt is routed to the session's specific terminal tab via TTY matching (iTerm2 and Terminal.app). Voice input uses ElevenLabs Scribe.

**Code block extraction.** Fenced code is stripped from speech but kept in the popup as copyable blocks below the text.

**Global hotkey.** `Cmd+Shift+V` from anywhere to show/hide the popup; auto-recenters if it ended up off-screen.

**One-click hook install.** A Settings button writes Claude Code's Stop hook into `~/.claude/settings.json` — no manual JSON editing.

**Local HTTP API.** `127.0.0.1:8765` exposes `/speak`, `/stop`, `/pause`, `/resume`, `/status`, `/hook/stop` for automation.

## Install

Claude Voice is currently distributed as an unsigned build. You'll need to build from source or download a release and bypass Gatekeeper.

### Build from source

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

Open Claude Voice → menu bar icon → **Settings…** → scroll to **Claude Code hook** → **Install / Update hook**. This adds a `Stop` hook to `~/.claude/settings.json` that POSTs to the local server. From then on, every Claude Code Stop event flows through Claude Voice.

If you'd rather edit manually, the hook is just a curl command the Settings window exposes for you to copy.

### API keys (optional)

- **ElevenLabs** (premium voice + speech-to-text): Settings → Voice backend → paste your key.
- **Anthropic** (summarizer): Settings → Summarizer → paste your key.

Both are optional. You can run Claude Voice with zero keys on the System or Browser voice backends.

## Shortcuts

Global:
- `Cmd+Shift+V` — show/hide popup (and recenter if off-screen)

In the popup:
- `Space` — pause / resume
- `Esc` — stop (or hide if pinned)
- `P` — toggle pin
- `T` — toggle spoken ↔ original
- `M` — minimize to orb
- `C` — copy current message

## Tech

- **Frontend:** Vanilla HTML/CSS/JS in a Tauri WebView (no framework).
- **Backend:** Rust with Tokio, Axum for the HTTP server, `reqwest` for ElevenLabs and Anthropic, Tauri 2 plugins for global shortcut and `opener`.
- **TTS:** macOS `say`, browser SpeechSynthesis API, or ElevenLabs `/with-timestamps`.
- **STT:** ElevenLabs Scribe.
- **Summarization:** Claude Haiku (model configurable).
- **Terminal routing:** AppleScript against iTerm2 and Terminal.app, matched by TTY.
- **Persistence:** JSON files in `~/Library/Application Support/claude-voice/`.

## Privacy

All audio, history, session preferences, and voice recordings stay on your machine in `~/Library/Application Support/claude-voice/`. Outbound network traffic goes only to the voice, summarizer, and STT providers you configure — only when those features are enabled. No analytics. No telemetry.

## Platform support

- **macOS** (primary target): everything works.
- **Windows / Linux:** the app builds, but reply-to-terminal and pause/resume (process signals) are macOS-only. The browser voice backend and global hotkey work cross-platform.

## Status

Early. Expect rough edges around unsigned-app distribution, first-run permissions, and anything outside the happy path. Issues and PRs welcome.

See [PRD.md](./PRD.md) for the full feature surface.

## License

MIT (or whichever you prefer — see `LICENSE`).
