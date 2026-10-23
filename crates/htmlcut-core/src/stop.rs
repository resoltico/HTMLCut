// SPDX-License-Identifier: MPL-2.0
//! Cooperative cancellation and a single absolute deadline.
use crate::{Error, Limit};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
/// Cloneable stop policy, reusable across phases and threads.
#[derive(Clone)]
pub struct Stop {
    deadline: Option<Instant>,
    limit: Option<Duration>,
    cancel: Option<Arc<AtomicBool>>,
    #[cfg(test)]
    checkpoint: Option<Arc<dyn Fn() -> Result<(), Error> + Send + Sync>>,
}
impl Stop {
    /// No cancellation or deadline.
    pub fn none() -> Self {
        Self {
            deadline: None,
            limit: None,
            cancel: None,
            #[cfg(test)]
            checkpoint: None,
        }
    }
    /// One absolute deadline; an unrepresentable duration is an invalid request.
    pub fn after(limit: Duration) -> Result<Self, Error> {
        let deadline = Instant::now()
            .checked_add(limit)
            .ok_or_else(|| Error::InvalidRequest {
                reason: "The deadline cannot be represented.".into(),
            })?;
        Ok(Self {
            deadline: Some(deadline),
            limit: Some(limit),
            ..Self::none()
        })
    }
    /// Also stop when this flag is set.
    pub fn with_cancel(mut self, flag: Arc<AtomicBool>) -> Self {
        self.cancel = Some(flag);
        self
    }
    /// Check cancellation and the original deadline immediately.
    pub fn check(&self) -> Result<(), Error> {
        #[cfg(test)]
        if let Some(checkpoint) = &self.checkpoint {
            checkpoint()?;
        }
        if self
            .cancel
            .as_ref()
            .is_some_and(|f| f.load(Ordering::Relaxed))
        {
            return Err(Error::Cancelled);
        }
        if self.deadline.is_some_and(|d| Instant::now() >= d) {
            return Err(Error::LimitExceeded {
                limit: Limit::Time,
                bound: serde_json::Number::from_f64(self.limit.unwrap().as_secs_f64()).unwrap(),
            });
        }
        Ok(())
    }
    #[cfg(test)]
    pub(crate) fn testing(
        checkpoint: impl Fn() -> Result<(), Error> + Send + Sync + 'static,
    ) -> Self {
        Self {
            checkpoint: Some(Arc::new(checkpoint)),
            ..Self::none()
        }
    }
}
/// Work counter private to one invocation. Clones share only that invocation.
#[derive(Clone)]
pub(crate) struct Clock {
    stop: Stop,
    iterations: std::rc::Rc<std::cell::Cell<usize>>,
}
impl std::ops::Deref for Clock {
    type Target = Stop;
    fn deref(&self) -> &Stop {
        &self.stop
    }
}
impl Clock {
    pub(crate) fn for_stop(stop: &Stop) -> Self {
        Self {
            stop: stop.clone(),
            iterations: Default::default(),
        }
    }
    pub(crate) fn tick(&self) -> Result<(), Error> {
        let n = self.iterations.get().wrapping_add(1);
        self.iterations.set(n);
        if n & 1023 == 0 {
            self.check()?;
        }
        Ok(())
    }
    #[cfg(test)]
    pub(crate) fn new(limit: Option<Duration>) -> Result<Self, Error> {
        Ok(Self::for_stop(
            &limit
                .map(Stop::after)
                .transpose()?
                .unwrap_or_else(Stop::none),
        ))
    }
    #[cfg(test)]
    pub(crate) fn testing(
        checkpoint: impl Fn() -> Result<(), Error> + Send + Sync + 'static,
    ) -> Self {
        Self::for_stop(&Stop::testing(checkpoint))
    }
    #[cfg(test)]
    pub(crate) fn work_ticks(&self) -> usize {
        self.iterations.get()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn absolute_deadline_cancellation_and_auto_traits() {
        fn traits<T: Clone + Send + Sync>() {}
        traits::<Stop>();
        assert!(Stop::after(Duration::MAX).is_err());
        assert!(matches!(
            Stop::after(Duration::ZERO).unwrap().check(),
            Err(Error::LimitExceeded {
                limit: Limit::Time,
                ..
            })
        ));
        let flag = Arc::new(AtomicBool::new(false));
        let stop = Stop::none().with_cancel(flag.clone());
        stop.check().unwrap();
        flag.store(true, Ordering::Relaxed);
        assert!(matches!(stop.check(), Err(Error::Cancelled)));
    }
}
