use std::fmt;
use std::fs;
use std::path::Path;

const DIRECTORY_TRACK: u8 = 18;
const DIRECTORY_SECTOR: u8 = 1;
const BAM_TRACK: u8 = 18;
const BAM_SECTOR: u8 = 0;
const SECTOR_SIZE: usize = 256;

/// Builds synthetic .d64 images in memory for tests and fixtures.
/// No copyrighted content: only the bytes you put in.
pub struct D64Builder {
    bytes: Vec<u8>,
    directory_slots: Vec<(u8, u8, u8, String, u16)>, // type, t, s, name, blocks
}

impl Default for D64Builder {
    fn default() -> Self {
        Self::new()
    }
}

impl D64Builder {
    pub fn new() -> Self {
        Self {
            bytes: vec![0_u8; total_sectors(35).expect("known") * SECTOR_SIZE],
            directory_slots: Vec::new(),
        }
    }

    /// Write a file into the first free sectors and add a directory entry.
    /// `file_type`: 0x82 = closed PRG, 0x81 = closed SEQ, 0x01 = open SEQ.
    pub fn add_file(&mut self, name: &str, file_type: u8, data: &[u8]) -> Result<(), D64Error> {
        // Find the first unused sector chain (files share the same start if
        // we don't track allocation).
        let mut track = 5_u8;
        let mut sector = 0_u8;
        while self.is_sector_used(track, sector) {
            let next = self.next_free(track, sector)?;
            track = next.0;
            sector = next.1;
        }
        let first_ts = (track, sector);
        let mut offset = 0;
        let mut blocks = 0_u16;
        while offset < data.len() {
            let off = sector_offset(track, sector)?;
            let chunk = &data[offset..(offset + 250).min(data.len())];
            let is_last = offset + chunk.len() >= data.len();
            self.bytes[off] = 1; // mark used
            if is_last {
                self.bytes[off] = 0;
                self.bytes[off + 1] = (chunk.len() + 1) as u8;
            } else {
                let next = self.next_free(track, sector)?;
                track = next.0;
                sector = next.1;
                self.bytes[off] = track;
                self.bytes[off + 1] = sector;
            }
            self.bytes[off + 2..off + 2 + chunk.len()].copy_from_slice(chunk);
            offset += chunk.len();
            blocks += 1;
        }
        self.directory_slots
            .push((file_type, first_ts.0, first_ts.1, name.to_string(), blocks));
        Ok(())
    }

    fn is_sector_used(&self, track: u8, sector: u8) -> bool {
        let off = sector_offset(track, sector).expect("valid");
        self.bytes[off..off + SECTOR_SIZE].iter().any(|&b| b != 0)
    }

    fn next_free(&self, track: u8, sector: u8) -> Result<(u8, u8), D64Error> {
        let mut t = track;
        let mut s = sector;
        loop {
            let sectors = sectors_on_track(t).ok_or(D64Error::InvalidTrackSector {
                track: t,
                sector: s,
            })?;
            s += 1;
            if s >= sectors {
                t += 1;
                s = 0;
            }
            if t > 35 {
                return Err(D64Error::InvalidTrackSector {
                    track: t,
                    sector: s,
                });
            }
            if !self.is_sector_used(t, s) {
                return Ok((t, s));
            }
        }
    }

