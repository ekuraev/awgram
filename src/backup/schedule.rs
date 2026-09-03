//! Расписание автобэкапа: чистые функции над локальным NaiveDateTime (без
//! TZ — так тесты не зависят от TZ машины) и фоновый таск по образцу
//! collector::run. Пока серия сбоев открыта, слоты расписания не важны:
//! повтор с нарастающим интервалом (`retry_interval`), уведомление не чаще
//! RENOTIFY_SECS.

use std::sync::Arc;

use chrono::{Datelike, Duration, Local, NaiveDate, NaiveDateTime, NaiveTime};
use teloxide::Bot;

use crate::backup::service;
use crate::config::Config;
use crate::store::{BackupFailure, BackupKind, BackupSchedule, Period, Store};
use crate::vpn::Vpn;

pub const RETRY_SECS: i64 = 3600;
pub const MAX_RETRY_SECS: i64 = 86_400;
pub const RENOTIFY_SECS: i64 = 21_600;
const TICK_SECS: u64 = 60;

/// Пауза до следующей попытки после `attempts` неудач: `RETRY_SECS × 2^(n−1)`,
/// но не больше суток. Часть сбоев неустранима без вмешательства человека
/// (архивы инсталлера в hardened-режиме — root:600), и ежечасный повтор в
/// таком случае вечно молотит диск, создавая каждый раз новый архив
/// инсталлера. Нарастающий интервал оставляет шанс на самовосстановление
/// (место на диске освободилось), но перестаёт быть фоновой нагрузкой.
pub fn retry_interval(attempts: u32) -> i64 {
    let shift = attempts.saturating_sub(1).min(31);
    RETRY_SECS.saturating_mul(1i64 << shift).min(MAX_RETRY_SECS)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    Idle,
    Run,
}

fn at(s: &BackupSchedule, d: NaiveDate) -> NaiveDateTime {
    d.and_time(NaiveTime::from_hms_opt(s.hour as u32, s.minute as u32, 0).unwrap_or_default())
}

fn monday_on_or_before(d: NaiveDate) -> NaiveDate {
    d - Duration::days(d.weekday().num_days_from_monday() as i64)
}

fn first_of_month(d: NaiveDate) -> NaiveDate {
    NaiveDate::from_ymd_opt(d.year(), d.month(), 1).unwrap_or(d)
}

fn prev_month_first(d: NaiveDate) -> NaiveDate {
    let (y, m) = if d.month() == 1 {
        (d.year() - 1, 12)
    } else {
        (d.year(), d.month() - 1)
    };
    NaiveDate::from_ymd_opt(y, m, 1).unwrap_or(d)
}

fn next_month_first(d: NaiveDate) -> NaiveDate {
    let (y, m) = if d.month() == 12 {
        (d.year() + 1, 1)
    } else {
        (d.year(), d.month() + 1)
    };
    NaiveDate::from_ymd_opt(y, m, 1).unwrap_or(d)
}

/// Последний слот расписания, не позже `now`.
pub fn due_slot(s: &BackupSchedule, now: NaiveDateTime) -> Option<NaiveDateTime> {
    let today = now.date();
    let candidate = match s.period {
        Period::Off => return None,
        Period::Daily => at(s, today),
        Period::Weekly => at(s, monday_on_or_before(today)),
        Period::Monthly => at(s, first_of_month(today)),
    };
    if candidate <= now {
        return Some(candidate);
    }
    let prev = match s.period {
        Period::Off => return None,
        Period::Daily => today - Duration::days(1),
        Period::Weekly => monday_on_or_before(today) - Duration::days(7),
        Period::Monthly => prev_month_first(today),
    };
    Some(at(s, prev))
}

/// Первый слот строго после `now` (для строки «Следующий запуск»).
pub fn next_slot(s: &BackupSchedule, now: NaiveDateTime) -> Option<NaiveDateTime> {
    let due = due_slot(s, now)?;
    let d = due.date();
    let next = match s.period {
        Period::Off => return None,
        Period::Daily => d + Duration::days(1),
        Period::Weekly => d + Duration::days(7),
        Period::Monthly => next_month_first(d),
    };
    Some(at(s, next))
}

pub fn decide(
    s: &BackupSchedule,
    last_auto: Option<i64>,
    failure: Option<&BackupFailure>,
    now_epoch: i64,
    now_local: NaiveDateTime,
    to_epoch: impl Fn(NaiveDateTime) -> i64,
) -> Decision {
    if s.period == Period::Off {
        return Decision::Idle;
    }
    if let Some(f) = failure {
        return if now_epoch - f.last_attempt >= retry_interval(f.attempts) {
            Decision::Run
        } else {
            Decision::Idle
        };
    }
    let Some(slot) = due_slot(s, now_local) else {
        return Decision::Idle;
    };
    let slot_epoch = to_epoch(slot);
    match last_auto {
        Some(t) if t >= slot_epoch => Decision::Idle,
        _ => Decision::Run,
    }
}

/// Число неудачных попыток, если успех закрыл серию сбоев.
pub fn after_success(prev: Option<&BackupFailure>) -> Option<u32> {
    prev.map(|f| f.attempts)
}

