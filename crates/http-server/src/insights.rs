use botflow_workflow_model::{RunDoc, RunStatus};
use chrono::{DateTime, Duration, Timelike, Utc};
use serde_json::{json, Value};
use std::collections::BTreeMap;

struct PeriodBounds {
    from: DateTime<Utc>,
    to: DateTime<Utc>,
    prev_from: DateTime<Utc>,
    prev_to: DateTime<Utc>,
}

fn period_bounds(days: i64) -> PeriodBounds {
    let days = days.max(1);
    let to = Utc::now();
    let from = to - Duration::days(days);
    let prev_to = from;
    let prev_from = from - Duration::days(days);
    PeriodBounds {
        from,
        to,
        prev_from,
        prev_to,
    }
}

struct Summary {
    total: f64,
    failed: f64,
    failure_rate: f64,
    time_saved_minutes: f64,
    average_run_time_ms: f64,
}

fn summarize(list: &[&RunDoc]) -> Summary {
    let total = list.len() as f64;
    let failed = list
        .iter()
        .filter(|r| matches!(r.status, RunStatus::Failed))
        .count() as f64;
    let failure_rate = if total > 0.0 { failed / total } else { 0.0 };
    let time_saved_minutes = list
        .iter()
        .map(|r| r.time_saved_minutes.unwrap_or(0.0))
        .sum();
    let with_duration: Vec<f64> = list.iter().filter_map(|r| run_duration_ms(r)).collect();
    let average_run_time_ms = if with_duration.is_empty() {
        0.0
    } else {
        with_duration.iter().sum::<f64>() / with_duration.len() as f64
    };
    Summary {
        total,
        failed,
        failure_rate,
        time_saved_minutes,
        average_run_time_ms,
    }
}

fn run_duration_ms(run: &RunDoc) -> Option<f64> {
    let finished = run.finished_at?;
    let micros = (finished - run.started_at).num_microseconds()?;
    if micros < 0 {
        return None;
    }
    // Sub-ms runs still count so the chart isn't a flat zero line.
    Some((micros as f64 / 1000.0).max(0.001))
}

fn in_range(run: &RunDoc, from: DateTime<Utc>, to: DateTime<Utc>, end_inclusive: bool) -> bool {
    if run.started_at < from {
        return false;
    }
    if end_inclusive {
        run.started_at <= to
    } else {
        run.started_at < to
    }
}

fn day_key(dt: DateTime<Utc>) -> String {
    dt.date_naive().format("%Y-%m-%d").to_string()
}

fn hour_key(dt: DateTime<Utc>) -> String {
    dt.format("%Y-%m-%d %H:00").to_string()
}

fn status_label(status: RunStatus) -> &'static str {
    match status {
        RunStatus::Created => "created",
        RunStatus::Queued => "queued",
        RunStatus::Running => "running",
        RunStatus::Waiting => "waiting",
        RunStatus::Retrying => "retrying",
        RunStatus::Succeeded => "completed",
        RunStatus::Failed => "failed",
        RunStatus::Cancelled => "cancelled",
    }
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    if sorted.len() == 1 {
        return sorted[0];
    }
    let rank = (p.clamp(0.0, 1.0)) * (sorted.len() as f64 - 1.0);
    let lo = rank.floor() as usize;
    let hi = rank.ceil() as usize;
    if lo == hi {
        sorted[lo]
    } else {
        let w = rank - lo as f64;
        sorted[lo] * (1.0 - w) + sorted[hi] * w
    }
}

fn latency_percentiles(list: &[&RunDoc]) -> (f64, f64, f64) {
    let mut durations: Vec<f64> = list.iter().filter_map(|r| run_duration_ms(r)).collect();
    durations.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    (
        percentile(&durations, 0.50),
        percentile(&durations, 0.95),
        percentile(&durations, 0.99),
    )
}

fn trigger_counts(list: &[&RunDoc]) -> BTreeMap<String, f64> {
    let mut map: BTreeMap<String, f64> = BTreeMap::new();
    for run in list {
        let key = run
            .trigger
            .as_deref()
            .filter(|t| !t.is_empty())
            .unwrap_or("manual")
            .to_string();
        *map.entry(key).or_default() += 1.0;
    }
    map
}

