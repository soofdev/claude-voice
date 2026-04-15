# Claude Voice — Product Requirements Document

## Overview

Claude Voice is a desktop companion that gives Claude Code a voice. It listens for completed responses from your Claude Code sessions and reads them aloud through high-quality speech synthesis, with a floating visual surface that highlights words as they're spoken and lets you replay anything past. It runs in the menu bar, stays out of your way, and lets you choose what you hear, when you hear it, and in whose voice.

It is designed for people who run Claude Code throughout the day and want to consume its output without staring at a terminal — while pairing, while doing other work, while away from the keyboard, or simply because hearing a thoughtful summary is faster than reading a wall of text.

## The problem

Claude Code is a powerful agent that produces a steady stream of text — often long, often dense, often arriving while you're focused elsewhere. Reading every response interrupts whatever you were doing. Glancing at the terminal misses the nuance. Running multiple Claude Code sessions in parallel makes this worse: notifications pile up and you lose track of which session needs attention.

Existing solutions (mostly small CLI scripts) read the raw response in a robotic voice and then disappear. They do not summarize, they do not let you replay, they do not distinguish between sessions, and they do not show you what was just said. Most are notification systems, not consumption surfaces.

## Who this is for

- Engineers running one or more long-lived Claude Code sessions
- People who pair-program with Claude Code and want hands-free awareness
- Developers with multi-monitor setups who want the terminal to stay quiet visually but still get audio updates
- Anyone who'd rather hear "Done — tests pass, ready for review" than scroll back to find out

## Goals

1. **Make Claude Code's output ambient.** A user should be able to start a long task, walk away, and know what happened without reading anything.
2. **Make speech listenable.** Raw assistant text is full of formatting, code blocks, and URLs that don't translate to spoken word. The product should read the *meaning*, not the *characters*.
3. **Make it easy to selectively hear sessions.** With multiple Claude Code sessions running in different projects, the user should be able to silence noisy ones without affecting the ones they care about.
4. **Make replay free and instant.** Re-listening to a past message should never require waiting for or paying for re-synthesis.
5. **Stay out of the way.** No persistent windows. No notifications stack. A menu bar icon, a popup that appears when needed and disappears when done.

## Non-goals

- This is not a Claude Code wrapper or replacement. Claude Code remains untouched; this product reads its output.
- This is not a voice input / dictation tool. It speaks what Claude says; it does not speak for you.
- This is not a general-purpose TTS app. The pipeline is tuned for assistant responses, not arbitrary text.
- This is not a notification system. The popup shows the *content*, not "you have a new response."

## Core experiences

### The speech experience

When Claude Code finishes a response, the user hears it spoken aloud in a natural-sounding voice. For long responses, the assistant's text is first rephrased by a fast model into a spoken-word version that flows naturally — no markdown read aloud, no URLs spelled out, no code blocks droning on. The user controls how aggressive that rephrasing is, from "preserve every detail" to "give me only the main point."

The user picks the voice from a curated set of presets (preconfigured, free-tier-compatible voice IDs they can use immediately) or supplies their own from a paid voice library. Two playback engines are supported: the system's built-in voice (free, always works, lower quality) and a premium AI voice (charges per character, much higher quality, comes with timing data that powers the visual surface).

### The visual surface

When speech begins, a small floating popup appears in the corner of the screen. It shows the spoken text, highlights each word as it's said, and auto-scrolls to keep the active word in view. The popup is borderless, semi-transparent, and stays on top of other windows.

The user can:
- **Drag** the popup anywhere on the screen
- **Resize** it taller or shorter to suit their layout
- **Pin** it so it stays visible after the speech ends (instead of auto-hiding)
- **Pause and resume** mid-speech, or **stop** entirely
- See **clickable link chips** below the text for any URLs the response contained, opening in their default browser
- Toggle a **history panel** inside the popup to browse and replay any past message

Pinning is persistent — once pinned, the popup stays pinned across all future responses until the user un-pins it.

### The multi-session experience

The user typically has more than one Claude Code session open: one for their main project, one for a side project, perhaps a quick experiment in a third terminal. Each session announces itself to Claude Voice when it produces a response, so the product can list every active session and let the user manage them independently.

For each session, the user can:
- **Mute** it — that session's responses will be silently dropped, no audio, no popup
- **Rename** it from the auto-generated label (the project folder name) to something memorable
- **Assign a different voice** so they can tell sessions apart by ear
- **Forget** the session entirely, removing it from their list

Session controls are accessible two ways: a full panel in the Settings window with all metadata visible, or a quick submenu in the menu bar so the user can mute a noisy session in two clicks without opening anything.

### The replay experience

Every message that gets spoken is automatically saved to a local history, capped at the most recent two hundred messages. The user can open a history panel at any time, see a list of messages with timestamps and previews, and replay any of them with one click. Replays are free — the audio is cached on disk along with its word-timing data, so a replay never costs an API call and starts instantly.

