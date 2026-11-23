//! Command-line arguments.

use std::path::PathBuf;

/// Usage line, printed on any argument error.
pub const USAGE: &str = "usage: limina check <file> [--format text|json]";

/// Output format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    /// One block per claim, for people.
    Text,
    /// The analysis response with file, line and column, for programs.
    Json,
}

/// A parsed `limina check` command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Command {
    /// The file to check.
    pub path: PathBuf,
    /// Output format.
    pub format: Format,
}

/// Parses the arguments after the program name.
///
/// Errors: a message ending with the usage line.
pub fn parse(args: &[String]) -> Result<Command, String> {
    let mut rest = args.iter();
    match rest.next().map(String::as_str) {
        Some("check") => {}
        Some(other) => return Err(format!("unknown command {other}\n{USAGE}")),
        None => return Err(USAGE.to_string()),
    }
    let mut path = None;
    let mut format = Format::Text;
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--format" => {
                format = match rest.next().map(String::as_str) {
                    Some("text") => Format::Text,
                    Some("json") => Format::Json,
                    _ => return Err(format!("--format takes text or json\n{USAGE}")),
                }
            }
            flag if flag.starts_with("--") => {
                return Err(format!("unknown option {flag}\n{USAGE}"))
            }
            file if path.is_none() => path = Some(PathBuf::from(file)),
            extra => return Err(format!("unexpected argument {extra}\n{USAGE}")),
        }
    }
    let path = path.ok_or_else(|| format!("missing <file>\n{USAGE}"))?;
    Ok(Command { path, format })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn check_with_and_without_format() {
        assert_eq!(
            parse(&args(&["check", "a.md"])),
            Ok(Command {
                path: "a.md".into(),
                format: Format::Text
            })
        );
        assert_eq!(
            parse(&args(&["check", "--format", "json", "a.md"])).map(|c| c.format),
            Ok(Format::Json)
        );
    }

    #[test]
    fn bad_arguments_give_the_usage() {
        for bad in [
            &[][..],
            &["lint", "a.md"][..],
            &["check"][..],
            &["check", "a.md", "b.md"][..],
            &["check", "a.md", "--format", "xml"][..],
            &["check", "a.md", "--verbose"][..],
        ] {
            let err = parse(&args(bad)).expect_err("refused");
            assert!(err.ends_with(USAGE), "{bad:?}: {err}");
        }
    }
}
