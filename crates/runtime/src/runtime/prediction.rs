use super::{InstructionKind, Runtime};
use std::collections::{HashSet, VecDeque};

impl Runtime {
    /// Predicts images on reachable paths without evaluating or executing story code.
    /// Traversal is bounded to 256 positions, 16 call frames and 16 unique images.
    #[must_use]
    pub fn upcoming_images(&self, limit: usize) -> Vec<String> {
        let limit = limit.min(16);
        let mut images = Vec::new();
        let stack = self
            .call_stack
            .iter()
            .rev()
            .take(16)
            .rev()
            .copied()
            .collect::<Vec<_>>();
        let mut queue = VecDeque::from([(self.instruction, stack)]);
        let mut visited = HashSet::new();
        while let Some((position, mut stack)) = queue.pop_front() {
            if images.len() >= limit || visited.len() >= 256 {
                break;
            }
            if !visited.insert((position, stack.clone())) {
                continue;
            }
            let Some(instruction) = self.program.instructions.get(position) else {
                continue;
            };
            let mut targets = Vec::new();
            match &instruction.kind {
                InstructionKind::Scene { path } | InstructionKind::Show { path, .. } => {
                    let paths = self.program.layered_images.get(path).map_or_else(
                        || vec![path.as_str()],
                        |image| {
                            image
                                .layers
                                .iter()
                                .flat_map(crate::syntax::ImageLayer::paths)
                                .collect()
                        },
                    );
                    for path in paths {
                        if images.len() < limit && !images.iter().any(|image| image == path) {
                            images.push(path.to_owned());
                        }
                    }
                    targets.push(position + 1);
                }
                InstructionKind::Jump { target } => targets.push(*target),
                InstructionKind::JumpIfFalse { target, .. } => {
                    targets.extend([position + 1, *target]);
                }
                InstructionKind::Choice { options } => {
                    targets.extend(options.iter().map(|option| option.target));
                }
                InstructionKind::Call { target, .. } if stack.len() < 16 => {
                    stack.push(position + 1);
                    targets.push(*target);
                }
                InstructionKind::Call { .. } => {}
                InstructionKind::Return { .. } => targets.extend(stack.pop()),
                _ => targets.push(position + 1),
            }
            for target in targets
                .into_iter()
                .take(256_usize.saturating_sub(queue.len()))
            {
                queue.push_back((target, stack.clone()));
            }
        }
        images
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn runtime(script: &str) -> Runtime {
        Runtime::new(crate::compile(&crate::parse_script(script, "test.rns").unwrap()).unwrap())
            .unwrap()
    }
    #[test]
    fn follows_distant_calls_returns_jumps_and_deduplicates_without_mutating() {
        let padding = "    \"Unreachable\"\n".repeat(80);
        let mut runtime = runtime(&format!(
            "label start:\n    \"Start\"\n    call distant\n    scene \"returned.png\"\n    return\nlabel padding:\n{padding}label distant:\n    show \"called.png\" as person at center\n    jump tail\nlabel tail:\n    scene \"called.png\"\n    return\n"
        ));
        runtime.advance().unwrap();
        let before = serde_json::to_string(&runtime.snapshot()).unwrap();
        assert_eq!(runtime.upcoming_images(4), ["called.png", "returned.png"]);
        assert_eq!(serde_json::to_string(&runtime.snapshot()).unwrap(), before);
        assert!(runtime.upcoming_images(0).is_empty());
    }
    #[test]
    fn predicts_alternative_choices_and_bounds_recursive_calls() {
        let mut runtime = runtime(
            "label start:\n    menu:\n        \"Left\":\n            scene \"left.png\"\n            return\n        \"Right\":\n            scene \"right.png\"\n            call recursive\nlabel recursive:\n    call recursive\n    scene \"unreachable.png\"\n    return\n",
        );
        runtime.advance().unwrap();
        assert_eq!(runtime.upcoming_images(16), ["left.png", "right.png"]);
    }
}
