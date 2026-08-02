use std::fmt;
use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

const STX: u8 = 0x02;
const API_VERSION: u8 = 0x02;
const EVENT_REQUEST_ID: u32 = 0xffff_ffff;

#[derive(Debug)]
pub enum MonitorError {
    Io(std::io::Error),
    Protocol(String),
    ResponseError { response_type: u8, code: ErrorCode },
}

impl fmt::Display for MonitorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(err) => write!(f, "monitor I/O error: {err}"),
            Self::Protocol(message) => write!(f, "monitor protocol error: {message}"),
            Self::ResponseError {
                response_type,
                code,
            } => write!(
                f,
                "monitor response 0x{response_type:02x} returned error {code:?}"
            ),
        }
    }
}

impl std::error::Error for MonitorError {}

impl From<std::io::Error> for MonitorError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

pub type Result<T> = std::result::Result<T, MonitorError>;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CpuRegisters {
    pub pc: u16,
    pub a: u8,
    pub x: u8,
    pub y: u8,
    pub sp: u8,
    pub status: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Memspace {
    Main = 0x00,
    Drive8 = 0x01,
    Drive9 = 0x02,
    Drive10 = 0x03,
    Drive11 = 0x04,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WatchMode {
    Read,
    Write,
    Execute,
    ReadWrite,
}

impl WatchMode {
    fn operation_mask(self) -> u8 {
        match self {
            Self::Read => 0x01,
            Self::Write => 0x02,
            Self::Execute => 0x04,
            Self::ReadWrite => 0x03,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CheckpointId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopReason {
    Running,
    Stopped(u16),
    Breakpoint(CheckpointId),
    Watchpoint(CheckpointId),
    Jam,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    Ok,
    ObjectNotFound,
    InvalidMemspace,
    BadCommandLength,
    InvalidParameter,
    UnsupportedApiVersion,
    UnknownCommand,
    GeneralFailure,
    Other(u8),
}

impl ErrorCode {
    fn from_byte(byte: u8) -> Self {
        match byte {
            0x00 => Self::Ok,
            0x01 => Self::ObjectNotFound,
            0x02 => Self::InvalidMemspace,
            0x80 => Self::BadCommandLength,
            0x81 => Self::InvalidParameter,
            0x82 => Self::UnsupportedApiVersion,
            0x83 => Self::UnknownCommand,
            0x8f => Self::GeneralFailure,
            value => Self::Other(value),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandType {
    MemoryGet = 0x01,
    MemorySet = 0x02,
    CheckpointGet = 0x11,
    CheckpointSet = 0x12,
    CheckpointDelete = 0x13,
    ConditionSet = 0x22,
    RegistersGet = 0x31,
    RegistersSet = 0x32,
    Dump = 0x41,
    Undump = 0x42,
    ResourceGet = 0x51,
    ResourceSet = 0x52,
    AdvanceInstructions = 0x71,
    KeyboardFeed = 0x72,
    ExecuteUntilReturn = 0x73,
    Ping = 0x81,
    BanksAvailable = 0x82,
    RegistersAvailable = 0x83,
    DisplayGet = 0x84,
    CpuHistory = 0x86,
    PaletteGet = 0x91,
    JoyportSet = 0xa2,
    Exit = 0xaa,
    Quit = 0xbb,
    Reset = 0xcc,
    Autostart = 0xdd,
}

/// Response types that can arrive as asynchronous events (request_id = 0xffff_ffff).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventType {
    Stopped(u16),
    Jam,
    Resumed(u16),
    CheckpointInfo,
    RegisterInfo,
    Other(u8),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandPacket {
    pub request_id: u32,
    pub command_type: u8,
    pub body: Vec<u8>,
}

impl CommandPacket {
    pub fn new(request_id: u32, command_type: CommandType, body: Vec<u8>) -> Self {
        Self {
            request_id,
            command_type: command_type as u8,
            body,
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(11 + self.body.len());
        bytes.push(STX);
        bytes.push(API_VERSION);
        bytes.extend_from_slice(&(self.body.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&self.request_id.to_le_bytes());
        bytes.push(self.command_type);
        bytes.extend_from_slice(&self.body);
        bytes
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResponsePacket {
    pub response_type: u8,
    pub error_code: ErrorCode,
    pub request_id: u32,
    pub body: Vec<u8>,
}

impl ResponsePacket {
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < 12 {
            return Err(MonitorError::Protocol(
                "response is shorter than header".to_string(),
            ));
        }
        if bytes[0] != STX {
            return Err(MonitorError::Protocol(
                "response missing STX prefix".to_string(),
            ));
        }
        if bytes[1] != API_VERSION {
            return Err(MonitorError::Protocol(format!(
                "unsupported response API version: {}",
                bytes[1]
            )));
        }

        let body_len = u32::from_le_bytes([bytes[2], bytes[3], bytes[4], bytes[5]]) as usize;
        let expected_len = 12 + body_len;
        if bytes.len() != expected_len {
            return Err(MonitorError::Protocol(format!(
                "response length mismatch: header says {expected_len}, got {}",
                bytes.len()
            )));
        }

        Ok(Self {
            response_type: bytes[6],
            error_code: ErrorCode::from_byte(bytes[7]),
            request_id: u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]),
            body: bytes[12..].to_vec(),
        })
    }

    pub fn is_event(&self) -> bool {
        self.request_id == EVENT_REQUEST_ID
    }

    /// Classify an event packet (STOPPED 0x62 / JAM 0x61 / RESUMED 0x63).
    pub fn event_type(&self) -> Option<EventType> {
        if !self.is_event() {
            return None;
        }
        let pc = |body: &[u8]| {
            u16::from_le_bytes([
                body.first().copied().unwrap_or(0),
                body.get(1).copied().unwrap_or(0),
            ])
        };
        Some(match self.response_type {
            0x62 => EventType::Stopped(pc(&self.body)),
            0x61 => EventType::Jam,
            0x63 => EventType::Resumed(pc(&self.body)),
            0x11 => EventType::CheckpointInfo,
            0x31 => EventType::RegisterInfo,
            other => EventType::Other(other),
        })
    }

    fn ensure_ok(&self) -> Result<()> {
        if self.error_code == ErrorCode::Ok {
            Ok(())
        } else {
            Err(MonitorError::ResponseError {
                response_type: self.response_type,
                code: self.error_code,
            })
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RegisterValue {
    pub id: u8,
    pub value: u16,
}

pub struct ViceMonitor {
    stream: TcpStream,
    next_request_id: u32,
    pending_events: Vec<ResponsePacket>,
}

impl ViceMonitor {
    pub fn connect(addr: impl ToSocketAddrs) -> Result<Self> {
        Ok(Self {
            stream: TcpStream::connect(addr)?,
            next_request_id: 1,
            pending_events: Vec::new(),
        })
    }

    pub fn ping(&mut self) -> Result<()> {
        let response = self.send_command(CommandType::Ping, Vec::new())?;
        response.ensure_ok()
    }

    pub fn reset(&mut self) -> Result<()> {
        let response = self.send_command(CommandType::Reset, vec![0x00])?;
        response.ensure_ok()
    }

    pub fn power_cycle(&mut self) -> Result<()> {
        let response = self.send_command(CommandType::Reset, vec![0x01])?;
        response.ensure_ok()
    }

    pub fn quit(&mut self) -> Result<()> {
        let response = self.send_command(CommandType::Quit, Vec::new())?;
        response.ensure_ok()
    }

    pub fn continue_run(&mut self) -> Result<()> {
        let response = self.send_command(CommandType::Exit, Vec::new())?;
        response.ensure_ok()
    }

    pub fn step_instructions(&mut self, count: u16, step_over: bool) -> Result<()> {
        let mut body = vec![u8::from(step_over)];
        body.extend_from_slice(&count.to_le_bytes());
        let response = self.send_command(CommandType::AdvanceInstructions, body)?;
        response.ensure_ok()
    }

    pub fn read_memory(&mut self, start: u16, end: u16) -> Result<Vec<u8>> {
        self.read_memory_in(Memspace::Main, start, end, false, 0)
    }

    pub fn read_memory_in(
        &mut self,
        memspace: Memspace,
        start: u16,
        end: u16,
        side_effects: bool,
        bank_id: u16,
    ) -> Result<Vec<u8>> {
        let mut body = vec![u8::from(side_effects)];
        body.extend_from_slice(&start.to_le_bytes());
        body.extend_from_slice(&end.to_le_bytes());
        body.push(memspace as u8);
        body.extend_from_slice(&bank_id.to_le_bytes());

        let response = self.send_command(CommandType::MemoryGet, body)?;
        response.ensure_ok()?;
        parse_memory_get_body(&response.body)
    }

    pub fn write_memory(&mut self, start: u16, bytes: &[u8]) -> Result<()> {
        if bytes.is_empty() {
            return Ok(());
        }
        let end = start.wrapping_add(bytes.len() as u16).wrapping_sub(1);
        let mut body = vec![0x00];
        body.extend_from_slice(&start.to_le_bytes());
        body.extend_from_slice(&end.to_le_bytes());
        body.push(Memspace::Main as u8);
        body.extend_from_slice(&0_u16.to_le_bytes());
        body.extend_from_slice(bytes);

        let response = self.send_command(CommandType::MemorySet, body)?;
        response.ensure_ok()
    }

    pub fn registers_raw(&mut self) -> Result<Vec<RegisterValue>> {
        let response = self.send_command(CommandType::RegistersGet, vec![Memspace::Main as u8])?;
        response.ensure_ok()?;
        parse_registers_body(&response.body)
    }

    pub fn registers(&mut self) -> Result<CpuRegisters> {
        let registers = self.registers_raw()?;
        Ok(best_effort_cpu_registers(&registers))
    }

    pub fn set_registers_raw(&mut self, registers: &[RegisterValue]) -> Result<()> {
        let mut body = vec![Memspace::Main as u8];
        body.extend_from_slice(&(registers.len() as u16).to_le_bytes());
        for register in registers {
            body.push(0x03);
            body.push(register.id);
            body.extend_from_slice(&register.value.to_le_bytes());
        }
        let response = self.send_command(CommandType::RegistersSet, body)?;
        response.ensure_ok()
    }

    pub fn breakpoint_set(&mut self, address: u16) -> Result<CheckpointId> {
        self.checkpoint_set(address, address, WatchMode::Execute, true, true, false)
    }

    pub fn watchpoint_set(
        &mut self,
        start: u16,
        end: u16,
        mode: WatchMode,
    ) -> Result<CheckpointId> {
        self.checkpoint_set(start, end, mode, true, true, false)
    }

    pub fn checkpoint_delete(&mut self, id: CheckpointId) -> Result<()> {
        let response =
            self.send_command(CommandType::CheckpointDelete, id.0.to_le_bytes().to_vec())?;
        response.ensure_ok()
    }

    pub fn joyport_set(&mut self, port: u16, value: u16) -> Result<()> {
        let mut body = Vec::with_capacity(4);
        body.extend_from_slice(&port.to_le_bytes());
        body.extend_from_slice(&value.to_le_bytes());
        let response = self.send_command(CommandType::JoyportSet, body)?;
        response.ensure_ok()
    }

    pub fn keyboard_feed(&mut self, petscii: &[u8]) -> Result<()> {
        if petscii.len() > u8::MAX as usize {
            return Err(MonitorError::Protocol(
                "keyboard feed is limited to 255 bytes".to_string(),
            ));
        }
        let mut body = vec![petscii.len() as u8];
        body.extend_from_slice(petscii);
        let response = self.send_command(CommandType::KeyboardFeed, body)?;
        response.ensure_ok()
    }

    pub fn autostart(&mut self, filename: &str, run: bool, file_index: u16) -> Result<()> {
        let filename = filename.as_bytes();
        if filename.len() > u8::MAX as usize {
            return Err(MonitorError::Protocol(
                "filename is limited to 255 bytes".to_string(),
            ));
        }
        let mut body = vec![u8::from(run)];
        body.extend_from_slice(&file_index.to_le_bytes());
        body.push(filename.len() as u8);
        body.extend_from_slice(filename);
        let response = self.send_command(CommandType::Autostart, body)?;
        response.ensure_ok()
    }

    pub fn read_response(&mut self) -> Result<ResponsePacket> {
        let mut header = [0_u8; 12];
        self.stream.read_exact(&mut header)?;
        let body_len = u32::from_le_bytes([header[2], header[3], header[4], header[5]]) as usize;
        let mut bytes = header.to_vec();
        bytes.resize(12 + body_len, 0);
        self.stream.read_exact(&mut bytes[12..])?;
        ResponsePacket::decode(&bytes)
    }

    /// Dump the machine state to a snapshot file on the VICE host (0x41).
    pub fn dump(&mut self, filename: &str, save_roms: bool, save_disks: bool) -> Result<()> {
        if filename.len() > u8::MAX as usize {
            return Err(MonitorError::Protocol(
                "dump filename is limited to 255 bytes".to_string(),
            ));
        }
        let mut body = vec![
            u8::from(save_roms),
            u8::from(save_disks),
            filename.len() as u8,
        ];
        body.extend_from_slice(filename.as_bytes());
        let response = self.send_command(CommandType::Dump, body)?;
        response.ensure_ok()
    }

    /// Restore a snapshot from a file on the VICE host (0x42).
    /// Returns the PC after restore.
    pub fn undump(&mut self, filename: &str) -> Result<u16> {
        if filename.len() > u8::MAX as usize {
            return Err(MonitorError::Protocol(
                "undump filename is limited to 255 bytes".to_string(),
            ));
        }
        let mut body = vec![filename.len() as u8];
        body.extend_from_slice(filename.as_bytes());
        let response = self.send_command(CommandType::Undump, body)?;
        response.ensure_ok()?;
        Ok(u16::from_le_bytes([
            response.body.first().copied().unwrap_or(0),
            response.body.get(1).copied().unwrap_or(0),
        ]))
    }

    /// Fetch a screenshot from VICE as an indexed-8 display buffer (0x84).
    pub fn display_get(&mut self, use_vic: bool) -> Result<DisplayImage> {
        let body = vec![u8::from(use_vic), 0x00]; // INDEXED8
        let response = self.send_command(CommandType::DisplayGet, body)?;
        response.ensure_ok()?;
        parse_display_image(&response.body)
    }

    /// List memory banks available for the main memspace (0x82).
    pub fn banks_available(&mut self) -> Result<Vec<BankInfo>> {
        let response = self.send_command(CommandType::BanksAvailable, Vec::new())?;
        response.ensure_ok()?;
        parse_bank_list(&response.body)
    }

    /// List register definitions for the main memspace (0x83).
    pub fn registers_available(&mut self) -> Result<Vec<RegisterInfo>> {
        let response =
            self.send_command(CommandType::RegistersAvailable, vec![Memspace::Main as u8])?;
        response.ensure_ok()?;
        parse_register_list(&response.body)
    }

    /// Fetch hit statistics for a checkpoint (0x11).
    pub fn checkpoint_get(&mut self, id: CheckpointId) -> Result<CheckpointInfo> {
        let response =
            self.send_command(CommandType::CheckpointGet, id.0.to_le_bytes().to_vec())?;
        response.ensure_ok()?;
        parse_checkpoint_info(&response.body)
    }

    /// Attach a text-monitor condition to a checkpoint (0x22).
    /// Condition uses monitor expression syntax, e.g. `raster == 100`.
    pub fn condition_set(&mut self, id: CheckpointId, condition: &str) -> Result<()> {
        if condition.len() > u8::MAX as usize {
            return Err(MonitorError::Protocol(
                "condition is limited to 255 bytes".to_string(),
            ));
        }
        let mut body = Vec::with_capacity(5 + condition.len());
        body.extend_from_slice(&id.0.to_le_bytes());
        body.push(condition.len() as u8);
        body.extend_from_slice(condition.as_bytes());
        let response = self.send_command(CommandType::ConditionSet, body)?;
        response.ensure_ok()
    }

    /// Fetch the CPU instruction history (0x86). `count` entries, most recent last.
    pub fn cpu_history(&mut self, count: u32) -> Result<Vec<CpuHistoryEntry>> {
        let mut body = vec![Memspace::Main as u8];
        body.extend_from_slice(&count.to_le_bytes());
        let response = self.send_command(CommandType::CpuHistory, body)?;
        response.ensure_ok()?;
        parse_cpu_history(&response.body)
    }

    /// Read a VICE resource by name (0x51). Returns the raw value bytes.
    pub fn resource_get(&mut self, name: &str) -> Result<ResourceValue> {
        if name.len() > u8::MAX as usize || name.is_empty() {
            return Err(MonitorError::Protocol(
                "resource name must be 1-255 bytes".to_string(),
            ));
        }
        let mut body = vec![name.len() as u8];
        body.extend_from_slice(name.as_bytes());
        let response = self.send_command(CommandType::ResourceGet, body)?;
        response.ensure_ok()?;
        parse_resource_value(&response.body)
    }

    /// Set a VICE resource by name (0x52). Strings or ints (1/2/4 bytes LE).
    pub fn resource_set(&mut self, name: &str, value: &ResourceValue) -> Result<()> {
        if name.len() > u8::MAX as usize || name.is_empty() {
            return Err(MonitorError::Protocol(
                "resource name must be 1-255 bytes".to_string(),
            ));
        }
        let mut body = vec![value.value_type()];
        body.push(name.len() as u8);
        body.extend_from_slice(name.as_bytes());
        match value {
            ResourceValue::String(bytes) => {
                if bytes.len() > u8::MAX as usize {
                    return Err(MonitorError::Protocol(
                        "resource string value is limited to 255 bytes".to_string(),
                    ));
                }
                body.push(bytes.len() as u8);
                body.extend_from_slice(bytes);
            }
            ResourceValue::Int(value) => {
                body.extend_from_slice(&value.to_le_bytes());
            }
        }
        let response = self.send_command(CommandType::ResourceSet, body)?;
        response.ensure_ok()
    }

    fn checkpoint_set(
        &mut self,
        start: u16,
        end: u16,
        mode: WatchMode,
        stop: bool,
        enabled: bool,
        temporary: bool,
    ) -> Result<CheckpointId> {
        let mut body = Vec::with_capacity(9);
        body.extend_from_slice(&start.to_le_bytes());
        body.extend_from_slice(&end.to_le_bytes());
        body.push(u8::from(stop));
        body.push(u8::from(enabled));
        body.push(mode.operation_mask());
        body.push(u8::from(temporary));
        body.push(Memspace::Main as u8);
        let response = self.send_command(CommandType::CheckpointSet, body)?;
        response.ensure_ok()?;
        parse_checkpoint_id(&response.body)
    }

    fn send_command(&mut self, command_type: CommandType, body: Vec<u8>) -> Result<ResponsePacket> {
        self.send_command_raw(command_type, body)
    }

    /// Send a raw command and return the raw response, queuing any events.
    pub fn send_command_raw(
        &mut self,
        command_type: CommandType,
        body: Vec<u8>,
    ) -> Result<ResponsePacket> {
        let request_id = self.next_id();
        let packet = CommandPacket::new(request_id, command_type, body).encode();
        self.stream.write_all(&packet)?;
        self.stream.flush()?;

        loop {
            let response = self.read_response()?;
            if response.is_event() {
                self.pending_events.push(response);
                continue;
            }
            if response.request_id == request_id {
                return Ok(response);
            }
            return Err(MonitorError::Protocol(format!(
                "response request_id {} does not match request {request_id}",
                response.request_id
            )));
        }
    }

    /// Returns any event packets (checkpoint hits, stop/resume notices) that
    /// arrived while waiting for command responses. Previously these were
    /// silently dropped.
    pub fn drain_events(&mut self) -> Vec<ResponsePacket> {
        std::mem::take(&mut self.pending_events)
    }

    /// Resume execution and wait for the next STOPPED event (a checkpoint
    /// hit or a breakpoint), collecting any other events into the queue.
    ///
    /// Discards any events queued *before* the resume (notably the STOPPED
    /// event VICE sends on connect), so only a stop caused by execution
    /// after this call is returned.
    pub fn continue_to_stop(&mut self) -> Result<StopReason> {
        self.drain_events();
        self.continue_run()?;
        self.wait_for_stop()
    }

    /// Wait for the next STOPPED/JAM event, reading packets until one arrives.
    pub fn wait_for_stop(&mut self) -> Result<StopReason> {
        for event in self.drain_events() {
            match event.event_type() {
                Some(EventType::Stopped(pc)) => return Ok(StopReason::Stopped(pc)),
                Some(EventType::Jam) => return Ok(StopReason::Jam),
                _ => {}
            }
        }
        loop {
            let response = self.read_response()?;
            if response.is_event() {
                match response.event_type() {
                    Some(EventType::Stopped(pc)) => return Ok(StopReason::Stopped(pc)),
                    Some(EventType::Jam) => return Ok(StopReason::Jam),
                    Some(EventType::Resumed(_)) => {
                        // VICE sends RESUMED when the monitor closes; ignore.
                    }
                    _ => self.pending_events.push(response),
                }
            } else {
                // A response for a command we already consumed; queue it.
                self.pending_events.push(response);
            }
        }
    }

    /// Set a read timeout on the underlying socket so a hung emulator
    /// surfaces as a `MonitorError::Io` instead of blocking forever.
    pub fn set_read_timeout(&mut self, timeout: Duration) -> Result<()> {
        self.stream
            .set_read_timeout(Some(timeout))
            .map_err(MonitorError::Io)
    }

    fn next_id(&mut self) -> u32 {
        let id = self.next_request_id;
        self.next_request_id = self.next_request_id.wrapping_add(1).max(1);
        id
    }
}

fn parse_memory_get_body(body: &[u8]) -> Result<Vec<u8>> {
    if body.len() < 2 {
        return Err(MonitorError::Protocol(
            "memory response body is too short".to_string(),
        ));
    }
    let declared = u16::from_le_bytes([body[0], body[1]]) as usize;
    let data = &body[2..];
    if declared != 0 && declared != data.len() {
        return Err(MonitorError::Protocol(format!(
            "memory response length mismatch: declared {declared}, got {}",
            data.len()
        )));
    }
    Ok(data.to_vec())
}

fn parse_registers_body(body: &[u8]) -> Result<Vec<RegisterValue>> {
    if body.len() < 2 {
        return Err(MonitorError::Protocol(
            "register response body is too short".to_string(),
        ));
    }
    let count = u16::from_le_bytes([body[0], body[1]]) as usize;
    let mut offset = 2;
    let mut registers = Vec::with_capacity(count);

    for _ in 0..count {
        let Some(&item_size) = body.get(offset) else {
            return Err(MonitorError::Protocol(
                "truncated register item".to_string(),
            ));
        };
        offset += 1;
        let end = offset + usize::from(item_size);
        let item = body
            .get(offset..end)
            .ok_or_else(|| MonitorError::Protocol("truncated register item body".to_string()))?;
        if item.len() >= 3 {
            registers.push(RegisterValue {
                id: item[0],
                value: u16::from_le_bytes([item[1], item[2]]),
            });
        }
        offset = end;
    }

    Ok(registers)
}

fn parse_checkpoint_id(body: &[u8]) -> Result<CheckpointId> {
    let bytes = body.get(0..4).ok_or_else(|| {
        MonitorError::Protocol("checkpoint response body is too short".to_string())
    })?;
    Ok(CheckpointId(u32::from_le_bytes([
        bytes[0], bytes[1], bytes[2], bytes[3],
    ])))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisplayImage {
    /// Full (uncropped) width of the display buffer.
    pub debug_width: u16,
    /// Full (uncropped) height of the display buffer.
    pub debug_height: u16,
    /// X offset of the inner screen within the buffer.
    pub offset_x: u16,
    /// Y offset of the inner screen within the buffer.
    pub offset_y: u16,
    /// Width of the inner (visible) screen.
    pub inner_width: u16,
    /// Height of the inner (visible) screen.
    pub inner_height: u16,
    /// Bits per pixel (8 for INDEXED8).
    pub bits_per_pixel: u8,
    /// Raw indexed-8 buffer, `debug_width * debug_height` bytes.
    pub pixels: Vec<u8>,
}

impl DisplayImage {
    pub fn is_empty(&self) -> bool {
        self.pixels.is_empty()
    }
}

fn parse_display_image(body: &[u8]) -> Result<DisplayImage> {
    if body.len() < 17 {
        return Err(MonitorError::Protocol(
            "display response body is too short".to_string(),
        ));
    }
    let _info_length = u32::from_le_bytes([body[0], body[1], body[2], body[3]]);
    let debug_width = u16::from_le_bytes([body[4], body[5]]);
    let debug_height = u16::from_le_bytes([body[6], body[7]]);
    let offset_x = u16::from_le_bytes([body[8], body[9]]);
    let offset_y = u16::from_le_bytes([body[10], body[11]]);
    let inner_width = u16::from_le_bytes([body[12], body[13]]);
    let inner_height = u16::from_le_bytes([body[14], body[15]]);
    let bits_per_pixel = body[16];
    if body.len() < 21 {
        return Err(MonitorError::Protocol(
            "display response missing buffer length".to_string(),
        ));
    }
    let buffer_length = u32::from_le_bytes([body[17], body[18], body[19], body[20]]) as usize;
    // VICE may send slightly fewer bytes than declared (observed: 4 short on
    // some builds). Take what is actually present rather than failing.
    let available = body.len().saturating_sub(21);
    let take = buffer_length.min(available);
    let pixels = body
        .get(21..21 + take)
        .ok_or_else(|| MonitorError::Protocol("display buffer truncated".to_string()))?
        .to_vec();
    Ok(DisplayImage {
        debug_width,
        debug_height,
        offset_x,
        offset_y,
        inner_width,
        inner_height,
        bits_per_pixel,
        pixels,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BankInfo {
    pub id: u16,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResourceValue {
    String(Vec<u8>),
    Int(i32),
}

impl ResourceValue {
    fn value_type(&self) -> u8 {
        match self {
            Self::String(_) => 0x00,
            Self::Int(_) => 0x01,
        }
    }
}

fn parse_resource_value(body: &[u8]) -> Result<ResourceValue> {
    let Some(&value_type) = body.first() else {
        return Err(MonitorError::Protocol(
            "resource response body is empty".to_string(),
        ));
    };
    match value_type {
        0x00 => {
            let Some(&len) = body.get(1) else {
                return Err(MonitorError::Protocol(
                    "resource string missing length".to_string(),
                ));
            };
            let bytes = body
                .get(2..2 + usize::from(len))
                .ok_or_else(|| MonitorError::Protocol("resource string truncated".to_string()))?;
            Ok(ResourceValue::String(bytes.to_vec()))
        }
        0x01 => {
            if body.len() < 6 {
                return Err(MonitorError::Protocol(
                    "resource int response too short".to_string(),
                ));
            }
            let len = usize::from(body[1]);
            let value_bytes = body
                .get(2..2 + len)
                .ok_or_else(|| MonitorError::Protocol("resource int truncated".to_string()))?;
            let value = match len {
                1 => i32::from(i8::from_le_bytes([value_bytes[0]])),
                2 => i32::from(i16::from_le_bytes([value_bytes[0], value_bytes[1]])),
                4 => i32::from_le_bytes([
                    value_bytes[0],
                    value_bytes[1],
                    value_bytes[2],
                    value_bytes[3],
                ]),
                _ => {
                    return Err(MonitorError::Protocol(
                        "unsupported resource int width".to_string(),
                    ))
                }
            };
            Ok(ResourceValue::Int(value))
        }
        other => Err(MonitorError::Protocol(format!(
            "unknown resource value type {other:02x}"
        ))),
    }
}

fn parse_bank_list(body: &[u8]) -> Result<Vec<BankInfo>> {
    if body.len() < 2 {
        return Err(MonitorError::Protocol(
            "bank list response body is too short".to_string(),
        ));
    }
    let count = u16::from_le_bytes([body[0], body[1]]) as usize;
    let mut offset = 2;
    let mut banks = Vec::with_capacity(count);
    for _ in 0..count {
        let Some(&item_size) = body.get(offset) else {
            return Err(MonitorError::Protocol("truncated bank item".to_string()));
        };
        offset += 1;
        let end = offset + usize::from(item_size);
        let item = body
            .get(offset..end)
            .ok_or_else(|| MonitorError::Protocol("truncated bank item body".to_string()))?;
        if item.len() < 3 {
            return Err(MonitorError::Protocol("bank item too short".to_string()));
        }
        let id = u16::from_le_bytes([item[0], item[1]]);
        let name = String::from_utf8_lossy(&item[2..]).into_owned();
        banks.push(BankInfo { id, name });
        offset = end;
    }
    Ok(banks)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegisterInfo {
    pub id: u8,
    pub size: u8,
    pub name: String,
}

fn parse_register_list(body: &[u8]) -> Result<Vec<RegisterInfo>> {
    if body.len() < 2 {
        return Err(MonitorError::Protocol(
            "register list response body is too short".to_string(),
        ));
    }
    let count = u16::from_le_bytes([body[0], body[1]]) as usize;
    let mut offset = 2;
    let mut registers = Vec::with_capacity(count);
    for _ in 0..count {
        let Some(&item_size) = body.get(offset) else {
            return Err(MonitorError::Protocol(
                "truncated register item".to_string(),
            ));
        };
        offset += 1;
        let end = offset + usize::from(item_size);
        let item = body
            .get(offset..end)
            .ok_or_else(|| MonitorError::Protocol("truncated register body".to_string()))?;
        // Item layout (VICE monitor_binary.c registers_available):
        // [id][size][name_len][name...]
        if item.len() < 3 {
            return Err(MonitorError::Protocol(
                "register item too short".to_string(),
            ));
        }
        let name_len = usize::from(item[2]);
        let name_end = (3 + name_len).min(item.len());
        let name = String::from_utf8_lossy(&item[3..name_end])
            .trim()
            .to_string();
        registers.push(RegisterInfo {
            id: item[0],
            size: item[1],
            name,
        });
        offset = end;
    }
    Ok(registers)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckpointInfo {
    pub id: CheckpointId,
    pub hit: bool,
    pub start: u16,
    pub end: u16,
    pub stop: bool,
    pub enabled: bool,
    pub operation: u8,
    pub temporary: bool,
    pub hit_count: u32,
    pub ignore_count: u32,
    pub has_condition: bool,
}

fn parse_checkpoint_info(body: &[u8]) -> Result<CheckpointInfo> {
    if body.len() < 23 {
        return Err(MonitorError::Protocol(
            "checkpoint info body is too short".to_string(),
        ));
    }
    let id = CheckpointId(u32::from_le_bytes([body[0], body[1], body[2], body[3]]));
    Ok(CheckpointInfo {
        id,
        hit: body[4] != 0,
        start: u16::from_le_bytes([body[5], body[6]]),
        end: u16::from_le_bytes([body[7], body[8]]),
        stop: body[9] != 0,
        enabled: body[10] != 0,
        operation: body[11],
        temporary: body[12] != 0,
        hit_count: u32::from_le_bytes([body[13], body[14], body[15], body[16]]),
        ignore_count: u32::from_le_bytes([body[17], body[18], body[19], body[20]]),
        has_condition: body[21] != 0,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CpuHistoryEntry {
    pub pc: u16,
    pub a: u8,
    pub x: u8,
    pub y: u8,
    pub sp: u8,
    pub flags: u8,
    pub cycle: u64,
    pub op: u8,
    pub p1: u8,
    pub p2: u8,
}

fn parse_cpu_history(body: &[u8]) -> Result<Vec<CpuHistoryEntry>> {
    if body.len() < 4 {
        return Err(MonitorError::Protocol(
            "cpu history response body is too short".to_string(),
        ));
    }
    let count = u32::from_le_bytes([body[0], body[1], body[2], body[3]]) as usize;
    let mut offset = 4;
    let mut entries = Vec::with_capacity(count);
    for _ in 0..count {
        let Some(&item_size) = body.get(offset) else {
            return Err(MonitorError::Protocol(
                "truncated cpu history item".to_string(),
            ));
        };
        offset += 1;
        let end = offset + usize::from(item_size);
        let item = body
            .get(offset..end)
            .ok_or_else(|| MonitorError::Protocol("truncated cpu history body".to_string()))?;
        if item.len() < 38 {
            return Err(MonitorError::Protocol(
                "cpu history item too short".to_string(),
            ));
        }
        // item: [count u16][8 regs x (size,id,val u16)][cycle u64][instr_len u8][op p1 p2 ff]
        let regs_offset = 2;
        let mut pc = 0;
        let mut a = 0;
        let mut x = 0;
        let mut y = 0;
        let mut sp = 0;
        let mut flags = 0;
        for r in 0..8 {
            let base = regs_offset + r * 4;
            if base + 3 > item.len() {
                return Err(MonitorError::Protocol(
                    "cpu history register list truncated".to_string(),
                ));
            }
            let _size = item[base];
            let id = item[base + 1];
            let value = u16::from_le_bytes([item[base + 2], item[base + 3]]);
            match id {
                0 => a = value as u8,
                1 => x = value as u8,
                2 => y = value as u8,
                3 => pc = value,
                4 => sp = value as u8,
                5 => flags = value as u8,
                _ => {}
            }
        }
        let cycle_offset = regs_offset + 32;
        let cycle = u64::from_le_bytes(
            item[cycle_offset..cycle_offset + 8]
                .try_into()
                .map_err(|_| MonitorError::Protocol("cpu history cycle truncated".to_string()))?,
        );
        let instr_len_offset = cycle_offset + 8;
        let instr_len = item[instr_len_offset];
        let code_offset = instr_len_offset + 1;
        if code_offset + usize::from(instr_len) > item.len() {
            return Err(MonitorError::Protocol(
                "cpu history instruction bytes truncated".to_string(),
            ));
        }
        let op = item[code_offset];
        let p1 = item.get(code_offset + 1).copied().unwrap_or(0);
        let p2 = item.get(code_offset + 2).copied().unwrap_or(0);
        entries.push(CpuHistoryEntry {
            pc,
            a,
            x,
            y,
            sp,
            flags,
            cycle,
            op,
            p1,
            p2,
        });
        offset = end;
    }
    Ok(entries)
}

fn best_effort_cpu_registers(registers: &[RegisterValue]) -> CpuRegisters {
    let mut cpu = CpuRegisters::default();
    for register in registers {
        match register.id {
            0 => cpu.a = register.value as u8,
            1 => cpu.x = register.value as u8,
            2 => cpu.y = register.value as u8,
            3 => cpu.pc = register.value,
            4 => cpu.sp = register.value as u8,
            5 => cpu.status = register.value as u8,
            _ => {}
        }
    }
    cpu
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_ping_command_header() {
        let packet = CommandPacket::new(0x1234_dead, CommandType::Ping, Vec::new()).encode();
        assert_eq!(
            packet,
            vec![0x02, 0x02, 0, 0, 0, 0, 0xad, 0xde, 0x34, 0x12, 0x81]
        );
    }

    #[test]
    fn encodes_memory_get_body() {
        let mut body = vec![0x00];
        body.extend_from_slice(&0x0801_u16.to_le_bytes());
        body.extend_from_slice(&0x0810_u16.to_le_bytes());
        body.push(Memspace::Main as u8);
        body.extend_from_slice(&0_u16.to_le_bytes());
        let packet = CommandPacket::new(1, CommandType::MemoryGet, body).encode();

        assert_eq!(&packet[0..11], &[0x02, 0x02, 8, 0, 0, 0, 1, 0, 0, 0, 0x01]);
        assert_eq!(
            &packet[11..],
            &[0x00, 0x01, 0x08, 0x10, 0x08, 0x00, 0x00, 0x00]
        );
    }

    #[test]
    fn decodes_response_packet() {
        let bytes = [
            0x02, 0x02, 0x02, 0, 0, 0, 0x62, 0x00, 0xff, 0xff, 0xff, 0xff, 0xcf, 0xe5,
        ];
        let response = ResponsePacket::decode(&bytes).unwrap();
        assert_eq!(response.response_type, 0x62);
        assert_eq!(response.error_code, ErrorCode::Ok);
        assert!(response.is_event());
        assert_eq!(response.body, vec![0xcf, 0xe5]);
    }

    #[test]
    fn parses_register_response_body() {
        let body = [0x02, 0x00, 0x03, 0x03, 0x34, 0x12, 0x03, 0x00, 0x56, 0x00];
        let registers = parse_registers_body(&body).unwrap();
        assert_eq!(
            registers,
            vec![
                RegisterValue {
                    id: 3,
                    value: 0x1234
                },
                RegisterValue {
                    id: 0,
                    value: 0x0056
                }
            ]
        );
    }

    #[test]
    fn parses_display_image_body() {
        let mut body = Vec::new();
        body.extend_from_slice(&13_u32.to_le_bytes()); // info length
        body.extend_from_slice(&384_u16.to_le_bytes()); // debug width
        body.extend_from_slice(&284_u16.to_le_bytes()); // debug height
        body.extend_from_slice(&24_u16.to_le_bytes()); // offset x
        body.extend_from_slice(&36_u16.to_le_bytes()); // offset y
        body.extend_from_slice(&336_u16.to_le_bytes()); // inner width
        body.extend_from_slice(&212_u16.to_le_bytes()); // inner height
        body.push(8); // bits per pixel
        body.extend_from_slice(&6_u32.to_le_bytes()); // buffer length
        body.extend_from_slice(&[1, 2, 3, 4, 5, 6]);

        let image = parse_display_image(&body).unwrap();
        assert_eq!(image.debug_width, 384);
        assert_eq!(image.debug_height, 284);
        assert_eq!(image.inner_width, 336);
        assert_eq!(image.inner_height, 212);
        assert_eq!(image.pixels, vec![1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn parses_bank_list_body() {
        let mut body = vec![2, 0];
        body.push(9);
        body.extend_from_slice(&0_u16.to_le_bytes());
        body.extend_from_slice(b"default");
        body.push(5);
        body.extend_from_slice(&1_u16.to_le_bytes());
        body.extend_from_slice(b"ram");

        let banks = parse_bank_list(&body).unwrap();
        assert_eq!(
            banks,
            vec![
                BankInfo {
                    id: 0,
                    name: "default".to_string()
                },
                BankInfo {
                    id: 1,
                    name: "ram".to_string()
                }
            ]
        );
    }

    #[test]
    fn parses_checkpoint_info_body() {
        let mut body = Vec::new();
        body.extend_from_slice(&7_u32.to_le_bytes());
        body.push(1); // hit
        body.extend_from_slice(&0xd400_u16.to_le_bytes());
        body.extend_from_slice(&0xd418_u16.to_le_bytes());
        body.push(1); // stop
        body.push(1); // enabled
        body.push(0x02); // write
        body.push(0); // temporary
        body.extend_from_slice(&3_u32.to_le_bytes()); // hit count
        body.extend_from_slice(&0_u32.to_le_bytes());
        body.push(0); // condition
        body.push(0); // memspace

        let info = parse_checkpoint_info(&body).unwrap();
        assert_eq!(info.id, CheckpointId(7));
        assert!(info.hit);
        assert_eq!(info.start, 0xd400);
        assert_eq!(info.end, 0xd418);
        assert_eq!(info.hit_count, 3);
    }

    #[test]
    fn parses_cpu_history_body() {
        // 1 entry: count u16, 8 regs (size, id, val u16), cycle u64, instr_len u8, op p1 p2 ff
        let mut item = Vec::new();
        item.extend_from_slice(&8_u16.to_le_bytes());
        let regs = [
            (0, 0x42_u16),
            (1, 0x01),
            (2, 0x02),
            (3, 0x1234),
            (4, 0xfd),
            (5, 0x24),
            (6, 0),
            (7, 0),
        ];
        for (id, value) in regs {
            item.push(3);
            item.push(id);
            item.extend_from_slice(&value.to_le_bytes());
        }
        item.extend_from_slice(&1_000_000_u64.to_le_bytes());
        item.push(4);
        item.extend_from_slice(&[0x8d, 0x00, 0xd4, 0xff]); // STA $D400

        let mut body = 1_u32.to_le_bytes().to_vec();
        body.push(item.len() as u8);
        body.extend_from_slice(&item);

        let entries = parse_cpu_history(&body).unwrap();
        assert_eq!(entries.len(), 1);
        let entry = &entries[0];
        assert_eq!(entry.pc, 0x1234);
        assert_eq!(entry.a, 0x42);
        assert_eq!(entry.cycle, 1_000_000);
        assert_eq!(entry.op, 0x8d);
        assert_eq!(entry.p1, 0x00);
        assert_eq!(entry.p2, 0xd4);
    }
}