    /// Finalize: write the BAM and directory, return the image bytes.
    pub fn build(mut self) -> Vec<u8> {
        let bam_off = sector_offset(BAM_TRACK, BAM_SECTOR).expect("valid");
        for t in 1..=35_u8 {
            let sectors = sectors_on_track(t).expect("validated");
            let base = bam_off + (t as usize - 1) * 4;
            self.bytes[base] = t;
            self.bytes[base + 1] = sectors;
            self.bytes[base + 2] = 0xff;
            self.bytes[base + 3] = 0xff;
            let extra = sectors.saturating_sub(16);
            if extra > 0 {
                self.bytes[base + 4] = (0xff >> (8 - extra)) as u8;
            }
        }
        self.bytes[bam_off + 0x90..bam_off + 0x90 + 16].fill(0xa0);
        self.bytes[bam_off + 0x90..bam_off + 0x92].copy_from_slice(b"SY");
        self.bytes[bam_off + 0xa2] = b'0';
        self.bytes[bam_off + 0xa3] = b'0';
        self.bytes[bam_off + 0xa5] = b'2';
        self.bytes[bam_off + 0xa6] = b'A';

        let dir_off = sector_offset(DIRECTORY_TRACK, DIRECTORY_SECTOR).expect("valid");
        self.bytes[dir_off] = 0;
        self.bytes[dir_off + 1] = 0xff;
        for (slot_index, (file_type, t, s, name, blocks)) in self.directory_slots.iter().enumerate()
        {
            let slot = dir_off + 2 + slot_index * 32;
            if slot + 30 > self.bytes.len() {
                break;
            }
            self.bytes[slot] = *file_type;
            self.bytes[slot + 1] = *t;
            self.bytes[slot + 2] = *s;
            let name_bytes = name.as_bytes();
            let pad = name_bytes.len().min(16);
            self.bytes[slot + 3..slot + 3 + pad].copy_from_slice(&name_bytes[..pad]);
            for i in pad..16 {
                self.bytes[slot + 3 + i] = 0xa0;
            }
            self.bytes[slot + 28] = (blocks & 0xff) as u8;
            self.bytes[slot + 29] = (blocks >> 8) as u8;
        }
        self.bytes
    }
}

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
    tracks: u8,
}

/// Accepted image layouts: 35 or 40 tracks, optionally followed by the
/// per-sector error-info block (one byte per sector).
fn layout_for_len(len: usize) -> Option<u8> {
    for tracks in [35_u8, 40] {
        let sectors = total_sectors(tracks)?;
        let base = sectors * SECTOR_SIZE;
        if len == base || len == base + sectors {
            return Some(tracks);
        }
    }
    None
}

impl D64Image {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, D64Error> {
        Self::from_bytes(fs::read(path)?)
    }

    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, D64Error> {
        let Some(tracks) = layout_for_len(bytes.len()) else {
            return Err(D64Error::UnsupportedSize(bytes.len()));
        };
        Ok(Self { bytes, tracks })
    }

    pub fn tracks(&self) -> u8 {
        self.tracks
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
        36..=40 => Some(17),
        _ => None,
    }
}

fn total_sectors(tracks: u8) -> Option<usize> {
    let mut total = 0_usize;
    for track in 1..=tracks {
        total += usize::from(sectors_on_track(track)?);
    }
    Some(total)
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
        assert_eq!(sector_offset(36, 0).unwrap(), 683 * SECTOR_SIZE);
        assert!(sector_offset(41, 0).is_err());
    }

    #[test]
    fn accepts_standard_size_variants() {
        for len in [174_848, 175_531, 196_608, 197_376] {
            let image = D64Image::from_bytes(vec![0_u8; len]).expect("accepted size");
            let expected_tracks = if len <= 175_531 { 35 } else { 40 };
            assert_eq!(image.tracks(), expected_tracks, "len {len}");
        }
    }

    #[test]
    fn rejects_unknown_sizes() {
        assert!(D64Image::from_bytes(vec![0_u8; 100]).is_err());
        assert!(D64Image::from_bytes(vec![0_u8; 174_849]).is_err());
    }

    #[test]
    fn builder_round_trips_prg_and_seq() {
        let mut builder = D64Builder::new();
        // Tiny PRG with a BASIC SYS stub.
        let prg = {
            let mut data = vec![0x01, 0x08]; // load address $0801
            data.extend_from_slice(&[
                0x0b, 0x08, 0xca, 0x07, 0x9e, 0x32, 0x30, 0x36, 0x31, 0x00, 0x00, 0x00,
            ]);
            data
        };
        builder.add_file("TESTPROG", 0x82, &prg).unwrap();
        builder.add_file("NOTES", 0x81, b"hello world").unwrap();
        let image = D64Image::from_bytes(builder.build()).unwrap();
        assert_eq!(image.tracks(), 35);
        let dir = image.directory().unwrap();
        assert_eq!(dir.len(), 2);
        assert_eq!(dir[0].name, "TESTPROG");
        assert_eq!(dir[0].file_type, FileType::Prg);
        assert_eq!(dir[1].name, "NOTES");
        let loaded = image.read_file(&dir[0]).unwrap();
        assert_eq!(loaded, prg);
        let notes = image.read_file(&dir[1]).unwrap();
        assert_eq!(notes, b"hello world");
    }
}

