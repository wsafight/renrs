use std::ffi::OsString;
use std::path::PathBuf;

const USAGE: &str = "usage: renrs [project|archive] [--window-size WIDTHxHEIGHT] [--smoke-test <new-output-directory>] [--profile <new-report.json>] [--benchmark <new-report.json>]";

pub(super) struct Options {
    pub(super) project: PathBuf,
    pub(super) window: (i32, i32),
    pub(super) smoke_test: Option<PathBuf>,
    pub(super) profile: Option<PathBuf>,
    pub(super) benchmark: Option<PathBuf>,
}

impl Options {
    pub(super) fn read() -> Self {
        let arguments: Vec<_> = std::env::args_os().skip(1).collect();
        if arguments.iter().any(|arg| arg == "--help" || arg == "-h") {
            println!("{USAGE}");
            std::process::exit(0);
        }
        Self::parse(arguments).unwrap_or_else(|error| {
            eprintln!("{error}\n{USAGE}");
            std::process::exit(2);
        })
    }

    fn parse(arguments: Vec<OsString>) -> Result<Self, String> {
        let mut project = None;
        let mut smoke_test = None;
        let mut profile = None;
        let mut benchmark = None;
        let mut window = (1280, 720);
        let mut arguments = arguments.into_iter();
        while let Some(argument) = arguments.next() {
            if argument == "--smoke-test" && smoke_test.is_none() {
                smoke_test = Some(PathBuf::from(
                    arguments.next().ok_or("missing smoke output directory")?,
                ));
            } else if argument == "--profile" && profile.is_none() {
                let path = PathBuf::from(arguments.next().ok_or("missing profile output")?);
                if path.exists() {
                    return Err("profile output already exists".to_owned());
                }
                profile = Some(path);
            } else if argument == "--benchmark" && benchmark.is_none() {
                let path = PathBuf::from(arguments.next().ok_or("missing benchmark output")?);
                if path.exists() || path.with_extension("ready.json").exists() {
                    return Err("benchmark output already exists".to_owned());
                }
                benchmark = Some(path);
            } else if argument == "--window-size" {
                let size = arguments.next().ok_or("missing window size")?;
                let (width, height) = size
                    .to_str()
                    .and_then(|size| size.split_once('x'))
                    .ok_or("invalid window size")?;
                window = (
                    width.parse().map_err(|_| "invalid width")?,
                    height.parse().map_err(|_| "invalid height")?,
                );
                if !(640..=3840).contains(&window.0) || !(360..=2160).contains(&window.1) {
                    return Err("window size must be within 640x360 and 3840x2160".to_owned());
                }
            } else if argument.to_string_lossy().starts_with('-') || project.is_some() {
                return Err(format!(
                    "unexpected argument: {}",
                    argument.to_string_lossy()
                ));
            } else {
                project = Some(PathBuf::from(argument));
            }
        }
        if benchmark.is_some() && (profile.is_some() || smoke_test.is_some()) {
            return Err("benchmark cannot be combined with profile or smoke-test".to_owned());
        }
        let project = match project {
            Some(project) => project,
            None => renrs::default_project_path(
                &std::env::current_exe().map_err(|error| error.to_string())?,
                &std::env::current_dir().map_err(|error| error.to_string())?,
            ),
        };
        Ok(Self {
            project,
            window,
            smoke_test,
            profile,
            benchmark,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_project_and_bounded_window_options() {
        let args = [
            "--smoke-test",
            "captures",
            "example.renrs",
            "--window-size",
            "800x600",
        ];
        let options = Options::parse(args.into_iter().map(OsString::from).collect()).unwrap();
        assert_eq!(options.project, PathBuf::from("example.renrs"));
        assert_eq!(options.window, (800, 600));
        assert_eq!(options.smoke_test, Some(PathBuf::from("captures")));
        for args in [
            vec!["--window-size", "0x600"],
            vec!["--smoke-test"],
            vec!["--unknown"],
        ] {
            assert!(Options::parse(args.into_iter().map(OsString::from).collect()).is_err());
        }
    }
}
