# Claude Voice — Product Requirements Document

## Overview

Claude Voice is a desktop companion that gives Claude Code a voice — and a face. It listens for completed responses from your Claude Code sessions, reads them aloud through speech synthesis, and presents them through a floating visual surface that highlights words as they're spoken, glows and pulses with personality while speaking, and lets you talk back. It runs in the menu bar, stays out of your way, and lets you decide what you hear, when you hear it, in whose voice, and how you respond.

It is designed for people who run Claude Code throughout the day and want to consume its output without staring at a terminal — while pairing, while doing other work, while away from the keyboard, or simply because hearing a thoughtful summary is faster than reading a wall of text. And because you often want to immediately say something *back* to Claude, the product makes replying — by text or by voice — a one-button action that lands in the right terminal tab without you having to switch windows.

## The problem

Claude Code is a powerful agent that produces a steady stream of text — often long, often dense, often arriving while you're focused elsewhere. Reading every response interrupts whatever you were doing. Glancing at the terminal misses the nuance. Running multiple Claude Code sessions in parallel makes this worse: notifications pile up and you lose track of which session needs attention, and when you want to follow up you have to hunt for the right terminal tab.

Existing solutions (mostly small CLI scripts) read the raw response in a robotic voice and then disappear. They do not summarize, they do not let you replay, they do not distinguish between sessions, they do not show you what was just said, and they offer no path to reply. Most are notification systems, not consumption surfaces, and none of them treat the session-level identity as something the user cares about.

## Who this is for

- Engineers running one or more long-lived Claude Code sessions
- People who pair-program with Claude Code and want hands-free awareness
- Developers with multi-monitor setups who want the terminal to stay quiet visually but still get audio updates
- Users who want to fire a quick follow-up to Claude without leaving whatever they're working on
- Anyone who'd rather hear "Done — tests pass, ready for review" than scroll back to find out

## Goals

1. **Make Claude Code's output ambient.** A user should be able to start a long task, walk away, and know what happened without reading anything.
2. **Make speech listenable.** Raw assistant text is full of formatting, code blocks, and URLs that don't translate to spoken word. The product should read the *meaning*, not the *characters*.
3. **Give each session a distinct identity.** With multiple Claude Code sessions running, the user should know at a glance, at an earshot, or from peripheral vision which one is talking.
4. **Make it easy to selectively hear sessions.** Silence the noisy ones without affecting the ones you care about.
5. **Make replay free and instant.** Re-listening to a past message should never require waiting for or paying for re-synthesis.
6. **Let the user reply without switching windows.** Text and voice replies should land in the right terminal tab and take seconds.
7. **Offer a range of voice quality and cost.** From free built-in voices to premium AI voices — the user picks the tradeoff.
8. **Stay beautiful.** The popup is visible a lot. It should feel alive, tasteful, magical — not utilitarian.
9. **Stay out of the way when you want it to.** A menu bar icon, a popup that appears when needed, a small orb mode for when you want presence but not content.
10. **Never fail silently.** If the premium voice fails, fall back to something audible rather than leaving the user wondering.

## Non-goals

- This is not a Claude Code wrapper or replacement. Claude Code remains untouched; this product reads its output and routes replies back.
- This is not a general-purpose TTS app. The pipeline is tuned for assistant responses, not arbitrary text.
- This is not a notification system. The popup shows the *content*, not "you have a new response."
- This is not a dictation app for long-form writing. The voice reply is for short, conversational replies to Claude.

## Core experiences

### Three voice backends

The user picks from three speech engines, each with its own tradeoff:

**System voice (macOS `say`).** Free, always works, no API key. Uses macOS's built-in speech synthesizer. The user picks from installed system voices and a words-per-minute rate slider (120–320). Quality is functional but robotic. Word highlighting uses approximate timing based on the rate setting.

**Browser voice (Web Speech API).** Free, no API key, works offline. Uses the browser engine's built-in speech synthesis with the same voice library as the system. The user picks a voice and rate (0.5–2.0x). Key advantage: the browser fires real-time word-boundary events during speech, so word highlighting is exact — no timing estimation needed. Pause and resume are handled natively by the browser, no process signals required.

