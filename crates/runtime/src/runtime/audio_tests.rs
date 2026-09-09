use super::*;

#[test]
fn music_queue_and_fades_are_serializable_channel_state() {
    let program = compile(
            &parse_script(
                "label start:\n    play music \"audio/one.ogg\" fadein 0.5 volume 0.4\n    queue music \"audio/two.ogg\" loop volume 0.7 fadein 0.25\n    play sound \"audio/click.wav\" volume 0.25\n    \"Playing\"\n    stop music fadeout 0.4\n    \"Stopped\"\n",
                "test.rns",
            )
            .unwrap(),
        )
        .unwrap();
    let mut runtime = Runtime::new(program.clone()).unwrap();
    assert_eq!(runtime.advance().unwrap(), WaitState::Dialogue);
    assert!((runtime.stage().music.as_ref().unwrap().fade_in - 0.5).abs() < f32::EPSILON);
    assert!((runtime.stage().music.as_ref().unwrap().volume - 0.4).abs() < f32::EPSILON);
    assert_eq!(runtime.stage().music_queue.len(), 1);
    assert!((runtime.stage().music_queue[0].volume - 0.7).abs() < f32::EPSILON);
    assert!(runtime.stage().music_queue[0].repeat);
    assert!(
        runtime
            .drain_audio_events()
            .any(|event| matches!(event, AudioEvent::PlaySound { volume: 0.25, .. }))
    );
    let snapshot = runtime.snapshot();
    let mut restored = Runtime::restore(program, snapshot).unwrap();
    assert_eq!(restored.stage().music_queue.len(), 1);
    assert!((restored.stage().music_queue[0].volume - 0.7).abs() < f32::EPSILON);
    restored.complete_music_track();
    assert_eq!(
        restored
            .stage()
            .music
            .as_ref()
            .map(|music| music.path.as_str()),
        Some("audio/two.ogg")
    );
    assert!(restored.stage().music_queue.is_empty());
}

#[test]
fn sound_queue_advances_and_preserves_looping_tracks() {
    let mut runtime = runtime(
        "label start:\n    play sound \"audio/one.wav\"\n    queue sound \"audio/two.wav\" loop\n    \"Playing\"",
    );
    assert_eq!(runtime.advance().unwrap(), WaitState::Dialogue);
    assert_eq!(runtime.stage().sound_queue.len(), 1);
    runtime.complete_sound_track();
    assert_eq!(
        runtime.stage().sound.as_ref().unwrap().path,
        "audio/two.wav"
    );
    assert!(runtime.stage().sound.as_ref().unwrap().repeat);
    runtime.complete_sound_track();
    assert_eq!(
        runtime.stage().sound.as_ref().unwrap().path,
        "audio/two.wav"
    );
}
