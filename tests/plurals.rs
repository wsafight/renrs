use renrs::{Localizer, Runtime, TranslationCatalog, compile, parse_script};

#[test]
fn plural_rules_refresh_dialogue_choices_and_reject_bad_counts_transactionally() {
    let script = parse_script("default count = 2\nlabel start:\n    @id \"items\" \"{count} items\"\n    menu:\n        \"Take items\" id \"pick\":\n            return\n        \"Leave\":\n            return\n", "plural.rns").unwrap();
    let mut runtime = Runtime::new(compile(&script).unwrap()).unwrap();
    let catalog = TranslationCatalog::from_reader(br#"{"language":"en","plurals":{"items":{"count":"count","forms":{"one":"{count} item","other":"{count} items"}},"pick":{"count":"count","forms":{"one":"Take one","other":"Take many"}}}}"#.as_slice()).unwrap();
    runtime.insert_translation_catalog(catalog).unwrap();
    runtime.set_language(Some("en-US".to_owned())).unwrap();
    runtime.advance().unwrap();
    assert_eq!(runtime.stage().dialogue.as_ref().unwrap().text, "2 items");
    runtime
        .set_screen_variable("count", renrs::syntax::Value::Integer(1))
        .unwrap();
    runtime.set_language(Some("en".to_owned())).unwrap();
    assert_eq!(runtime.stage().dialogue.as_ref().unwrap().text, "1 item");
    runtime.continue_story().unwrap();
    assert_eq!(
        runtime.advance().unwrap(),
        renrs::WaitState::Choice {
            options: vec!["Take one".to_owned(), "Leave".to_owned()]
        }
    );
    let mut localizer = Localizer::new(Some("fr".to_owned()));
    localizer.insert(TranslationCatalog::from_reader(br#"{"language":"fr","plurals":{"pick":{"count":"absent","forms":{"other":"bad"}}}}"#.as_slice()).unwrap()).unwrap();
    let before = runtime.snapshot();
    assert!(runtime.set_localizer(localizer).is_err());
    assert_eq!(runtime.snapshot().waiting, before.waiting);
}

#[test]
fn arabic_six_forms_and_russian_few_use_cldr_categories() {
    use renrs::localization::TranslationId;
    use renrs::syntax::Value;
    use std::collections::BTreeMap;
    for (language, counts) in [
        (
            "ar",
            vec![
                (0, "zero"),
                (1, "one"),
                (2, "two"),
                (3, "few"),
                (11, "many"),
                (102, "other"),
            ],
        ),
        ("ru", vec![(1, "one"), (2, "few"), (5, "many"), (21, "one")]),
    ] {
        let json = serde_json::json!({"language":language,"plurals":{"n":{"count":"count","forms":{"zero":"zero","one":"one","two":"two","few":"few","many":"many","other":"other"}}}});
        let mut localizer = Localizer::new(Some(language.to_owned()));
        localizer
            .insert(TranslationCatalog::from_reader(json.to_string().as_bytes()).unwrap())
            .unwrap();
        for (count, expected) in counts {
            assert_eq!(
                localizer
                    .translate_values(
                        &TranslationId::new("n").unwrap(),
                        "source",
                        &BTreeMap::from([("count".to_owned(), Value::Integer(count))])
                    )
                    .unwrap(),
                expected
            );
        }
    }
}
