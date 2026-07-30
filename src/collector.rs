//! Фоновый сборщик: раз в минуту снимает vpn.stats() и складывает в Store.
//! Благодаря ему статистика и история не зависят от времени жизни
//! kernel-счётчиков WireGuard и переживают ребуты VPS.

use std::sync::Arc;

use crate::store::{Sample, Store};
use crate::vpn::Vpn;

const TICK_SECS: u64 = 60;
const ROLLUP_EVERY_TICKS: u32 = 5;

pub async fn tick(vpn: &Vpn, store: &Arc<Store>, now: i64) {
    let clients = match vpn.stats().await {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(error = %e, "collector: stats недоступен, пропускаю тик");
            return;
        }
    };
    let samples: Vec<Sample> = clients
        .into_iter()
        .map(|c| Sample {
            name: c.name,
            ip: c.ip,
            rx: c.rx,
            tx: c.tx,
            last_handshake: c.last_handshake,
        })
        .collect();
    let store = store.clone();
    // rusqlite синхронный — пачку пишем вне асинхронного реактора
    let _ = tokio::task::spawn_blocking(move || store.ingest(now, &samples)).await;
}

pub async fn run(vpn: Arc<Vpn>, store: Arc<Store>) {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(TICK_SECS));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let mut n: u32 = 0;
    loop {
        interval.tick().await;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        tick(&vpn, &store, now).await;
        n = n.wrapping_add(1);
        if n.is_multiple_of(ROLLUP_EVERY_TICKS) {
            let s = store.clone();
            let _ = tokio::task::spawn_blocking(move || {
                s.rollup(now);
                s.prune(now);
            })
            .await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::Store;
    use std::sync::Arc;

    fn vpn_with_script(body: &str) -> (tempfile::TempDir, crate::vpn::Vpn) {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let script = dir.path().join("manage.sh");
        std::fs::write(&script, format!("#!/bin/sh\n{body}\n")).unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        let vpn = crate::vpn::Vpn::test_with_script(script, dir.path().to_path_buf());
        (dir, vpn)
    }

    #[tokio::test]
    #[serial_test::serial] // гонка ETXTBSY — как в тестах vpn
    async fn tick_ingests_stats_into_store() {
        let (_d, vpn) = vpn_with_script(
            r#"case "$1" in
            stats) echo '[{"name":"alice","ip":"10.0.0.2","rx":100,"tx":50,"last_handshake":990,"status":"Активен","status_code":"active"}]' ;;
            esac"#,
        );
        let store = Arc::new(Store::open_in_memory());
        tick(&vpn, &store, 1000).await;
        let n: i64 = store
            .with_conn(|c| c.query_row("SELECT COUNT(*) FROM traffic_samples", [], |r| r.get(0)))
            .unwrap();
        assert_eq!(n, 1);
    }

    #[tokio::test]
    #[serial_test::serial] // гонка ETXTBSY — как в тестах vpn
    async fn tick_survives_script_failure() {
        let (_d, vpn) = vpn_with_script(r#"echo boom >&2; exit 3"#);
        let store = Arc::new(Store::open_in_memory());
        tick(&vpn, &store, 1000).await; // не паникует, ничего не пишет
        let n: i64 = store
            .with_conn(|c| c.query_row("SELECT COUNT(*) FROM traffic_samples", [], |r| r.get(0)))
            .unwrap();
        assert_eq!(n, 0);
    }
}
