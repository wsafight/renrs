use std::sync::mpsc::{self, Sender};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

const FRAME_INTERVAL: Duration = Duration::from_micros(16_667);
const MAX_TIMER_STEP: Duration = Duration::from_millis(250);

#[derive(Clone, Copy)]
enum WakeCommand {
    Schedule(Option<Duration>),
    Stop,
}

pub(super) struct WakeTimer {
    sender: Sender<WakeCommand>,
    thread: Option<JoinHandle<()>>,
    deadline: Option<Instant>,
    interval: Option<Duration>,
}

impl WakeTimer {
    pub(super) fn new() -> Self {
        let (sender, receiver) = mpsc::channel();
        let thread = std::thread::spawn(move || {
            while let Ok(command) = receiver.recv() {
                let WakeCommand::Schedule(Some(mut delay)) = command else {
                    if matches!(command, WakeCommand::Stop) {
                        return;
                    }
                    continue;
                };
                loop {
                    match receiver.recv_timeout(delay) {
                        Ok(WakeCommand::Schedule(Some(next))) => delay = next,
                        Ok(WakeCommand::Schedule(None)) => break,
                        Ok(WakeCommand::Stop) | Err(mpsc::RecvTimeoutError::Disconnected) => {
                            return;
                        }
                        Err(mpsc::RecvTimeoutError::Timeout) => {
                            request_update();
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
            interval: None,
        }
    }

    pub(super) fn schedule(&mut self, interval: Option<Duration>, frame_started: Instant) {
        if !macroquad::miniquad::window::blocking_event_loop() {
            return;
        }
        if self.interval != interval {
            self.deadline = None;
        }
        self.interval = interval;
        let delay = interval.map(|interval| {
            let deadline = next_deadline(self.deadline, frame_started, interval);
            self.deadline = Some(deadline);
            deadline.saturating_duration_since(Instant::now())
        });
        if interval.is_none() {
            self.deadline = None;
        }
        let _ = self.sender.send(WakeCommand::Schedule(delay));
    }
}

pub(super) fn request_update() {
    macroquad::miniquad::window::schedule_update();
    wake_event_loop();
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
        let _ = self.sender.send(WakeCommand::Stop);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

impl super::app::App {
    pub(super) fn wake_interval(&self, active: bool) -> Option<Duration> {
        if active {
            return Some(FRAME_INTERVAL);
        }
        let mut next = self.storage_wake_in();
        if let Some((_, remaining)) = &self.notice {
            update_minimum(&mut next, timer_delay(*remaining));
        }
        if self.screen == super::app::Screen::Playing && self.overlay.is_none() {
            use renrs::WaitState;
            match self.runtime.as_ref().and_then(renrs::Runtime::waiting) {
                Some(WaitState::Pause { .. }) => {
                    update_minimum(&mut next, timer_delay(self.pause_remaining));
                }
                Some(WaitState::Dialogue) => {
                    if let Some(remaining) = self.dialogue_cue_remaining {
                        update_minimum(&mut next, timer_delay(remaining));
                    } else if (self.playback.auto || self.dialogue_view.no_wait())
                        && self.visible_characters >= self.dialogue_view.character_count() as f32
                        && (!self.settings.wait_voice
                            || self.settings.voice_volume <= f32::EPSILON
                            || !self.audio.voice_busy()
                            || self.dialogue_view.no_wait())
                    {
                        update_minimum(&mut next, timer_delay(self.auto_remaining));
                    }
                }
                _ => {}
            }
        }
        next
    }

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

fn timer_delay(seconds: f32) -> Option<Duration> {
    (seconds.is_finite() && seconds > 0.0)
        .then(|| Duration::from_secs_f32(seconds).min(MAX_TIMER_STEP))
}

fn update_minimum(current: &mut Option<Duration>, candidate: Option<Duration>) {
    if let Some(candidate) = candidate {
        *current = Some(current.map_or(candidate, |current| current.min(candidate)));
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

    #[test]
    fn long_story_timers_use_bounded_steps() {
        assert_eq!(timer_delay(2.0), Some(MAX_TIMER_STEP));
        assert_eq!(timer_delay(0.05), Some(Duration::from_secs_f32(0.05)));
        assert_eq!(timer_delay(f32::INFINITY), None);
        assert_eq!(timer_delay(0.0), None);
    }
}
