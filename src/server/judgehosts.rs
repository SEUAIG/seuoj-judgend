//! Judge hosts: each host judges one submission at a time and is pinned to
//! its own CPU core. Hosts can be added, removed, paused and resumed while
//! the server runs; a host that is paused or removed finishes its current
//! submission first. Submissions wait in FIFO order for an idle host.

use crate::config::AijConfig;
use crate::error::{AijError, Result};
use crate::judger::runtime::available_cores;
use crate::server::AppJson;
use crate::server::timing_probe::{PROBE_INTERVAL_SECS, ProbeSample, probe_state};
use axum::Json;
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::{HashSet, VecDeque};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::oneshot;
use tracing::info;

/// Most hosts accepted by `PUT /judge/hosts`.
pub(crate) const MAX_HOSTS: usize = 64;
/// Largest host id accepted by `PUT /judge/hosts`.
pub(crate) const MAX_HOST_ID: u32 = 999;
/// Load windows in seconds: 5 minutes, 15 minutes, 1 hour, 5 hours.
const LOAD_WINDOWS_SECS: [i64; 4] = [300, 900, 3600, 18000];
/// Busy intervals kept per host for the load figures.
const BUSY_KEEP: usize = 4096;

pub(crate) fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

pub(crate) fn host_name(id: u32) -> String {
    format!("judge-{}", id)
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CurrentJudging {
    submission_id: String,
    started_at: i64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LastJudging {
    submission_id: String,
    started_at: i64,
    finished_at: i64,
}

struct Host {
    id: u32,
    core: Option<usize>,
    enabled: bool,
    /// Removed from the configuration; dropped once the current judging ends.
    removing: bool,
    current: Option<CurrentJudging>,
    last: Option<LastJudging>,
    judged: u64,
    /// Finished judgings as (start, end) in Unix milliseconds, oldest first.
    busy: VecDeque<(i64, i64)>,
}

impl Host {
    fn new(id: u32, core: Option<usize>, enabled: bool) -> Self {
        Self {
            id,
            core,
            enabled,
            removing: false,
            current: None,
            last: None,
            judged: 0,
            busy: VecDeque::new(),
        }
    }

    fn accepts_work(&self) -> bool {
        self.enabled && !self.removing && self.current.is_none()
    }

    /// Busy fraction of the last `window_secs` seconds.
    fn load(&self, now: i64, window_secs: i64) -> f64 {
        let window = window_secs * 1000;
        let start = now - window;
        let mut busy: i64 = self
            .busy
            .iter()
            .map(|&(s, e)| (e.min(now) - s.max(start)).max(0))
            .sum();
        if let Some(cur) = &self.current {
            busy += (now - cur.started_at.max(start)).max(0);
        }
        (busy as f64 / window as f64).clamp(0.0, 1.0)
    }
}

/// What a waiting submission receives when a host frees up.
#[derive(Debug, Clone, Copy)]
struct Assignment {
    id: u32,
    core: Option<usize>,
    started_at: i64,
}

struct Waiter {
    submission_id: String,
    tx: oneshot::Sender<Assignment>,
}

struct PoolState {
    hosts: Vec<Host>,
    waiters: VecDeque<Waiter>,
}

impl PoolState {
    /// Hand idle hosts to waiting submissions, lowest host id first.
    fn dispatch(&mut self) {
        while !self.waiters.is_empty() {
            let Some(host) = self.hosts.iter_mut().find(|h| h.accepts_work()) else {
                return;
            };
            let Some(waiter) = self.waiters.pop_front() else {
                return;
            };
            let started_at = now_millis();
            host.current = Some(CurrentJudging {
                submission_id: waiter.submission_id,
                started_at,
            });
            let assignment = Assignment {
                id: host.id,
                core: host.core,
                started_at,
            };
            if waiter.tx.send(assignment).is_err() {
                // The waiting task went away; the host stays free.
                host.current = None;
            }
        }
    }
}

pub(crate) struct HostPool {
    state: Mutex<PoolState>,
    cores: Vec<usize>,
}

/// One host reserved for one submission; returned to the pool on drop.
pub(crate) struct HostLease<'a> {
    pool: &'a HostPool,
    id: u32,
    core: Option<usize>,
    submission_id: String,
    started_at: i64,
}

impl HostLease<'_> {
    pub(crate) fn name(&self) -> String {
        host_name(self.id)
    }

    pub(crate) fn core(&self) -> Option<usize> {
        self.core
    }

    pub(crate) fn started_at(&self) -> i64 {
        self.started_at
    }
}

