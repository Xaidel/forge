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
    },
];

pub const WARNINGS: &[&str] = &[
    "migration touches a table with production traffic -- needs a maintenance window",
    "expands task scope -- the original task did not include a schema migration",
];

pub const AFFECTED: &[&str] = &["src/payments/** -- write", "migrations/** -- write"];
pub const ARTIFACT: &str = "TDD -- what the system must do";

pub const RECORD_PATH: &str = "spec/tdd/accepted/2026-10-14-idempotent-capture.md (TDD-004)";

pub const BLOCK_ATTEMPT: &str = "write spec/tdd/accepted/2026-10-14-idempotent-capture.md";
pub const BLOCK_REASON: &str =
    "agent writes under /spec/** are never allowed -- records are core-written from the approved verdict";
pub const BLOCK_STEER: &str = "implement to the accepted TDD; the record is not agent-editable";

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
                    "task complete -- TDD-004 accepted, warnings acked, 1 boundary attempt steered",
                ));
                self.journal.push(ev(Kind::Done, "task complete"));
            }
        }
    }

    pub fn choose(&mut self, choice: Choice) {
        match choice {
            Choice::Option(_) => {
                let Some(v) = self.verdict.as_mut() else { return };
                v.choice = Some(choice);
                v.stage = if WARNINGS.is_empty() { VStage::Rationale } else { VStage::Acking };
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
        self.journal
            .push(ev(Kind::Ack, format!("warning acknowledged: {warning}")));
        if v.acked >= WARNINGS.len() {
            v.stage = VStage::Rationale;
        }
    }

    pub fn submit_rationale(&mut self) {
        let Some(v) = self.verdict.take() else { return };
        let rationale = v.rationale.trim().to_string();
        if !rationale.is_empty() {
            self.journal
                .push(ev(Kind::Ack, format!("rationale recorded: {rationale}")));
        }
        if let Some(Choice::Option(i)) = v.choice {
            let option = &OPTIONS[i];
            self.events
                .push(ev(Kind::Grant, format!("verdict: {} -- {}", i + 1, option.label)));
            self.events.push(ev(Kind::Decision, format!("record written -- {RECORD_PATH}")));
            self.events.push(ev(
                Kind::Grant,
                "decision grant active -- src/payments/** - migrations/** (task-scoped)",
            ));
            self.journal.push(ev(Kind::Decision, format!("TDD-004 accepted -- {}", option.label)));
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
