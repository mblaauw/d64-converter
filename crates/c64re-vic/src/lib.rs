#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct VicState {
    pub bank_select_dd00: u8,
    pub memory_setup_d018: u8,
    pub sprite_enable_d015: u8,
    pub sprite_multicolor_d01c: u8,
    pub sprite_extra_x_d010: u8,
    pub background_color_d021: u8,
    pub multicolor_0_d025: u8,
    pub multicolor_1_d026: u8,
    pub sprite_colors_d027_d02e: [u8; 8],
    pub sprite_x: [u16; 8],
    pub sprite_y: [u8; 8],
}

impl VicState {
    pub fn vic_bank_base(self) -> u16 {
        match self.bank_select_dd00 & 0x03 {
            0 => 0xc000,
            1 => 0x8000,
            2 => 0x4000,
            _ => 0x0000,
        }
    }

    pub fn screen_base(self) -> u16 {
        self.vic_bank_base() + u16::from((self.memory_setup_d018 >> 4) & 0x0f) * 0x0400
    }

    pub fn charset_base(self) -> u16 {
        self.vic_bank_base() + u16::from((self.memory_setup_d018 >> 1) & 0x07) * 0x0800
    }

    pub fn sprite_pointer_table(self) -> u16 {
        self.screen_base() + 0x03f8
    }

    pub fn sprite_enabled(self, index: usize) -> bool {
        index < 8 && (self.sprite_enable_d015 & (1 << index)) != 0
    }
}
