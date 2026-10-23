// SPDX-License-Identifier: MPL-2.0
//! Stop requests, commit arbitration and host completion.
use crate::Error;
use crate::local_io::{StageOwnership, cleanup_owned_stages};
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Stop {
    Cancelled,
    Deadline,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Phase {
    Active,
    CommitOwned,
    Completed,
    SupervisorOwned(Stop),
}
struct State {
    phase: Phase,
    stop: Option<Stop>,
}
struct Shared {
    state: Mutex<State>,
    deadline: Option<Instant>,
    duration: Option<Duration>,
    staging: Mutex<Vec<Arc<StageOwnership>>>,
    #[cfg(test)]
    cleanup_gate: Mutex<Option<(Arc<std::sync::Barrier>, Arc<std::sync::Barrier>)>>,
}
/// Result of a request to stop a job.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CancelOutcome {
    Accepted,
    CommitInProgress,
    AlreadyFinished,
    EnforcementInProgress,
}
/// Completion returned to the owning host, after owned cleanup.
pub enum Completion<T> {
    Finished(Result<T, Error>),
    Supervised(Error),
}
/// Result of an executable supervisor's deadline check.
pub enum Supervision {
    /// An existing supervisor owns enforcement.
    EnforcementInProgress,
    Pending,
    CommitInProgress,
    Finished,
    Claimed(Error),
}
/// An observer/cancellation handle. Dropping any clone has no effect.
#[derive(Clone)]
pub struct JobControl(Arc<Shared>);
/// The unique owner of one pipeline and at most one delivery.
pub struct Job {
    control: JobControl,
}
impl Shared {
    fn cleanup(&self, budget: Duration) {
        let records = self.staging.lock().unwrap().clone();
        cleanup_owned_stages(&records, Instant::now(), budget);
    }
    fn stop(&self, state: &mut State) -> Option<Stop> {
        if state.stop.is_none() && self.deadline.is_some_and(|t| Instant::now() >= t) {
            state.stop = Some(Stop::Deadline);
        }
        state.stop
    }
    fn error(&self, stop: Stop) -> Error {
        match stop {
            Stop::Cancelled => Error::Cancelled,
            Stop::Deadline => Error::LimitExceeded {
                limit: htmlcut_core::Limit::Time,
                bound: serde_json::Number::from_f64(self.duration.unwrap().as_secs_f64()).unwrap(),
            },
        }
    }
    fn check(&self) -> Result<(), Error> {
        let disposition = {
            let mut state = self.state.lock().unwrap();
            match state.phase {
                Phase::Active => self.stop(&mut state).map_or(Ok(()), |stop| Err(Some(stop))),
                Phase::SupervisorOwned(stop) => Err(Some(stop)),
                Phase::CommitOwned | Phase::Completed => Err(None),
            }
        };
        match disposition {
            Ok(()) => Ok(()),
            Err(Some(stop)) => Err(self.error(stop)),
            Err(None) => Err(Error::InvalidRequest {
                reason: ("This job has already delivered or finished.").into(),
            }),
        }
    }
    fn supervised_error(&self, stop: Stop) -> Error {
        self.error(stop)
    }
}
impl JobControl {
    /// Requests cooperative cancellation, respecting an earlier stop or elapsed deadline.
    pub fn cancel(&self) -> CancelOutcome {
        let mut state = self.0.state.lock().unwrap();
        match state.phase {
            Phase::Active => {
                if self.0.stop(&mut state).is_none() {
                    state.stop = Some(Stop::Cancelled);
                }
                CancelOutcome::Accepted
            }
            Phase::CommitOwned => CancelOutcome::CommitInProgress,
            Phase::Completed => CancelOutcome::AlreadyFinished,
            Phase::SupervisorOwned(_) => CancelOutcome::EnforcementInProgress,
        }
    }
    /// Absolute deadline shared with the executable supervisor.
    pub fn deadline(&self) -> Option<Instant> {
        self.0.deadline
    }
    /// Claims hard enforcement only after the deadline and before commit/completion.
    /// Library callers need not use this method; it never terminates a process.
    pub fn supervise(&self) -> Supervision {
        let stop = {
            let mut state = self.0.state.lock().unwrap();
            match state.phase {
                Phase::Completed => return Supervision::Finished,
                Phase::CommitOwned => return Supervision::CommitInProgress,
                Phase::SupervisorOwned(_) => return Supervision::EnforcementInProgress,
                Phase::Active => {
                    if !self
                        .0
                        .deadline
                        .is_some_and(|deadline| Instant::now() >= deadline)
                    {
                        return Supervision::Pending;
                    }
                    let stop = self.0.stop(&mut state).unwrap();
                    state.phase = Phase::SupervisorOwned(stop);
                    stop
                }
            }
        };
        Supervision::Claimed(self.0.supervised_error(stop))
    }