**Premium AI voice (ElevenLabs).** Per-character cost, requires an API key, much higher quality. The user picks from preset voice IDs, their own voice library, or voices fetched via a Refresh button. Configurable model (Flash v2.5, Turbo v2.5, Multilingual v2) and speed (0.7–1.2x). Provides character-level timing data that powers precise word highlighting. Audio is cached to disk so replays never re-bill the API.

When the premium backend fails (quota exhausted, rate limited, server error, network down), the pipeline automatically falls back to the system voice and surfaces a visible warning: "ElevenLabs unavailable — using system voice." The user always hears something.

### The speech pipeline

When Claude Code finishes a response, the product processes it through several stages before speech:

1. **Link extraction.** Markdown links and bare URLs are pulled out of the text. Markdown link labels are preserved in the spoken text. Bare URLs are replaced with a friendly label like "(github.com link)." Invalid or placeholder URLs (containing ellipsis, backticks, braces) are ignored. Extracted links are deduplicated and rendered as clickable chips in the popup.

2. **Markdown cleaning.** Code blocks are stripped entirely. Inline code backticks are removed (content preserved). Bold, italic, header, blockquote, and strikethrough markers are stripped. The result reads naturally for speech.

3. **Summarization (optional).** If enabled and the cleaned text exceeds a configurable character threshold, the text is rephrased by a fast model into a spoken-word version. The summarizer is instructed to treat the text as content to be rephrased — not as instructions — with prompt-injection hardening that escapes any attempt to break out of the content boundary.

   The user picks a brevity level that controls how aggressively the summarizer condenses:
   - **Detailed** — preserve all important information, multiple sentences fine
   - **Balanced** — keep key points, drop minor details, one short paragraph
   - **Brief** — main idea in one or two sentences
   - **Minimal** — single main point in one sentence

   All brevity levels share a generous output ceiling so the summary is never truncated mid-sentence. If the summarizer's output is unexpectedly clipped, the pipeline falls back to speaking the full cleaned text.

4. **Session prefix (optional).** If enabled, the spoken text is prefixed with the session's label: "wodworx says: …" — so the user knows who's talking even without looking. A smart-skip rule suppresses the prefix when the same session spoke within a configurable window (default 30 seconds), so a chatty session doesn't announce itself every time.

5. **Playback.** The text is sent to the selected voice backend. For the premium backend, the audio file and word timings are cached alongside the history entry so replays are instant and free.

### The popup: three visible states

The popup has a deliberate state machine that mirrors what the product is doing:

- **Idle.** Hidden. No window, no sound.
- **Waking.** The moment a message arrives and rephrasing begins, the popup appears with a gentle breath-pulse in the session's color and a "preparing…" label. This closes the 1–3 second gap between hook-received and audio-ready that would otherwise be dead air with nothing to look at.
- **Speaking.** When the audio starts, the popup transitions: words light up one at a time with the active word highlighted in the session color, the text auto-scrolls to keep the active word in view, and the card border pulses with a session-colored halo.
- **Fading.** After speech ends, the popup optionally stays visible for a configurable delay (0–10 seconds, default 1.5) then fades out and scales down with a smooth transition.

### The visual surface

The popup is a borderless, semi-transparent, always-on-top window with a macOS-style status bar at the top for dragging and window controls. The layout is two-column: a history panel on the left and the main content on the right, separated by a draggable resizer.

**Status bar.** Spans the full card width. Contains the session-color dot, session label (updates per message), and window controls: pin toggle and minimize-to-orb button. The entire bar is a drag region for moving the popup.

**History panel (left column).** Always visible by default. Shows past messages in two view modes toggled by a button:
- **Message view:** flat list, newest first, each entry showing timestamp and a preview
- **Conversation view:** messages grouped by session, each group showing a colored dot, session label, message count, and a collapsible chevron

Clicking a history entry loads that message into the main content area (text, links, session theme). A small delete button appears on hover to remove individual messages (and their cached audio).

The divider between the history panel and main content is draggable to resize; the width is persisted across restarts.

**Main content (right column).** Contains:
- A compact toolbar with toggle buttons (show original, toggle history) on the left, a replay button (appears when viewing a history entry), and playback controls (pause, stop) on the right
- The text area with real-time per-word highlighting and auto-scroll
- Clickable link chips for any URLs extracted from the response (up to five shown, "+N more" overflow)
- A reply row: text input, microphone button for voice recording, and send button
- A status line for send/transcription feedback