impl Drop for HostLease<'_> {
    fn drop(&mut self) {
        self.pool
            .release(self.id, &self.submission_id, self.started_at);
    }
}

/// One host as reported by `GET /judge/hosts`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HostView {
    id: u32,
    name: String,
    core: Option<usize>,
    enabled: bool,
    removing: bool,
    current: Option<CurrentJudging>,
    last: Option<LastJudging>,
    judged: u64,
    /// Busy fraction over 5 min / 15 min / 1 h / 5 h.
    load: [f64; 4],
}

impl HostPool {
    pub(crate) fn new(count: usize, cores: Vec<usize>) -> Self {
        let mut pool = Self {
            state: Mutex::new(PoolState {
                hosts: Vec::new(),
                waiters: VecDeque::new(),
            }),
            cores,
        };
        let hosts = (0..count.clamp(1, MAX_HOSTS) as u32)
            .map(|id| Host::new(id, pool.core_for(id), true))
            .collect();
        pool.state
            .get_mut()
            .unwrap_or_else(|e| e.into_inner())
            .hosts = hosts;
        pool
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, PoolState> {
        // The critical sections never panic; recover the data if one did.
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Host `id` runs on the `id`-th core counted from the highest one, so
    /// hosts stay away from core 0 where most other work lands.
    fn core_for(&self, id: u32) -> Option<usize> {
        if self.cores.is_empty() {
            return None;
        }
        let n = self.cores.len();
        Some(self.cores[n - 1 - (id as usize % n)])
    }

    /// Wait for an idle host. Submissions are served in arrival order.
    pub(crate) async fn acquire(&self, submission_id: &str) -> Option<HostLease<'_>> {
        let rx = {
            let mut state = self.lock();
            if state.waiters.is_empty()
                && let Some(host) = state.hosts.iter_mut().find(|h| h.accepts_work())
            {
                let started_at = now_millis();
                host.current = Some(CurrentJudging {
                    submission_id: submission_id.to_string(),
                    started_at,
                });
                return Some(HostLease {
                    pool: self,
                    id: host.id,
                    core: host.core,
                    submission_id: submission_id.to_string(),
                    started_at,
                });
            }
            let (tx, rx) = oneshot::channel();
            state.waiters.push_back(Waiter {
                submission_id: submission_id.to_string(),
                tx,
            });
            rx
        };
        let assignment = rx.await.ok()?;
        Some(HostLease {
            pool: self,
            id: assignment.id,
            core: assignment.core,
            submission_id: submission_id.to_string(),
            started_at: assignment.started_at,
        })
    }

    fn release(&self, id: u32, submission_id: &str, started_at: i64) {
        let finished_at = now_millis();
        let mut state = self.lock();
        if let Some(pos) = state.hosts.iter().position(|h| h.id == id) {
            let host = &mut state.hosts[pos];
            host.current = None;
            host.judged += 1;
            host.last = Some(LastJudging {
                submission_id: submission_id.to_string(),
                started_at,
                finished_at,
            });
            host.busy.push_back((started_at, finished_at));
            let oldest = finished_at - LOAD_WINDOWS_SECS[3] * 1000;
            while host
                .busy
                .front()
                .is_some_and(|&(_, e)| e < oldest || host.busy.len() > BUSY_KEEP)
            {
                host.busy.pop_front();
            }
            if host.removing {
                state.hosts.remove(pos);
            }
        }
        state.dispatch();
    }

    /// Apply a full host list: add missing hosts, update `enabled`, remove
    /// hosts not listed (busy ones after their current judging).
    pub(crate) fn configure(&self, wanted: &[(u32, bool)]) {
        let mut state = self.lock();
        for &(id, enabled) in wanted {
            match state.hosts.iter_mut().find(|h| h.id == id) {
                Some(host) => {
                    host.enabled = enabled;
                    host.removing = false;
                }
                None => {
                    let core = self.core_for(id);
                    state.hosts.push(Host::new(id, core, enabled));
                }
            }
        }
        let keep: HashSet<u32> = wanted.iter().map(|&(id, _)| id).collect();
        state.hosts.retain_mut(|h| {
            if keep.contains(&h.id) {
                return true;
            }
            h.enabled = false;
            h.removing = true;
            h.current.is_some()
        });
        state.hosts.sort_by_key(|h| h.id);
        state.dispatch();
    }

    pub(crate) fn running(&self) -> usize {
        self.lock()
            .hosts
            .iter()
            .filter(|h| h.current.is_some())
            .count()
    }

    pub(crate) fn waiting(&self) -> usize {
        self.lock()
            .waiters
            .iter()
            .filter(|w| !w.tx.is_closed())
            .count()
    }

    pub(crate) fn hosts(&self) -> Vec<HostView> {
        let now = now_millis();
        self.lock()
            .hosts
            .iter()
            .map(|h| HostView {
                id: h.id,
                name: host_name(h.id),
                core: h.core,
                enabled: h.enabled,
                removing: h.removing,
                current: h.current.clone(),
                last: h.last.clone(),
                judged: h.judged,
                load: LOAD_WINDOWS_SECS.map(|w| h.load(now, w)),
            })
            .collect()
    }
}

static POOL: OnceLock<HostPool> = OnceLock::new();

/// The process-wide pool: `max_concurrent_requests` hosts at start.
pub(crate) fn judge_hosts() -> &'static HostPool {
    POOL.get_or_init(|| HostPool::new(AijConfig::get().max_concurrent_requests, available_cores()))
}

