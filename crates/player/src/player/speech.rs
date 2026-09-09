use std::{
    cell::RefCell,
    io::Write,
    process::{Child, Command, Stdio},
    sync::mpsc,
    time::Duration,
};

thread_local! {
    static READER: RefCell<Reader> = RefCell::new(Reader::default());
}

#[derive(Default)]
struct Reader {
    enabled: bool,
    focused: Option<String>,
    last: String,
    sender: Option<mpsc::SyncSender<String>>,
    errors: Option<mpsc::Receiver<String>>,
}

pub(super) fn begin(enabled: bool) {
    READER.with(|reader| {
        let mut reader = reader.borrow_mut();
        reader.enabled = enabled;
        reader.focused = None;
    });
}
pub(super) fn label(text: &str, focused: bool) {
    if focused {
        READER.with(|reader| reader.borrow_mut().focused = Some(text.to_owned()));
    }
}
pub(super) fn finish(dialogue: &str) -> Option<String> {
    READER.with(|reader| {
        let mut reader = reader.borrow_mut();
        let text = if reader.enabled {
            reader.focused.take().unwrap_or_else(|| dialogue.to_owned())
        } else {
            String::new()
        };
        if text != reader.last {
            if reader.sender.is_none() && reader.enabled {
                let (sender, incoming) = mpsc::sync_channel::<String>(1);
                let (errors, receiver) = mpsc::channel();
                std::thread::spawn(move || {
                    let mut child: Option<Child> = None;
                    while let Ok(text) = incoming.recv() {
                        if let Some(mut child) = child.take() {
                            let _ = child.kill();
                            let _ = child.wait();
                        }
                        if !text.is_empty() {
                            match speak(&text) {
                                Ok(next) => child = Some(next),
                                Err(error) => {
                                    let _ = errors.send(error);
                                }
                            }
                        }
                    }
                    if let Some(mut child) = child {
                        let _ = child.kill();
                        let _ = child.wait();
                    }
                });
                reader.sender = Some(sender);
                reader.errors = Some(receiver);
            }
            if reader
                .sender
                .as_ref()
                .is_some_and(|sender| sender.try_send(text.clone()).is_ok())
            {
                reader.last = text;
            }
        }
        reader
            .errors
            .as_ref()
            .and_then(|errors| errors.recv_timeout(Duration::ZERO).ok())
    })
}

fn speak(text: &str) -> Result<Child, String> {
    let mut command = if cfg!(target_os = "macos") {
        let mut command = Command::new("say");
        command.args(["-f", "-"]);
        command
    } else if cfg!(target_os = "windows") {
        let mut command = Command::new("powershell.exe");
        command.args(["-NoProfile", "-NonInteractive", "-Command", "Add-Type -AssemblyName System.Speech; $reader = New-Object System.Speech.Synthesis.SpeechSynthesizer; $reader.Speak([Console]::In.ReadToEnd())"]);
        command
    } else {
        let mut command = Command::new("espeak-ng");
        command.arg("--stdin");
        command
    };
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| format!("Speech unavailable: {error}"))?;
    if let Some(mut input) = child.stdin.take() {
        input
            .write_all(text.as_bytes())
            .map_err(|error| error.to_string())?;
    }
    Ok(child)
}
