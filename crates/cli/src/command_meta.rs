use crate::{Add, CardsWhat, ChangeWhat, Command, Delete, FolderWhat, MoveWhat, Rename};

/// `alm add area|iteration [<id>] <name>`: the ID given, if any, and the name.
pub(crate) fn id_and_name<'a>(
    first: &'a str,
    name: &'a Option<String>,
) -> (Option<&'a str>, &'a str) {
    match name {
        Some(name) => (Some(first), name),
        None => (None, first),
    }
}

impl Command {
    /// The task or area a command acts on, for the commit message.
    pub(super) fn target(&self) -> Option<&str> {
        match self {
            Command::Add(Add::Area { first, name } | Add::Iteration { first, name, .. }) => {
                id_and_name(first, name).0
            }
            Command::Rename(Rename::Label { old, .. } | Rename::Iteration { old, .. }) => Some(old),
            Command::Rename(Rename::Area { id, .. })
            | Command::Change(ChangeWhat::Iteration { id, .. })
            | Command::Move(MoveWhat::Iteration { id, .. })
            | Command::Delete(Delete::Area { id } | Delete::Iteration { id }) => Some(id),
            Command::Show { id }
            | Command::History { id }
            | Command::Claim { id, .. }
            | Command::Session { id, .. }
            | Command::Base { id, .. }
            | Command::Trace { id, .. }
            | Command::Link { id, .. }
            | Command::CheckResult { id, .. }
            | Command::Label { id, .. }
            | Command::Wait { id, .. }
            | Command::Block { id, .. }
            | Command::Unblock { id, .. }
            | Command::Done { id, .. }
            | Command::Checked { id, .. }
            | Command::Recheck { id, .. }
            | Command::AiDecided { id, .. }
            | Command::Reviewed { id }
            | Command::Release { id, .. }
            | Command::Needs { id, .. }
            | Command::Plan { id, .. }
            | Command::Ready { id }
            | Command::Backlog { id }
            | Command::Title { id, .. }
            | Command::Describe { id, .. }
            | Command::Note { id, .. }
            | Command::Folders { id, .. }
            | Command::Difficulty { id, .. }
            | Command::Handoff { id, .. }
            | Command::Card { id, .. }
            | Command::Drop { id }
            | Command::Reopen { id, .. } => Some(id),
            Command::Add(Add::Task { id, .. }) => id.as_deref(),
            Command::Beat { id, .. } => id.as_deref(),
            Command::Approve { ids, .. } => ids.first().map(String::as_str),
            Command::Withdraw { .. } => None,
            Command::Effect { id, .. } => Some(id),
            Command::Effects { .. } => None,
            Command::Exception { id, .. } => Some(id),
            Command::Exceptions { .. } => None,
            Command::Redact { id, .. } => Some(id),
            Command::Classify { id, .. } => Some(id),
            Command::Shared { .. }
            | Command::Init { .. }
            | Command::Labels { .. }
            | Command::Project { .. }
            | Command::TaskLanguage { .. }
            | Command::Brief
            | Command::Quota { .. }
            | Command::Guide { .. }
            | Command::Confirm { .. }
            | Command::Progress { .. }
            | Command::List { .. }
            | Command::Human { .. }
            | Command::MoveIn { .. }
            | Command::Translate { .. }
            | Command::HandoffRules { .. }
            | Command::HeartbeatSettings { .. }
            | Command::FileSizeSettings { .. }
            | Command::AuditSettings { .. }
            | Command::Cards { .. }
            | Command::Upgrade { .. }
            | Command::Audit
            | Command::Checkpoint { .. }
            | Command::Folder { .. }
            | Command::Next { .. }
            | Command::Monitor { .. }
            | Command::Traces { .. }
            | Command::Links { .. } => None,
        }
    }

    /// Whether the command writes to the taskbag.
    pub(super) fn changes(&self) -> bool {
        if let Command::Confirm { task, .. } = self {
            return task.is_some();
        }
        if let Command::Project { name, id } = self {
            return name.is_some() || id.is_some();
        }
        if let Command::TaskLanguage { value } = self {
            return value.is_some();
        }
        if let Command::HandoffRules {
            quota,
            context,
            write_when,
            clear,
            ..
        } = self
        {
            return quota.is_some() || context.is_some() || write_when.is_some() || *clear;
        }
        if let Command::HeartbeatSettings {
            interval,
            lost_after,
            push_work,
            clear,
        } = self
        {
            return interval.is_some() || lost_after.is_some() || push_work.is_some() || *clear;
        }
        if let Command::FileSizeSettings { limit, clear } = self {
            return limit.is_some() || *clear;
        }
        if let Command::AuditSettings {
            identity,
            reauthenticate,
            separation_of_duties,
            retention,
            legal_hold,
            witness,
            witness_service,
            witness_second,
            witness_when,
            ..
        } = self
        {
            return identity.is_some()
                || reauthenticate.is_some()
                || separation_of_duties.is_some()
                || retention.is_some()
                || legal_hold.is_some()
                || witness.is_some()
                || witness_service.is_some()
                || witness_second.is_some()
                || witness_when.is_some();
        }
        if let Command::Cards { what } = self {
            return what.is_some();
        }
        if let Command::Upgrade { confirm } = self {
            return *confirm;
        }
        match self {
            Command::Folder {
                what: FolderWhat::List,
            } => return false,
            Command::Folder {
                what: FolderWhat::Remove { confirm, .. },
            } => return *confirm,
            _ => {}
        }
        !matches!(
            self,
            Command::List { .. }
                | Command::Human { .. }
                | Command::Show { .. }
                | Command::History { .. }
                | Command::Next { .. }
                | Command::Monitor { .. }
                | Command::Confirm { .. }
                | Command::Progress { .. }
                | Command::Brief
                | Command::Quota { .. }
                | Command::Guide { .. }
                | Command::MoveIn { .. }
                | Command::Translate { .. }
                | Command::Audit
                | Command::Exceptions { .. }
        )
    }

