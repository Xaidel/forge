// Throwaway prototype state for wayfinder ticket #7 (TUI decision interaction).
// Scenario: a small payments service; the agent runs a routine task, pauses at a
// consequential decision, the developer decides, and a later boundary attempt is
// hard-blocked. Content is placeholder material -- the real demo scenario is #13's.

use std::time::Instant;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Running,
    Paused,
    Blocked,
    Complete,
    Aborted,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Task,
    Info,
    Read,
    Edit,
    Test,
    Decision,
    Ack,
    Grant,
    Block,
    Done,
}

#[derive(Clone)]
pub struct LogEvent {
    pub kind: Kind,
    pub text: String,
}

fn ev(kind: Kind, text: impl Into<String>) -> LogEvent {
    LogEvent { kind, text: text.into() }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Choice {
    Option(usize),
    Reject,
    Abort,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum VStage {
    Selecting,
    Acking,
    Rationale,
}

#[derive(Clone)]
pub struct Verdict {
    pub stage: VStage,
    pub highlight: usize,
    pub choice: Option<Choice>,
    pub acked: usize,
    pub rationale: String,
}

impl Verdict {
    fn new() -> Self {
        Verdict {
            stage: VStage::Selecting,
            highlight: 1,
            choice: None,
            acked: 0,
            rationale: String::new(),
        }
    }
}

#[derive(Clone, Copy)]
pub struct DecOption {
    pub label: &'static str,
    pub gist: &'static str,
    pub tradeoffs: &'static [&'static str],
    pub risks: &'static [&'static str],
    pub consequences: &'static str,
    pub recommended: bool,
    /// What the record says if this option wins (#4 canonical ADR shape).
    pub record_title: &'static str,
    pub decision_text: &'static str,
    /// Why this option lost, when it appears under Alternatives Considered.
    pub why_not: &'static str,
}

pub const STATEMENT: &str = "How should payment-capture idempotency keys be stored?";
pub const CONTEXT: &str = "Retries are wired up, but a retried capture can still post twice unless the \
key store survives crashes and redeploys. Storage choice fixes the failure behavior, the ops \
burden, and whether the schema changes.";

pub const OPTIONS: &[DecOption] = &[
    DecOption {
        label: "In-memory idempotency map",
        gist: "zero deps, fastest",
        tradeoffs: &[
            "no new dependencies; contained to one module",
            "trivial to implement and unit-test",
        ],
        risks: &[
            "keys vanish on restart -- the crash window still double-charges",
            "process-local: breaks under a second instance",
        ],
        consequences: "captures dedupe via an in-process map; keys do not survive restarts.",
        recommended: false,
        record_title: "In-memory idempotency keys for payment capture",
        decision_text: "Payment-capture idempotency keys are held in an in-process map keyed by the \
            client request key. Keys are process-local: they do not survive restarts or scale-out.",
        why_not: "keys are lost on restart -- the crash window still double-charges",
    },
    DecOption {
        label: "Idempotency table + unique constraint",
        gist: "durable, single source of truth",
        tradeoffs: &[
            "survives restarts and scale-out",
            "the unique constraint makes duplicates impossible at the storage layer",
        ],
        risks: &["a migration touches a table carrying production traffic"],
        consequences: "captures dedupe on a persisted unique key; duplicates fail the constraint.",
        recommended: true,
        record_title: "Persist payment-capture idempotency keys in a constrained table",
        decision_text: "Payment-capture idempotency keys are persisted in a dedicated table with a \
            unique constraint. Duplicate capture attempts fail the constraint instead of posting \
            a second charge.",
        why_not: "adds a migration touching a table with production traffic",
    },
    DecOption {
        label: "Redis with a 24h TTL",
        gist: "fast, externalized",
        tradeoffs: &["no relational schema change", "TTL expiry is automatic"],
        risks: &[
            "adds Redis to the runtime and ops surface",
            "a cache flush silently re-opens the dedupe window",
        ],
        consequences: "captures dedupe in Redis; the capture path depends on the cache being up.",
        recommended: false,
        record_title: "Keep payment-capture idempotency keys in Redis with a TTL",
        decision_text: "Payment-capture idempotency keys live in Redis with a 24-hour TTL; the \
            capture path depends on the cache being available.",
        why_not: "adds Redis as a runtime dependency; eviction can silently drop keys",
    },
];

pub const WARNINGS: &[&str] = &[
    "migration touches a table with production traffic -- needs a maintenance window",
    "expands task scope -- the original task did not include a schema migration",
];

pub const AFFECTED: &[&str] = &["src/payments/** -- write", "migrations/** -- write"];
pub const ARTIFACT: &str = "ADR -- why / structural (storage strategy)";

pub const RECORD_ID: &str = "ADR-0007";
pub const RECORD_FILE: &str =
    "spec/adr/accepted/architecture/2026-10-14-idempotent-capture-storage.md";
