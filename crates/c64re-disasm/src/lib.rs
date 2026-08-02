use c64re_provenance::ProvenanceMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AddressRange {
    pub start: u16,
    pub end_inclusive: u16,
}

impl AddressRange {
    pub fn len(self) -> usize {
        usize::from(self.end_inclusive - self.start) + 1
    }

    pub fn is_empty(self) -> bool {
        false
    }
}

pub fn executed_ranges(provenance: &ProvenanceMap) -> Vec<AddressRange> {
    let mut ranges = Vec::new();
    let mut current_start: Option<u16> = None;

    for address in 0_u16..=u16::MAX {
        let executed = provenance.get(address).executed;
        match (current_start, executed) {
            (None, true) => current_start = Some(address),
            (Some(start), false) => {
                ranges.push(AddressRange {
                    start,
                    end_inclusive: address.wrapping_sub(1),
                });
                current_start = None;
            }
            _ => {}
        }
    }

    if let Some(start) = current_start {
        ranges.push(AddressRange {
            start,
            end_inclusive: u16::MAX,
        });
    }

    ranges
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InstructionStub {
    pub address: u16,
    pub opcode: u8,
}

pub fn linear_stub_disassembly(load_address: u16, bytes: &[u8]) -> Vec<InstructionStub> {
    bytes
        .iter()
        .enumerate()
        .filter_map(|(index, &opcode)| {
            let address = load_address.checked_add(index as u16)?;
            Some(InstructionStub { address, opcode })
        })
        .collect()
}
