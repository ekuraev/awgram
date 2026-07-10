use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use awg_bot::error::Error;
use awg_bot::vpn::runner::{run, RunSpec};

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
async fn returns_stdout_on_success() {
    let (_d, script) = make_script("#!/bin/sh\necho \"$1-ok\"\n");
    let spec = RunSpec { script: &script, sudo_prefix: "", timeout_secs: 5 };
    let out = run(&spec, &["list"]).await.unwrap();
    assert_eq!(out.trim(), "list-ok");
}

#[tokio::test]
async fn maps_nonzero_exit_to_script_failed() {
    let (_d, script) = make_script("#!/bin/sh\necho boom 1>&2\nexit 3\n");
    let spec = RunSpec { script: &script, sudo_prefix: "", timeout_secs: 5 };
    let err = run(&spec, &["add"]).await.unwrap_err();
    match err {
        Error::ScriptFailed { code, stderr } => {
            assert_eq!(code, Some(3));
            assert!(stderr.contains("boom"));
        }
        other => panic!("expected ScriptFailed, got {other:?}"),
    }
}

#[tokio::test]
async fn times_out_long_running_script() {
    let (_d, script) = make_script("#!/bin/sh\nsleep 10\n");
    let spec = RunSpec { script: &script, sudo_prefix: "", timeout_secs: 1 };
    let err = run(&spec, &["list"]).await.unwrap_err();
    assert!(matches!(err, Error::Timeout));
}