fn bucket_point(label: String, runs: &[&RunDoc]) -> Value {
    let s = summarize(runs);
    let (p50, p95, p99) = latency_percentiles(runs);
    json!({
        "date": label,
        "total": s.total,
        "success": (s.total - s.failed).max(0.0),
        "failed": s.failed,
        "failureRate": s.failure_rate,
        "timeSavedMinutes": s.time_saved_minutes,
        "averageRunTimeMs": s.average_run_time_ms,
        "p50Ms": p50,
        "p95Ms": p95,
        "p99Ms": p99,
    })
}

fn build_by_day(list: &[&RunDoc], from: DateTime<Utc>, to: DateTime<Utc>) -> Vec<Value> {
    let mut buckets: BTreeMap<String, Vec<&RunDoc>> = BTreeMap::new();
    let mut cursor = from
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .map(|n| DateTime::<Utc>::from_naive_utc_and_offset(n, Utc))
        .unwrap_or(from)
        .with_nanosecond(0)
        .unwrap_or(from);
    let end = to
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .map(|n| DateTime::<Utc>::from_naive_utc_and_offset(n, Utc))
        .unwrap_or(to);

    while cursor <= end {
        buckets.insert(day_key(cursor), Vec::new());
        cursor += Duration::days(1);
    }

    for run in list {
        let key = day_key(run.started_at);
        if let Some(bucket) = buckets.get_mut(&key) {
            bucket.push(*run);
        }
    }

    buckets
        .into_iter()
        .map(|(date, day_runs)| bucket_point(date, &day_runs))
        .collect()
}

fn build_by_hour(list: &[&RunDoc], from: DateTime<Utc>, to: DateTime<Utc>) -> Vec<Value> {
    let mut buckets: BTreeMap<String, Vec<&RunDoc>> = BTreeMap::new();
    let mut cursor = from
        .with_minute(0)
        .and_then(|d| d.with_second(0))
        .and_then(|d| d.with_nanosecond(0))
        .unwrap_or(from);
    let end = to
        .with_minute(0)
        .and_then(|d| d.with_second(0))
        .and_then(|d| d.with_nanosecond(0))
        .unwrap_or(to);

    while cursor <= end {
        buckets.insert(hour_key(cursor), Vec::new());
        cursor += Duration::hours(1);
    }

    for run in list {
        let key = hour_key(run.started_at);
        if let Some(bucket) = buckets.get_mut(&key) {
            bucket.push(*run);
        }
    }

    // Drop leading empty hours so the chart isn't a flat line with a spike at the end.
    let mut points: Vec<Value> = buckets
        .into_iter()
        .map(|(label, hour_runs)| bucket_point(label, &hour_runs))
        .collect();
    while points
        .first()
        .and_then(|p| p.get("total"))
        .and_then(|v| v.as_f64())
        == Some(0.0)
        && points.len() > 1
    {
        points.remove(0);
    }
    points
}

/// Prefer hourly buckets when traffic is concentrated (≤2 active days);
/// otherwise keep daily buckets for longer ranges.
fn build_by_time(list: &[&RunDoc], from: DateTime<Utc>, to: DateTime<Utc>) -> Vec<Value> {
    let active_days: BTreeMap<String, ()> = list
        .iter()
        .map(|r| (day_key(r.started_at), ()))
        .collect();
    if active_days.len() <= 2 {
        // Focus the window on traffic (pad 2h before first run) so charts aren't empty.
        let start = list
            .iter()
            .map(|r| r.started_at)
            .min()
            .map(|t| t - Duration::hours(2))
            .unwrap_or(from)
            .max(from);
        build_by_hour(list, start, to)
    } else {
        let mut points = build_by_day(list, from, to);
        while points
            .first()
            .and_then(|p| p.get("total"))
            .and_then(|v| v.as_f64())
            == Some(0.0)
            && points.len() > 1
        {
            points.remove(0);
        }
        points
    }
}

fn execution_rows(list: &[&RunDoc]) -> Vec<Value> {
    let mut rows: Vec<Value> = list
        .iter()
        .map(|r| {
            let duration_ms = run_duration_ms(r);
            json!({
                "id": r.id.clone().unwrap_or_default(),
                "at": r.started_at.to_rfc3339(),
                "date": day_key(r.started_at),
                "durationMs": duration_ms,
                "status": status_label(r.status),
                "trigger": r.trigger.clone().unwrap_or_else(|| "manual".into()),
            })
        })
        .collect();
    rows.sort_by(|a, b| {
        let at = a.get("at").and_then(|v| v.as_str()).unwrap_or("");
        let bt = b.get("at").and_then(|v| v.as_str()).unwrap_or("");
        bt.cmp(at)
    });
    rows.truncate(50);
    rows
}

