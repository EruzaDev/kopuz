//! The command line, which only says how the app starts.

use std::ffi::{OsStr, OsString};
use std::path::PathBuf;

/// What the command line asked for.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Args {
    /// Enter offline mode once the daemon answers.
    pub offline: bool,
    /// A `kopuz://` link the OS opened this app with.
    pub link: Option<String>,
    /// Local audio files passed on the command line or as `file://` URIs.
    pub files: Vec<PathBuf>,
}

impl Args {
    /// Read the arguments after the program name; an unknown one is said and skipped, not fatal.
    pub fn parse(args: impl IntoIterator<Item = OsString>) -> Self {
        let mut parsed = Self::default();
        for arg in args {
            match arg.to_str() {
                Some("--offline") => parsed.offline = true,
                Some(link) if is_link(link) => parsed.link = Some(link.to_string()),
                Some(uri) if is_file_url(uri) => {
                    if let Some(path) = parse_file_url(uri) {
                        parsed.files.push(path);
                    } else {
                        tracing::warn!(uri, "invalid file URI; ignored");
                    }
                }
                Some(flag) if flag.starts_with('-') || flag.contains("://") => {
                    tracing::warn!(flag, "unknown argument; ignored");
                }
                _ => {
                    if let Some(path) = parse_local_path(&arg) {
                        parsed.files.push(path);
                    } else {
                        tracing::warn!(?arg, "unknown argument; ignored");
                    }
                }
            }
        }
        parsed
    }
}

/// A URL this app registers to open.
pub fn is_link(arg: &str) -> bool {
    arg.get(..8)
        .is_some_and(|scheme| scheme.eq_ignore_ascii_case("kopuz://"))
}

/// A `file://` URI handed over by a file manager or desktop launcher.
pub fn is_file_url(arg: &str) -> bool {
    arg.get(..7)
        .is_some_and(|scheme| scheme.eq_ignore_ascii_case("file://"))
}

/// Decode a `file://` URI into a local filesystem path.
pub fn parse_file_url(arg: &str) -> Option<PathBuf> {
    if !is_file_url(arg) {
        return None;
    }
    url::Url::parse(arg).ok()?.to_file_path().ok()
}

fn parse_local_path(arg: &OsStr) -> Option<PathBuf> {
    if arg.is_empty() {
        return None;
    }
    let path = PathBuf::from(arg);
    if path.is_absolute() {
        return Some(path);
    }
    Some(
        std::env::current_dir()
            .map(|cwd| cwd.join(&path))
            .unwrap_or(path),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Args {
        Args::parse(args.iter().map(OsString::from))
    }

    #[test]
    fn offline_is_asked_for_only_by_its_flag() {
        assert_eq!(parse(&[]), Args::default());
        assert!(parse(&["--offline"]).offline);
        assert!(parse(&["--verbose", "--offline"]).offline);
        assert!(!parse(&["--offline=no", "offline"]).offline);
    }

    #[test]
    fn a_kopuz_link_is_kept_and_any_other_url_is_not() {
        let link = "kopuz://j/1/ytmusic/p0/abc";
        assert_eq!(parse(&[link]).link.as_deref(), Some(link));
        assert_eq!(parse(&["KOPUZ://j/x"]).link.as_deref(), Some("KOPUZ://j/x"));
        assert_eq!(parse(&["https://kopuz.moe/j#1"]).link, None);
        assert!(parse(&["https://kopuz.moe/j#1"]).files.is_empty());
        assert_eq!(parse(&["kopuz:"]).link, None);
    }

    #[test]
    fn file_paths_and_file_uris_are_parsed() {
        let direct = std::env::temp_dir().join("song.flac");
        let via_uri = std::env::temp_dir().join("My Track.mp3");
        let file_uri = url::Url::from_file_path(&via_uri).unwrap().to_string();
        let parsed = Args::parse([OsString::from(&direct), OsString::from(file_uri)]);
        assert_eq!(parsed.files, vec![direct, via_uri]);
    }
}