pub const RECORD_DATE: &str = "2026-10-14";
pub const RECORD_CLASSIFICATION: &str = "architecture";
pub const RECORD_DRIVERS: &[&str] = &[
    "duplicate captures must be impossible across restarts and redeploys",
    "failure behavior must be deterministic and observable",
    "operational cost must stay proportionate to the money at risk",
];
pub const RECORD_CONFIRMATION: &str =
    "The sample-repo walkthrough replays a capture request after a simulated restart; exactly one charge posts.";

pub const BLOCK_ATTEMPT: &str = "write spec/adr/accepted/architecture/2026-10-14-idempotent-capture-storage.md";
pub const BLOCK_REASON: &str =
    "agent writes under /spec/** are never allowed -- records are core-written from the approved verdict";
pub const BLOCK_STEER: &str = "implement to the accepted TDD; the record is not agent-editable";

/// The content the agent tried to write, as a diff for the inspector.
/// Finishing implementation, it tried to bring the accepted record into line
/// with its code (PRD §8.2) -- denied before anything was written.
pub const BLOCK_DIFF: &[(&str, &str)] = &[
    ("@@", "header"),
    (" ", "ID: ADR-0007"),
    (
        "+",
        "Implementation: src/payments/capture.rs · migrations/20261014_add_idempotency_keys.sql",
    ),
    ("@@", "## Decision"),
    (
        "-",
        "Payment-capture idempotency keys are persisted in a dedicated table with a unique constraint.",
    ),
    (
        "+",
        "Payment-capture idempotency keys are persisted in a dedicated table with a unique constraint, using a partial index on (account_id, idempotency_key) so soft-deleted captures are excluded.",
    ),
];

pub enum Beat {
    Log(Kind, &'static str),
    Raise,
    Backstop,
    Complete,
}

pub const SCRIPT: &[Beat] = &[
    Beat::Log(Kind::Task, "task accepted -- \"make payment capture retry-safe\""),
    Beat::Log(Kind::Read, "read src/payments/capture.rs (212 lines)"),
    Beat::Log(Kind::Read, "read src/payments/mod.rs, src/http/routes.rs"),
    Beat::Log(Kind::Info, "grep \"capture(\" -- 9 call sites, all via PaymentClient"),
    Beat::Log(Kind::Info, "plan: wrap capture in retry -- duplicates need crash-safe idempotency keys"),
    Beat::Log(Kind::Edit, "edit src/payments/retry.rs (+38 -4)"),
    Beat::Log(Kind::Test, "cargo test -- 12 passed, 2 failed (duplicate capture on retry)"),
    Beat::Raise,
    Beat::Log(Kind::Edit, "edit src/payments/capture.rs (+41 -9) -- follows the accepted decision"),
    Beat::Log(Kind::Test, "cargo test -- 14 passed, 0 failed"),
    Beat::Backstop,
    Beat::Complete,
];

pub fn choice_at(idx: usize) -> Choice {
    if idx < OPTIONS.len() {
        Choice::Option(idx)
    } else if idx == OPTIONS.len() {
        Choice::Reject
    } else {
        Choice::Abort
    }
}

pub struct App {
    pub variant: usize,
    pub phase: Phase,
    pub beat: usize,
    pub events: Vec<LogEvent>,
    pub verdict: Option<Verdict>,
    pub journal: Vec<LogEvent>,
    pub show_journal: bool,
    pub show_preview: bool,
    pub preview_scroll: u16,
    pub preview_target: usize,
    pub chosen: Option<usize>,
    pub amended: Option<String>,
    pub amend_field: usize,
    pub note: String,
    pub v2_detail: bool,
    pub quit: bool,
    pub started: Instant,
    pub last_tick: Instant,
}

impl App {
    pub fn new() -> Self {
        App {
            variant: 0,
            phase: Phase::Running,
            beat: 0,
            events: Vec::new(),
            verdict: None,
            journal: Vec::new(),
            show_journal: false,
            show_preview: false,
            preview_scroll: 0,
            preview_target: 0,
            chosen: None,
            amended: None,
            amend_field: 0,
            note: String::new(),
            v2_detail: false,
            quit: false,
            started: Instant::now(),
            last_tick: Instant::now(),
        }
    }

    pub fn new_with_variant(variant: usize) -> Self {
        let mut app = Self::new();
        app.variant = variant;
        app
    }

    pub fn elapsed(&self) -> String {
        let s = self.started.elapsed().as_secs();
        format!("{:02}:{:02}", s / 60, s % 60)
    }

    pub fn accepted_count(&self) -> usize {
        self.events
            .iter()
            .filter(|e| e.kind == Kind::Grant && e.text.starts_with("verdict:"))
            .count()
    }

    pub fn block_count(&self) -> usize {
        self.events.iter().filter(|e| e.kind == Kind::Block).count()
    }