/// Metadata derived from an extracted file's bytes.
#[derive(Debug, Clone)]
pub struct ExtractedFileMetadata {
    pub name: String,
    pub file_type: String,
    pub path: String,
    pub bytes: usize,
    pub load_address: Option<u16>,
    pub end_address_exclusive: Option<u16>,
    pub checksum16: u16,
    pub basic_sys: Option<u16>,
}

impl ExtractedFileMetadata {
    pub fn from_bytes(entry: &DirectoryEntry, path: String, bytes: &[u8]) -> Self {
        let load_address = (entry.file_type == FileType::Prg && bytes.len() >= 2)
            .then(|| u16::from_le_bytes([bytes[0], bytes[1]]));
        let payload_len = bytes.len().saturating_sub(2);
        let end_address_exclusive =
            load_address.map(|address| address.wrapping_add(payload_len as u16));
        let checksum16 = bytes.iter().fold(0_u16, |checksum, &byte| {
            checksum.wrapping_add(u16::from(byte))
        });
        let basic_sys = load_address.and_then(|address| detect_basic_sys(address, bytes));

        Self {
            name: entry.name.clone(),
            file_type: entry.file_type.as_str().to_string(),
            path,
            bytes: bytes.len(),
            load_address,
            end_address_exclusive,
            checksum16,
            basic_sys,
        }
    }
}

/// Load a PRG (with its 2-byte load address prefix) into RAM.
pub fn load_prg_into_ram(ram: &mut [u8], bytes: &[u8]) {
    if bytes.len() < 2 {
        return;
    }
    let load_address = usize::from(u16::from_le_bytes([bytes[0], bytes[1]]));
    let payload = &bytes[2..];
    let available = ram.len().saturating_sub(load_address);
    let len = payload.len().min(available);
    ram[load_address..load_address + len].copy_from_slice(&payload[..len]);
}

/// Detect a `SYS n` token in the first BASIC program line.
pub fn detect_basic_sys(load_address: u16, bytes: &[u8]) -> Option<u16> {
    if load_address != 0x0801 || bytes.len() < 6 {
        return None;
    }

    let mut offset = 2;
    while offset + 4 <= bytes.len() {
        let next_line = u16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
        if next_line == 0 {
            return None;
        }
        offset += 4;

        while offset < bytes.len() && bytes[offset] != 0 {
            if bytes[offset] == 0x9e {
                return parse_decimal_after_sys(&bytes[offset + 1..]);
            }
            offset += 1;
        }
        offset += 1;
    }

    None
}

fn parse_decimal_after_sys(bytes: &[u8]) -> Option<u16> {
    let mut value = 0_u16;
    let mut found_digit = false;
    for &byte in bytes {
        if byte == 0 || byte == b':' {
            break;
        }
        if byte.is_ascii_digit() {
            found_digit = true;
            value = value
                .saturating_mul(10)
                .saturating_add(u16::from(byte - b'0'));
        } else if found_digit {
            break;
        }
    }
    found_digit.then_some(value)
}

/// Make a PETSCII name safe for a filesystem path.
pub fn safe_filename(name: &str) -> String {
    let mut out = String::new();
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.' {
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        "unnamed".to_string()
    } else {
        out
    }
}
