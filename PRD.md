# Claude Voice — Product Requirements Document

## Overview

Claude Voice is a desktop companion that gives Claude Code a voice — and a face. It listens for completed responses from your Claude Code sessions, reads them aloud through high-quality speech synthesis, and presents them through a floating visual surface that highlights words as they're spoken, glows and pulses with personality while speaking, and lets you talk back. It runs in the menu bar, stays out of your way, and lets you decide what you hear, when you hear it, in whose voice, and how you respond.

It is designed for people who run Claude Code throughout the day and want to consume its output without staring at a terminal — while pairing, while doing other work, while away from the keyboard, or simply because hearing a thoughtful summary is faster than reading a wall of text. And because you often want to immediately say something *back* to Claude, the product makes replying — by text or by voice — a one-button action that lands in the right terminal without you having to switch windows.

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
3. **Give each session a distinct identity.** With multiple Claude Code sessions running, the user should know at a glance, at an earshot, or from a peripheral vision which one is talking.
4. **Make it easy to selectively hear sessions.** Silence the noisy ones without affecting the ones you care about.
5. **Make replay free and instant.** Re-listening to a past message should never require waiting for or paying for re-synthesis.
6. **Let the user reply without switching windows.** Text and voice replies should land in the right terminal and take seconds.
7. **Stay beautiful.** The popup is visible a lot. It should feel alive, tasteful, magical — not utilitarian.
8. **Stay out of the way when you want it to.** A menu bar icon, a popup that appears when needed, a small orb mode for when you want presence but not content.
9. **Never fail silently.** If the premium voice fails, fall back to something audible rather than leaving the user wondering.

## Non-goals

- This is not a Claude Code wrapper or replacement. Claude Code remains untouched; this product reads its output and routes replies back.
- This is not a general-purpose TTS app. The pipeline is tuned for assistant responses, not arbitrary text.
- This is not a notification system. The popup shows the *content*, not "you have a new response."
- This is not a dictation app for long-form writing. The voice reply is for short, conversational replies to Claude.

## Core experiences

### The speech experience

When Claude Code finishes a response, the user hears it spoken aloud in a natural-sounding voice. For long responses, the assistant's text is first rephrased by a fast model into a spoken-word version that flows naturally — no markdown read aloud, no URLs spelled out, no code blocks droning on. The user controls how aggressive that rephrasing is, from "preserve every detail" to "give me only the main point" via a brevity setting in the popup settings.

The user picks the voice from a curated set of presets (preconfigured, free-tier-compatible voice IDs they can use immediately) or supplies their own from a paid voice library. Two playback engines are supported: the system's built-in voice (free, always works, lower quality) and a premium AI voice (per-character pricing, much higher quality, word-level timing data that powers the visual surface). The premium path gracefully falls back to the system voice if the API fails or credits run out — the user always hears *something*.

### The popup: three visible states

The popup has a deliberate state machine that mirrors what the product is doing:

- **Idle.** Hidden. No window, no sound.
- **Waking.** The moment a message arrives and rephrasing begins, the popup appears with a gentle breath-pulse in the session's color and a "preparing…" title. This closes the 1–3s gap between hook-received and audio-ready that would otherwise be dead air with nothing to look at.
- **Speaking.** When the audio starts, the popup transitions: words light up one at a time with the active word highlighted in the session color, the text auto-scrolls to keep it in view, the border pulses with a session-colored halo, and the whole card does a subtle vertical bounce in rhythm with the speech.
- **Fading.** After speech ends, the popup optionally stays visible for a configurable delay then fades and scales out with a smooth transition.

This state machine is small but gives the app a sense of life. The user's peripheral vision picks up "something's incoming" before they hear anything.

### The visual surface

Inside the speaking popup, the user can:

