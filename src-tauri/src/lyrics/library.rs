//! The audio file a playing track names, so the lyrics it carries (tagged.rs) can be read. The
//! user points the island at a folder (Settings → Music → Lyrics); subfolders are searched, and a
//! file named "Artist - Title.ext" beats one named "Title.ext", the rule a .lrc folder follows
//! (local.rs). Windows gives no path, so a name is all a folder match can rest on.

use std::path::{Path, PathBuf};

use super::local::stem_score;

/// The extensions that may carry lyrics.
const AUDIO_EXTENSIONS: [&str; 7] = ["flac", "mp3", "m4a", "ogg", "opus", "wav", "ape"];
/// How deep into the folder subfolders are followed.
const MAX_DEPTH: usize = 6;
/// A tree bigger than this is not a music library; stop rather than walk all of it.
const MAX_ENTRIES: usize = 50_000;

/// The best-named audio file under `root` for the track, or none when no name matches it.
pub(super) fn find(root: &Path, title: &str, artist: &str) -> Option<PathBuf> {
    let mut best: Option<(u8, PathBuf)> = None;
    let mut seen = 0usize;
    let mut folders = vec![(root.to_path_buf(), 0usize)];
    while let Some((folder, depth)) = folders.pop() {
        let Ok(entries) = std::fs::read_dir(&folder) else {
            continue;
        };
        for entry in entries.flatten() {
            seen += 1;
            if seen > MAX_ENTRIES {
                return best.map(|(_, path)| path);
            }
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            let path = entry.path();
            if kind.is_dir() {
                if depth < MAX_DEPTH && !is_hidden(&path) {
                    folders.push((path, depth + 1));
                }
                continue;
            }
            if !is_audio(&path) {
                continue;
            }
            let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) else {
                continue;
            };
            let score = stem_score(stem, title, artist);
            if score > best.as_ref().map_or(0, |(best, _)| *best) {
                best = Some((score, path));
            }
        }
    }
    best.map(|(_, path)| path)
}

fn is_audio(path: &Path) -> bool {
    path.extension().is_some_and(|extension| AUDIO_EXTENSIONS.iter().any(|known| extension.eq_ignore_ascii_case(known)))
}

/// Folders whose name starts with a dot are skipped, as a music library rarely keeps songs there.
fn is_hidden(path: &Path) -> bool {
    path.file_name().is_some_and(|name| name.to_string_lossy().starts_with('.'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_named_file_wins_and_subfolders_are_searched() {
        let dir = std::env::temp_dir().join(format!("focusd-library-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("专辑")).unwrap();
        std::fs::write(dir.join("专辑").join("周杰伦 - 晴天.flac"), b"x").unwrap();
        std::fs::write(dir.join("晴天.flac"), b"x").unwrap();
        std::fs::write(dir.join("专辑").join("别人 - 晴天.flac"), b"x").unwrap();
        std::fs::write(dir.join("专辑").join("周杰伦 - 七里香.mp3"), b"x").unwrap();
        std::fs::write(dir.join("readme.txt"), b"x").unwrap();

        let found = find(&dir, "晴天", "周杰伦").expect("the named file");
        assert!(found.ends_with("周杰伦 - 晴天.flac"));
        assert!(find(&dir, "七里香", "周杰伦").is_some());
        assert_eq!(find(&dir, "不存在", "周杰伦"), None);
        assert_eq!(find(&dir.join("missing"), "晴天", "周杰伦"), None);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}

