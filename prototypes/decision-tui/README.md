# decision-tui — THROWAWAY PROTOTYPE (wayfinder ticket #7)

Rough Ratatui prototype of Forge's developer-decision interaction, built to be
reacted to and then thrown away. It scripts the §16 loop's central moment:
routine agent work → a consequential decision pauses execution → the developer
decides (options, trade-offs, risks, warning acknowledgments, optional
rationale) → the record materializes → execution resumes → a later boundary
attempt is hard-blocked.

Scenario content (payments idempotency) is placeholder material — the real demo
scenario is settled by ticket #13.

Three structurally different variants of the same flow, switch with `[` / `]`:

1. **Focus** — the decision takes the whole screen (modal takeover).
2. **Inline** — the decision lives inside the agent's activity stream.
3. **Mission** — persistent panes: activity left, decision pane right, task
   journal on `h`.

Iteration 4: Mission is the base, with a **content inspector** (`p`) and
**editable amendments**. The inspector shows the canonical ADR the decision is
writing (tracking the highlighted option, byte-for-byte what gets written on
confirm) and the blocked write — the diff the agent tried to make to the
accepted record when the gate denied it (PRD §8.2); `←→` switches docs. The
amendments stage makes the record text itself editable: your wording is what
the core writes, with an optional rationale note journaled alongside.

Keys: `[` / `]` switch variant · `space` advance the scripted run · `ctrl+r`
replay · `h` open the task journal · `p` open the content inspector
(`←→` switch doc, `↑↓` scroll) · `q` quit. Decisions: `↑↓` / `j k` move,
`1-5` select, `enter` confirm, `space` acknowledges warnings. Amendments:
`↑↓`/`tab` switch fields, typing edits the focused field, `ctrl+u` clears it,
`enter` submits. `d` (inline variant) expands option details.

Run it (Windows Terminal recommended):

```
cargo run
```

Static renders of every variant × state (no TTY needed):

```
cargo run -- --snapshot
```