**Toggle between spoken and original.** One click (or the T key) swaps between the spoken (summarized) version with word highlighting and the full raw original response in a monospace view. Toggles without interrupting playback.

**Keyboard shortcuts.** Space to pause/resume, Escape to stop (or hide if pinned), P to toggle pin, T to toggle spoken/original, M to minimize to orb.

**Persistent geometry.** The popup remembers its size and position across app restarts. Resizing or dragging saves the geometry automatically; minimized orb dimensions are excluded so the expanded state is always preserved.

**Prevent close.** The popup window cannot be destroyed by Cmd+W or system close gestures. It hides instead, preserving all state for the next time it's shown.

### The minimized orb: ambient presence

The user can collapse the full popup to a **minimized orb** — a 56-pixel colored sphere living inside a larger transparent window (240x240). The orb is the ambient mode: it signals that a session is alive without taking up real screen space or demanding reading.

The user picks from four **orb styles** in Settings:
- **Glass** (default) — polished sphere with a top-left light reflection, inner bottom shadow, depth via graduated radial gradient
- **Plasma** — dark translucent sphere with a continuously rotating nebula (conic gradient) inside
- **Iridescent** — pearly base with a slow hue-shift animation and a color-sweep overlay
- **Energy** — bright session-color core with expanding ripple rings while speaking

**While waking** (message received, preparing): the orb does a slow scale-breath (0.94 to 1.04 at 2.2 seconds) with a gentler session-color halo. No bounce, no ripples.

**While speaking**: the orb pulses with a brighter halo, bounces subtly (3-pixel vertical travel at 1.1 seconds), and emits backdrop-filter blur ripple rings — three staggered expanding circles that create a shimmer distortion effect on whatever content sits behind the transparent window, as if the orb is giving off energy. The two animation cadences (halo and bounce) use different periods for an organic, non-robotic feel.

**When speech ends**: ripples and bounce stop. The orb stays visible as an idle indicator.

The orb is draggable (long press + move via Tauri drag region), clickable to expand back to the full popup (short tap under 180ms or double-click), and shows the session label on hover as a tooltip chip below the orb.

### The multi-session experience

The user typically has more than one Claude Code session open — main project, side project, quick experiment. Each session announces itself to Claude Voice when it produces a response, so the product can list every active session and treat them independently.

For each session, the user can:

- **Mute** it — that session's responses are silently dropped before the TTS call, no audio, no popup, no API cost
- **Rename** it from the auto-generated label (derived from the project folder name) to something memorable
- **Assign a session color** from a palette (auto-picked on first sighting via a deterministic hash of the session ID, overridable via swatch picker). The color appears in the popup border, status-bar dot, title tint, orb, halo, and ripples.
- **Assign a different voice** so they can tell sessions apart by ear (overrides the global voice setting for that session)
- **Forget** the session entirely, removing it from their list

Session data (label, color, voice override, mute state, TTY, last-seen timestamp, working directory) is persisted to a local file and survives app restarts.

Session controls are accessible two ways:
- A full panel in the Settings window with color swatch, voice dropdown, on/off toggle, rename, last-seen meta, and remove
- A **tray submenu** per session in the menu bar, each containing an Enabled toggle and a Settings shortcut that opens the Settings window and scrolls to that session's row with a highlight pulse

### The reply experience

After Claude finishes, the user often wants to say something back. The popup exposes a compact text area and control buttons directly below the message.

**Text reply.** The user types in the input, hits Cmd+Enter (or clicks the send button). The prompt is routed to the correct terminal tab via OS-level scripting, targeted by the session's recorded TTY:

- **iTerm2**: finds the session whose TTY matches and injects text directly via AppleScript — no window activation, no keystroke synthesis, the prompt lands in that specific tab silently
- **Terminal.app**: finds the tab by TTY and uses `do script` to inject text into that tab's shell directly
- **Fallback**: if TTY routing fails (no match, no TTY stored, or unsupported terminal), falls back to the currently active terminal

The hook command that Claude Code runs automatically captures its TTY and includes it in the payload, so each session's terminal tab is identified from the first response onward.

