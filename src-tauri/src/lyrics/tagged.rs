//! Lyrics kept inside the audio file itself: the `LYRICS` tag of a FLAC file's Vorbis comment
//! (LDDC and most taggers write word-timed LRC there), the `USLT` frame of an MP3, or the .lrc
//! file beside the track, which is easiest to fix by hand and so wins over the tag.
//!
//! Windows tells the island a track's title and artist but never its path, so the file is found by
//! name first (library.rs). Nothing here touches the network.

use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::Path,
};

/// Tags and .lrc files larger than this are not lyrics.
const MAX_BYTES: u64 = 1024 * 1024;
/// How many metadata blocks of a FLAC file are looked at.
const MAX_BLOCKS: usize = 64;
/// The tag names that hold lyrics, most specific first.
const LYRICS_TAGS: [&str; 4] = ["LYRICS", "SYNCEDLYRICS", "UNSYNCEDLYRICS", "UNSYNCED LYRICS"];

/// The lyrics of the track at `path`: the .lrc beside it when there is one, else the file's own
/// tag. A tag with nothing in it counts as no lyrics.
pub(super) fn lyrics_of(path: &Path) -> Option<String> {
    if let Some(text) = beside(path) {
        return Some(text);
    }
    let mut file = File::open(path).ok()?;
    let len = file.metadata().ok()?.len();
    let mut start = [0u8; 10];
    let read = read_at(&mut file, &mut start, 0, len)?;
    if read >= 4 && start[..4] == *b"fLaC" {
        return flac(&mut file, len);
    }
    if read >= 10 && start[..3] == *b"ID3" {
        return id3(&mut file, &start, len);
    }
    None
}

/// The .lrc file named after the track, which players and taggers write next to it.
fn beside(path: &Path) -> Option<String> {
    let lrc = path.with_extension("lrc");
    let meta = std::fs::metadata(&lrc).ok()?;
    if !meta.is_file() || meta.len() > MAX_BYTES {
        return None;
    }
    let text = decode(&std::fs::read(&lrc).ok()?);
    (!text.trim().is_empty()).then_some(text)
}

/// Some .lrc files and tags are saved with a byte order mark.
fn decode(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).trim_start_matches('\u{feff}').to_string()
}

/// Up to `buf.len()` bytes at `offset`, and how many were read; short only at the end of the file.
fn read_at(file: &mut File, buf: &mut [u8], offset: u64, len: u64) -> Option<usize> {
    if offset >= len {
        return Some(0);
    }
    file.seek(SeekFrom::Start(offset)).ok()?;
    let want = usize::try_from((len - offset).min(buf.len() as u64)).ok()?;
    let mut read = 0;
    while read < want {
        let step = file.read(&mut buf[read..want]).ok()?;
        if step == 0 {
            break;
        }
        read += step;
    }
    Some(read)
}

fn flac(file: &mut File, len: u64) -> Option<String> {
    let mut offset = 4u64;
    let mut header = [0u8; 4];
    for _ in 0..MAX_BLOCKS {
        if read_at(file, &mut header, offset, len)? < 4 {
            return None;
        }
        let last = header[0] & 0x80 != 0;
        let kind = header[0] & 0x7f;
        let size = u64::from(u32::from_be_bytes([0, header[1], header[2], header[3]]));
        if kind == 4 {
            if size > MAX_BYTES {
                return None;
            }
            let mut body = vec![0u8; usize::try_from(size).ok()?];
            if read_at(file, &mut body, offset + 4, len)? < body.len() {
                return None;
            }
            return vorbis(&body);
        }
        offset = offset.checked_add(4 + size)?;
        if last || offset >= len {
            return None;
        }
    }
    None
}

