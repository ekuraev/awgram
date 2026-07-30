//! Приём снапшотов от collector: реестр клиентов, сэмплы с дельтами
//! (устойчивость к сбросу kernel-счётчиков WG при ребуте/regen/restart),
//! события online/offline (история хэндшейков).

use crate::store::Store;
use crate::vpn::model::ONLINE_THRESHOLD_SECS;
use rusqlite::OptionalExtension;

// Сэмплы хранятся 7 дней (старые удаляются)
const RAW_RETENTION_SECS: i64 = 7 * 86400;
// Часовые агрегаты хранятся 90 дней (старые удаляются)
const HOURLY_RETENTION_SECS: i64 = 90 * 86400;

pub struct Sample {
    pub name: String,
    pub ip: String,
    pub rx: u64,
    pub tx: u64,
    pub last_handshake: Option<i64>,
}

/// Суммарный трафик и время online за период (из traffic_daily).
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct PeriodTotals {
    pub rx: u64,
    pub tx: u64,
    pub online_minutes: u64,
}

impl PeriodTotals {
    fn add(&mut self, other: &PeriodTotals) {
        self.rx += other.rx;
        self.tx += other.tx;
        self.online_minutes += other.online_minutes;
    }
}

/// Сводка трафика по стандартным окнам для UI: сегодня, 7/30 дней, всё время,
/// плюс предыдущие 7 дней — для расчёта тренда (текущая неделя vs прошлая).
#[derive(Debug, Default, Clone, PartialEq)]
pub struct TrafficSummary {
    pub today: PeriodTotals,
    pub d7: PeriodTotals,
    pub d30: PeriodTotals,
    pub total: PeriodTotals,
    pub prev7: PeriodTotals,
}

impl TrafficSummary {
    /// Поэлементное сложение — агрегация сводки группы из пер-клиентских сводок.
    pub fn add(&mut self, other: &TrafficSummary) {
        self.today.add(&other.today);
        self.d7.add(&other.d7);
        self.d30.add(&other.d30);
        self.total.add(&other.total);
        self.prev7.add(&other.prev7);
    }
}

impl Store {
    pub fn ingest(&self, now: i64, samples: &[Sample]) {
        let res = self.with_conn(|c| {
            let tx_guard = c.unchecked_transaction()?;
            for smp in samples {
                let online = matches!(smp.last_handshake, Some(hs) if hs > 0 && now - hs < ONLINE_THRESHOLD_SECS);
                // upsert реестра: возвращение клиента снимает removed_at
                c.execute(
                    "INSERT INTO clients(name, ip, first_seen, last_seen)
                     VALUES(?1, ?2, ?3, ?3)
                     ON CONFLICT(name) DO UPDATE SET ip=?2, last_seen=?3, removed_at=NULL",
                    rusqlite::params![smp.name, smp.ip, now],
                )?;
                let client_id: i64 = c.query_row(
                    "SELECT id FROM clients WHERE name=?1",
                    [&smp.name],
                    |r| r.get(0),
                )?;
                // предыдущий сэмпл — база для дельты и прежний online-статус
                let prev: Option<(i64, i64, i64)> = c
                    .query_row(
                        "SELECT rx, tx, online FROM traffic_samples
                         WHERE client_id=?1 ORDER BY ts DESC LIMIT 1",
                        [client_id],
                        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
                    )
                    .optional()?;
                let (rx_delta, tx_delta) = match prev {
                    // Счётчик уменьшился → интерфейс пересоздан (ребут/regen):
                    // новое значение и есть трафик с момента сброса.
                    Some((prx, ptx, _)) => (
                        if (smp.rx as i64) < prx { smp.rx as i64 } else { smp.rx as i64 - prx },
                        if (smp.tx as i64) < ptx { smp.tx as i64 } else { smp.tx as i64 - ptx },
                    ),
                    None => (0, 0), // первый сэмпл — только базовая линия
                };
                c.execute(
                    "INSERT INTO traffic_samples(client_id, ts, rx, tx, rx_delta, tx_delta, online)
                     VALUES(?1,?2,?3,?4,?5,?6,?7)",
                    rusqlite::params![client_id, now, smp.rx as i64, smp.tx as i64, rx_delta, tx_delta, online as i64],
                )?;
                // переход online/offline → событие (история хэндшейков)
                let was_online = prev.map(|(_, _, o)| o == 1).unwrap_or(false);
                if online != was_online {
                    let kind = if online { "online" } else { "offline" };
                    c.execute(
                        "INSERT INTO events(ts, kind, client) VALUES(?1, ?2, ?3)",
                        rusqlite::params![now, kind, smp.name],
                    )?;
                }
            }
            // клиенты, пропавшие из выдачи (удалены через CLI) — пометить
            if !samples.is_empty() {
                let names: Vec<String> = samples.iter().map(|s| s.name.clone()).collect();
                let placeholders = vec!["?"; names.len()].join(",");
                let sql = format!(
                    "UPDATE clients SET removed_at=?1 WHERE removed_at IS NULL
                     AND name NOT IN ({placeholders})"
                );
                let mut params: Vec<&dyn rusqlite::ToSql> = vec![&now];
                for n in &names { params.push(n); }
                c.execute(&sql, params.as_slice())?;
            } else {
                // Когда нет сэмплов, все существующие клиенты считаются удалёнными
                c.execute(
                    "UPDATE clients SET removed_at=?1 WHERE removed_at IS NULL",
                    [&now],
                )?;
            }
            tx_guard.commit()
        });
        if let Err(e) = res {
            tracing::error!(error = %e, "ingest сэмплов не записан");
        }
    }

