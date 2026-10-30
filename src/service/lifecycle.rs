// SPDX-License-Identifier: GPL-3.0-only
//! Release input ownership on normal shutdown and early startup errors.
use crate::input::Inputs;
use std::sync::{Arc, atomic::Ordering};

pub(super) struct InputSession(Arc<Inputs>);

impl InputSession {
    pub(super) fn acquire(input: Arc<Inputs>) -> Self {
        input.navigation_owned.store(true, Ordering::Relaxed);
        input.suspended.store(true, Ordering::Relaxed);
        Self(input)
    }
}

impl Drop for InputSession {
    fn drop(&mut self) {
        self.0.suspended.store(true, Ordering::Relaxed);
        self.0.release_panel();
        self.0.navigation_owned.store(false, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dropping_session_releases_panel_and_navigation_ownership() {
        let input = Arc::new(Inputs::default());
        {
            let _session = InputSession::acquire(input.clone());
            assert!(input.navigation_owned.load(Ordering::Relaxed));
            let mut report = [0; 14];
            report[0] = 1;
            report[4] = 0x20;
            input.update_panel(&report);
            input.suspended.store(false, Ordering::Relaxed);
            assert_ne!(input.key_bitmap(), [0; 4]);
        }
        assert!(!input.navigation_owned.load(Ordering::Relaxed));
        assert!(input.suspended.load(Ordering::Relaxed));
        assert_eq!(input.mcdu.load(Ordering::Relaxed), 0);
        assert_eq!(input.key_bitmap(), [0; 4]);
        let mut released = [0; 14];
        released[0] = 1;
        assert_eq!(input.panel_snapshot(), released);
    }
}