/// Build the `{ insights: … }` payload, optionally scoped to a single workflow.
pub fn build_insights_filtered(runs: &[RunDoc], days: i64, workflow_id: Option<&str>) -> Value {
    let bounds = period_bounds(days);
    let scoped: Vec<&RunDoc> = runs
        .iter()
        .filter(|r| workflow_id.map(|id| r.workflow_id == id).unwrap_or(true))
        .collect();
    let current: Vec<&RunDoc> = scoped
        .iter()
        .copied()
        .filter(|r| in_range(r, bounds.from, bounds.to, true))
        .collect();
    let previous: Vec<&RunDoc> = scoped
        .iter()
        .copied()
        .filter(|r| in_range(r, bounds.prev_from, bounds.prev_to, false))
        .collect();

    let cur = summarize(&current);
    let prev = summarize(&previous);
    let (p50, p95, p99) = latency_percentiles(&current);
    let (prev_p50, _prev_p95, _prev_p99) = latency_percentiles(&previous);
    let triggers = trigger_counts(&current);

    let mut by_map: BTreeMap<String, Vec<&RunDoc>> = BTreeMap::new();
    for run in &current {
        by_map
            .entry(run.workflow_id.clone())
            .or_default()
            .push(*run);
    }

    let mut by_workflow: Vec<Value> = by_map
        .into_iter()
        .map(|(workflow_id, list)| {
            let s = summarize(&list);
            let workflow_name = list
                .first()
                .and_then(|r| r.workflow_name.clone())
                .filter(|n| !n.is_empty())
                .unwrap_or_else(|| workflow_id.clone());
            let (wf_p50, wf_p95, wf_p99) = latency_percentiles(&list);
            json!({
                "workflowId": workflow_id,
                "workflowName": workflow_name,
                "total": s.total,
                "success": (s.total - s.failed).max(0.0),
                "failed": s.failed,
                "failureRate": s.failure_rate,
                "timeSavedMinutes": s.time_saved_minutes,
                "averageRunTimeMs": s.average_run_time_ms,
                "p50Ms": wf_p50,
                "p95Ms": wf_p95,
                "p99Ms": wf_p99,
                "byTime": build_by_time(&list, bounds.from, bounds.to),
                "executions": execution_rows(&list),
            })
        })
        .collect();
    by_workflow.sort_by(|a, b| {
        let at = a.get("total").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let bt = b.get("total").and_then(|v| v.as_f64()).unwrap_or(0.0);
        bt.partial_cmp(&at).unwrap_or(std::cmp::Ordering::Equal)
    });

    json!({
        "insights": {
            "from": bounds.from.to_rfc3339(),
            "to": bounds.to.to_rfc3339(),
            "workflowId": workflow_id,
            "total": { "value": cur.total, "previous": prev.total, "unit": "count" },
            "success": {
                "value": (cur.total - cur.failed).max(0.0),
                "previous": (prev.total - prev.failed).max(0.0),
                "unit": "count"
            },
            "failed": { "value": cur.failed, "previous": prev.failed, "unit": "count" },
            "failureRate": {
                "value": cur.failure_rate,
                "previous": prev.failure_rate,
                "unit": "ratio"
            },
            "timeSaved": {
                "value": cur.time_saved_minutes,
                "previous": prev.time_saved_minutes,
                "unit": "minute"
            },
            "averageRunTime": {
                "value": cur.average_run_time_ms,
                "previous": prev.average_run_time_ms,
                "unit": "millisecond"
            },
            "p50": { "value": p50, "previous": prev_p50, "unit": "millisecond" },
            "p95": { "value": p95, "unit": "millisecond" },
            "p99": { "value": p99, "unit": "millisecond" },
            "byTrigger": triggers
                .into_iter()
                .map(|(k, v)| json!({ "trigger": k, "total": v }))
                .collect::<Vec<_>>(),
            "byWorkflow": by_workflow,
            "byTime": build_by_time(&current, bounds.from, bounds.to),
            "executions": execution_rows(&current),
        }
    })
}