    /// Best-effort cleanup for the owning executable supervisor. Other observers
    /// cannot remove an active job's stages. Hosts must bound blocking calls.
    pub fn cleanup(&self, budget: Duration) {
        if !matches!(
            self.0.state.lock().unwrap().phase,
            Phase::SupervisorOwned(_)
        ) {
            return;
        }
        self.0.cleanup(budget);
    }
}
impl Job {
    /// Creates a job without starting a worker or modifying process state.
    pub fn new(duration: Option<Duration>) -> Result<Self, Error> {
        let deadline = duration
            .map(|d| {
                Instant::now()
                    .checked_add(d)
                    .ok_or_else(|| Error::InvalidRequest {
                        reason: ("The deadline cannot be represented.").into(),
                    })
            })
            .transpose()?;
        Ok(Self {
            control: JobControl(Arc::new(Shared {
                state: Mutex::new(State {
                    phase: Phase::Active,
                    stop: None,
                }),
                deadline,
                duration,
                staging: Mutex::new(Vec::new()),
                #[cfg(test)]
                cleanup_gate: Mutex::new(None),
            })),
        })
    }
    /// Returns an inert observer of this job.
    pub fn control(&self) -> JobControl {
        self.control.clone()
    }
    pub(crate) fn context(&self) -> htmlcut_core::ExecutionContext {
        let control = self.control.clone();
        htmlcut_core::ExecutionContext::with_checkpoint(move || control.0.check())
    }
    pub(crate) fn check(&self) -> Result<(), Error> {
        self.control.0.check()
    }
    pub(crate) fn reserve(&self, record: Arc<StageOwnership>) {
        self.control.0.staging.lock().unwrap().push(record);
    }
    pub(crate) fn commit(&self) -> Result<(), Error> {
        let (phase, stop) = {
            let mut state = self.control.0.state.lock().unwrap();
            let phase = state.phase;
            let stop = if phase == Phase::Active {
                self.control.0.stop(&mut state)
            } else {
                None
            };
            if phase == Phase::Active && stop.is_none() {
                state.phase = Phase::CommitOwned;
            }
            (phase, stop)
        };
        match (phase, stop) {
            (Phase::Active, None) => Ok(()),
            (Phase::Active, Some(stop)) | (Phase::SupervisorOwned(stop), _) => {
                Err(self.control.0.error(stop))
            }
            (Phase::CommitOwned | Phase::Completed, _) => Err(Error::InvalidRequest {
                reason: ("A job may deliver only once.").into(),
            }),
        }
    }
    pub(crate) fn finish<T>(&self, result: Result<T, Error>) -> Completion<T> {
        let (phase, stop) = {
            let mut state = self.control.0.state.lock().unwrap();
            let phase = state.phase;
            let stop = if phase == Phase::Active {
                self.control.0.stop(&mut state)
            } else {
                None
            };
            if !matches!(phase, Phase::SupervisorOwned(_)) {
                state.phase = Phase::Completed;
            }
            (phase, stop)
        };
        match (phase, stop) {
            (Phase::Active, Some(stop)) => Completion::Finished(Err(self.control.0.error(stop))),
            (Phase::Active, None) | (Phase::CommitOwned, _) => Completion::Finished(result),
            (Phase::Completed, _) => Completion::Finished(Err(Error::InvalidRequest {
                reason: ("A job may finish only once.").into(),
            })),
            (Phase::SupervisorOwned(stop), _) => Completion::Supervised(self.control.0.error(stop)),
        }
    }
    pub(crate) fn perform<T>(&self, action: impl FnOnce() -> Result<T, Error>) -> Completion<T> {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.check()?;
            action()
        }))
        .unwrap_or_else(|_| {
            Err(Error::Internal {
                reason: ("An internal error interrupted the operation.").into(),
            })
        });
        self.control.0.cleanup(Duration::from_millis(50));
        #[cfg(test)]
        if let Some((entered, release)) = self.control.0.cleanup_gate.lock().unwrap().take() {
            entered.wait();
            release.wait();
        }
        self.control.0.staging.lock().unwrap().clear();
        self.finish(result)
    }
}
impl Drop for Job {
    fn drop(&mut self) {
        // This owner encloses cleanup. Observers have no Drop behavior.
        let phase = self.control.0.state.lock().unwrap().phase;
        if matches!(phase, Phase::Completed | Phase::CommitOwned) {
            return;
        }
        self.control.0.cleanup(Duration::from_millis(50));
        if phase == Phase::Active {
            let _ = self.finish::<()>(Err(Error::Internal {
                reason: ("The operation owner was dropped before completion.").into(),
            }));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ErrorCode;
    fn expired(job: &mut Job) {
        Arc::get_mut(&mut job.control.0).unwrap().deadline =
            Some(Instant::now() - Duration::from_secs(1));
    }
    fn code<T>(result: Completion<T>) -> ErrorCode {
        match result {
            Completion::Finished(Err(error)) | Completion::Supervised(error) => error.code(),
            Completion::Finished(Ok(_)) => panic!("expected refusal"),
        }
    }
    #[test]
    fn stop_precedence_is_based_on_absolute_time_and_prior_acceptance() {
        let mut early = Job::new(Some(Duration::from_secs(10))).unwrap();
        early.control.cancel();
        expired(&mut early);
        assert_eq!(code(early.finish::<()>(Ok(()))), ErrorCode::Cancelled);
        let mut late = Job::new(Some(Duration::from_secs(10))).unwrap();
        expired(&mut late);
        late.control.cancel();
        let Completion::Finished(Err(error)) = late.finish::<()>(Err(Error::Internal {
            reason: ("late failure").into(),
        })) else {
            panic!()
        };
        assert_eq!(
            serde_json::to_value(&error).unwrap()["limit"].as_str(),
            Some("time")
        );
        let job = Job::new(None).unwrap();
        assert_eq!(
            code(job.finish::<()>(Err(Error::InputUnreadable {
                file: "test".into(),
                detail: "ordinary failure".into()
            }))),
            ErrorCode::InputUnreadable
        );
    }
    #[test]
    fn commit_winner_uses_actual_outcome_and_rejects_second_delivery() {
        for success in [true, false] {
            let mut job = Job::new(Some(Duration::from_secs(10))).unwrap();
            job.commit().unwrap();
            expired(&mut job);
            assert_eq!(job.control.cancel(), CancelOutcome::CommitInProgress);
            assert!(matches!(
                job.control.supervise(),
                Supervision::CommitInProgress
            ));
            assert!(job.commit().is_err());
            let result = if success {
                Ok(())
            } else {
                Err(Error::OutputUnwritable {
                    file: "test".into(),
                    detail: "actual rename failure".into(),
                })
            };
            match job.finish(result) {
                Completion::Finished(result) => assert_eq!(result.is_ok(), success),
                Completion::Supervised(_) => panic!("commit lost to deadline"),
            }
            assert_eq!(code(job.finish::<()>(Ok(()))), ErrorCode::InvalidRequest);
        }
    }
    #[test]
    fn cancelling_and_claiming_commit_have_exactly_one_winner() {
        for _ in 0..64 {
            let job = Arc::new(Job::new(None).unwrap());
            let barrier = Arc::new(std::sync::Barrier::new(2));
            let other = job.clone();
            let gate = barrier.clone();
            let worker = std::thread::spawn(move || {
                gate.wait();
                other.control.cancel()
            });
            barrier.wait();
            let committed = job.commit().is_ok();
            let cancelled = worker.join().unwrap();
            assert_eq!(committed, cancelled == CancelOutcome::CommitInProgress);
            match job.finish(Ok(())) {
                Completion::Finished(result) => assert_eq!(result.is_ok(), committed),
                Completion::Supervised(_) => panic!(),
            }
        }
    }
    #[test]
    fn dropping_the_owner_finishes_uncommitted_jobs_and_preserves_commit_ownership() {
        for cancelled in [false, true] {
            let job = Job::new(None).unwrap();
            let control = job.control();
            if cancelled {
                assert_eq!(control.cancel(), CancelOutcome::Accepted);
            }
            drop(job);
            assert!(matches!(control.supervise(), Supervision::Finished));
            assert_eq!(control.cancel(), CancelOutcome::AlreadyFinished);
        }

        let job = Job::new(None).unwrap();
        let control = job.control();
        job.commit().unwrap();
        drop(job);
        assert!(matches!(control.supervise(), Supervision::CommitInProgress));
        assert_eq!(control.cancel(), CancelOutcome::CommitInProgress);
    }

    #[test]
    fn dropping_observers_is_inert_and_unwind_does_not_invent_cancellation() {
        let job = Job::new(None).unwrap();
        drop(job.control());
        job.check().unwrap();
        assert_eq!(
            code(job.perform::<()>(|| panic!("worker panic"))),
            ErrorCode::Internal
        );
        assert!(matches!(job.control.supervise(), Supervision::Finished));
    }
    #[test]
    fn cooperative_stop_then_held_cleanup_does_not_disarm_supervision() {
        let job = Job::new(Some(Duration::ZERO)).unwrap();
        let control = job.control();
        let entered = Arc::new(std::sync::Barrier::new(2));
        let release = Arc::new(std::sync::Barrier::new(2));
        *job.control.0.cleanup_gate.lock().unwrap() = Some((entered.clone(), release.clone()));
        let worker = std::thread::spawn(move || {
            job.perform::<()>(|| panic!("expired job must refuse before work"))
        });
        entered.wait();
        assert!(matches!(control.supervise(), Supervision::Claimed(_)));
        assert_eq!(control.cancel(), CancelOutcome::EnforcementInProgress);
        release.wait();
        assert!(matches!(worker.join().unwrap(), Completion::Supervised(_)));
    }
    #[test]
    fn finish_and_expiry_choose_exactly_one_owner_for_disconnected_delivery() {
        for cancellation in [false, true] {
            for supervisor_first in [false, true] {
                let mut job = Job::new(Some(Duration::from_secs(10))).unwrap();
                if cancellation {
                    job.control.cancel();
                }
                expired(&mut job);
                if !cancellation {
                    job.control.cancel();
                }
                let job = Arc::new(job);
                let control = job.control();
                let expected = if cancellation {
                    ErrorCode::Cancelled
                } else {
                    ErrorCode::LimitExceeded
                };
                let (release_sender, release_receiver) = std::sync::mpsc::sync_channel(1);
                if supervisor_first {
                    let (done_sender, done_receiver) = std::sync::mpsc::sync_channel(1);
                    let worker = job.clone();
                    let thread = std::thread::spawn(move || {
                        release_receiver
                            .recv_timeout(Duration::from_secs(2))
                            .unwrap();
                        done_sender
                            .send(worker.finish(Ok(crate::Delivery::Disconnected)))
                            .ok();
                    });
                    let Supervision::Claimed(error) = control.supervise() else {
                        panic!("supervisor did not own expiry")
                    };
                    assert_eq!(error.code(), expected);
                    release_sender.send(()).unwrap();
                    let completion = done_receiver.recv_timeout(Duration::from_secs(2)).unwrap();
                    assert!(
                        matches!(completion,Completion::Supervised(error) if error.code()==expected)
                    );
                    thread.join().unwrap();
                    assert!(matches!(
                        control.supervise(),
                        Supervision::EnforcementInProgress
                    ));
                } else {
                    let (done_sender, done_receiver) = std::sync::mpsc::sync_channel(1);
                    let thread = std::thread::spawn(move || {
                        release_receiver
                            .recv_timeout(Duration::from_secs(2))
                            .unwrap();
                        done_sender.send(control.supervise()).ok();
                    });
                    assert_eq!(
                        code(job.finish(Ok(crate::Delivery::Disconnected))),
                        expected
                    );
                    release_sender.send(()).unwrap();
                    assert!(matches!(
                        done_receiver.recv_timeout(Duration::from_secs(2)).unwrap(),
                        Supervision::Finished
                    ));
                    thread.join().unwrap();
                }
            }
        }
    }
    #[test]
    fn ordinary_failure_completed_before_expiry_keeps_its_terminal_owner() {
        let mut job = Job::new(Some(Duration::from_secs(10))).unwrap();
        assert_eq!(
            code(job.finish::<()>(Err(Error::OutputUnwritable {
                file: "test".into(),
                detail: "native stream failed".into()
            }))),
            ErrorCode::OutputUnwritable
        );
        expired(&mut job);
        let control = job.control();
        assert!(matches!(control.supervise(), Supervision::Finished));
        assert_eq!(control.cancel(), CancelOutcome::AlreadyFinished);
    }
}
