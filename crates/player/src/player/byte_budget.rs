use std::sync::{Arc, Condvar, Mutex};

#[derive(Default)]
struct State {
    used: usize,
    closed: bool,
}

pub(super) struct ByteBudget {
    limit: usize,
    state: Mutex<State>,
    changed: Condvar,
}

pub(super) struct Reservation {
    budget: Arc<ByteBudget>,
    bytes: usize,
}

impl ByteBudget {
    pub(super) fn new(limit: usize) -> Arc<Self> {
        Arc::new(Self {
            limit,
            state: Mutex::default(),
            changed: Condvar::new(),
        })
    }

    pub(super) fn reserve(self: &Arc<Self>, bytes: usize) -> Result<Reservation, String> {
        if bytes > self.limit {
            return Err("resource exceeds decode byte budget".to_owned());
        }
        let mut state = self.state.lock().unwrap();
        while !state.closed && state.used + bytes > self.limit {
            state = self.changed.wait(state).unwrap();
        }
        if state.closed {
            return Err("resource worker stopped".to_owned());
        }
        state.used += bytes;
        Ok(Reservation {
            budget: self.clone(),
            bytes,
        })
    }

    pub(super) fn used(&self) -> usize {
        self.state.lock().unwrap().used
    }

    pub(super) fn close(&self) {
        self.state.lock().unwrap().closed = true;
        self.changed.notify_all();
    }
}

impl Drop for Reservation {
    fn drop(&mut self) {
        self.budget.state.lock().unwrap().used -= self.bytes;
        self.budget.changed.notify_all();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reservations_bound_queued_bytes_and_shutdown_unblocks_workers() {
        let budget = ByteBudget::new(100);
        assert!(budget.reserve(101).is_err());
        let first = budget.reserve(80).unwrap();
        let next = budget.clone();
        let worker = std::thread::spawn(move || next.reserve(30));
        assert_eq!(budget.used(), 80);
        drop(first);
        let second = worker.join().unwrap().unwrap();
        assert_eq!(budget.used(), 30);
        let next = budget.clone();
        let worker = std::thread::spawn(move || next.reserve(90));
        budget.close();
        assert!(worker.join().unwrap().is_err());
        drop(second);
        assert_eq!(budget.used(), 0);
    }
}
