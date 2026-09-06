use renrs::{Localizer, TranslationId};
use std::cell::RefCell;
use std::collections::HashMap;

thread_local! {
    static LOCALIZER: RefCell<Localizer> = RefCell::new(Localizer::default());
    static CHINESE: HashMap<String, String> = serde_json::from_str(include_str!("ui_zh.json")).expect("bundled UI catalog is valid");
}

pub(super) fn set_localizer(localizer: Localizer) {
    LOCALIZER.with(|current| *current.borrow_mut() = localizer);
}

pub(super) fn tr(text: &str) -> String {
    LOCALIZER.with(|current| {
        let localizer = current.borrow();
        let key = format!(
            "ui.{}",
            text.to_ascii_lowercase()
                .split_whitespace()
                .collect::<Vec<_>>()
                .join("_")
        );
        if let Ok(id) = TranslationId::new(key) {
            let translated = localizer.translate(&id, text);
            if translated != text {
                return translated.to_owned();
            }
        }
        if localizer
            .language()
            .is_some_and(|language| language.starts_with("zh"))
        {
            return CHINESE.with(|catalog| {
                catalog
                    .get(text)
                    .cloned()
                    .unwrap_or_else(|| text.to_owned())
            });
        }
        text.to_owned()
    })
}