- **Drag** the popup anywhere on the screen via its header; it remembers position.
- **Resize** it taller or shorter to suit their layout; hard minimum prevents collapse to nothing.
- **Pin** it so it stays visible after the speech ends (instead of auto-hiding). Pinning is persistent across sessions until un-pinned.
- **Pause, resume, stop** the currently-playing message.
- **See clickable link chips** below the text for any URLs the response contained — the links are extracted, validated, and rendered as pill buttons that open in the default browser. Spoken speech refers to them naturally ("see the github dot com link") rather than spelling out URLs.
- **Toggle between spoken and original.** The default view shows the rephrased spoken text with word highlighting. One click (or the T key) swaps to the full raw original in a monospace view, for when the summary skipped something.
- **Open a history panel** inside the popup to browse and replay past messages.

### The minimized orb: ambient presence

The user can collapse the full popup to a **minimized orb** — a 56px colored sphere living inside a larger transparent window. The orb is the ambient mode: it signals that the session is alive without taking up real screen space or demanding reading.

While speaking, the orb:
- **Pulses** with a halo in the session's color
- **Bounces** subtly in rhythm
- **Emits ripples** outward — expanding blur rings that create a shimmer distortion effect on whatever content is behind the popup window, as if the orb is giving off energy

When speech ends, the orb stays put; ripples and bounce stop. The orb's job in that idle state is to reassure: "I'm here, waiting."

The user can pick from several **orb styles**: glass (polished sphere with highlight), plasma (rotating nebula inside a dark sphere), iridescent (pearly hue-shifting sheen), or energy (bright core with stronger ripples). Styles are set in Settings and apply live.

The orb is draggable (long press + move), clickable to expand back to the full popup (short tap), and shows the session label on hover.

### The multi-session experience

The user typically has more than one Claude Code session open — main project, side project, quick experiment. Each session announces itself to Claude Voice when it produces a response, so the product can list every active session and treat them independently.

For each session, the user can:

- **Mute** it — that session's responses are silently dropped before the TTS call, no audio, no popup, no API cost
- **Rename** it from the auto-generated label (the project folder name) to something memorable
- **Assign a session color** from a palette (auto-picked on first sighting, swatch-editable). The color appears in the popup border, dot, title tint, orb, and halo so the user can identify the session visually at a glance.
- **Assign a different voice** so they can tell sessions apart by ear
- **Forget** the session entirely, removing it from their list

On top of visual and voice identity, there's a **spoken session prefix**: the TTS reads "{session label} says: …" at the start of a message so the user knows who's talking even if they can't see the popup. A smart-skip rule suppresses the prefix when the same session spoke within a configurable window (default 30s), so a chatty session doesn't announce itself every two seconds.

Session controls are accessible two ways:
- A full panel in the Settings window with all metadata visible, color swatch, voice override, and on/off toggle
- A **tray submenu** per session in the menu bar with Enabled toggle and a Settings… shortcut that opens the full panel and highlights the relevant row

### The reply experience

After Claude finishes, the user often wants to say something back. The popup exposes a compact text area and a mic button directly below the message.

**Text reply.** The user types, hits Cmd+Enter (or clicks send). The prompt is routed to the active terminal via OS-level scripting: iTerm2 gets the message via clean API injection; Terminal.app via activate + keystrokes. No context switching, no hunting for the right tab.

**Voice reply.** The user clicks the mic. The popup records audio from the microphone and posts it to a speech-to-text service; the transcribed text drops into the text area for review. The user can edit and then send (or configure to auto-send when transcription completes). Visual feedback during recording: a pulsing red mic button with clear states (idle, recording, uploading).

Reply surfaces are platform-dependent today (macOS only) and require a one-time Accessibility permission for terminals without scriptable APIs.

### The replay experience

Every message that gets spoken is automatically saved to a local history, capped at the most recent two hundred messages. The user can open a history panel inside the popup at any time, see a list of messages with timestamps and previews, and replay any of them with one click. Replays are **free and instant** — the audio is cached on disk along with its word-timing data, so a replay never costs an API call and the voice/visualization match exactly what the user heard the first time.

