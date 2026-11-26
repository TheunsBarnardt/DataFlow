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

fn build_by_time(list: &[&RunDoc], from: DateTime<Utc>, to: DateTime<Utc>) -> Vec<Value> {
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
        .map(|(date, day_runs)| {
            let s = summarize(&day_runs);
            json!({
                "date": date,
                "total": s.total,
                "failed": s.failed,
                "failureRate": s.failure_rate,
                "timeSavedMinutes": s.time_saved_minutes,
                "averageRunTimeMs": s.average_run_time_ms,
            })
        })
        .collect()
}

/// Build the TypeScript-compatible `{ insights: InsightsSummary }` payload.
pub fn build_insights(runs: &[RunDoc], days: i64) -> Value {
    let bounds = period_bounds(days);
    let current: Vec<&RunDoc> = runs
        .iter()
        .filter(|r| in_range(r, bounds.from, bounds.to, true))
        .collect();
    let previous: Vec<&RunDoc> = runs
        .iter()
        .filter(|r| in_range(r, bounds.prev_from, bounds.prev_to, false))
        .collect();

    let cur = summarize(&current);
    let prev = summarize(&previous);

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
            let executions: Vec<Value> = list
                .iter()
                .filter_map(|r| {
                    let duration_ms = run_duration_ms(r)?;
                    Some(json!({
                        "at": r.started_at.to_rfc3339(),
                        "date": day_key(r.started_at),
                        "durationMs": duration_ms,
                        "status": format!("{:?}", r.status).to_lowercase(),
                    }))
                })
                .collect();
            json!({
                "workflowId": workflow_id,
                "workflowName": workflow_name,
                "total": s.total,
                "failed": s.failed,
                "failureRate": s.failure_rate,
                "timeSavedMinutes": s.time_saved_minutes,
                "averageRunTimeMs": s.average_run_time_ms,
                "byTime": build_by_time(&list, bounds.from, bounds.to),
                "executions": executions,
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
            "total": { "value": cur.total, "previous": prev.total, "unit": "count" },
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
            "byWorkflow": by_workflow,
            "byTime": build_by_time(&current, bounds.from, bounds.to),
        }
    })
}
