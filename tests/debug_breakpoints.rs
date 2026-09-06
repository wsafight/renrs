#[test]
fn stops_before_assignment_and_single_steps_without_losing_story_state() {
    let program = renrs::compile(
        &renrs::parse_script(
            "default score = 0\nlabel start:\n    \"Start\"\n    set score = 9\n    \"End\"",
            "test.rns",
        )
        .unwrap(),
    )
    .unwrap();
    let assignment = program
        .instructions
        .iter()
        .find(|item| matches!(item.kind, renrs::compiler::InstructionKind::Set { .. }))
        .unwrap()
        .id
        .clone();
    let mut runtime = renrs::Runtime::new(program).unwrap();
    runtime.enable_tracing();
    runtime.set_breakpoints(vec![assignment]).unwrap();
    runtime.advance().unwrap();
    assert_eq!(
        runtime.continue_story(),
        Err(renrs::RuntimeError::DebugPaused)
    );
    assert_eq!(
        runtime.variables()["score"],
        renrs::syntax::Value::Integer(0)
    );
    assert_eq!(
        runtime.debug_resume(true),
        Err(renrs::RuntimeError::DebugPaused)
    );
    assert_eq!(
        runtime.variables()["score"],
        renrs::syntax::Value::Integer(9)
    );
    assert_eq!(
        runtime.debug_resume(false).unwrap(),
        renrs::WaitState::Dialogue
    );
    assert_eq!(runtime.visited_instructions().count(), 3);
}
