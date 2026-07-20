use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use awgram::vpn::runner::{run, RunSpec};
use serial_test::serial;

fn make_script(body: &str) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("fake.sh");
    let mut f = std::fs::File::create(&path).unwrap();
    f.write_all(body.as_bytes()).unwrap();
    let mut perm = std::fs::metadata(&path).unwrap().permissions();
    perm.set_mode(0o755);
    std::fs::set_permissions(&path, perm).unwrap();
    (dir, path)
}

#[tokio::test]
#[serial] // гонка ETXTBSY: форк параллельного теста удерживает write-fd чужого скрипта до execve
async fn returns_stdout_on_success() {
    let (_d, script) = make_script("#!/bin/sh\necho \"$1-ok\"\n");
    let spec = RunSpec {
        script: &script,
        sudo_prefix: "",
        timeout_secs: 5,
        extra_env: &[],
    };
    let (out, _code) = run(&spec, &["list"]).await.unwrap();
    assert_eq!(out.trim(), "list-ok");
}

#[tokio::test]
#[serial] // гонка ETXTBSY: форк параллельного теста удерживает write-fd чужого скрипта до execve
async fn returns_stdout_and_exit_code_on_nonzero() {
    // run() возвращает (stdout, exit_code) ВСЕГДА — даже при ненулевом exit.
    // Это контракт, на котором основан Fix 2 (P1.1): JSON на stdout при exit 1.
    let (_d, script) = make_script("#!/bin/sh\necho boom 1>&2\nexit 3\n");
    let spec = RunSpec {
        script: &script,
        sudo_prefix: "",
        timeout_secs: 5,
        extra_env: &[],
    };
    let (out, code) = run(&spec, &["add"]).await.unwrap();
    assert_eq!(code, 3);
    // runner мержит stderr в out при пустом stdout — boom попадает в out.
    assert!(out.contains("boom"));
}

#[tokio::test]
#[serial] // гонка ETXTBSY: форк параллельного теста удерживает write-fd чужого скрипта до execve
async fn times_out_long_running_script() {
    let (_d, script) = make_script("#!/bin/sh\nsleep 10\n");
    // timeout_secs: 3 (was 1) — still far below the 10s sleep, so this still genuinely
    // exercises the timeout path, but is more robust against flakes under heavy machine load.
    let spec = RunSpec {
        script: &script,
        sudo_prefix: "",
        timeout_secs: 3,
        extra_env: &[],
    };
    let err = run(&spec, &["list"]).await.unwrap_err();
    assert!(matches!(err, awgram::error::Error::Timeout));
}

#[tokio::test]
#[serial] // гонка ETXTBSY: форк параллельного теста удерживает write-fd чужого скрипта до execve
async fn run_returns_output_on_nonzero_exit() {
    // Бывший run_capture-тест: diagnose/check печатают stdout и при exit 1.
    // Теперь run() покрывает этот кейс — отдельной функции не нужно.
    let (_d, script) = make_script("#!/bin/sh\necho diag\nexit 1\n");
    let spec = RunSpec {
        script: &script,
        sudo_prefix: "",
        timeout_secs: 5,
        extra_env: &[],
    };
    let (out, code) = run(&spec, &["check"]).await.unwrap();
    assert!(out.contains("diag"));
    assert_eq!(code, 1);
}