/// A Vorbis comment block: the vendor, then `NAME=value` entries.
fn vorbis(body: &[u8]) -> Option<String> {
    let mut pos = usize::try_from(le_u32(body, 0)?).ok()?.checked_add(4)?;
    let count = le_u32(body, pos)?;
    pos = pos.checked_add(4)?;
    let mut found: Vec<(usize, String)> = Vec::new();
    for _ in 0..count {
        let size = usize::try_from(le_u32(body, pos)?).ok()?;
        pos = pos.checked_add(4)?;
        let end = pos.checked_add(size)?;
        let entry = body.get(pos..end)?;
        pos = end;
        let text = String::from_utf8_lossy(entry);
        let Some((name, value)) = text.split_once('=') else {
            continue;
        };
        let name = name.trim().to_ascii_uppercase();
        if let Some(rank) = LYRICS_TAGS.iter().position(|tag| *tag == name) {
            found.push((rank, value.trim().to_string()));
        }
    }
    found.sort_by_key(|(rank, _)| *rank);
    found.into_iter().map(|(_, value)| value).find(|value| !value.is_empty())
}

fn le_u32(data: &[u8], at: usize) -> Option<u32> {
    let bytes = data.get(at..at.checked_add(4)?)?;
    Some(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

fn id3(file: &mut File, header: &[u8; 10], len: u64) -> Option<String> {
    // ID3v2.2 names frames with three letters; tags written since use v2.3 or v2.4.
    if header[3] < 3 {
        return None;
    }
    let size = syncsafe(header.get(6..10)?)?;
    let size = u64::from(size);
    if size == 0 || size > MAX_BYTES {
        return None;
    }
    let mut body = vec![0u8; usize::try_from(size).ok()?];
    if read_at(file, &mut body, 10, len)? < body.len() {
        return None;
    }
    let mut pos = 0usize;
    while let Some(frame) = body.get(pos..pos.checked_add(4)?) {
        if !frame.iter().all(|byte| byte.is_ascii_alphanumeric()) {
            break;
        }
        let name = std::str::from_utf8(frame).ok()?;
        let size = body.get(pos + 4..pos + 8)?;
        let frame_size = if header[3] >= 4 {
            usize::try_from(syncsafe(size)?).ok()?
        } else {
            usize::try_from(u32::from_be_bytes(<[u8; 4]>::try_from(size).ok()?)).ok()?
        };
        let start = pos + 10;
        let end = start.checked_add(frame_size)?;
        let frame_body = body.get(start..end)?;
        pos = end;
        if name == "USLT" {
            if let Some(text) = uslt(frame_body) {
                return Some(text);
            }
        }
    }
    None
}

fn syncsafe(bytes: &[u8]) -> Option<u32> {
    let [a, b, c, d] = <[u8; 4]>::try_from(bytes).ok()?;
    Some((u32::from(a & 0x7f) << 21) | (u32::from(b & 0x7f) << 14) | (u32::from(c & 0x7f) << 7) | u32::from(d & 0x7f))
}

/// An `USLT` frame: the encoding, a three-letter language, a description, then the words.
fn uslt(body: &[u8]) -> Option<String> {
    let (&encoding, rest) = body.split_first()?;
    let text = rest.get(3..)?;
    let after = descriptor_end(text, encoding)?;
    let words = decode_text(text.get(after..)?, encoding)?;
    let trimmed = words.trim().to_string();
    (!trimmed.is_empty()).then_some(trimmed)
}

/// Where a frame's description ends: two zero bytes in UTF-16, one in the single-byte encodings.
fn descriptor_end(text: &[u8], encoding: u8) -> Option<usize> {
    match encoding {
        1 | 2 => (0..text.len().saturating_sub(1))
            .step_by(2)
            .find(|at| text[*at] == 0 && text[at + 1] == 0)
            .map(|at| at + 2),
        _ => text.iter().position(|byte| *byte == 0).map(|at| at + 1),
    }
}

fn decode_text(bytes: &[u8], encoding: u8) -> Option<String> {
    let text = match encoding {
        0 => bytes.iter().map(|byte| char::from(*byte)).collect(),
        1 => {
            let (big_endian, start) = match bytes.get(..2)? {
                [0xff, 0xfe] => (false, 2),
                [0xfe, 0xff] => (true, 2),
                _ => (false, 0),
            };
            utf16(bytes.get(start..)?, big_endian)
        }
        2 => utf16(bytes, true),
        3 => String::from_utf8_lossy(bytes).to_string(),
        _ => return None,
    };
    Some(text.trim_start_matches('\u{feff}').to_string())
}

fn utf16(bytes: &[u8], big_endian: bool) -> String {
    let units: Vec<u16> = bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| if big_endian { u16::from_be_bytes(*pair) } else { u16::from_le_bytes(*pair) })
        .collect();
    String::from_utf16_lossy(&units)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn folder(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("focusd-tagged-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// A FLAC header: STREAMINFO, then one comment block (last) with an optional LYRICS tag.
    fn flac(dir: &Path, name: &str, tag: Option<&str>) -> PathBuf {
        let vendor = b"test";
        let mut body = Vec::new();
        body.extend_from_slice(&u32::try_from(vendor.len()).unwrap().to_le_bytes());
        body.extend_from_slice(vendor);
        let entry = tag.map(|lyrics| format!("LYRICS={lyrics}"));
        body.extend_from_slice(&u32::from(entry.is_some() as u8).to_le_bytes());
        if let Some(entry) = entry {
            body.extend_from_slice(&u32::try_from(entry.len()).unwrap().to_le_bytes());
            body.extend_from_slice(entry.as_bytes());
        }
        let mut file = b"fLaC".to_vec();
        file.extend_from_slice(&[0x00, 0, 0, 34]);
        file.extend_from_slice(&[0u8; 34]);
        file.push(0x84);
        file.extend_from_slice(&u32::try_from(body.len()).unwrap().to_be_bytes()[1..]);
        file.extend_from_slice(&body);
        let path = dir.join(name);
        std::fs::write(&path, file).unwrap();
        path
    }

    /// An MP3 with a single UTF-8 USLT frame.
    fn mp3(dir: &Path, name: &str, lyrics: &str) -> PathBuf {
        let mut frame = vec![3u8];
        frame.extend_from_slice(b"eng");
        frame.push(0);
        frame.extend_from_slice(lyrics.as_bytes());
        let mut body = b"USLT".to_vec();
        body.extend_from_slice(&u32::try_from(frame.len()).unwrap().to_be_bytes());
        body.extend_from_slice(&[0, 0]);
        body.extend_from_slice(&frame);
        let size = u32::try_from(body.len()).unwrap();
        let mut file = b"ID3".to_vec();
        file.extend_from_slice(&[3, 0, 0]);
        file.extend_from_slice(&[((size >> 21) & 0x7f) as u8, ((size >> 14) & 0x7f) as u8, ((size >> 7) & 0x7f) as u8, (size & 0x7f) as u8]);
        file.extend_from_slice(&body);
        let path = dir.join(name);
        std::fs::write(&path, file).unwrap();
        path
    }

    #[test]
    fn a_flac_comment_gives_its_lyrics_tag() {
        let dir = folder("flac");
        let path = flac(&dir, "track.flac", Some("[00:01.00]故事的小黄花"));
        assert_eq!(lyrics_of(&path).as_deref(), Some("[00:01.00]故事的小黄花"));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_tag_with_no_lyrics_reads_as_none() {
        let dir = folder("empty");
        assert_eq!(lyrics_of(&flac(&dir, "a.flac", None)), None);
        assert_eq!(lyrics_of(&mp3(&dir, "b.mp3", "  ")), None);
        assert_eq!(lyrics_of(&dir.join("missing.flac")), None);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn an_mp3_uslt_frame_gives_its_words_past_the_description() {
        let dir = folder("id3");
        let path = mp3(&dir, "track.mp3", "[00:02.00]窗外的麻雀");
        assert_eq!(lyrics_of(&path).as_deref(), Some("[00:02.00]窗外的麻雀"));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn the_lrc_file_beside_the_track_wins_over_its_tag() {
        let dir = folder("beside");
        let path = flac(&dir, "track.flac", Some("[00:01.00]tag"));
        std::fs::write(dir.join("track.lrc"), "\u{feff}[00:01.00]beside").unwrap();
        assert_eq!(lyrics_of(&path).as_deref(), Some("[00:01.00]beside"));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}

