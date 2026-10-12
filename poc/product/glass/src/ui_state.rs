//! Renderer-independent, allocation-free scaffold lifecycle. No authority types.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Idle,
    Summoned,
    Interpreting,
    Preview,
    Authorizing,
    Executing,
    Active,
    Completed,
    Clarifying,
    Denied,
    Failed,
    Dissolved,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    Summon,
    Submit,
    Preview,
    Clarify,
    Edit,
    Approve,
    Unavailable,
    Cancel,
    Reset,
}
#[derive(Debug, PartialEq, Eq)]
pub enum Reject {
    Stale,
    WrongState,
    Exhausted,
}
pub struct Session {
    pub phase: Phase,
    pub revision: u64,
}
impl Default for Session {
    fn default() -> Self {
        Self {
            phase: Phase::Idle,
            revision: 0,
        }
    }
}
impl Session {
    pub fn apply(&mut self, event: Event, expected: u64) -> Result<(), Reject> {
        if expected != self.revision {
            return Err(Reject::Stale);
        }
        use Event as E;
        use Phase as P;
        let next = match (self.phase, event) {
            (P::Idle | P::Dissolved, E::Summon) => P::Summoned,
            (P::Summoned | P::Clarifying, E::Submit) => P::Interpreting,
            (P::Interpreting, E::Preview) => P::Preview,
            (P::Interpreting, E::Clarify) => P::Clarifying,
            (P::Preview | P::Denied, E::Edit) => P::Preview,
            (P::Preview, E::Approve) => P::Authorizing,
            (P::Authorizing, E::Unavailable) => P::Denied,
            (
                P::Summoned | P::Interpreting | P::Preview | P::Clarifying | P::Denied | P::Failed,
                E::Cancel,
            ) => P::Dissolved,
            (P::Dissolved, E::Reset) => P::Idle,
            _ => return Err(Reject::WrongState),
        };
        let revision = self.revision.checked_add(1).ok_or(Reject::Exhausted)?;
        self.phase = next;
        self.revision = revision;
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stale_consent_and_execution_shortcuts_are_rejected() {
        let mut s = Session::default();
        for e in [Event::Summon, Event::Submit, Event::Preview] {
            s.apply(e, s.revision).unwrap();
        }
        let old = s.revision;
        s.apply(Event::Edit, old).unwrap();
        assert_eq!(s.apply(Event::Approve, old), Err(Reject::Stale));
        s.apply(Event::Approve, s.revision).unwrap();
        assert_eq!(s.apply(Event::Preview, s.revision), Err(Reject::WrongState));
        s.apply(Event::Unavailable, s.revision).unwrap();
        assert_eq!(s.phase, Phase::Denied);
    }
    #[test]
    fn cancel_dissolves_without_dispatching() {
        let mut s = Session::default();
        s.apply(Event::Summon, 0).unwrap();
        s.apply(Event::Cancel, 1).unwrap();
        s.apply(Event::Reset, 2).unwrap();
        assert_eq!(s.phase, Phase::Idle);
        assert_eq!(s.apply(Event::Approve, 3), Err(Reject::WrongState));
    }
}
