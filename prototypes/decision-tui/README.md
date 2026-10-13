# decision-tui — THROWAWAY PROTOTYPE (wayfinder ticket #7)

Rough Ratatui prototype of Forge's developer-decision interaction, built to be
reacted to and then thrown away. It scripts the §16 loop's central moment:
routine agent work → a consequential decision pauses execution → the developer
decides (options, trade-offs, risks, warning acknowledgments, amendments) →
the record materializes → execution resumes → a later boundary attempt is
hard-blocked.

Scenario content (payments idempotency) is placeholder material — the real demo
scenario is settled by ticket #13.

**Base: Focus.** The decision interaction is a full-screen takeover in a single
bordered document — one thing on the screen at a time. (`[` / `]` still cycles
to the Inline and Mission variants, kept as primary sources, not the base.)

**Viewer (`v`).** Everything the run writes is displayed in the same takeover
language, as full-screen pages you flip with `←→`:

- the record doc the decision is writing — canonical ADR shape, tracking the
  highlighted option live, byte-for-byte what gets written on confirm;
- the agent's file writes — code diffs of the edits it made during the run;
- the denied write at a hard block — the diff the agent tried to make to the
  accepted record when the gate stopped it (PRD §8.2).

**Amendments.** After the warning acks, the record text is directly editable:
your wording is what the core writes, with an optional rationale note journaled
alongside.

Keys: `[` / `]` switch variant · `space` advance the scripted run · `ctrl+r`
replay · `v` open the viewer (`←→` page, `↑↓` scroll) · `h` open the task
journal · `q` quit. Decisions: `↑↓` / `j k` move, `1-5` select, `enter`
confirm, `space` acknowledges warnings. Amendments: `↑↓`/`tab` switch fields,
typing edits the focused field, `ctrl+u` clears it, `enter` submits. `d`
(inline variant) expands option details.

Run it (Windows Terminal recommended):

```
cargo run
```

Static renders of every variant × state (no TTY needed):

```
cargo run -- --snapshot
```
