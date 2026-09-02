// SPDX-License-Identifier: MPL-2.0

use dho_core::{KOVS_HEADER_SIZE, KovsHeader, decode_kovs};
use serde::Serialize;
use std::error::Error;
use std::fmt;
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};

pub const AUDIO_PAGE_SIZE: usize = 40;
const CLIENT_AUDIO_DIRECTORY: &str = "0006";
const DEFAULT_AUDIO_SAMPLE_RATE: u32 = 44_100;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioCatalogSummary {
    pub track_count: usize,
    pub total_payload_bytes: u64,
    pub unrecognized_file_count: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioTrackItem {
    pub id: u32,
    pub file_name: String,
    pub payload_bytes: u32,
    pub loop_start_sample: u32,
    pub loop_start_seconds: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioTrackPage {
    pub query: String,
    pub offset: usize,
    pub page_size: usize,
    pub total_count: usize,
    pub items: Vec<AudioTrackItem>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AudioTrackOgg {
    pub item: AudioTrackItem,
    pub sample_rate: Option<u32>,
    pub channels: Option<u8>,
    pub ogg: Vec<u8>,
}

#[derive(Debug, Clone)]
struct AudioTrackEntry {
    item: AudioTrackItem,
    path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct AudioCatalog {
    entries: Vec<AudioTrackEntry>,
    unrecognized_file_count: usize,
}

impl AudioCatalog {
    pub fn load(game_directory: impl AsRef<Path>) -> Result<Self, AudioCatalogError> {
        let directory = game_directory.as_ref().join(CLIENT_AUDIO_DIRECTORY);
        let read_dir =
            fs::read_dir(&directory).map_err(|source| AudioCatalogError::ReadDirectory {
                path: directory.clone(),
                source,
            })?;
        let mut entries = Vec::new();
        let mut unrecognized_file_count = 0;
        for entry in read_dir {
            let entry = entry.map_err(|source| AudioCatalogError::ReadDirectory {
                path: directory.clone(),
                source,
            })?;
            let path = entry.path();
            if !path
                .extension()
                .and_then(|value| value.to_str())
                .is_some_and(|value| value.eq_ignore_ascii_case("bin"))
            {
                continue;
            }
            let Some(id) = path
                .file_stem()
                .and_then(|value| value.to_str())
                .filter(|value| value.len() == 6 && value.bytes().all(|byte| byte.is_ascii_digit()))
                .and_then(|value| value.parse::<u32>().ok())
            else {
                unrecognized_file_count += 1;
                continue;
            };
            let metadata = fs::metadata(&path).map_err(|source| AudioCatalogError::ReadFile {
                path: path.clone(),
                source,
            })?;
            let mut header_bytes = [0_u8; KOVS_HEADER_SIZE];
            let mut file = File::open(&path).map_err(|source| AudioCatalogError::ReadFile {
                path: path.clone(),
                source,
            })?;
            if file.read_exact(&mut header_bytes).is_err() || &header_bytes[..4] != b"KOVS" {
                unrecognized_file_count += 1;
                continue;
            }
            let header = KovsHeader::parse(&header_bytes, metadata.len()).map_err(|source| {
                AudioCatalogError::ParseFile {
                    path: path.clone(),
                    source: source.to_string(),
                }
            })?;
            entries.push(AudioTrackEntry {
                item: AudioTrackItem {
                    id,
                    file_name: format!("{id:06}.ogg"),
                    payload_bytes: header.payload_size,
                    loop_start_sample: header.loop_start_sample,
                    loop_start_seconds: header
                        .loop_start_seconds(DEFAULT_AUDIO_SAMPLE_RATE)
                        .unwrap_or(0.0),
                },
                path,
            });
        }
        entries.sort_by_key(|entry| entry.item.id);
        Ok(Self {
            entries,
            unrecognized_file_count,
        })
    }

    pub fn summary(&self) -> AudioCatalogSummary {
        AudioCatalogSummary {
            track_count: self.entries.len(),
            total_payload_bytes: self
                .entries
                .iter()
                .map(|entry| u64::from(entry.item.payload_bytes))
                .sum(),
            unrecognized_file_count: self.unrecognized_file_count,
        }
    }

    pub fn page(
        &self,
        query: &str,
        offset: usize,
        page_size: usize,
    ) -> Result<AudioTrackPage, AudioCatalogError> {
        let normalized = query.trim().to_ascii_lowercase();
        let matches = self.entries.iter().filter(|entry| {
            normalized.is_empty()
                || entry.item.id.to_string().contains(&normalized)
                || entry
                    .item
                    .file_name
                    .to_ascii_lowercase()
                    .contains(&normalized)
        });
        let total_count = matches.clone().count();
        if offset > 0 && offset >= total_count {
            return Err(AudioCatalogError::PageOutOfRange {
                offset,
                total_count,
            });
        }
        let items = matches
            .skip(offset)
            .take(page_size)
            .map(|entry| entry.item.clone())
            .collect();
        Ok(AudioTrackPage {
            query: query.trim().to_owned(),
            offset,
            page_size,
            total_count,
            items,
        })
    }

    pub fn ogg(&self, id: u32) -> Result<AudioTrackOgg, AudioCatalogError> {
        let entry = self
            .entries
            .iter()
            .find(|entry| entry.item.id == id)
            .ok_or(AudioCatalogError::TrackNotFound(id))?;
        let bytes = fs::read(&entry.path).map_err(|source| AudioCatalogError::ReadFile {
            path: entry.path.clone(),
            source,
        })?;
        let decoded = decode_kovs(&bytes).map_err(|source| AudioCatalogError::ParseFile {
            path: entry.path.clone(),
            source: source.to_string(),
        })?;
        let mut item = entry.item.clone();
        if let Some(sample_rate) = decoded.sample_rate {
            item.loop_start_seconds = decoded
                .header
                .loop_start_seconds(sample_rate)
                .unwrap_or(0.0);
        }
        Ok(AudioTrackOgg {
            item,
            sample_rate: decoded.sample_rate,
            channels: decoded.channels,
            ogg: decoded.ogg,
        })
    }
}

#[derive(Debug)]
pub enum AudioCatalogError {
    ReadDirectory { path: PathBuf, source: io::Error },
    ReadFile { path: PathBuf, source: io::Error },
    ParseFile { path: PathBuf, source: String },
    PageOutOfRange { offset: usize, total_count: usize },
    TrackNotFound(u32),
}

impl fmt::Display for AudioCatalogError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ReadDirectory { path, source } => {
                write!(
                    formatter,
                    "오디오 폴더를 읽지 못했습니다 ({}): {source}",
                    path.display()
                )
            }
            Self::ReadFile { path, source } => {
                write!(
                    formatter,
                    "오디오 파일을 읽지 못했습니다 ({}): {source}",
                    path.display()
                )
            }
            Self::ParseFile { path, source } => {
                write!(
                    formatter,
                    "오디오 파일을 해석하지 못했습니다 ({}): {source}",
                    path.display()
                )
            }
            Self::PageOutOfRange {
                offset,
                total_count,
            } => {
                write!(
                    formatter,
                    "오디오 시작 위치가 범위를 벗어났습니다: {offset}/{total_count}"
                )
            }
            Self::TrackNotFound(id) => write!(formatter, "오디오 ID {id}를 찾지 못했습니다."),
        }
    }
}

impl Error for AudioCatalogError {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn test_directory() -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        std::env::temp_dir().join(format!("dho-audio-catalog-{}-{nonce}", std::process::id()))
    }

    fn write_track(path: &Path, id: u32) {
        let mut ogg = vec![0_u8; 64];
        ogg[..4].copy_from_slice(b"OggS");
        for (index, byte) in ogg.iter_mut().enumerate() {
            *byte ^= index as u8;
        }
        let mut wrapped = vec![0_u8; KOVS_HEADER_SIZE];
        wrapped[..4].copy_from_slice(b"KOVS");
        wrapped[4..8].copy_from_slice(&(ogg.len() as u32).to_le_bytes());
        wrapped.extend(ogg);
        fs::write(path.join(format!("{id:06}.bin")), wrapped).expect("write track");
    }

    #[test]
    fn scans_pages_and_decodes_tracks_without_loading_unknown_files() {
        let root = test_directory();
        let audio = root.join(CLIENT_AUDIO_DIRECTORY);
        fs::create_dir_all(&audio).expect("create audio directory");
        write_track(&audio, 1);
        write_track(&audio, 42);
        fs::write(audio.join("001000.bin"), b"not audio").expect("write unknown");

        let catalog = AudioCatalog::load(&root).expect("load catalog");
        assert_eq!(catalog.summary().track_count, 2);
        assert_eq!(catalog.summary().unrecognized_file_count, 1);
        assert_eq!(catalog.page("42", 0, 40).expect("page").items[0].id, 42);
        assert!(catalog.ogg(1).expect("ogg").ogg.starts_with(b"OggS"));

        fs::remove_dir_all(root).expect("remove fixture");
    }

    #[test]
    #[ignore = "requires DHO_GAME_DIRECTORY pointing to an installed client"]
    fn decodes_every_audio_track_from_a_real_client() {
        let game_directory = std::env::var_os("DHO_GAME_DIRECTORY")
            .map(PathBuf::from)
            .expect("DHO_GAME_DIRECTORY");
        let catalog = AudioCatalog::load(game_directory).expect("load real audio catalog");
        assert!(!catalog.entries.is_empty());
        for entry in &catalog.entries {
            let track = catalog.ogg(entry.item.id).expect("decode real track");
            assert!(track.ogg.starts_with(b"OggS"));
            assert!(track.sample_rate.is_some());
            assert!(track.channels.is_some());
        }
    }
}
