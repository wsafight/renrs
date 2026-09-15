use renrs_syntax::syntax::Value;
use std::collections::BTreeMap;
use std::sync::Mutex;
use velin::{
    DEFAULT_RNG_SEED, EvalError, FastYield, MachineInvoker, PureModule, PureModuleError, Type,
};

// Bound retained VM workspaces after a burst of concurrent extension calls.
const MAX_IDLE_INVOKERS_PER_MODULE: usize = 4;

#[derive(Clone)]
pub struct Extensions {
    modules: BTreeMap<String, ExtensionModule>,
}

struct ExtensionModule {
    module: PureModule,
    input_slot: Option<u32>,
    return_host_id: Option<u32>,
    fail_host_id: Option<u32>,
    idle_invokers: Mutex<Vec<MachineInvoker>>,
}

impl Clone for ExtensionModule {
    fn clone(&self) -> Self {
        Self {
            module: self.module.clone(),
            input_slot: self.input_slot,
            return_host_id: self.return_host_id,
            fail_host_id: self.fail_host_id,
            idle_invokers: Mutex::new(Vec::new()),
        }
    }
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
                ExtensionModule::compile(name, source).map(|module| (name.clone(), module))
            })
            .collect::<Result<_, _>>()?;
        Ok(Self { modules })
    }

    /// Invokes a reset Velin VM containing only immutable input; the result is story data.
    /// # Errors
    /// Reports missing extensions, execution limits and invalid result types.
    pub fn invoke(&self, name: &str, input: &Value) -> Result<Value, String> {
        input.validate_data().map_err(str::to_owned)?;
        let module = self
            .modules
            .get(name)
            .ok_or_else(|| format!("unknown extension: {name}"))?;
        let output = module
            .invoke(input.clone())
            .map_err(|error| format!("extension {name}: {error}"))?;
        output.validate_data().map_err(str::to_owned)?;
        Ok(output)
    }
}

impl ExtensionModule {
    fn compile(name: &str, source: &str) -> Result<Self, String> {
        let module = compile_pure_module(name, source)?;
        let script = module.script();
        let input_slot =
            if module.inputs().contains_key("input") {
                Some(
                    script.program.slots.get("input").ok_or_else(|| {
                        format!("extension {name}: compiled input slot is invalid")
                    })?,
                )
            } else {
                None
            };
        let return_host_id = host_id(&script.hosts, "return");
        let fail_host_id = host_id(&script.hosts, "fail");
        Ok(Self {
            module,
            input_slot,
            return_host_id,
            fail_host_id,
            idle_invokers: Mutex::new(Vec::new()),
        })
    }

    fn invoke(&self, input: Value) -> Result<Value, PureModuleError> {
        let available = self
            .idle_invokers
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .pop();
        let mut invoker = available.map_or_else(|| self.new_invoker(), Ok)?;
        let result = self.invoke_with(&mut invoker, input);
        let mut idle = self
            .idle_invokers
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if idle.len() < MAX_IDLE_INVOKERS_PER_MODULE {
            idle.push(invoker);
        }
        result
    }

    fn new_invoker(&self) -> Result<MachineInvoker, PureModuleError> {
        let script = self.module.script();
        let initial = script.initial_frame().ok_or_else(|| {
            PureModuleError::Execution(EvalError::new(0, "compiled initial frame is invalid"))
        })?;
        MachineInvoker::new(script.validated_program(), DEFAULT_RNG_SEED, initial).ok_or_else(
            || PureModuleError::Execution(EvalError::new(0, "compiled initial frame is invalid")),
        )
    }

    fn invoke_with(
        &self,
        invoker: &mut MachineInvoker,
        input: Value,
    ) -> Result<Value, PureModuleError> {
        invoker
            .restart()
            .map_err(|error| PureModuleError::Execution(EvalError::new(0, error)))?;
        if let Some(slot) = self.input_slot {
            invoker
                .machine_mut()
                .try_set_slot(slot, input)
                .map_err(|error| {
                    PureModuleError::Execution(EvalError::new(0, error.to_string()))
                })?;
        }
        self.finish(invoker)
    }

    fn finish(&self, invoker: &mut MachineInvoker) -> Result<Value, PureModuleError> {
        let mut hosts = [0; 2];
        let mut host_count = 0;
        for host_id in [self.return_host_id, self.fail_host_id]
            .into_iter()
            .flatten()
        {
            hosts[host_count] = host_id;
            host_count += 1;
        }
        match invoker
            .machine_mut()
            .run_with_single_argument_hosts(&hosts[..host_count])
            .map_err(PureModuleError::Execution)?
        {
            FastYield::Finished => Err(PureModuleError::MissingReturn),
            FastYield::HostOne { host_id, value } => self.finish_host(host_id, value),
            FastYield::Host { host_id, values } => {
                if values.len() != 1 {
                    return Err(PureModuleError::InvalidReturn(format!(
                        "host command expects exactly one value, found {}",
                        values.len()
                    )));
                }
                let value = values.into_iter().next().ok_or_else(|| {
                    PureModuleError::InvalidReturn("host command returned no value".into())
                })?;
                self.finish_host(host_id, value)
            }
        }
    }

    fn finish_host(&self, host_id: u32, value: Value) -> Result<Value, PureModuleError> {
        if Some(host_id) == self.return_host_id {
            return Ok(value);
        }
        if Some(host_id) == self.fail_host_id {
            let Value::String(message) = value else {
                return Err(PureModuleError::InvalidReturn(
                    "fail expects a string message".into(),
                ));
            };
            return Err(PureModuleError::ExplicitFailure(message.to_string()));
        }
        let name = self
            .module
            .script()
            .host_name(host_id)
            .unwrap_or("<unknown>");
        Err(PureModuleError::InvalidReturn(format!(
            "unexpected host command `{name}` in pure module"
        )))
    }
}

fn host_id(hosts: &[String], name: &str) -> Option<u32> {
    hosts
        .iter()
        .position(|host| host == name)
        .and_then(|id| u32::try_from(id).ok())
}

fn compile_pure_module(name: &str, source: &str) -> Result<PureModule, String> {
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
        assert_eq!(
            extensions.modules["reward"]
                .idle_invokers
                .lock()
                .unwrap()
                .len(),
            1
        );
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
        assert_eq!(
            extensions.modules["loop"]
                .idle_invokers
                .lock()
                .unwrap()
                .len(),
            1
        );
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

    #[test]
    fn concurrent_invocations_keep_machine_state_isolated() {
        let extensions = std::sync::Arc::new(
            Extensions::new(&BTreeMap::from([(
                "sum".into(),
                "set total = 0\nset index = 0\nwhile index < input:\n    set total = total + index\n    set index = index + 1\nperform return(total)\n".into(),
            )]))
            .unwrap(),
        );
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(8));
        let threads: Vec<_> = (1..=8)
            .map(|input| {
                let extensions = extensions.clone();
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    barrier.wait();
                    extensions
                        .invoke("sum", &Value::Integer(input))
                        .map(|output| (input, output))
                })
            })
            .collect();
        for thread in threads {
            let (input, output) = thread.join().unwrap().unwrap();
            assert_eq!(output, Value::Integer(input * (input - 1) / 2));
        }
        let idle = extensions.modules["sum"]
            .idle_invokers
            .lock()
            .unwrap()
            .len();
        assert!((1..=MAX_IDLE_INVOKERS_PER_MODULE).contains(&idle));
    }
}
