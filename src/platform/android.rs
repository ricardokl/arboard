use std::borrow::Cow;
use std::io::{self, Write};
use std::process::{Command, Stdio};
use std::path::PathBuf;

use crate::common::{Error, ImageData};

fn termux_get() -> Command {
    if cfg!(test) {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/termux-clipboard-get.sh");
        Command::new(path)
    } else {
        Command::new("termux-clipboard-get")
    }
}

fn termux_set() -> Command {
    if cfg!(test) {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/termux-clipboard-set.sh");
        Command::new(path)
    } else {
        Command::new("termux-clipboard-set")
    }
}


pub struct Clipboard;

impl Clipboard {
	pub fn new() -> Result<Self, Error> {
		// Check for `termux-clipboard-get`
		match termux_get().spawn() {
			Ok(mut child) => {
				child.kill().map_err(|e| Error::unknown(format!("Failed to kill test process for 'termux-clipboard-get': {}", e)))?;
			}
			Err(e) => {
				if e.kind() == io::ErrorKind::NotFound {
					return Err(Error::unknown(
						"'termux-clipboard-get' command not found. Please install Termux:API.",
					));
				} else {
					return Err(Error::unknown(format!(
						"Error while testing for 'termux-clipboard-get': {}",
						e
					)));
				}
			}
		};

		// Check for `termux-clipboard-set`
		match termux_set().spawn() {
			Ok(mut child) => {
				child.kill().map_err(|e| Error::unknown(format!("Failed to kill test process for 'termux-clipboard-set': {}", e)))?;
			}
			Err(e) => {
				if e.kind() == io::ErrorKind::NotFound {
					return Err(Error::unknown(
						"'termux-clipboard-set' command not found. Please install Termux:API.",
					));
				} else {
					return Err(Error::unknown(format!(
						"Error while testing for 'termux-clipboard-set': {}",
						e
					)));
				}
			}
		};

		Ok(Clipboard)
	}
}

pub(crate) struct Get<'clipboard> {
	_clipboard: &'clipboard Clipboard,
}

impl<'clipboard> Get<'clipboard> {
	pub(crate) fn new(clipboard: &'clipboard mut Clipboard) -> Self {
		Self { _clipboard: clipboard }
	}

	pub(crate) fn text(self) -> Result<String, Error> {
		let output = termux_get().output().map_err(|e| {
			Error::unknown(format!("Failed to execute 'termux-clipboard-get': {}", e))
		})?;

		if !output.status.success() {
			return Err(Error::unknown(format!(
				"'termux-clipboard-get' exited with non-zero status: {}",
				String::from_utf8_lossy(&output.stderr)
			)));
		}

		String::from_utf8(output.stdout).map_err(|_| Error::ConversionFailure)
	}

    pub(crate) fn html(self) -> Result<String, Error> {
		Err(Error::ClipboardNotSupported)
	}

	#[cfg(feature = "image-data")]
	pub(crate) fn image(self) -> Result<ImageData<'static>, Error> {
		Err(Error::ClipboardNotSupported)
	}

    pub(crate) fn file_list(self) -> Result<Vec<PathBuf>, Error> {
        Err(Error::ClipboardNotSupported)
    }
}

pub(crate) struct Set<'clipboard> {
	_clipboard: &'clipboard mut Clipboard,
}

impl<'clipboard> Set<'clipboard> {
	pub(crate) fn new(clipboard: &'clipboard mut Clipboard) -> Self {
		Self { _clipboard: clipboard }
	}

	pub(crate) fn text(self, text: Cow<'_, str>) -> Result<(), Error> {
        let mut process = termux_set()
			.stdin(Stdio::piped())
			.spawn()
			.map_err(|e| Error::unknown(format!("Failed to execute 'termux-clipboard-set': {}", e)))?;

		if let Some(mut stdin) = process.stdin.take() {
			stdin
				.write_all(text.as_bytes())
				.map_err(|e| Error::unknown(format!("Failed to write to stdin of 'termux-clipboard-set': {}", e)))?;
		}

		let status = process
			.wait()
			.map_err(|e| Error::unknown(format!("Failed to wait for 'termux-clipboard-set': {}", e)))?;

		if !status.success() {
			return Err(Error::unknown(
				"'termux-clipboard-set' exited with non-zero status.",
			));
		}

		Ok(())
	}

    pub(crate) fn html(self, _html: Cow<'_, str>, _alt_text: Option<Cow<'_, str>>) -> Result<(), Error> {
		Err(Error::ClipboardNotSupported)
	}

	#[cfg(feature = "image-data")]
	pub(crate) fn image(self, _image: ImageData) -> Result<(), Error> {
		Err(Error::ClipboardNotSupported)
	}

    pub(crate) fn file_list(self, _file_list: &[impl AsRef<std::path::Path>]) -> Result<(), Error> {
        Err(Error::ClipboardNotSupported)
    }
}

pub(crate) struct Clear<'clipboard> {
	clipboard: &'clipboard mut Clipboard,
}

impl<'clipboard> Clear<'clipboard> {
	pub(crate) fn new(clipboard: &'clipboard mut Clipboard) -> Self {
		Self { clipboard }
	}

	pub(crate) fn clear(self) -> Result<(), Error> {
        Set::new(self.clipboard).text(Cow::from(""))
	}
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_text() {
        let mut clipboard = Clipboard::new().unwrap();
        let text = Get::new(&mut clipboard).text().unwrap();
        assert_eq!(text.trim(), "hello from mock clipboard");
    }

    #[test]
    fn test_set_text() {
        use std::fs;
        let mut clipboard = Clipboard::new().unwrap();
        let text = "hello to mock clipboard";
        Set::new(&mut clipboard).text(Cow::from(text)).unwrap();

        let content = fs::read_to_string("/tmp/arboard-test-clipboard").unwrap();
        assert_eq!(content, text);
    }

    #[test]
    fn test_clear() {
        use std::fs;
        let mut clipboard = Clipboard::new().unwrap();
        // first set some text
        let text = "some text";
        Set::new(&mut clipboard).text(Cow::from(text)).unwrap();
        let content = fs::read_to_string("/tmp/arboard-test-clipboard").unwrap();
        assert_eq!(content, text);
        // now clear
        Clear::new(&mut clipboard).clear().unwrap();
        let content = fs::read_to_string("/tmp/arboard-test-clipboard").unwrap();
        assert_eq!(content, "");
    }
}