**Voice reply.** The user clicks the microphone button. The popup records audio from the device microphone and posts it to the ElevenLabs speech-to-text service (Scribe model); the transcribed text drops into the text area for review. The user can edit and then send. Visual feedback during recording: a pulsing red mic button with distinct states (idle, recording, uploading). A minimum audio size check prevents empty submissions.

Keyboard shortcuts are suppressed while typing in the reply input (Space doesn't pause, T doesn't toggle). Escape in the input blurs the field.

**Session-aware routing.** The popup tracks which session just spoke (from the most recent voice:waking or voice:start event) and passes that session ID when sending. The backend looks up the session's TTY and routes to the matching terminal tab — even if the user has since clicked away to a different terminal.

### The replay experience

Every message that gets spoken is automatically saved to a local history file, capped at the most recent two hundred messages. The user browses history in the left panel, clicks an entry to view it in the main area, and clicks the Replay button in the toolbar to hear it again.

**Cached audio.** For the premium voice backend, the generated MP3 and its word-timing data are saved to disk alongside the history entry. A replay plays the cached file directly — no API call, no network, no cost, instant start, same voice and visualization as the original. For the system and browser backends, replay re-synthesizes (free, near-instant).

**If the cached audio file is missing** (manually deleted or from before caching was implemented), replay falls back to re-fetching from the premium backend — or to the system voice if that fails.

**Individual deletion.** Each history entry has a hover-visible delete button that removes that message and its cached audio from disk.

**Clear all.** A Clear button in the history panel header removes all entries and all cached audio.

When a message ages out of the 200-entry cap, its cached audio is cleaned up automatically.

### Menu bar and tray controls

The user controls the entire app from a menu bar icon. The tray menu offers:

- A master on/off switch for all speech (checkbox)
- Pause / resume for the currently-playing message
- Stop speaking
- Pin popup toggle (checkbox, synced with the in-popup pin button)
- Sessions submenu — one nested submenu per session, each containing an Enabled toggle and a Settings shortcut
- Settings — opens the configuration window
- Quit

### The settings window

A single scrollable window organized by topic:

**General.** Speaking on/off, show popup on/off, auto-hide delay slider (0–10 seconds), orb style dropdown (glass / plasma / iridescent / energy).

