use std::fmt;
use std::fs;
use std::path::Path;

const DIRECTORY_TRACK: u8 = 18;
const DIRECTORY_SECTOR: u8 = 1;
const BAM_TRACK: u8 = 18;
const BAM_SECTOR: u8 = 0;
const SECTOR_SIZE: usize = 256;

#[derive(Debug)]
pub enum D64Error {
    Io(std::io::Error),
    UnsupportedSize(usize),
    InvalidTrackSector { track: u8, sector: u8 },
    ChainLoop { track: u8, sector: u8 },
}

impl fmt::Display for D64Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(err) => write!(f, "I/O error: {err}"),
            Self::UnsupportedSize(size) => write!(f, "unsupported D64 size: {size} bytes"),
            Self::InvalidTrackSector { track, sector } => {
                write!(f, "invalid track/sector: {track}/{sector}")
            }
            Self::ChainLoop { track, sector } => {
                write!(f, "track/sector chain loops at {track}/{sector}")
            }
        }
    }
}

impl std::error::Error for D64Error {}

impl From<std::io::Error> for D64Error {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileType {
    Del,
    Seq,
    Prg,
    Usr,
    Rel,
    Unknown(u8),
}

impl FileType {
    pub fn from_directory_byte(byte: u8) -> Self {
        match byte & 0x07 {
            0 => Self::Del,
            1 => Self::Seq,
            2 => Self::Prg,
            3 => Self::Usr,
            4 => Self::Rel,
            value => Self::Unknown(value),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Del => "DEL",
            Self::Seq => "SEQ",
            Self::Prg => "PRG",
            Self::Usr => "USR",
            Self::Rel => "REL",
            Self::Unknown(_) => "???",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectoryEntry {
    pub name: String,
    pub file_type: FileType,
    pub closed: bool,
    pub locked: bool,
    pub first_track: u8,
    pub first_sector: u8,
    pub blocks: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiskInfo {
    pub name: String,
    pub id: String,
    pub dos_type: String,
}

#[derive(Debug, Clone)]
pub struct D64Image {
    bytes: Vec<u8>,
}

impl D64Image {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, D64Error> {
        Self::from_bytes(fs::read(path)?)
    }

    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, D64Error> {
        if bytes.len() < standard_d64_len() || !bytes.len().is_multiple_of(SECTOR_SIZE) {
            return Err(D64Error::UnsupportedSize(bytes.len()));
        }
        Ok(Self { bytes })
    }

    pub fn directory(&self) -> Result<Vec<DirectoryEntry>, D64Error> {
        let mut entries = Vec::new();
        let mut seen = Vec::new();
        let mut track = DIRECTORY_TRACK;
        let mut sector = DIRECTORY_SECTOR;

        while track != 0 {
            if seen.contains(&(track, sector)) {
                return Err(D64Error::ChainLoop { track, sector });
            }
            seen.push((track, sector));

            let block = self.sector(track, sector)?;
            for slot in 0..8 {
                let offset = 2 + slot * 32;
                if offset + 30 > block.len() {
                    break;
                }
                let file_type_byte = block[offset];
                if file_type_byte == 0 {
                    continue;
                }
                entries.push(DirectoryEntry {
                    name: petscii_filename(&block[offset + 3..offset + 19]),
                    file_type: FileType::from_directory_byte(file_type_byte),
                    closed: file_type_byte & 0x80 != 0,
                    locked: file_type_byte & 0x40 != 0,
                    first_track: block[offset + 1],
                    first_sector: block[offset + 2],
                    blocks: u16::from_le_bytes([block[offset + 28], block[offset + 29]]),
                });
            }

            track = block[0];
            sector = block[1];
        }

        Ok(entries)
    }

    pub fn disk_info(&self) -> Result<DiskInfo, D64Error> {
        let block = self.sector(BAM_TRACK, BAM_SECTOR)?;
        Ok(DiskInfo {
            name: petscii_filename(&block[0x90..0xa0]),
            id: petscii_filename(&block[0xa2..0xa4]),
            dos_type: petscii_filename(&block[0xa5..0xa7]),
        })
    }

    pub fn read_file(&self, entry: &DirectoryEntry) -> Result<Vec<u8>, D64Error> {
        let mut data = Vec::new();
        let mut seen = Vec::new();
        let mut track = entry.first_track;
        let mut sector = entry.first_sector;

        while track != 0 {
            if seen.contains(&(track, sector)) {
                return Err(D64Error::ChainLoop { track, sector });
            }
            seen.push((track, sector));

            let block = self.sector(track, sector)?;
            let next_track = block[0];
            let next_sector = block[1];
            if next_track == 0 {
                let used = usize::from(next_sector).saturating_sub(1).min(254);
                data.extend_from_slice(&block[2..2 + used]);
            } else {
                data.extend_from_slice(&block[2..]);
            }
            track = next_track;
            sector = next_sector;
        }

        Ok(data)
    }

    pub fn sector(&self, track: u8, sector: u8) -> Result<&[u8], D64Error> {
        let offset = sector_offset(track, sector)?;
        let end = offset + SECTOR_SIZE;
        self.bytes
            .get(offset..end)
            .ok_or(D64Error::InvalidTrackSector { track, sector })
    }
}

pub fn sectors_on_track(track: u8) -> Option<u8> {
    match track {
        1..=17 => Some(21),
        18..=24 => Some(19),
        25..=30 => Some(18),
        31..=35 => Some(17),
        _ => None,
    }
}

pub fn sector_offset(track: u8, sector: u8) -> Result<usize, D64Error> {
    let sectors = sectors_on_track(track).ok_or(D64Error::InvalidTrackSector { track, sector })?;
    if sector >= sectors {
        return Err(D64Error::InvalidTrackSector { track, sector });
    }

    let prior_sectors: usize = (1..track)
        .map(|prior| usize::from(sectors_on_track(prior).expect("validated track range")))
        .sum();
    Ok((prior_sectors + usize::from(sector)) * SECTOR_SIZE)
}

fn standard_d64_len() -> usize {
    (1..=35)
        .map(|track| usize::from(sectors_on_track(track).expect("known track")))
        .sum::<usize>()
        * SECTOR_SIZE
}

fn petscii_filename(bytes: &[u8]) -> String {
    let mut out = String::new();
    for &byte in bytes {
        if byte == 0xa0 || byte == 0x00 {
            continue;
        }
        let ch = match byte {
            b'A'..=b'Z' | b'0'..=b'9' | b' ' | b'-' | b'_' | b'.' => byte as char,
            0xc1..=0xda => (byte - 0x80) as char,
            _ => '.',
        };
        out.push(ch);
    }
    out.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn computes_known_offsets() {
        assert_eq!(sector_offset(1, 0).unwrap(), 0);
        assert_eq!(sector_offset(18, 0).unwrap(), 357 * SECTOR_SIZE);
        assert!(sector_offset(36, 0).is_err());
    }
}
