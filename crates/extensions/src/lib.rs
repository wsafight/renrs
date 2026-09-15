use renrs_syntax::syntax::Value;
use std::collections::BTreeMap;
use velin::{PureModule, PureModuleError, Type};

#[derive(Clone)]
pub struct Extensions {
    modules: BTreeMap<String, PureModule>,
}

impl std::fmt::Debug for Extensions {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Extensions")
            .field("names", &self.modules.keys())
            .finish_non_exhaustive()
    }
}

impl Extensions {
    /// Compiles deterministic extension programs with bounded execution and data.
    /// # Errors
    /// Rejects invalid scripts, names and source budgets.
    pub fn new(sources: &BTreeMap<String, String>) -> Result<Self, String> {
        if sources.is_empty() {
            return Ok(Self {
                modules: BTreeMap::new(),
            });
        }
        if sources.len() > 64 || sources.values().map(String::len).sum::<usize>() > 1024 * 1024 {
            return Err("extensions exceed 64 programs or 1 MiB of source".to_owned());
        }
        let modules = sources
            .iter()
            .map(|(name, source)| {
                if name.is_empty()
                    || !name.bytes().all(|byte| {
                        byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b'-')
                    })
                {
                    return Err(format!("invalid extension name: {name}"));
                }
                compile_module(name, source).map(|module| (name.clone(), module))
            })
            .collect::<Result<_, _>>()?;
        Ok(Self { modules })
    }

    /// Invokes a fresh Velin VM containing only immutable input; the result is story data.
    /// # Errors
    /// Reports missing extensions, execution limits and invalid result types.
    pub fn invoke(&self, name: &str, input: &Value) -> Result<Value, String> {
        input.validate_data().map_err(str::to_owned)?;
        let module = self
            .modules
            .get(name)
            .ok_or_else(|| format!("unknown extension: {name}"))?;
        let values = if module.inputs().contains_key("input") {
            BTreeMap::from([("input".to_owned(), input.clone())])
        } else {
            BTreeMap::new()
        };
        let output = module
            .invoke(values)
            .map_err(|error| format!("extension {name}: {error}"))?;
        output.validate_data().map_err(str::to_owned)?;
        Ok(output)
    }
}

fn compile_module(name: &str, source: &str) -> Result<PureModule, String> {
    let source_name = format!("{name}.velin");
    let inputs = BTreeMap::from([("input".to_owned(), Type::Unknown)]);
    let compiled = match PureModule::compile(&source_name, source, inputs) {
        Err(PureModuleError::UnknownInput(input)) if input == "input" => {
            PureModule::compile(&source_name, source, BTreeMap::new())
        }
        result => result,
    };
    compiled.map_err(|error| format!("extension {name}: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn computes_data_without_leaking_scope_between_invocations() {
        let extensions = Extensions::new(&BTreeMap::from([(
            "reward".into(),
            "set score = input * 2 + 1\nperform return(score)\n".into(),
        )]))
        .unwrap();
        for _ in 0..2 {
            assert_eq!(
                extensions.invoke("reward", &Value::Integer(21)).unwrap(),
                Value::Integer(43)
            );
        }
    }
    #[test]
    fn rejects_host_effects_and_bounds_execution_and_return_protocol() {
        assert!(
            Extensions::new(&BTreeMap::from([(
                "bad".into(),
                "perform filesystem(\"secrets\")\n".into()
            )]))
            .is_err()
        );
        let extensions = Extensions::new(&BTreeMap::from([
            ("loop".into(), "while true:\n    set value = 1\n".into()),
            ("missing".into(), "set value = 1\n".into()),
        ]))
        .unwrap();
        assert!(extensions.invoke("loop", &Value::Integer(0)).is_err());
        assert!(extensions.invoke("missing", &Value::Integer(0)).is_err());
    }

    #[test]
    fn constant_modules_can_ignore_the_host_input() {
        let extensions = Extensions::new(&BTreeMap::from([(
            "constant".into(),
            "perform return(7)\n".into(),
        )]))
        .unwrap();
        assert_eq!(
            extensions.invoke("constant", &Value::Integer(99)).unwrap(),
            Value::Integer(7)
        );
    }
}
