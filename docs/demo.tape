# questa demo
# Generates docs/screenshots/demo.gif
# Run from the repo root:
#   cargo build --release
#   vhs docs/demo.tape

Output docs/screenshots/demo.gif

# ── Terminal settings ────────────────────────────────────────────────────
Set Shell        "bash"
Set FontSize     14
Set Width        1280
Set Height       720
Set Theme        "Catppuccin Mocha"
Set TypingSpeed  55ms
Set PlaybackSpeed 1.0

# ── Launch ───────────────────────────────────────────────────────────────
Type "./target/release/questa"
Enter
Sleep 1.5s

# ── Browse the list ──────────────────────────────────────────────────────
Down 3
Sleep 700ms
Up 1
Sleep 600ms

# ── Filter: all, then back to active ─────────────────────────────────────
Type "1"
Sleep 1.2s
Type "2"
Sleep 900ms

# ── Cycle sort: date → status → company → date ───────────────────────────
Type "o"
Sleep 1.1s
Type "o"
Sleep 1.1s
Type "o"
Sleep 800ms

# ── Fuzzy search ─────────────────────────────────────────────────────────
Type "/"
Sleep 200ms
Type "robot"
Sleep 1.5s
Escape
Sleep 600ms

# ── Change status of the selected entry ─────────────────────────────────
Type "s"
Sleep 700ms
Down 2
Sleep 500ms
Enter
Sleep 1.2s

# ── Add a note ───────────────────────────────────────────────────────────
Type "n"
Sleep 500ms
Type "follow up next monday"
Sleep 800ms
Enter
Sleep 1.2s

# ── Help overlay ─────────────────────────────────────────────────────────
Type "?"
Sleep 2.5s
Type "?"
Sleep 500ms

# ── Quit ─────────────────────────────────────────────────────────────────
Type "q"
Sleep 600ms
