//! **Monitor ready tasks** without the app (Task-A03-cow112, approver 2026-11-09): the loop that
//! gives ready tasks to agents in new terminal sessions, moved out of the app's interface so `alm
//! monitor` and the app run the same one. A run is an `alm monitor run` process of the person's
//! own, apart from any terminal or window; this computer keeps one small file per run in one fixed
//! folder ([`registry`]), never in the taskbag.

pub mod flow;
pub mod notify;
pub mod registry;
pub mod request;
pub mod runner;
pub mod session_tab;
pub mod terminal;
