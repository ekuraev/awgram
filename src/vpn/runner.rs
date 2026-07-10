use std::path::Path;
use std::process::Stdio;
use std::time::Duration;

use tokio::process::Command;
use tokio::time::timeout;

use crate::error::{Error, Result};

pub struct RunSpec<'a> {
    pub script: &'a Path,
    pub sudo_prefix: &'a str,
    pub timeout_secs: u64,
}

pub async fn run(spec: &RunSpec<'_>, args: &[&str]) -> Result<String> {
    let mut cmd = if spec.sudo_prefix.is_empty() {
        let mut c = Command::new(spec.script);
        c.args(args);
        c
    } else {
        let mut c = Command::new(spec.sudo_prefix);
        c.arg(spec.script);
        c.args(args);
        c
    };
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped()).kill_on_drop(true);

    let child = cmd.spawn()?;
    let dur = Duration::from_secs(spec.timeout_secs);

    let output = match timeout(dur, child.wait_with_output()).await {
        Ok(res) => res?,
        Err(_) => return Err(Error::Timeout), // child убивается через kill_on_drop
    };

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        Err(Error::ScriptFailed {
            code: output.status.code(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        })
    }
}
