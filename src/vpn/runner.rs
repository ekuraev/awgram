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
    /// Доп. env-переменные для вызова (например AWG_STRICT_CONFIRM=1).
    pub extra_env: &'a [(&'a str, &'a str)],
}

fn build_cmd(spec: &RunSpec<'_>, args: &[&str]) -> Command {
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
    for (k, v) in spec.extra_env {
        cmd.env(k, v);
    }
    cmd.stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    cmd
}

pub async fn run(spec: &RunSpec<'_>, args: &[&str]) -> Result<String> {
    let mut cmd = build_cmd(spec, args);
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

/// Как `run`, но возвращает stdout и код выхода независимо от успеха.
/// Тайм-аут по-прежнему → Error::Timeout. Нужен для `check` (код 1 = «проблемы», не ошибка).
pub async fn run_capture(spec: &RunSpec<'_>, args: &[&str]) -> Result<(String, i32)> {
    let mut cmd = build_cmd(spec, args);
    let child = cmd.spawn()?;
    let dur = Duration::from_secs(spec.timeout_secs);
    let output = match timeout(dur, child.wait_with_output()).await {
        Ok(res) => res?,
        Err(_) => return Err(Error::Timeout),
    };
    let mut out = String::from_utf8_lossy(&output.stdout).into_owned();
    if out.is_empty() {
        out = String::from_utf8_lossy(&output.stderr).into_owned();
    }
    Ok((out, output.status.code().unwrap_or(-1)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::os::unix::fs::PermissionsExt;

    fn write_script(dir: &tempfile::TempDir, body: &str) -> std::path::PathBuf {
        let p = dir.path().join("stub.sh");
        let mut f = std::fs::File::create(&p).unwrap();
        f.write_all(body.as_bytes()).unwrap();
        let mut perm = std::fs::metadata(&p).unwrap().permissions();
        perm.set_mode(0o755);
        std::fs::set_permissions(&p, perm).unwrap();
        p
    }

    #[tokio::test]
    async fn extra_env_reaches_script() {
        let dir = tempfile::tempdir().unwrap();
        // Stub печатает значение env-переменной, если она задана.
        let script = write_script(
            &dir,
            "#!/bin/sh\nprintf '%s' \"${AWG_STRICT_CONFIRM:-unset}\"\n",
        );
        let spec = RunSpec {
            script: &script,
            sudo_prefix: "",
            timeout_secs: 5,
            extra_env: &[("AWG_STRICT_CONFIRM", "1")],
        };
        let out = run(&spec, &[]).await.unwrap();
        assert_eq!(out, "1");
    }

    #[tokio::test]
    async fn no_extra_env_means_unset() {
        let dir = tempfile::tempdir().unwrap();
        let script = write_script(
            &dir,
            "#!/bin/sh\nprintf '%s' \"${AWG_STRICT_CONFIRM:-unset}\"\n",
        );
        let spec = RunSpec {
            script: &script,
            sudo_prefix: "",
            timeout_secs: 5,
            extra_env: &[],
        };
        let out = run(&spec, &[]).await.unwrap();
        assert_eq!(out, "unset");
    }
}
