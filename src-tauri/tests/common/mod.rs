//! Shared by every test binary that can reach the runner.

use std::path::PathBuf;

/// A config directory of this process's own, so the suite stops writing `runner.pid` and
/// `last-run.log` into the directory the installed app is using. Call it before anything
/// that can start a runner: the override is taken once per process and the first caller
/// wins.
pub fn isolate_config_dir() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("llamaport-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("test config dir");
    llamaport_lib::store::use_config_dir(dir.clone());
    dir
}