    pub(super) fn name(&self) -> &'static str {
        match self {
            Command::Shared { .. } => "shared",
            Command::Init { .. } => "init",
            Command::Confirm { .. } => "confirm",
            Command::Progress { .. } => "progress",
            Command::Labels { .. } => "labels",
            Command::TaskLanguage { .. } => "task-language",
            Command::Brief => "brief",
            Command::Quota { .. } => "quota",
            Command::Guide { .. } => "guide",
            Command::Add(Add::Area { .. }) => "add area",
            Command::Add(Add::Iteration { .. }) => "add iteration",
            Command::Add(Add::Task { .. }) => "add task",
            Command::Rename(Rename::Area { .. }) => "rename area",
            Command::Rename(Rename::Label { .. }) => "rename label",
            Command::Rename(Rename::Iteration { .. }) => "rename iteration",
            Command::Change(ChangeWhat::Iteration { .. }) => "change iteration",
            Command::Move(MoveWhat::Iteration { .. }) => "move iteration",
            Command::Delete(Delete::Area { .. }) => "delete area",
            Command::Delete(Delete::Iteration { .. }) => "delete iteration",
            Command::List { .. } => "list",
            Command::Folders { .. } => "folders",
            Command::Difficulty { .. } => "difficulty",
            Command::Upgrade { .. } => "upgrade",
            Command::Audit => "audit",
            Command::Checkpoint { .. } => "checkpoint",
            Command::Folder { what } => match what {
                FolderWhat::List => "folder list",
                FolderWhat::Add { .. } => "folder add",
                FolderWhat::Rename { .. } => "folder rename",
                FolderWhat::Locate { .. } => "folder locate",
                FolderWhat::Remove { .. } => "folder remove",
            },
            Command::Show { .. } => "show",
            Command::History { .. } => "history",
            Command::Claim { .. } => "claim",
            Command::Session { .. } => "session",
            Command::Base { .. } => "base",
            Command::Trace { .. } => "trace",
            Command::Traces { .. } => "traces",
            Command::Link { .. } => "link",
            Command::CheckResult { .. } => "check-result",
            Command::Links { .. } => "links",
            Command::Label { .. } => "label",
            Command::Wait { .. } => "wait",
            Command::Block { .. } => "block",
            Command::Unblock { .. } => "unblock",
            Command::Done { .. } => "done",
            Command::Checked { .. } => "checked",
            Command::Recheck { .. } => "recheck",
            Command::AiDecided { .. } => "ai-decided",
            Command::Reviewed { .. } => "reviewed",
            Command::Approve { .. } => "approve",
            Command::Withdraw { .. } => "withdraw",
            Command::Effect { .. } => "effect",
            Command::Effects { .. } => "effects",
            Command::Exception { .. } => "exception",
            Command::Exceptions { .. } => "exceptions",
            Command::Redact { .. } => "redact",
            Command::Classify { .. } => "classify",
            Command::Human { .. } => "human",
            Command::Release { .. } => "release",
            Command::Needs { .. } => "needs",
            Command::Plan { .. } => "plan",
            Command::Ready { .. } => "ready",
            Command::Backlog { .. } => "backlog",
            Command::Title { .. } => "title",
            Command::Describe { .. } => "describe",
            Command::Note { .. } => "note",
            Command::Beat { .. } => "beat",
            Command::Cards { what: None } => "cards",
            Command::Cards {
                what: Some(CardsWhat::Found { .. }),
            } => "cards found",
            Command::Card { .. } => "card",
            Command::Project { .. } => "project",
            Command::Drop { .. } => "drop",
            Command::Reopen { .. } => "reopen",
            Command::Next { .. } => "next",
            Command::Monitor { what } => what.name(),
            Command::MoveIn { .. } => "move-in",
            Command::Translate { .. } => "translate",
            Command::HandoffRules { .. } => "handoff-rules",
            Command::HeartbeatSettings { .. } => "heartbeat-settings",
            Command::FileSizeSettings { .. } => "file-size-settings",
            Command::AuditSettings { .. } => "audit-settings",
            Command::Handoff { .. } => "handoff",
        }
    }
}