When a message ages out of the cap, its cached audio is cleaned up automatically.

### Voice control

The user controls the entire app from a menu bar icon. The tray menu offers:
- A master on/off switch for all speech
- Pause / resume / stop for the currently-playing message
- Pin popup toggle (matches the in-popup pin button state)
- Sessions submenu, one item per session, with quick mute and a shortcut to open settings focused on that session
- Settings — opens the configuration window
- Quit

The Settings window is a single scrollable surface organized by topic: speaking on/off, popup behavior, voice backend selection with backend-specific options, summarizer configuration with brevity slider, sessions panel, and the integration snippet the user copies into their Claude Code config to wire everything up.

## Key user journeys

### "I'm running Claude on a long task and want to walk away"

The user kicks off a refactor in Claude Code, then walks to the kitchen. When the response finishes a few minutes later, they hear a natural-sounding summary from the menu bar speaker: "Done. Ran the migration, tests pass, ready for you to review." If they missed something, they tap the menu bar icon, open the popup, and click Replay on the last message.

### "Two sessions are talking and I only want to hear one"

The user has the main app session and a documentation session running. The docs session is generating long-winded explanations they don't need to hear right now. They click the menu bar icon, hover over Sessions, and uncheck the docs session. It goes silent immediately. The main session keeps speaking. Later, they re-enable docs without losing any preferences.

### "I want different voices for different projects"

The user wants their two main projects to sound distinct so they don't have to look at the popup to know which is talking. In Settings they pick Preset A for one session and Preset B for the other. From then on, every response from each session uses its assigned voice — overriding the global default at the moment the response arrives.

### "A response had links I want to click"

Claude returns a response mentioning the documentation URL. The popup speaks the response and shows a row of clickable link chips below the text. The user clicks the chip; the link opens in their browser. Speech continues uninterrupted.

### "I want to hear a previous response again"

The user toggles the history panel inside the popup. They scroll back, find the response from earlier, and click Replay. The same voice plays the same message instantly — no waiting, no API charge.

## Quality and constraints

**Latency.** The user expects to hear the response within a couple of seconds of Claude finishing. The premium voice backend introduces a fetch step (the audio is generated remotely); this is measured and acceptable as long as it stays under three seconds for typical responses. Replays should start within a fraction of a second.

**Cost discipline.** The premium voice backend charges per character. The product avoids waste by: (1) summarizing long responses before sending them to the voice engine, (2) caching every generated audio file so replays are free, (3) silently dropping responses from muted sessions before any API call is made, (4) capping summary length so a runaway response does not produce a runaway bill.

**Reliability.** A failed voice synthesis must not silently fail. The popup surfaces playback errors visibly so the user knows when something went wrong (e.g. "your voice ID requires a paid plan") rather than wondering why nothing was spoken.

**Safety against prompt injection.** The summarizer treats Claude's output as *content to be rephrased*, not as instructions. If a response contains text that looks like a command to the summarizer, the summarizer ignores it and rephrases it as ordinary text. This matters because Claude Code's responses can include arbitrary text from arbitrary tools and files.

**Privacy.** All audio, history, and session preferences are stored locally on the user's machine. The only outbound network traffic is to the voice and summarizer providers, only when those features are enabled, and only with text the user is already routing to Claude. No analytics, no telemetry.

**Persistence.** The user's preferences, session list, message history, and cached audio survive app restarts and OS reboots. Restarting the app picks up exactly where it left off, including pin state, muted sessions, and per-session voice overrides.

## Surfaces summary

| Surface | Purpose |
|---|---|
| Menu bar icon | Always visible. Click to access all controls. |
| Floating popup | Appears during speech. Shows text, words highlighted, controls, links, history panel. Draggable, resizable, pinnable. |
| Settings window | Configuration for all options. Opened from menu. |
| Sessions submenu (in tray) | Quick mute / focus per session without opening settings. |
| Local HTTP endpoint | Receives the assistant's response from Claude Code's hook system. Invisible to the user. |

## What's intentionally not in the product (today)

- **Cross-platform builds.** Currently macOS-only because of menu bar conventions and audio playback assumptions. Windows is achievable but unscoped.
- **Local-only speech.** All premium voice synthesis goes through a third-party API. A local on-device voice option would address privacy and cost concerns but is not built.
- **A "Claude decides what to say" mode.** Today, the product reads everything (filtered by the summarizer). It does not yet support a mode where Claude opts in by embedding markers in its response.
- **Multi-user / team sharing.** Settings and history are per-machine. There is no concept of sharing sessions or histories between users.
- **In-popup transcript search.** History is browsed by scrolling, not by querying.
- **Custom voice training.** The product uses third-party voices; it does not offer voice cloning workflows.

## Success criteria

The product is successful when its user can:
- Stop checking the Claude Code window between responses
- Run multiple Claude Code sessions without going crazy from the audio
- Trust that a replay will be instant and free
- Pin a session's response and read it later without having to scroll back through their terminal
- Forget the product is there until it speaks
