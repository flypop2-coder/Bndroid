#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ThreadState {
    Inactive = 0,
    Runnable = 1,
    Running = 2,
    Sleeping = 3,
    Faulted = 4,
    Zombie = 5,
    Waiting = 6,
}

impl ThreadState {
    pub const fn raw(self) -> u8 {
        self as u8
    }
}

pub const fn valid_transition(from: ThreadState, to: ThreadState) -> bool {
    matches!(
        (from, to),
        (ThreadState::Inactive, ThreadState::Runnable)
            | (ThreadState::Inactive, ThreadState::Running)
            | (ThreadState::Runnable, ThreadState::Running)
            | (ThreadState::Running, ThreadState::Runnable)
            | (ThreadState::Running, ThreadState::Sleeping)
            | (ThreadState::Running, ThreadState::Waiting)
            | (ThreadState::Running, ThreadState::Faulted)
            | (ThreadState::Running, ThreadState::Zombie)
            | (ThreadState::Runnable, ThreadState::Zombie)
            | (ThreadState::Sleeping, ThreadState::Runnable)
            | (ThreadState::Waiting, ThreadState::Runnable)
            | (ThreadState::Waiting, ThreadState::Zombie)
            | (ThreadState::Faulted, ThreadState::Inactive)
            | (ThreadState::Zombie, ThreadState::Inactive)
    )
}

#[cfg(test)]
mod tests {
    use super::{ThreadState, valid_transition};

    #[test]
    fn accepts_scheduler_lifecycle_transitions() {
        assert!(valid_transition(
            ThreadState::Inactive,
            ThreadState::Runnable
        ));
        assert!(valid_transition(
            ThreadState::Runnable,
            ThreadState::Running
        ));
        assert!(valid_transition(
            ThreadState::Running,
            ThreadState::Sleeping
        ));
        assert!(valid_transition(
            ThreadState::Sleeping,
            ThreadState::Runnable
        ));
        assert!(valid_transition(ThreadState::Running, ThreadState::Waiting));
        assert!(valid_transition(
            ThreadState::Waiting,
            ThreadState::Runnable
        ));
        assert!(valid_transition(ThreadState::Running, ThreadState::Faulted));
        assert!(valid_transition(ThreadState::Running, ThreadState::Zombie));
        assert!(valid_transition(ThreadState::Runnable, ThreadState::Zombie));
        assert!(valid_transition(ThreadState::Waiting, ThreadState::Zombie));
        assert!(valid_transition(
            ThreadState::Faulted,
            ThreadState::Inactive
        ));
        assert!(valid_transition(ThreadState::Zombie, ThreadState::Inactive));
    }

    #[test]
    fn rejects_direct_or_duplicate_sleep_transitions() {
        assert!(!valid_transition(
            ThreadState::Runnable,
            ThreadState::Sleeping
        ));
        assert!(!valid_transition(
            ThreadState::Sleeping,
            ThreadState::Running
        ));
        assert!(!valid_transition(
            ThreadState::Sleeping,
            ThreadState::Sleeping
        ));
        assert!(!valid_transition(
            ThreadState::Runnable,
            ThreadState::Waiting
        ));
        assert!(!valid_transition(
            ThreadState::Sleeping,
            ThreadState::Zombie
        ));
        assert!(!valid_transition(
            ThreadState::Waiting,
            ThreadState::Running
        ));
        assert!(!valid_transition(
            ThreadState::Running,
            ThreadState::Running
        ));
    }
}