When a message ages out of the cap, its cached audio is cleaned up automatically.

### Voice control

The user controls the entire app from a menu bar icon. The tray menu offers:

- A master on/off switch for all speech
- Pause / resume / stop for the currently-playing message
- Pin popup toggle (matches the in-popup pin button state)
- Sessions submenu, one item per session, each containing Enabled toggle + Settings… shortcut
- Settings — opens the configuration window
- Quit

The Settings window is a single scrollable surface organized by topic: speaking on/off, popup behavior and auto-hide delay, orb style, voice backend selection with backend-specific options, summarizer configuration with brevity levels, sessions panel with per-session voice and color overrides, and the integration snippet the user copies into their Claude Code config to wire everything up.

## Key user journeys

### "I'm running Claude on a long task and want to walk away"

The user kicks off a refactor in Claude Code, then walks to the kitchen. When the response finishes a few minutes later, they hear a natural-sounding summary: "Wodworx says: done. Ran the migration, tests pass, ready for you to review." If they missed something, they click the menu bar icon, open the popup, toggle the history panel, and click Replay on the last message.

### "Two sessions are talking and I only want to hear one"

The user has the main app session and a documentation session running. The docs session is generating long-winded explanations they don't need to hear right now. They click the menu bar icon, hover over Sessions, navigate to Docs, and uncheck Enabled. It goes silent immediately. The main session keeps speaking. Later, they re-enable docs without losing any preferences.

### "I want to know which session is speaking without looking up"

The user assigns a different color and voice to each of their three sessions. From then on, each response arrives in that session's voice, with its colored popup border, halo, and orb — and starts with a spoken prefix of the session label. The user learns the sound and color of each session within a day and stops needing to look.

### "I want to work alongside Claude but keep my screen clean"

The user minimizes the popup to the orb. While Claude is preparing a response, the orb breathes gently in the session color. While speaking, the orb pulses, bounces, and emits energy ripples that distort the content behind it. When speech ends, the orb stays put but goes still. The user knows at a glance: "Claude is thinking / speaking / idle" without reading anything.

### "A response had links I want to click"

Claude returns a response mentioning the documentation URL. The popup speaks the response and shows a row of clickable link chips below the text. The user clicks the chip; the link opens in their browser. Speech continues uninterrupted.

### "I want to hear a previous response again"

The user opens the history panel inside the popup. They scroll back, find the response from earlier, and click Replay. The same voice plays the same message instantly — no waiting, no API charge, same word-level highlighting.

### "I want to follow up without leaving my editor"

Claude finishes speaking: "I found three failing tests; the retry logic is wrong." The user clicks the mic button in the popup, says "fix the retry logic and re-run the tests," and hits send. The text is transcribed, dropped into the popup's input, and routed to the correct terminal — Claude picks up the follow-up and starts working. The user never left their editor.

### "I want to see what Claude actually wrote, not the summary"

The user heard a summary but wants the full original. They click the toggle icon in the popup (or press T). The view swaps to the raw response in monospace. They read, then hit T again to return to the spoken view for replay.

### "ElevenLabs credits ran out mid-day"

The user exhausts their ElevenLabs quota. The next response: the popup flashes briefly with "ElevenLabs unavailable — using system voice," then continues in the macOS `say` voice. Speech is still heard, no dead air, no retry dance.

## Quality and constraints

**Latency.** The user expects to hear the response within a couple of seconds of Claude finishing. The premium voice backend introduces a fetch step (the audio is generated remotely); this is measured and acceptable as long as it stays under three seconds for typical responses. Replays start within a fraction of a second. Voice-reply transcription returns in 1–2 seconds for typical short prompts.

**The waking state closes the dead-air gap.** Between "hook fired" and "audio ready," the popup already shows *something* — a breath-pulse in the session color — so the user knows a response is incoming rather than wondering if the app crashed.