/// Обновлённая серия сбоев и надо ли уведомлять сейчас.
pub fn after_failure(prev: Option<&BackupFailure>, now: i64) -> (BackupFailure, bool) {
    match prev {
        None => (
            BackupFailure {
                since: now,
                attempts: 1,
                last_attempt: now,
                last_notified: now,
            },
            true,
        ),
        Some(f) => {
            let notify = now - f.last_notified >= RENOTIFY_SECS;
            (
                BackupFailure {
                    since: f.since,
                    attempts: f.attempts + 1,
                    last_attempt: now,
                    last_notified: if notify { now } else { f.last_notified },
                },
                notify,
            )
        }
    }
}

pub fn local_now() -> (i64, NaiveDateTime) {
    let n = Local::now();
    (n.timestamp(), n.naive_local())
}

pub fn local_to_epoch(t: NaiveDateTime) -> i64 {
    use chrono::TimeZone;
    Local
        .from_local_datetime(&t)
        .earliest()
        .map(|d| d.timestamp())
        .unwrap_or_else(|| t.and_utc().timestamp())
}

/// Один тик: решение, запуск, учёт результата. Возвращает событие для
/// рассылки (подключается в notify), чтобы сам тик оставался тестируемым.
pub async fn tick(vpn: &Arc<Vpn>, store: &Arc<Store>) -> Option<TickEvent> {
    let s = store.backup_schedule();
    let (now, local) = local_now();
    let failure = store.backup_failure();
    if decide(
        &s,
        store.backup_last_auto(),
        failure.as_ref(),
        now,
        local,
        local_to_epoch,
    ) == Decision::Idle
    {
        return None;
    }
    match service::create(
        vpn,
        store,
        BackupKind::Auto,
        None,
        None,
        s.include_db,
        s.keep,
    )
    .await
    {
        Ok(created) => {
            store.set_backup_last_auto(now);
            let recovered = after_success(failure.as_ref());
            store.set_backup_failure(None);
            Some(TickEvent::Ok {
                created,
                recovered_after: recovered,
                notify_ok: s.notify_ok,
            })
        }
        Err(e) => {
            tracing::error!(error = %e, "автобэкап провалился");
            let (f, notify) = after_failure(failure.as_ref(), now);
            store.set_backup_failure(Some(&f));
            Some(TickEvent::Failed {
                error: e,
                failure: f,
                notify,
            })
        }
    }
}

pub enum TickEvent {
    Ok {
        created: service::Created,
        recovered_after: Option<u32>,
        notify_ok: bool,
    },
    Failed {
        error: crate::error::Error,
        failure: BackupFailure,
        notify: bool,
    },
}