    /// Сворачивает сэмплы в hourly, hourly — в daily. Обрабатываются только
    /// часы/дни, затронутые с прошлого запуска (минус час перекрытия), поэтому
    /// дёшев и идемпотентен: строки пересчитываются целиком (INSERT OR REPLACE).
    ///
    /// ИНВАРИАНТ: rollup вызывается ПЕРЕД prune в одном тике collector'а.
    /// Clamp окон к границам ретенции — защита от затирания агрегатов частичными
    /// данными, если инвариант нарушат (независимый prune, потеря meta).
    pub fn rollup(&self, now: i64) {
        let res = self.with_conn(|c| {
            let last: i64 = c
                .query_row("SELECT value FROM meta WHERE key='last_rollup_ts'", [], |r| r.get::<_, String>(0))
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0);
            let from_hour = ((last - 3600).max(now - RAW_RETENTION_SECS)).max(0) / 3600 * 3600;
            let tx = c.unchecked_transaction()?;
            c.execute(
                "INSERT OR REPLACE INTO traffic_hourly(client_id, hour_ts, rx_bytes, tx_bytes, online_minutes)
                 SELECT client_id, ts/3600*3600, SUM(rx_delta), SUM(tx_delta), SUM(online)
                 FROM traffic_samples WHERE ts >= ?1 GROUP BY client_id, ts/3600*3600",
                [from_hour],
            )?;
            let from_day = ((last - 86400).max(now - HOURLY_RETENTION_SECS)).max(0) / 86400 * 86400;
            c.execute(
                "INSERT OR REPLACE INTO traffic_daily(client_id, day_ts, rx_bytes, tx_bytes, online_minutes)
                 SELECT client_id, hour_ts/86400*86400, SUM(rx_bytes), SUM(tx_bytes), SUM(online_minutes)
                 FROM traffic_hourly WHERE hour_ts >= ?1 GROUP BY client_id, hour_ts/86400*86400",
                [from_day],
            )?;
            c.execute(
                "INSERT INTO meta(key,value) VALUES('last_rollup_ts',?1)
                 ON CONFLICT(key) DO UPDATE SET value=?1",
                [now.to_string()],
            )?;
            tx.commit()
        });
        if let Err(e) = res {
            tracing::error!(error = %e, "rollup не выполнен");
        }
    }

    /// Удаляет устаревшие сэмплы (старше 7 дней) и часовые агрегаты
    /// (старше 90 дней). Дневные агрегаты и события хранятся неограниченно.
    pub fn prune(&self, now: i64) {
        let res = self.with_conn(|c| {
            c.execute(
                "DELETE FROM traffic_samples WHERE ts < ?1",
                [now - RAW_RETENTION_SECS],
            )?;
            c.execute(
                "DELETE FROM traffic_hourly WHERE hour_ts < ?1",
                [now - HOURLY_RETENTION_SECS],
            )
        });
        if let Err(e) = res {
            tracing::error!(error = %e, "prune не выполнен");
        }
    }

    /// Сумма rx/tx/online_minutes по дневным бакетам в диапазоне
    /// `[from_day, to_day]` включительно, опционально по одному клиенту.
    fn totals(&self, client: Option<&str>, from_day: i64, to_day: i64) -> PeriodTotals {
        const SQL: &str = "SELECT COALESCE(SUM(rx_bytes),0), COALESCE(SUM(tx_bytes),0), COALESCE(SUM(online_minutes),0)
             FROM traffic_daily d JOIN clients c ON c.id = d.client_id
             WHERE day_ts BETWEEN ?1 AND ?2";
        let res = self.with_conn(|c| {
            let row = |r: &rusqlite::Row| -> rusqlite::Result<(i64, i64, i64)> {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?))
            };
            match client {
                Some(name) => c.query_row(
                    &format!("{SQL} AND c.name = ?3"),
                    rusqlite::params![from_day, to_day, name],
                    row,
                ),
                None => c.query_row(SQL, rusqlite::params![from_day, to_day], row),
            }
        });
        match res {
            Ok((rx, tx, online_minutes)) => PeriodTotals {
                rx: rx as u64,
                tx: tx as u64,
                online_minutes: online_minutes as u64,
            },
            Err(e) => {
                tracing::error!(error = %e, "traffic totals не получены");
                PeriodTotals::default()
            }
        }
    }

    /// client=None → по всем клиентам. Всё из traffic_daily (текущий день
    /// обновляется rollup'ом каждые 5 мин — лаг задокументирован).
    pub fn traffic_summary(&self, client: Option<&str>, now: i64) -> TrafficSummary {
        let day = now / 86400 * 86400;
        TrafficSummary {
            today: self.totals(client, day, day),
            d7: self.totals(client, day - 6 * 86400, day),
            d30: self.totals(client, day - 29 * 86400, day),
            total: self.totals(client, 0, day),
            prev7: self.totals(client, day - 13 * 86400, day - 7 * 86400),
        }
    }

    /// Топ клиентов по rx+tx за последние `days` дней (включая сегодня;
    /// участвуют и уже удалённые клиенты — история трафика не привязана
    /// к текущему статусу removed_at).
    pub fn top_clients(&self, days: i64, limit: usize, now: i64) -> Vec<(String, u64)> {
        let day = now / 86400 * 86400;
        let from_day = day - (days - 1) * 86400;
        let res = self.with_conn(|c| {
            let mut stmt = c.prepare(
                "SELECT c.name, SUM(d.rx_bytes + d.tx_bytes) AS total
                 FROM traffic_daily d JOIN clients c ON c.id = d.client_id
                 WHERE d.day_ts BETWEEN ?1 AND ?2
                 GROUP BY c.name ORDER BY total DESC LIMIT ?3",
            )?;
            let rows = stmt.query_map(rusqlite::params![from_day, day, limit as i64], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
            })?;
            rows.collect::<rusqlite::Result<Vec<_>>>()
        });
        match res {
            Ok(rows) => rows.into_iter().map(|(name, t)| (name, t as u64)).collect(),
            Err(e) => {
                tracing::error!(error = %e, "top_clients не получены");
                Vec::new()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::Store;

    fn s(name: &str, rx: u64, tx: u64, hs: Option<i64>) -> Sample {
        Sample {
            name: name.into(),
            ip: "10.0.0.2".into(),
            rx,
            tx,
            last_handshake: hs,
        }
    }
    /// Сидирует traffic_daily напрямую (upsert клиента + запись за день),
    /// минуя ingest/rollup — для тестов агрегатных запросов.
    /// `day_index` — абсолютный номер дня (day_ts = day_index * 86400).
    fn seed_daily(store: &Store, name: &str, day_index: i64, rx: u64, tx: u64) {
        store
            .with_conn(|c| {
                c.execute(
                    "INSERT INTO clients(name, ip, first_seen, last_seen) VALUES(?1, '', 0, 0)
                     ON CONFLICT(name) DO NOTHING",
                    [name],
                )?;
                let client_id: i64 = c.query_row(
                    "SELECT id FROM clients WHERE name=?1",
                    [name],
                    |r| r.get(0),
                )?;
                c.execute(
                    "INSERT INTO traffic_daily(client_id, day_ts, rx_bytes, tx_bytes, online_minutes)
                     VALUES(?1, ?2, ?3, ?4, ?5)
                     ON CONFLICT(client_id, day_ts) DO UPDATE SET rx_bytes=?3, tx_bytes=?4, online_minutes=?5",
                    rusqlite::params![client_id, day_index * 86400, rx as i64, tx as i64, 1_i64],
                )?;
                Ok(())
            })
            .unwrap();
    }

    #[test]
    fn traffic_summary_periods_and_trend_windows() {
        let store = Store::open_in_memory();
        let now = 100 * 86400 + 3600; // день 100, 01:00 UTC
                                      // День 99 (не 100!): бриф просил "сегодня — ничего", а день 100 совпал бы
                                      // с today и дал бы rx=10 там; день 99 всё ещё внутри d7/d30/total.
        seed_daily(&store, "alice", 99, 10, 1);
        seed_daily(&store, "alice", 97, 20, 2); // внутри 7д
        seed_daily(&store, "alice", 91, 40, 4); // prev7 (93..100-7)
        seed_daily(&store, "alice", 50, 80, 8); // день 50 — вне окна 30д, только в total
        let s = store.traffic_summary(Some("alice"), now);
        assert_eq!(s.today.rx, 0); // за сегодня (день 100) ничего
        assert_eq!(s.d7.rx, 10 + 20); // дни 99 и 97
        assert_eq!(s.prev7.rx, 40); // день 91
        assert_eq!(s.d30.rx, 10 + 20 + 40);
        assert_eq!(s.total.rx, 10 + 20 + 40 + 80);
    }

    #[test]
    fn traffic_summary_add_sums_componentwise() {
        let mut a = TrafficSummary {
            today: PeriodTotals {
                rx: 1,
                tx: 2,
                online_minutes: 3,
            },
            total: PeriodTotals {
                rx: 10,
                tx: 20,
                online_minutes: 30,
            },
            ..Default::default()
        };
        let b = TrafficSummary {
            today: PeriodTotals {
                rx: 4,
                tx: 5,
                online_minutes: 6,
            },
            prev7: PeriodTotals {
                rx: 7,
                tx: 8,
                online_minutes: 9,
            },
            ..Default::default()
        };
        a.add(&b);
        assert_eq!(
            a.today,
            PeriodTotals {
                rx: 5,
                tx: 7,
                online_minutes: 9
            }
        );
        assert_eq!(
            a.total,
            PeriodTotals {
                rx: 10,
                tx: 20,
                online_minutes: 30
            }
        );
        assert_eq!(
            a.prev7,
            PeriodTotals {
                rx: 7,
                tx: 8,
                online_minutes: 9
            }
        );
    }

    #[test]
    fn top_clients_orders_by_total_traffic() {
        let store = Store::open_in_memory();
        let now = 100 * 86400;
        seed_daily(&store, "alice", 99, 100, 0);
        seed_daily(&store, "bob", 99, 500, 0);
        let top = store.top_clients(7, 5, now);
        assert_eq!(top[0].0, "bob");
        assert_eq!(top[0].1, 500);
    }

    fn sample_rows(store: &Store) -> Vec<(i64, i64, i64, i64)> {
        // (ts, rx_delta, tx_delta, online)
        store
            .with_conn(|c| {
                let mut st = c.prepare(
                    "SELECT ts, rx_delta, tx_delta, online FROM traffic_samples ORDER BY ts",
                )?;
                let rows =
                    st.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))?;
                rows.collect()
            })
            .unwrap()
    }

    #[test]
    fn first_sample_creates_client_with_zero_delta() {
        let store = Store::open_in_memory();
        store.ingest(1000, &[s("alice", 500, 300, Some(990))]);
        assert_eq!(sample_rows(&store), vec![(1000, 0, 0, 1)]); // первый сэмпл — базовая линия
    }

    #[test]
    fn delta_is_diff_from_previous_sample() {
        let store = Store::open_in_memory();
        store.ingest(1000, &[s("alice", 500, 300, Some(990))]);
        store.ingest(1060, &[s("alice", 800, 450, Some(1050))]);
        assert_eq!(sample_rows(&store)[1], (1060, 300, 150, 1));
    }

    // Ключ к устойчивости при ребутах VPS: счётчики WG обнулились → дельта =
    // новое значение (весь трафик с ребута), а не отрицательный мусор.
    #[test]
    fn counter_reset_uses_new_value_as_delta() {
        let store = Store::open_in_memory();
        store.ingest(1000, &[s("alice", 500, 300, Some(990))]);
        store.ingest(1060, &[s("alice", 120, 40, Some(1050))]); // rx < prev → сброс
        assert_eq!(sample_rows(&store)[1], (1060, 120, 40, 1));
    }

    #[test]
    fn online_offline_transitions_logged_as_events() {
        let store = Store::open_in_memory();
        store.ingest(1000, &[s("alice", 0, 0, Some(950))]); // online
        store.ingest(1600, &[s("alice", 0, 0, Some(950))]); // 650с от hs → offline
        store.ingest(1660, &[s("alice", 0, 0, Some(1655))]); // снова online
        let kinds: Vec<String> = store
            .with_conn(|c| {
                let mut st = c.prepare("SELECT kind FROM events ORDER BY id")?;
                let rows = st.query_map([], |r| r.get(0))?;
                rows.collect()
            })
            .unwrap();
        assert_eq!(kinds, vec!["online", "offline", "online"]);
    }

    #[test]
    fn absent_client_marked_removed_and_revived() {
        let store = Store::open_in_memory();
        store.ingest(1000, &[s("alice", 0, 0, None), s("bob", 0, 0, None)]);
        store.ingest(1060, &[s("alice", 0, 0, None)]); // bob исчез (удалён через CLI)
        let removed: Option<i64> = store
            .with_conn(|c| {
                c.query_row("SELECT removed_at FROM clients WHERE name='bob'", [], |r| {
                    r.get(0)
                })
            })
            .unwrap();
        assert_eq!(removed, Some(1060));
        store.ingest(1120, &[s("alice", 0, 0, None), s("bob", 0, 0, None)]); // вернулся
        let removed: Option<i64> = store
            .with_conn(|c| {
                c.query_row("SELECT removed_at FROM clients WHERE name='bob'", [], |r| {
                    r.get(0)
                })
            })
            .unwrap();
        assert_eq!(removed, None);
    }

    #[test]
    fn empty_samples_marks_all_removed() {
        let store = Store::open_in_memory();
        store.ingest(1000, &[s("alice", 0, 0, None)]);
        store.ingest(1060, &[]);
        let removed: Option<i64> = store
            .with_conn(|c| {
                c.query_row(
                    "SELECT removed_at FROM clients WHERE name='alice'",
                    [],
                    |r| r.get(0),
                )
            })
            .unwrap();
        assert_eq!(removed, Some(1060));
    }

    #[test]
    fn rollup_aggregates_hourly_and_daily() {
        let store = Store::open_in_memory();
        let day = 1_700_006_400; // кратно 86400 (UTC-полночь)
                                 // два сэмпла в час 0, один — в час 1
        store.ingest(day + 60, &[s("alice", 100, 10, Some(day + 50))]); // базовая линия
        store.ingest(day + 120, &[s("alice", 400, 40, Some(day + 110))]); // +300/+30
        store.ingest(day + 3660, &[s("alice", 500, 90, None)]); // +100/+50, offline
        store.rollup(day + 3700);
        let hourly: Vec<(i64, i64, i64, i64)> = store.with_conn(|c| {
            let mut st = c.prepare("SELECT hour_ts, rx_bytes, tx_bytes, online_minutes FROM traffic_hourly ORDER BY hour_ts")?;
            let rows = st.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))?;
            rows.collect()
        }).unwrap();
        assert_eq!(hourly, vec![(day, 300, 30, 2), (day + 3600, 100, 50, 0)]);
        let daily: (i64, i64, i64) = store
            .with_conn(|c| {
                c.query_row(
                    "SELECT rx_bytes, tx_bytes, online_minutes FROM traffic_daily WHERE day_ts=?1",
                    [day],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
                )
            })
            .unwrap();
        assert_eq!(daily, (400, 80, 2));
    }

    #[test]
    fn rollup_is_idempotent() {
        let store = Store::open_in_memory();
        let day = 1_700_006_400;
        store.ingest(day + 60, &[s("alice", 100, 10, Some(day))]);
        store.ingest(day + 120, &[s("alice", 200, 20, Some(day + 110))]);
        store.rollup(day + 200);
        store.rollup(day + 260); // повторный прогон не задваивает
        let rx: i64 = store
            .with_conn(|c| {
                c.query_row(
                    "SELECT rx_bytes FROM traffic_hourly WHERE hour_ts=?1",
                    [day],
                    |r| r.get(0),
                )
            })
            .unwrap();
        assert_eq!(rx, 100);
    }

    #[test]
    fn prune_removes_old_rows_keeps_daily() {
        let store = Store::open_in_memory();
        let now = 100 * 86400;
        store.ingest(now - 8 * 86400, &[s("alice", 1, 1, None)]); // старше 7 дней
        store.ingest(now - 60, &[s("alice", 2, 2, None)]);
        store.rollup(now);
        store.prune(now);
        let samples: i64 = store
            .with_conn(|c| c.query_row("SELECT COUNT(*) FROM traffic_samples", [], |r| r.get(0)))
            .unwrap();
        assert_eq!(samples, 1); // только свежий
        let daily: i64 = store
            .with_conn(|c| c.query_row("SELECT COUNT(*) FROM traffic_daily", [], |r| r.get(0)))
            .unwrap();
        assert!(daily >= 1); // дневные живут вечно
    }

    #[test]
    fn rollup_never_overwrites_aggregates_beyond_retention() {
        let store = Store::open_in_memory();
        let day = 1_700_006_400;
        store.ingest(day + 60, &[s("alice", 100, 10, Some(day))]);
        store.ingest(day + 120, &[s("alice", 400, 40, Some(day))]); // дельта 300
        store.rollup(day + 200); // hourly(day) = 300
                                 // имитация нарушенного инварианта: часть сэмплов удалена, meta потеряна
        store
            .with_conn(|c| c.execute("DELETE FROM traffic_samples WHERE ts = ?1", [day + 120]))
            .unwrap();
        store
            .with_conn(|c| c.execute("UPDATE meta SET value='0' WHERE key='last_rollup_ts'", []))
            .unwrap();
        let now = day + 8 * 86400; // час day уже за границей RAW_RETENTION
        store.rollup(now);
        let rx: i64 = store
            .with_conn(|c| {
                c.query_row(
                    "SELECT rx_bytes FROM traffic_hourly WHERE hour_ts=?1",
                    [day],
                    |r| r.get(0),
                )
            })
            .unwrap();
        assert_eq!(rx, 300); // агрегат НЕ затёрт частичной суммой
    }
}