**Cost discipline.** The premium voice backend charges per character. The product avoids waste by: (1) summarizing long responses before sending them to the voice engine, (2) caching every generated audio file so replays are free, (3) silently dropping responses from muted sessions before any API call is made, (4) queuing overlapping responses from the same or different sessions so two never play over each other (which would have cost two API calls for one outcome). Session mute is cost-efficient: the API is never called for muted sessions.

**Reliability through fallback.** If premium voice synthesis fails (quota, rate limit, server error, network), the pipeline falls back to the system voice and surfaces a non-fatal warning in the popup rather than going silent. The user's message is always heard.

**Failed speech is never silent.** Errors that can't be recovered from (invalid API key, missing permission) surface visibly in the popup with the actual reason, rather than leaving the user wondering why nothing was spoken.

**Safety against prompt injection.** The summarizer treats Claude's output as *content to be rephrased*, not as instructions. If a response contains text that looks like a command to the summarizer, the summarizer ignores it and rephrases it as ordinary text. This matters because Claude Code's responses can include arbitrary text from arbitrary tools and files.

**Privacy.** All audio, history, session preferences, and voice recordings are stored locally on the user's machine. The only outbound network traffic is to the voice, summarizer, and speech-to-text providers, only when those features are enabled, and only with text or audio the user is already routing through them. No analytics, no telemetry.

**Persistence.** The user's preferences, session list, session colors, message history, cached audio, and voice overrides survive app restarts and OS reboots. Restarting the app picks up exactly where it left off, including pin state, muted sessions, orb style, and per-session voice assignments.

## Surfaces summary

| Surface | Purpose |
|---|---|
| Menu bar icon | Always visible. Click to access all controls. |
| Floating popup | Appears on waking, stays during speech. Shows text, word highlighting, links, reply input, mic button, history panel toggle, minimize toggle. Draggable, resizable, pinnable. |
| Minimized orb | Compact ambient mode inside a larger transparent window. Glows, pulses, bounces, and emits backdrop-blur ripple rings while speaking. Draggable, clickable to expand, hover shows session label. |
| Settings window | Configuration for all options, organized by topic. Opened from tray or a per-session shortcut. |
| Sessions submenu (in tray) | Nested submenu per session with Enabled toggle and Settings shortcut. |
| Reply input (in popup) | Text area with send button and mic button. Sends typed or transcribed prompts to the active terminal. |
| Local HTTP endpoint | Receives the assistant's response from Claude Code's hook system. Invisible to the user. |

## What's intentionally not in the product (today)

- **Cross-platform parity.** A Windows build exists in CI but the reply-to-terminal and voice-input paths are macOS-only (AppleScript, `say`, `afplay`). Windows users can still hear speech; reply and voice-in need additional work.
- **Local-only speech.** All premium voice synthesis and speech-to-text go through third-party APIs. A local on-device voice / STT option would address privacy and cost concerns but is not built.
- **A "Claude decides what to say" mode.** Today, the product reads everything (filtered by the summarizer). It does not yet support a mode where Claude opts in by embedding markers in its response.
- **Multi-user / team sharing.** Settings and history are per-machine. There is no concept of sharing sessions or histories between users.
- **In-popup transcript search.** History is browsed by scrolling, not by querying.
- **Custom voice training.** The product uses third-party voices; it does not offer voice cloning workflows.
- **Terminal-scroll deep linking.** The reply input sends prompts to the active terminal, but there's no "jump to the terminal tab that owns this session and scroll to this message" — the popup is the intended reading surface.

## Success criteria

The product is successful when its user can:

- Stop checking the Claude Code window between responses
- Run four Claude Code sessions in parallel and always know, without looking, which one is talking
- Trust that a replay will be instant and free
- Follow up on a response by voice or text without leaving their editor
- Glance at the orb and know whether Claude is thinking, speaking, or idle
- Have ElevenLabs credits run out mid-day and not lose a beat
- Forget the product is there until it speaks — and then forget it again after it stops
