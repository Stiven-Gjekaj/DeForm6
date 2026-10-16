//! The picture that `export-lift-builds` gives to a program that loads a
//! picture from the folder of its executable.
//!
//! Many programs of the corpus load `App.Path & "\sample.jpg"` when they
//! start. The corpus does not hold that file, so the original and the
//! rebuilt program both stopped with error 53, and a check of the screen
//! compared two error boxes. This module makes a picture of its own, with
//! no bytes from a third party, and the export writes it under each such
//! name that the source folder does not hold.
//!
//! The picture is a Windows bitmap. `LoadPicture` reads the format from the
//! bytes and not from the extension, so a bitmap named `sample.jpg` loads.

use std::collections::BTreeSet;
use std::path::Path;

/// The width of the picture in pixels. It is 256, so a column gives a byte.
const WIDTH: u32 = 256;

/// The height of the picture in pixels.
const HEIGHT: u32 = 192;

/// The extensions of a file that `LoadPicture` reads.
const PICTURE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "bmp", "gif"];

/// The extensions of a source file of Basic.
const SOURCE_EXTENSIONS: &[&str] = &["frm", "bas", "cls", "ctl"];

/// Gives each picture name that `text` loads from the folder of the
/// executable: the `NAME` of each `App.Path & "\NAME"` whose extension is
/// one of [`PICTURE_EXTENSIONS`]. A name in a subfolder is left out.
pub(crate) fn picture_names(text: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let lower = text.to_ascii_lowercase();
    let mut rest = lower.as_str();
    while let Some(at) = rest.find("app.path") {
        rest = rest
            .get(at.saturating_add("app.path".len())..)
            .unwrap_or_default();
        let tail = rest.trim_start();
        let Some(tail) = tail.strip_prefix('&') else {
            continue;
        };
        let Some(tail) = tail.trim_start().strip_prefix("\"\\") else {
            continue;
        };
        let Some(end) = tail.find('"') else {
            continue;
        };
        let name = tail.get(..end).unwrap_or_default();
        let picture = Path::new(name)
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| PICTURE_EXTENSIONS.contains(&ext));
        if picture && !name.is_empty() && !name.contains('\\') && !name.contains('/') {
            out.insert(name.to_owned());
        }
    }
    out
}

/// Gives each picture name that the source files in `dir` load and that
/// `dir` does not hold, in lower case.
pub(crate) fn missing_pictures(dir: &Path) -> Result<BTreeSet<String>, String> {
    let entries =
        std::fs::read_dir(dir).map_err(|err| format!("reading {}: {err}", dir.display()))?;
    let mut names = BTreeSet::new();
    let mut present = BTreeSet::new();
    for entry in entries {
        let path = entry
            .map_err(|err| format!("reading an entry of {}: {err}", dir.display()))?
            .path();
        if let Some(name) = path.file_name().and_then(|name| name.to_str()) {
            present.insert(name.to_ascii_lowercase());
        }
        let source = path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| SOURCE_EXTENSIONS.contains(&ext.to_ascii_lowercase().as_str()));
        if source && path.is_file() {
            let bytes =
                std::fs::read(&path).map_err(|err| format!("reading {}: {err}", path.display()))?;
            names.extend(picture_names(&String::from_utf8_lossy(&bytes)));
        }
    }
    Ok(names.difference(&present).cloned().collect())
}

/// Gives the bytes of the picture: a 24-bit Windows bitmap of [`WIDTH`] by
/// [`HEIGHT`] pixels. Red grows from left to right, green from top to
/// bottom, and blue makes squares of 32 pixels, so each filter of the corpus
/// changes many different colours.
pub(crate) fn sample_bitmap() -> Vec<u8> {
    let row = WIDTH.saturating_mul(3);
    let pixels = row.saturating_mul(HEIGHT);
    // The file header is 14 bytes and the information header 40.
    let offset: u32 = 54;
    let size = offset.saturating_add(pixels);
    let mut out = Vec::new();
    out.extend_from_slice(b"BM");
    out.extend_from_slice(&size.to_le_bytes());
    out.extend_from_slice(&[0; 4]);
    out.extend_from_slice(&offset.to_le_bytes());
    out.extend_from_slice(&40_u32.to_le_bytes());
    out.extend_from_slice(&WIDTH.to_le_bytes());
    out.extend_from_slice(&HEIGHT.to_le_bytes());
    out.extend_from_slice(&1_u16.to_le_bytes());
    out.extend_from_slice(&24_u16.to_le_bytes());
    out.extend_from_slice(&[0; 4]);
    out.extend_from_slice(&pixels.to_le_bytes());
    out.extend_from_slice(&[0; 16]);
    // A bitmap holds the bottom row first, and each pixel as blue, green
    // and red.
    for y in (0..HEIGHT).rev() {
        for x in 0..WIDTH {
            let red = u8::try_from(x).unwrap_or(u8::MAX);
            let green =
                u8::try_from(y.saturating_mul(4).checked_div(3).unwrap_or(0)).unwrap_or(u8::MAX);
            let square = (x.checked_shr(5).unwrap_or(0) ^ y.checked_shr(5).unwrap_or(0)) & 1;
            let blue = if square == 0 { 40 } else { 220 };
            out.extend_from_slice(&[blue, green, red]);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{HEIGHT, WIDTH, picture_names, sample_bitmap};

    #[test]
    fn a_picture_name_comes_from_the_folder_of_the_executable() {
        let text = concat!(
            "    srcFilePath = App.Path & \"\\sample.jpg\"\r\n",
            "    Open App.Path&\"\\Demo.map\" For Binary As #1\r\n",
            "    Set p = LoadPicture(App.Path &  \"\\Pics\\a.bmp\")\r\n",
            "    Set q = LoadPicture(APP.PATH & \"\\Back.GIF\")\r\n",
        );
        let names: Vec<String> = picture_names(text).into_iter().collect();
        assert_eq!(names, ["back.gif", "sample.jpg"]);
    }

    #[test]
    fn the_picture_is_a_bitmap_of_its_size() {
        let bytes = sample_bitmap();
        let word = |at: usize| {
            bytes
                .get(at..at.saturating_add(4))
                .and_then(|four| four.try_into().ok())
                .map(u32::from_le_bytes)
        };
        assert_eq!(bytes.get(..2), Some(&b"BM"[..]));
        assert_eq!(word(2), u32::try_from(bytes.len()).ok());
        assert_eq!(word(18), Some(WIDTH));
        assert_eq!(word(22), Some(HEIGHT));
        assert_eq!(bytes.get(28), Some(&24));
        // The first pixel of the file is the bottom left one: no red, the
        // most green, and the colour of its square.
        assert_eq!(bytes.get(54..57), Some(&[220, 254, 0][..]));
    }
}
