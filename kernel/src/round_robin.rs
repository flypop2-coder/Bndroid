/// Finds the next runnable context after `current`, wrapping at `context_count`.
///
/// The current context is considered last, so it is returned only when no
/// other context is runnable. Keeping this policy separate from the AArch64
/// context-switch mechanism makes the selection rule host-testable.
pub fn next_runnable(
    current: usize,
    context_count: usize,
    mut is_runnable: impl FnMut(usize) -> bool,
) -> Option<usize> {
    if context_count == 0 || current >= context_count {
        return None;
    }

    for offset in 1..=context_count {
        let candidate = current.wrapping_add(offset) % context_count;
        if is_runnable(candidate) {
            return Some(candidate);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::next_runnable;

    #[test]
    fn advances_and_wraps() {
        assert_eq!(next_runnable(0, 3, |_| true), Some(1));
        assert_eq!(next_runnable(2, 3, |_| true), Some(0));
    }

    #[test]
    fn skips_contexts_that_are_not_runnable() {
        let states = [true, false, true, false];
        assert_eq!(next_runnable(0, states.len(), |i| states[i]), Some(2));
        assert_eq!(next_runnable(2, states.len(), |i| states[i]), Some(0));
    }

    #[test]
    fn falls_back_to_current_when_it_is_the_only_runnable_context() {
        assert_eq!(next_runnable(1, 3, |i| i == 1), Some(1));
    }

    #[test]
    fn reports_no_candidate_for_invalid_or_empty_sets() {
        assert_eq!(next_runnable(0, 0, |_| true), None);
        assert_eq!(next_runnable(3, 3, |_| true), None);
        assert_eq!(next_runnable(0, 3, |_| false), None);
    }
}
