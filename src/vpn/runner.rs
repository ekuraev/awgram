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

/// Запускает скрипт и возвращает `(stdout, exit_code)` ВСЕГДА — даже при
/// ненулевом exit code. Это критично для JSON-контракта manage v5.21.0:
/// скрипт печатает JSON в stdout и ЗАТЕМ выходит с кодом 1 для легитимных
/// статусов (exists/not_found/partial/rollback/repair rc). Отбрасывание
/// stdout на non-zero делало все status-ветки в add/remove/regen/restore
/// недостижимыми в проде. Timeout по-прежнему → Error::Timeout.
pub async fn run(spec: &RunSpec<'_>, args: &[&str]) -> Result<(String, i32)> {
    let mut cmd = build_cmd(spec, args);
    // ETXTBSY при execve: скрипт в этот момент открыт кем-то на запись.
    // Два легитимных источника: `awgram-setup update` переписывает
    // manage-скрипт под работающим ботом, и fork-окно параллельного spawn
    // (другой поток процесса успел сделать fork, пока чей-то fd записи ещё
    // открыт, — проявлялось флейком тестов в Linux CI). Окно — микросекунды
    // и миллисекунды, поэтому короткий ретрай надёжно его пережидает, не
    // маскируя настоящие ошибки запуска.
    const BUSY_RETRIES: u32 = 10;
    const BUSY_PAUSE: Duration = Duration::from_millis(20);
    let mut busy_attempts = 0;
    let child = loop {
        match cmd.spawn() {
            Ok(c) => break c,
            Err(e)
                if e.kind() == std::io::ErrorKind::ExecutableFileBusy
                    && busy_attempts < BUSY_RETRIES =>
            {
                busy_attempts += 1;
                tokio::time::sleep(BUSY_PAUSE).await;
            }
            Err(e) => return Err(e.into()),
        }
    };
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
        let (out, _) = run(&spec, &[]).await.unwrap();
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
        let (out, _) = run(&spec, &[]).await.unwrap();
        assert_eq!(out, "unset");
    }

    #[tokio::test]
    async fn run_retries_while_script_briefly_busy() {
        // Детерминированная эмуляция ETXTBSY-гонки (см. комментарий в run):
        // держим fd записи скрипта открытым 50 мс параллельно с запуском.
        // На Linux execve в этом окне даёт «Text file busy» — без ретрая
        // тест падает; macOS ETXTBSY для скриптов не выдаёт, там тест
        // проверяет только успешный запуск.
        let dir = tempfile::tempdir().unwrap();
        let script = write_script(&dir, "#!/bin/sh\nprintf 'ok'\n");
        let held = std::fs::OpenOptions::new()
            .write(true)
            .open(&script)
            .unwrap();
        let holder = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(50));
            drop(held);
        });
        let spec = RunSpec {
            script: &script,
            sudo_prefix: "",
            timeout_secs: 5,
            extra_env: &[],
        };
        let (out, code) = run(&spec, &[]).await.unwrap();
        assert_eq!((out.as_str(), code), ("ok", 0));
        holder.join().unwrap();
    }
}