pub async fn run(bot: Bot, cfg: Arc<Config>, vpn: Arc<Vpn>, store: Arc<Store>) {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(TICK_SECS));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        interval.tick().await;
        if let Some(ev) = tick(&vpn, &store).await {
            crate::backup::notify::on_tick(&bot, &cfg, &store, &vpn, ev).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::{BackupFailure, BackupSchedule, Period};
    use chrono::NaiveDate;

    fn dt(y: i32, m: u32, d: u32, h: u32, mi: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(y, m, d)
            .unwrap()
            .and_hms_opt(h, mi, 0)
            .unwrap()
    }
    fn sched(p: Period) -> BackupSchedule {
        BackupSchedule {
            period: p,
            hour: 3,
            minute: 0,
            ..BackupSchedule::default()
        }
    }
    fn epoch(t: NaiveDateTime) -> i64 {
        t.and_utc().timestamp()
    }

    #[test]
    fn due_slot_daily_before_and_after_time() {
        let s = sched(Period::Daily);
        // 02:59 → вчерашний слот
        assert_eq!(
            due_slot(&s, dt(2026, 9, 3, 2, 59)),
            Some(dt(2026, 9, 2, 3, 0))
        );
        // 03:00 → сегодняшний
        assert_eq!(
            due_slot(&s, dt(2026, 9, 3, 3, 0)),
            Some(dt(2026, 9, 3, 3, 0))
        );
        assert_eq!(
            due_slot(&s, dt(2026, 9, 3, 23, 0)),
            Some(dt(2026, 9, 3, 3, 0))
        );
        assert_eq!(
            next_slot(&s, dt(2026, 9, 3, 3, 0)),
            Some(dt(2026, 9, 4, 3, 0))
        );
        assert_eq!(
            next_slot(&s, dt(2026, 9, 3, 2, 0)),
            Some(dt(2026, 9, 3, 3, 0))
        );
    }

    #[test]
    fn due_slot_weekly_monday_and_monthly_first() {
        let s = sched(Period::Weekly);
        // 2026-09-03 — четверг; ближайший прошедший понедельник 2026-08-31
        assert_eq!(
            due_slot(&s, dt(2026, 9, 3, 12, 0)),
            Some(dt(2026, 8, 31, 3, 0))
        );
        // понедельник 02:00 — ещё прошлый понедельник
        assert_eq!(
            due_slot(&s, dt(2026, 8, 31, 2, 0)),
            Some(dt(2026, 8, 24, 3, 0))
        );
        assert_eq!(
            next_slot(&s, dt(2026, 9, 3, 12, 0)),
            Some(dt(2026, 9, 7, 3, 0))
        );
        let m = sched(Period::Monthly);
        assert_eq!(
            due_slot(&m, dt(2026, 9, 3, 12, 0)),
            Some(dt(2026, 9, 1, 3, 0))
        );
        assert_eq!(
            due_slot(&m, dt(2026, 9, 1, 2, 0)),
            Some(dt(2026, 8, 1, 3, 0))
        );
        assert_eq!(
            next_slot(&m, dt(2026, 12, 15, 0, 0)),
            Some(dt(2027, 1, 1, 3, 0))
        );
        assert_eq!(due_slot(&sched(Period::Off), dt(2026, 9, 3, 12, 0)), None);
    }

    #[test]
    fn decide_runs_once_per_slot_and_catches_up_after_downtime() {
        let s = sched(Period::Daily);
        let now = dt(2026, 9, 3, 3, 1);
        // никогда не запускался, слот прошёл → Run
        assert_eq!(
            decide(&s, None, None, epoch(now), now, epoch),
            Decision::Run
        );
        // уже бежал в этом слоте → Idle
        let ran = epoch(dt(2026, 9, 3, 3, 0));
        assert_eq!(
            decide(&s, Some(ran), None, epoch(now), now, epoch),
            Decision::Idle
        );
        // даунтайм: последний запуск позавчера, сейчас полдень → один Run
        let old = epoch(dt(2026, 9, 1, 3, 0));
        let noon = dt(2026, 9, 3, 12, 0);
        assert_eq!(
            decide(&s, Some(old), None, epoch(noon), noon, epoch),
            Decision::Run
        );
        // после него — Idle до завтра
        assert_eq!(
            decide(&s, Some(epoch(noon)), None, epoch(noon), noon, epoch),
            Decision::Idle
        );
        // Off — никогда
        assert_eq!(
            decide(&sched(Period::Off), None, None, epoch(now), now, epoch),
            Decision::Idle
        );
    }

    #[test]
    fn retry_interval_doubles_and_caps_at_a_day() {
        assert_eq!(retry_interval(1), 3600);
        assert_eq!(retry_interval(2), 7200);
        assert_eq!(retry_interval(5), 57_600);
        assert_eq!(retry_interval(6), 86_400);
        assert_eq!(retry_interval(10), 86_400);
        // без переполнения на любой глубине серии
        assert_eq!(retry_interval(u32::MAX), MAX_RETRY_SECS);
        // attempts=0 в БД не появляется, но и он не должен ломать арифметику
        assert_eq!(retry_interval(0), RETRY_SECS);
    }

    #[test]
    fn decide_retries_with_backoff_while_failing() {
        let s = sched(Period::Daily);
        let now = dt(2026, 9, 3, 12, 0);
        // две неудачи подряд → следующая попытка не раньше чем через 2 часа
        let f = BackupFailure {
            since: epoch(now) - 7200,
            attempts: 2,
            last_attempt: epoch(now) - 7199,
            last_notified: 0,
        };
        assert_eq!(
            decide(
                &s,
                Some(epoch(now) - 86_400 * 2),
                Some(&f),
                epoch(now),
                now,
                epoch
            ),
            Decision::Idle
        );
        // час после второй неудачи — ещё рано (при прежнем поведении был бы Run)
        let hour_ago = BackupFailure {
            last_attempt: epoch(now) - 3600,
            ..f.clone()
        };
        assert_eq!(
            decide(
                &s,
                Some(epoch(now) - 86_400 * 2),
                Some(&hour_ago),
                epoch(now),
                now,
                epoch
            ),
            Decision::Idle
        );
        let f2 = BackupFailure {
            last_attempt: epoch(now) - 7200,
            ..f.clone()
        };
        assert_eq!(
            decide(
                &s,
                Some(epoch(now) - 86_400 * 2),
                Some(&f2),
                epoch(now),
                now,
                epoch
            ),
            Decision::Run
        );
        // сбой при выключенном расписании не ретраится
        assert_eq!(
            decide(&sched(Period::Off), None, Some(&f2), epoch(now), now, epoch),
            Decision::Idle
        );
    }

    #[test]
    fn failure_bookkeeping() {
        let (f, notify) = after_failure(None, 1000);
        assert_eq!(
            (f.since, f.attempts, f.last_attempt, f.last_notified),
            (1000, 1, 1000, 1000)
        );
        assert!(notify);
        let (f2, notify) = after_failure(Some(&f), 1000 + 3600);
        assert_eq!(
            (f2.since, f2.attempts, f2.last_attempt, f2.last_notified),
            (1000, 2, 4600, 1000)
        );
        assert!(!notify);
        let (f3, notify) = after_failure(Some(&f2), 1000 + RENOTIFY_SECS);
        assert_eq!(f3.last_notified, 1000 + RENOTIFY_SECS);
        assert!(notify);
        assert_eq!(after_success(None), None);
        assert_eq!(after_success(Some(&f3)), Some(3));
    }
}