    pub fn advance(&mut self) {
        if self.phase != Phase::Running {
            return;
        }
        let Some(beat) = SCRIPT.get(self.beat) else { return };
        self.beat += 1;
        match beat {
            Beat::Log(kind, text) => self.events.push(ev(*kind, *text)),
            Beat::Raise => {
                self.phase = Phase::Paused;
                self.events
                    .push(ev(Kind::Decision, "decision required -- agent called request_decision"));
                self.verdict = Some(Verdict::new());
            }
            Beat::Backstop => {
                self.phase = Phase::Blocked;
                self.events
                    .push(ev(Kind::Block, format!("HARD BLOCK -- attempt: {BLOCK_ATTEMPT} -- denied")));
                self.journal.push(ev(
                    Kind::Block,
                    format!("hard block: {BLOCK_ATTEMPT} denied -- records are core-written"),
                ));
            }
            Beat::Complete => {
                self.phase = Phase::Complete;
                self.events.push(ev(
                    Kind::Done,
                    format!("task complete -- {RECORD_ID} accepted, warnings acked, 1 boundary attempt steered"),
                ));
                self.journal.push(ev(Kind::Done, "task complete"));
            }
        }
    }

    pub fn choose(&mut self, choice: Choice) {
        self.amended = None;
        match choice {
            Choice::Option(_) => {
                let Some(v) = self.verdict.as_mut() else { return };
                v.choice = Some(choice);
                v.stage = if WARNINGS.is_empty() { VStage::Rationale } else { VStage::Acking };
                if v.stage == VStage::Rationale {
                    self.amend_field = 0;
                }
            }
            Choice::Reject => {
                self.events.push(ev(Kind::Info, "reject -- replan requested, no record written"));
                self.journal
                    .push(ev(Kind::Decision, "proposal rejected -- replan requested"));
                self.events
                    .push(ev(Kind::Decision, "revised proposal (replan round) -- decision required"));
                self.verdict = Some(Verdict::new());
            }
            Choice::Abort => {
                self.events
                    .push(ev(Kind::Info, "task aborted -- grants released, no record written"));
                self.journal.push(ev(Kind::Decision, "task aborted by developer"));
                self.phase = Phase::Aborted;
                self.verdict = None;
            }
        }
    }

    pub fn acknowledge(&mut self) {
        let Some(v) = self.verdict.as_mut() else { return };
        if v.stage != VStage::Acking || v.acked >= WARNINGS.len() {
            return;
        }
        let warning = WARNINGS[v.acked];
        v.acked += 1;
        let all_acked = v.acked >= WARNINGS.len();
        if all_acked {
            v.stage = VStage::Rationale;
        }
        self.journal
            .push(ev(Kind::Ack, format!("warning acknowledged: {warning}")));
        if all_acked {
            self.amend_field = 0;
        }
    }

    fn record_base(&self) -> String {
        let i = match self.verdict.as_ref().and_then(|v| v.choice) {
            Some(Choice::Option(i)) => i,
            _ => 0,
        };
        OPTIONS[i].decision_text.to_string()
    }

    pub fn amend_char(&mut self, c: char) {
        let base = self.record_base();
        let buffer = self.amended.get_or_insert_with(|| base);
        buffer.push(c);
    }

    pub fn amend_backspace(&mut self) {
        if let Some(buffer) = self.amended.as_mut() {
            buffer.pop();
        }
    }

    pub fn amend_clear(&mut self) {
        self.amended = Some(String::new());
    }

    pub fn amended_text(&self) -> Option<&str> {
        self.amended.as_deref()
    }

    pub fn submit_rationale(&mut self) {
        let base = self.record_base();
        let Some(v) = self.verdict.take() else { return };
        if let Some(amended) = self.amended.clone() {
            if amended.trim().is_empty() || amended.trim() == base.trim() {
                self.amended = None;
            } else {
                self.journal
                    .push(ev(Kind::Ack, "record text amended by developer before acceptance"));
            }
        }
        let rationale = v.rationale.trim().to_string();
        if !rationale.is_empty() {
            self.note = rationale.clone();
            self.journal
                .push(ev(Kind::Ack, format!("rationale recorded: {rationale}")));
        }
        if let Some(Choice::Option(i)) = v.choice {
            let option = &OPTIONS[i];
            self.chosen = Some(i);
            self.events
                .push(ev(Kind::Grant, format!("verdict: {} -- {}", i + 1, option.label)));
            self.events.push(ev(
                Kind::Decision,
                format!("record written -- {RECORD_FILE} ({RECORD_ID})"),
            ));
            self.events.push(ev(
                Kind::Grant,
                "decision grant active -- src/payments/** - migrations/** (task-scoped)",
            ));
            self.journal
                .push(ev(Kind::Decision, format!("{RECORD_ID} accepted -- {}", option.label)));
        }
        self.phase = Phase::Running;
    }

    pub fn acknowledge_block(&mut self) {
        if self.phase != Phase::Blocked {
            return;
        }
        self.phase = Phase::Running;
        self.events.push(ev(
            Kind::Info,
            "steer sent -- implementation follows the accepted TDD; spec/** is core-written",
        ));
        self.journal
            .push(ev(Kind::Ack, "hard block acknowledged -- steering applied"));
    }
}
