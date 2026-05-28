# questa demo
# Generates docs/screenshots/demo.gif
#
# Run from the repo root:
#   cargo build --release
#   cp examples/applications.json /tmp/questa-demo.json
#   vhs docs/demo.tape
#   rm -rf /tmp/questa-demo.json /tmp/exports
#
# The demo runs against a throwaway copy of the bundled dataset so it
# does not mutate examples/applications.json.

Output docs/screenshots/demo.gif

# ── Terminal settings ────────────────────────────────────────────────────
Set Shell         "bash"
Set FontSize      14
Set Width         1280
Set Height        720
Set Theme         "Catppuccin Mocha"
Set TypingSpeed   55ms
Set PlaybackSpeed 1.0

# ── Launch ───────────────────────────────────────────────────────────────
Type "./target/release/questa --data /tmp/questa-demo.json"
Enter
Sleep 1.8s

# ── Browse the list ──────────────────────────────────────────────────────
Down 3
Sleep 500ms
Up 1
Sleep 500ms

# ── Filters ──────────────────────────────────────────────────────────────
Type "1"
Sleep 900ms
Type "3"
Sleep 1.1s
Type "2"
Sleep 800ms

# ── Sort cycle ───────────────────────────────────────────────────────────
Type "o"
Sleep 900ms
Type "o"
Sleep 900ms
Type "o"
Sleep 600ms

# ── Fuzzy search ─────────────────────────────────────────────────────────
Type "/"
Sleep 200ms
Type "robot"
Sleep 1.2s
Escape
Sleep 500ms

# ── Add a new application via the form ───────────────────────────────────
Type "a"
Sleep 700ms
Type "Quantum Robotics"
Sleep 250ms
Tab
Type "Senior Rust Engineer"
Sleep 250ms
Tab
Type "Remote"
Sleep 250ms
Tab
Type "full-time"
Sleep 300ms
Enter
Sleep 1.5s

# ── Edit the new entry: jump to top (date desc), change status field ────
Type "g"
Sleep 300ms
Type "e"
Sleep 700ms
# Step to the Status field (9 fields away from Company).
Tab 9
Sleep 300ms
Ctrl+u
Type "interview"
Sleep 400ms
Enter
Sleep 1.5s

# ── Quick status change via the picker on a different entry ─────────────
Down 2
Sleep 400ms
Type "s"
Sleep 600ms
Down 2
Sleep 400ms
Enter
Sleep 1.2s

# ── Append a note dated today ────────────────────────────────────────────
Type "n"
Sleep 500ms
Type "Recruiter screen booked for next Monday"
Sleep 500ms
Enter
Sleep 1.2s

# ── Append a contact ─────────────────────────────────────────────────────
Type "c"
Sleep 500ms
Type "Jordan Lee — Hiring Manager"
Sleep 500ms
Enter
Sleep 1.2s

# ── Delete with confirm, then undo to recover ───────────────────────────
Type "G"
Sleep 400ms
Type "d"
Sleep 900ms
Type "y"
Sleep 1.2s
Type "u"
Sleep 1.3s

# ── PDF export via Typst ────────────────────────────────────────────────
Type "x"
Sleep 1.8s

# ── Help overlay ─────────────────────────────────────────────────────────
Type "?"
Sleep 3s
Type "?"
Sleep 500ms

# ── Quit ─────────────────────────────────────────────────────────────────
Type "q"
Sleep 600ms