// ---------------------------------------------------------------- HTTP

fn status_json(include_samples: bool) -> serde_json::Value {
    let pool = judge_hosts();
    let hosts = pool.hosts();
    let waiting = pool.waiting();
    let probe = probe_state();
    let samples: Vec<&ProbeSample> = if include_samples {
        probe.samples.iter().collect()
    } else {
        Vec::new()
    };
    json!({
        "code": 0,
        "message": "Success",
        "data": {
            "hosts": hosts,
            "waiting": waiting,
            "maxHosts": MAX_HOSTS,
            "defaultHosts": AijConfig::get().max_concurrent_requests.clamp(1, MAX_HOSTS),
            "probe": {
                "intervalSeconds": PROBE_INTERVAL_SECS,
                "error": probe.error,
                "samples": samples,
            },
        },
    })
}

/// `GET /judge/hosts`: hosts, queue and timing probe samples.
pub(crate) async fn get_hosts() -> Result<Json<serde_json::Value>> {
    Ok(Json(status_json(true)))
}

#[derive(Deserialize)]
pub(crate) struct HostSetting {
    id: u32,
    enabled: bool,
}

#[derive(Deserialize)]
pub(crate) struct SetHostsRequest {
    hosts: Vec<HostSetting>,
}

fn invalid(message: String) -> AijError {
    AijError::Request(
        StatusCode::BAD_REQUEST,
        "INVALID_HOSTS".to_string(),
        message,
    )
}

