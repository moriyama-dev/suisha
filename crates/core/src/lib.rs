//! What Suisha's window and its command line share: reading what each
//! package manager has installed, planning and running the commands that
//! change it, the schedule, the history and the settings.

pub mod background;
pub mod exec;
pub mod managers;
pub mod model;
pub mod ops;
pub mod privileged;
pub mod schedule;
pub mod self_update;
pub mod setup_file;
pub mod store;
pub mod system;

/// The version of this copy of Suisha.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
