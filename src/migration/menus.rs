use super::conversion::LineConversion;
use super::expressions::{closing_quote, convert_dialogue};

pub(super) fn menu_prompt(
    lines: &[&str],
    menu_line: usize,
    menu_indent: usize,
) -> Option<(usize, String)> {
    if lines.get(menu_line)?.trim() != "menu:" {
        return None;
    }
    for (index, raw) in lines.iter().enumerate().skip(menu_line + 1) {
        let content = raw.trim();
        if content.is_empty() || content.starts_with('#') {
            continue;
        }
        let indent = raw.len() - raw.trim_start_matches(' ').len();
        if indent != menu_indent + 4 || content.ends_with(':') {
            return None;
        }
        return match convert_dialogue(content) {
            Some(LineConversion::One(prompt)) => Some((index, prompt)),
            _ => None,
        };
    }
    None
}

pub(super) fn menu_has_explicit_exit(lines: &[&str], menu_line: usize, menu_indent: usize) -> bool {
    let choice_indent = menu_indent + 4;
    let branch_indent = choice_indent + 4;
    let mut saw_choice = false;
    let mut branch_exits = false;
    for raw in lines.iter().skip(menu_line + 1) {
        let content = raw.trim();
        if content.is_empty() || content.starts_with('#') {
            continue;
        }
        let indent = raw.len() - raw.trim_start_matches(' ').len();
        if indent <= menu_indent {
            break;
        }
        if indent == choice_indent && is_static_choice(content) {
            if saw_choice && !branch_exits {
                return false;
            }
            saw_choice = true;
            branch_exits = false;
        } else if indent == branch_indent && saw_choice {
            branch_exits = statement_has_explicit_exit(content);
        } else if indent <= choice_indent && (saw_choice || convert_dialogue(content).is_none()) {
            return false;
        }
    }
    saw_choice && branch_exits
}

pub(super) fn statement_has_explicit_exit(content: &str) -> bool {
    content == "return" || content.starts_with("return ") || content.starts_with("jump ")
}

fn is_static_choice(content: &str) -> bool {
    let Some(quote) = content
        .starts_with('"')
        .then(|| closing_quote(content, 0))
        .flatten()
    else {
        return false;
    };
    content[quote + 1..].trim() == ":"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requires_every_menu_branch_to_exit() {
        let complete = [
            "    menu:",
            "        \"Choose\"",
            "        \"One\":",
            "            jump one",
            "        \"Two\":",
            "            return 2",
        ];
        assert!(menu_has_explicit_exit(&complete, 0, 4));

        let incomplete = [
            "    menu:",
            "        \"One\":",
            "            jump one",
            "        \"Two\":",
            "            \"Keep going\"",
        ];
        assert!(!menu_has_explicit_exit(&incomplete, 0, 4));
    }
}