/// `PUT /judge/hosts`: replace the host list without restarting.
pub(crate) async fn put_hosts(
    AppJson(payload): AppJson<SetHostsRequest>,
) -> Result<Json<serde_json::Value>> {
    if payload.hosts.is_empty() || payload.hosts.len() > MAX_HOSTS {
        return Err(invalid(format!("need between 1 and {} hosts", MAX_HOSTS)));
    }
    let mut seen = HashSet::new();
    for host in &payload.hosts {
        if host.id > MAX_HOST_ID || !seen.insert(host.id) {
            return Err(invalid(format!("bad or duplicate host id {}", host.id)));
        }
    }
    let wanted: Vec<(u32, bool)> = payload.hosts.iter().map(|h| (h.id, h.enabled)).collect();
    judge_hosts().configure(&wanted);
    info!(
        "Judge hosts configured: {}",
        wanted
            .iter()
            .map(|(id, on)| format!("{}{}", host_name(*id), if *on { "" } else { "(paused)" }))
            .collect::<Vec<_>>()
            .join(", ")
    );
    Ok(Json(status_json(false)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::time::Duration;

    fn busy_ids(p: &HostPool) -> Vec<u32> {
        p.hosts()
            .iter()
            .filter(|h| h.current.is_some())
            .map(|h| h.id)
            .collect()
    }

    fn ids(p: &HostPool) -> Vec<u32> {
        p.hosts().iter().map(|h| h.id).collect()
    }

    #[tokio::test]
    async fn hosts_get_distinct_cores_from_the_top() {
        let p = HostPool::new(3, vec![0, 1, 2, 3]);
        let cores: Vec<_> = p.hosts().iter().map(|h| h.core).collect();
        assert_eq!(cores, vec![Some(3), Some(2), Some(1)]);
        let a = p.acquire("a").await.unwrap_or_else(|| unreachable!());
        assert_eq!((a.name(), a.core()), ("judge-0".to_string(), Some(3)));
        assert_eq!(HostPool::new(2, vec![]).hosts()[0].core, None);
    }

    #[tokio::test]
    async fn waiters_are_served_in_order_when_a_host_frees() {
        let p = Arc::new(HostPool::new(1, vec![]));
        let first = p.acquire("a").await.unwrap_or_else(|| unreachable!());
        let mut order = Vec::new();
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        for sid in ["b", "c"] {
            let p = p.clone();
            let tx = tx.clone();
            tokio::spawn(async move {
                let lease = p.acquire(sid).await;
                let _ = tx.send(sid);
                tokio::time::sleep(Duration::from_millis(20)).await;
                drop(lease);
            });
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert_eq!(p.waiting(), 2);
        drop(first);
        order.push(rx.recv().await.unwrap_or_default());
        order.push(rx.recv().await.unwrap_or_default());
        assert_eq!(order, vec!["b", "c"]);
        tokio::time::sleep(Duration::from_millis(40)).await;
        assert_eq!((p.running(), p.waiting()), (0, 0));
        assert_eq!(p.hosts()[0].judged, 3);
        assert_eq!(
            p.hosts()[0].last.as_ref().map(|l| l.submission_id.clone()),
            Some("c".to_string())
        );
    }

    #[tokio::test]
    async fn pausing_a_busy_host_lets_it_finish_and_then_stops_it() {
        let p = Arc::new(HostPool::new(2, vec![]));
        let a = p.acquire("a").await.unwrap_or_else(|| unreachable!());
        let b = p.acquire("b").await.unwrap_or_else(|| unreachable!());
        p.configure(&[(0, false), (1, true)]);
        assert_eq!(busy_ids(&p), vec![0, 1]);
        let p2 = p.clone();
        let waiter = tokio::spawn(async move {
            let lease = p2.acquire("c").await;
            lease.map(|l| l.name())
        });
        tokio::time::sleep(Duration::from_millis(10)).await;
        drop(a);
        // host 0 is paused: it does not take "c"
        tokio::time::sleep(Duration::from_millis(10)).await;
        assert_eq!((busy_ids(&p), p.waiting()), (vec![1], 1));
        drop(b);
        assert_eq!(waiter.await.ok().flatten(), Some("judge-1".to_string()));
        // resuming hands queued work to the host again
        p.configure(&[(0, true), (1, false)]);
        let c = p.acquire("d").await.unwrap_or_else(|| unreachable!());
        assert_eq!(c.name(), "judge-0");
    }

    #[tokio::test]
    async fn removed_hosts_leave_after_their_current_judging() {
        let p = HostPool::new(2, vec![]);
        let a = p.acquire("a").await.unwrap_or_else(|| unreachable!());
        p.configure(&[(1, true), (5, true)]);
        // idle host would be gone at once; busy host 0 stays until done
        assert_eq!(ids(&p), vec![0, 1, 5]);
        assert!(p.hosts()[0].removing && !p.hosts()[0].enabled);
        drop(a);
        assert_eq!(ids(&p), vec![1, 5]);
        p.configure(&[(5, true)]);
        assert_eq!(ids(&p), vec![5]);
    }

    #[tokio::test]
    async fn adding_a_host_admits_a_waiter_at_once() {
        let p = Arc::new(HostPool::new(1, vec![]));
        let _a = p.acquire("a").await.unwrap_or_else(|| unreachable!());
        let p2 = p.clone();
        let waiter = tokio::spawn(async move { p2.acquire("b").await.map(|l| l.name()) });
        tokio::time::sleep(Duration::from_millis(10)).await;
        p.configure(&[(0, true), (1, true)]);
        assert_eq!(waiter.await.ok().flatten(), Some("judge-1".to_string()));
    }

    #[test]
    fn load_counts_overlap_with_each_window() {
        let mut h = Host::new(0, None, true);
        let now = 10_000_000;
        // busy for the last 150 s of the 5-minute window
        h.busy.push_back((now - 150_000, now));
        let load = LOAD_WINDOWS_SECS.map(|w| h.load(now, w));
        assert!((load[0] - 0.5).abs() < 1e-9);
        assert!((load[1] - 150.0 / 900.0).abs() < 1e-9);
        h.current = Some(CurrentJudging {
            submission_id: "x".into(),
            started_at: now - 150_000,
        });
        assert!((h.load(now, 300) - 1.0).abs() < 1e-9);
    }
}