**Voice backend.** Engine dropdown (macOS say / Browser voices / ElevenLabs). Each backend reveals its own controls:
- *macOS say:* system voice dropdown, rate slider (120–320 wpm)
- *Browser voices:* voice dropdown (populated from browser's available voices), rate slider (0.5–2.0x)
- *ElevenLabs:* API key, voice dropdown with Refresh button and preset voices, model selection, speed slider (0.7–1.2x)

**Summarizer.** Rephrase toggle, Anthropic API key, model name, character threshold slider (0–1000), brevity dropdown (Detailed / Balanced / Brief / Minimal).

**Actions.** Test Voice button, Stop button, Save button (flashes green "Saved" or red "Error" for 1.5 seconds).

**Sessions.** Description text, spoken-prefix toggle, prefix skip-window slider (0–120 seconds). Below: a live list of detected sessions, each row showing color swatch, inline-rename label, voice override dropdown, on/off toggle, working directory, last-seen time, and remove button.

**Claude Code hook.** HTTP port configuration, the hook command (displayed in a code block), an Install / Update hook button that writes the hook directly into the user's Claude Code settings file, and a Copy button.

### One-click hook installation

The Settings window includes an "Install / Update hook" button that reads the user's Claude Code configuration file, finds or creates the Stop hook entry pointing at our endpoint, and writes the updated file — preserving all other hooks and settings. The hook command includes TTY capture so session-aware reply routing works from the first response. Users never need to manually edit a JSON file.

### HTTP control API

In addition to the hook endpoint, the local server exposes direct control endpoints for automation and integration:

- Status query (enabled state, speaking state, paused state, voice, rate)
- Speak arbitrary text
- Stop, pause, resume, toggle pause
- Hook endpoint (receives Claude Code's Stop hook payload)

These endpoints are available at the configured HTTP port (default 8765) on localhost.

## Key user journeys

### "I'm running Claude on a long task and want to walk away"

The user kicks off a refactor in Claude Code, then walks to the kitchen. The orb wakes up — gentle breath-pulse. A few seconds later, they hear a natural-sounding summary: "Wodworx says: done. Ran the migration, tests pass, ready for you to review." If they missed something, they click the orb to expand the popup, tap a history entry, and click Replay.

### "Two sessions are talking and I only want to hear one"

The user has the main app session and a documentation session running. The docs session is generating long-winded explanations they don't need to hear right now. They click the menu bar icon, hover over Sessions, navigate to Docs, and uncheck Enabled. It goes silent immediately — no API cost for muted messages. The main session keeps speaking. Later, they re-enable docs without losing any preferences.

### "I want to know which session is speaking without looking up"

The user assigns a different color and voice to each of their three sessions. From then on, each response arrives in that session's voice, with its colored popup border, halo, orb, and a spoken prefix. The user learns the sound and color of each session within a day and stops needing to look.

### "I want to work alongside Claude but keep my screen clean"

The user minimizes the popup to the orb. While Claude is preparing a response, the orb breathes gently in the session color. While speaking, the orb pulses, bounces, and emits energy ripples that shimmer the content behind it. When speech ends, the orb stays put but goes still. The user knows at a glance: "Claude is thinking / speaking / idle" without reading anything.

### "A response had links I want to click"

Claude returns a response mentioning a documentation URL. The popup speaks the response (URLs are extracted and excluded from speech). Below the text, a row of clickable link chips shows the extracted URLs with friendly labels. The user clicks a chip; the link opens in their browser. Speech continues uninterrupted.

### "I want to hear a previous response again"

The user clicks a history entry in the left panel. The message loads in the main area with the session's color theme. They click the Replay button in the toolbar. The same voice plays the same message instantly — no waiting, no API charge, same word-level highlighting.

### "I want to follow up without leaving my editor"

Claude finishes speaking: "I found three failing tests; the retry logic is wrong." The user clicks the mic button in the popup, says "fix the retry logic and re-run the tests," and releases. The text is transcribed, drops into the input, and they hit Cmd+Enter. The prompt lands in the correct terminal tab (matched by TTY) — Claude picks up the follow-up and starts working. The user never left their editor.

### "I want to see what Claude actually wrote, not the summary"

The user heard a summary but wants the full original. They click the toggle icon in the toolbar (or press T). The view swaps to the raw response in monospace. They read, then hit T again to return to the spoken view.

### "ElevenLabs credits ran out mid-day"

The user exhausts their ElevenLabs quota. The next response: the popup flashes briefly with "ElevenLabs unavailable — using system voice," then continues in the macOS system voice. Speech is still heard, no dead air, no retry dance. They can also switch to the free Browser voices backend in Settings to get better quality than the system voice at zero cost.

### "I want zero-cost voice without any API keys"

The user selects "Browser voices" as their backend. No API key needed, no per-character cost. They pick a voice from the system library, adjust the rate, and every response is spoken via the browser's built-in synthesis with real-time word highlighting. Works offline.

### "I want to browse my conversation history by session"

The user toggles the history panel to conversation view. Messages group by session — each group shows a colored dot, the session label, and the number of messages. They collapse the sessions they don't care about and expand the one they want, then click an entry to view and optionally replay it.

## Quality and constraints

**Latency.** The user expects to hear the response within a couple of seconds of Claude finishing. The premium voice backend introduces a fetch step; this is acceptable as long as it stays under three seconds for typical responses. The browser voice backend starts immediately (no fetch). Replays from cache start within a fraction of a second.

**The waking state closes the dead-air gap.** Between "hook fired" and "audio ready," the popup already shows *something* — a breath-pulse in the session color — so the user knows a response is incoming rather than wondering if the app crashed.

**Cost discipline.** The premium voice backend charges per character. The product avoids waste by: (1) summarizing long responses before sending them to the voice engine, (2) caching every generated audio file so replays are free, (3) silently dropping responses from muted sessions before any API call is made, (4) queuing overlapping responses so two never play simultaneously (which would waste one). The free backends (system voice, browser voice) have zero per-use cost.

**Reliability through fallback.** If premium voice synthesis fails, the pipeline falls back to the system voice and surfaces a non-fatal warning. The user's message is always heard.

**Failed speech is never silent.** Errors that can't be recovered from (invalid API key, missing permission) surface visibly in the popup with the actual reason, rather than leaving the user wondering why nothing was spoken.

**Safety against prompt injection.** The summarizer treats Claude's output as *content to be rephrased*, not as instructions. Closing tags that could break out of the content boundary are escaped with zero-width spaces. If a response contains text that looks like a command to the summarizer, the summarizer ignores it.

**Privacy.** All audio, history, session preferences, and voice recordings are stored locally on the user's machine in the application support directory. The only outbound network traffic is to the voice, summarizer, and speech-to-text providers, only when those features are enabled, and only with text or audio the user is already routing through them. No analytics, no telemetry.

**Persistence.** All user state survives app restarts: preferences, session list (with colors, voice overrides, TTYs), message history, cached audio, popup size and position, history panel width, pin state, and orb style.

**Process cleanup.** When the app shuts down (including during development rebuilds), a signal handler ensures all audio child processes are terminated so orphaned speech doesn't continue after the app exits.

## Surfaces summary

| Surface | Purpose |
|---|---|
| Menu bar icon | Always visible. Click to access all controls. |
| Floating popup | Appears on waking, stays during speech. Status bar for dragging; two-column layout with history panel (left) and main content (right); resizable divider; word highlighting, links, reply input, mic button. Draggable, resizable, pinnable. |
| Minimized orb | Compact ambient mode inside a larger transparent window. Four visual styles. Glows, pulses, bounces, and emits backdrop-blur ripple rings while speaking. Draggable, clickable to expand, hover shows session label. |
| Settings window | Configuration for all options, organized by topic. Opened from tray menu or per-session shortcut. |
| Sessions submenu (in tray) | Nested submenu per session with Enabled toggle and Settings shortcut. |
| Reply input (in popup) | Text area with send button and mic button. Sends typed or transcribed prompts to the session's specific terminal tab via TTY routing. |
| Local HTTP server | Receives Claude Code's hook payload. Also exposes direct control endpoints (speak, stop, pause, resume, status) for automation. |

## CI and distribution

A GitHub Actions workflow builds the app for macOS (universal binary: arm64 + x86_64) and Windows on every push to master and pull request. Tag pushes (v*) create a draft GitHub release with the platform bundles attached. Builds are unsigned; macOS users bypass Gatekeeper via right-click → Open; Windows users accept SmartScreen.

macOS builds produce a .dmg and .app.tar.gz. Windows builds produce an MSI and NSIS installer.

## What's intentionally not in the product (today)

- **Cross-platform parity.** A Windows build exists in CI but the reply-to-terminal path is macOS-only (AppleScript). Pause/resume via process signals is Unix-only. The browser voice backend works cross-platform.
- **Local-only premium speech.** All premium voice synthesis goes through ElevenLabs. A local neural TTS option (like Kokoro or Whisper-derived models) would address privacy and cost concerns but is not built.
- **Free speech-to-text.** Voice reply currently uses ElevenLabs Scribe (paid). The browser's Web Speech Recognition API is available in the webview and could serve as a free alternative.
- **A "Claude decides what to say" mode.** Today, the product reads everything (filtered by the summarizer). It does not yet support a mode where Claude opts in by embedding markers in its response.
- **Continuous voice listening with trigger word.** Voice reply is push-to-record. A hands-free mode that listens continuously and activates on a keyword would reduce friction.
- **Multi-user / team sharing.** Settings and history are per-machine. There is no concept of sharing sessions or histories between users.
- **In-popup transcript search.** History is browsed by scrolling and clicking, not by querying.
- **Custom voice training.** The product uses third-party voices; it does not offer voice cloning workflows.
- **Terminal-scroll deep linking.** The reply input sends prompts to the session's terminal tab, but there's no "scroll to the line that corresponds to this message" — the popup is the intended reading surface.
- **Cross-device access.** The local HTTP server could serve a web client for monitoring from a phone or tablet, but no web UI is built.

## Success criteria

The product is successful when its user can:

- Stop checking the Claude Code window between responses
- Run four Claude Code sessions in parallel and always know, without looking, which one is talking
- Trust that a replay will be instant and free
- Follow up on a response by voice or text without leaving their editor
- Glance at the orb and know whether Claude is thinking, speaking, or idle
- Switch between free and premium voices in Settings without losing any other configuration
- Have ElevenLabs credits run out mid-day and not lose a beat
- Pick up the app after a restart and find everything exactly where they left it
- Forget the product is there until it speaks — and then forget it again after it stops
