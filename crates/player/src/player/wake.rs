use std::sync::mpsc::{self, Sender};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

pub(super) struct WakeTimer {
    sender: Sender<Option<Duration>>,
    thread: Option<JoinHandle<()>>,
    deadline: Option<Instant>,
    animated: bool,
}

impl WakeTimer {
    pub(super) fn new() -> Self {
        let (sender, receiver) = mpsc::channel();
        let thread = std::thread::spawn(move || {
            while let Ok(Some(mut delay)) = receiver.recv() {
                loop {
                    match receiver.recv_timeout(delay) {
                        Ok(Some(next)) => delay = next,
                        Ok(None) | Err(mpsc::RecvTimeoutError::Disconnected) => return,
                        Err(mpsc::RecvTimeoutError::Timeout) => {
                            macroquad::miniquad::window::schedule_update();
                            wake_event_loop();
                            break;
                        }
                    }
                }
            }
        });
        Self {
            sender,
            thread: Some(thread),
            deadline: None,
            animated: false,
        }
    }

    pub(super) fn schedule(&mut self, animated: bool, frame_started: Instant) {
        if !macroquad::miniquad::window::blocking_event_loop() {
            return;
        }
        let interval = if animated {
            Duration::from_micros(16_667)
        } else {
            Duration::from_millis(100)
        };
        if self.animated != animated {
            self.deadline = None;
        }
        self.animated = animated;
        let deadline = next_deadline(self.deadline, frame_started, interval);
        self.deadline = Some(deadline);
        let _ = self
            .sender
            .send(Some(deadline.saturating_duration_since(Instant::now())));
    }
}

fn next_deadline(previous: Option<Instant>, started: Instant, interval: Duration) -> Instant {
    // Keep timer oversleep from accumulating across animation frames.
    match previous {
        Some(due) if due > started => due,
        Some(due) if due + interval > started => due + interval,
        _ => started + interval,
    }
}

// Miniquad's request channel alone does not wake Cocoa's nextEvent wait.
#[cfg(target_os = "macos")]
fn wake_event_loop() {
    dispatch2::DispatchQueue::main().exec_async(|| {
        use objc2_app_kit::{NSApplication, NSEvent, NSEventModifierFlags, NSEventType};
        let main = objc2::MainThreadMarker::new().expect("main dispatch queue");
        let event = NSEvent::otherEventWithType_location_modifierFlags_timestamp_windowNumber_context_subtype_data1_data2(
            NSEventType::ApplicationDefined, objc2_foundation::NSPoint::new(0.0, 0.0),
            NSEventModifierFlags::empty(), 0.0, 0, None, 0, 0, 0,
        );
        if let Some(event) = event { NSApplication::sharedApplication(main).postEvent_atStart(&event, false); }
    });
}

#[cfg(not(target_os = "macos"))]
fn wake_event_loop() {}

impl Drop for WakeTimer {
    fn drop(&mut self) {
        let _ = self.sender.send(None);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

impl super::app::App {
    pub(super) fn animated(&self) -> bool {
        use renrs::WaitState;
        self.overlay.is_none()
            && self.screen == super::app::Screen::Playing
            && self.runtime.as_ref().is_some_and(|runtime| {
                (!self.theme.reduced_motion
                    && runtime.stage().sprites.iter().any(|sprite| {
                        sprite.composition.as_ref().is_some_and(|image| {
                            image.layers.iter().any(|layer| {
                                !layer.frames.is_empty()
                                    && (!layer.speaking || self.audio.voice_busy())
                            })
                        })
                    }))
                    || match runtime.waiting() {
                        Some(WaitState::Dialogue) => {
                            self.visible_characters < self.dialogue_view.character_count() as f32
                                || self.playback.skip_read
                        }
                        Some(WaitState::Effect { .. }) => true,
                        _ => false,
                    }
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timer_oversleep_preserves_cadence_without_catch_up_bursts() {
        let start = Instant::now();
        let interval = Duration::from_millis(16);
        let first = next_deadline(None, start, interval);
        assert_eq!(first, start + interval);
        let late = first + Duration::from_millis(4);
        assert_eq!(next_deadline(Some(first), late, interval), first + interval);
        let stalled = first + Duration::from_millis(200);
        assert_eq!(
            next_deadline(Some(first), stalled, interval),
            stalled + interval
        );
        assert_eq!(next_deadline(Some(first), start, interval), first);
    }
}
