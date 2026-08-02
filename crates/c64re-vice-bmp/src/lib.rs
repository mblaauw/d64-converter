use std::fmt;
use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};

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
    CheckpointSet = 0x12,
    CheckpointDelete = 0x13,
    RegistersGet = 0x31,
    RegistersSet = 0x32,
    AdvanceInstructions = 0x71,
    KeyboardFeed = 0x72,
    ExecuteUntilReturn = 0x73,
    Ping = 0x81,
    DisplayGet = 0x84,
    CpuHistory = 0x86,
    PaletteGet = 0x91,
    JoyportSet = 0xa2,
    Exit = 0xaa,
    Quit = 0xbb,
    Reset = 0xcc,
    Autostart = 0xdd,
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
}

impl ViceMonitor {
    pub fn connect(addr: impl ToSocketAddrs) -> Result<Self> {
        Ok(Self {
            stream: TcpStream::connect(addr)?,
            next_request_id: 1,
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
        let request_id = self.next_id();
        let packet = CommandPacket::new(request_id, command_type, body).encode();
        self.stream.write_all(&packet)?;
        self.stream.flush()?;

        loop {
            let response = self.read_response()?;
            if response.request_id == request_id {
                return Ok(response);
            }
        }
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
}
