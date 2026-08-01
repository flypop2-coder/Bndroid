//! Allocation-free policy for M62's terminal-proof fallback injection.
//!
//! The production Offline transition already requires an independently
//! verified terminal IRQ/DMA boundary. M62 deliberately makes the first safe
//! proof unavailable exactly once so the existing cooperative quarantine path
//! must establish a fresh proof. This policy owns only that non-renewable test
//! gate; it does not own the block device, broker, scheduler, or EL0 authority.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Phase {
    Idle = 0,
    Armed = 1,
    Consumed = 2,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Decision {
    Unsafe,
    RunFallback,
    Publish,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GateError {
    MissingArm,
    AlreadyArmed,
    AlreadyConsumed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Snapshot {
    pub phase: Phase,
    pub arms: u64,
    pub proof_deferrals: u64,
    pub unsafe_observations: u64,
    pub publications: u64,
    pub invariant_errors: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TerminalProofGate {
    phase: Phase,
    arms: u64,
    proof_deferrals: u64,
    unsafe_observations: u64,
    publications: u64,
    invariant_errors: u64,
}

impl TerminalProofGate {
    pub const fn new() -> Self {
        Self {
            phase: Phase::Idle,
            arms: 0,
            proof_deferrals: 0,
            unsafe_observations: 0,
            publications: 0,
            invariant_errors: 0,
        }
    }

    /// Arms the one-shot gate. There is deliberately no reset or renew API.
    pub fn arm(&mut self) -> Result<(), GateError> {
        match self.phase {
            Phase::Idle => {
                self.phase = Phase::Armed;
                self.arms = self.arms.saturating_add(1);
                Ok(())
            }
            Phase::Armed => {
                self.invariant_errors = self.invariant_errors.saturating_add(1);
                Err(GateError::AlreadyArmed)
            }
            Phase::Consumed => {
                self.invariant_errors = self.invariant_errors.saturating_add(1);
                Err(GateError::AlreadyConsumed)
            }
        }
    }

    /// Evaluates one complete terminal proof.
    ///
    /// Unsafe observations never consume the fault. The first safe proof after
    /// arming is deferred, and every later safe proof is publishable.
    pub fn evaluate(&mut self, proof_safe: bool) -> Result<Decision, GateError> {
        if !proof_safe {
            self.unsafe_observations = self.unsafe_observations.saturating_add(1);
            return Ok(Decision::Unsafe);
        }
        match self.phase {
            Phase::Idle => {
                self.invariant_errors = self.invariant_errors.saturating_add(1);
                Err(GateError::MissingArm)
            }
            Phase::Armed => {
                self.phase = Phase::Consumed;
                self.proof_deferrals = self.proof_deferrals.saturating_add(1);
                Ok(Decision::RunFallback)
            }
            Phase::Consumed => {
                self.publications = self.publications.saturating_add(1);
                Ok(Decision::Publish)
            }
        }
    }

    pub const fn snapshot(&self) -> Snapshot {
        Snapshot {
            phase: self.phase,
            arms: self.arms,
            proof_deferrals: self.proof_deferrals,
            unsafe_observations: self.unsafe_observations,
            publications: self.publications,
            invariant_errors: self.invariant_errors,
        }
    }
}

impl Default for TerminalProofGate {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{Decision, GateError, Phase, TerminalProofGate};

    #[test]
    fn new_gate_is_idle_and_empty() {
        let snapshot = TerminalProofGate::new().snapshot();
        assert_eq!(snapshot.phase, Phase::Idle);
        assert_eq!(snapshot.arms, 0);
        assert_eq!(snapshot.proof_deferrals, 0);
        assert_eq!(snapshot.unsafe_observations, 0);
        assert_eq!(snapshot.publications, 0);
        assert_eq!(snapshot.invariant_errors, 0);
    }

    #[test]
    fn arm_is_exactly_one_way() {
        let mut gate = TerminalProofGate::new();
        assert_eq!(gate.arm(), Ok(()));
        assert_eq!(gate.snapshot().phase, Phase::Armed);
        assert_eq!(gate.snapshot().arms, 1);
    }

    #[test]
    fn first_safe_proof_is_deferred_once() {
        let mut gate = TerminalProofGate::new();
        gate.arm().unwrap();
        assert_eq!(gate.evaluate(true), Ok(Decision::RunFallback));
        let snapshot = gate.snapshot();
        assert_eq!(snapshot.phase, Phase::Consumed);
        assert_eq!(snapshot.proof_deferrals, 1);
        assert_eq!(snapshot.publications, 0);
    }

    #[test]
    fn unsafe_proof_does_not_consume_the_gate() {
        let mut gate = TerminalProofGate::new();
        gate.arm().unwrap();
        assert_eq!(gate.evaluate(false), Ok(Decision::Unsafe));
        assert_eq!(gate.snapshot().phase, Phase::Armed);
        assert_eq!(gate.evaluate(true), Ok(Decision::RunFallback));
    }

    #[test]
    fn fresh_safe_proof_after_fallback_is_publishable() {
        let mut gate = TerminalProofGate::new();
        gate.arm().unwrap();
        assert_eq!(gate.evaluate(true), Ok(Decision::RunFallback));
        assert_eq!(gate.evaluate(true), Ok(Decision::Publish));
        let snapshot = gate.snapshot();
        assert_eq!(snapshot.proof_deferrals, 1);
        assert_eq!(snapshot.publications, 1);
    }

    #[test]
    fn safe_proof_without_kernel_arm_fails_closed() {
        let mut gate = TerminalProofGate::new();
        assert_eq!(gate.evaluate(true), Err(GateError::MissingArm));
        assert_eq!(gate.snapshot().phase, Phase::Idle);
        assert_eq!(gate.snapshot().invariant_errors, 1);
    }

    #[test]
    fn duplicate_arm_and_rearm_are_rejected() {
        let mut gate = TerminalProofGate::new();
        gate.arm().unwrap();
        assert_eq!(gate.arm(), Err(GateError::AlreadyArmed));
        assert_eq!(gate.evaluate(true), Ok(Decision::RunFallback));
        assert_eq!(gate.arm(), Err(GateError::AlreadyConsumed));
        assert_eq!(gate.snapshot().arms, 1);
        assert_eq!(gate.snapshot().invariant_errors, 2);
    }

    #[test]
    fn repeated_unsafe_observations_cannot_renew_or_consume() {
        let mut gate = TerminalProofGate::new();
        gate.arm().unwrap();
        for _ in 0..4 {
            assert_eq!(gate.evaluate(false), Ok(Decision::Unsafe));
        }
        let snapshot = gate.snapshot();
        assert_eq!(snapshot.phase, Phase::Armed);
        assert_eq!(snapshot.arms, 1);
        assert_eq!(snapshot.unsafe_observations, 4);
        assert_eq!(snapshot.proof_deferrals, 0);
    }
}
