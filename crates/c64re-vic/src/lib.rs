/// Video mode derived from $D011 (bit 5 bitmap, bits 6-7 ECM) and
/// $D016 (bit 4 multicolor).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum DisplayMode {
    StandardText,
    MulticolorText,
    ExtendedBackground,
    HiresBitmap,
    MulticolorBitmap,
}

impl DisplayMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::StandardText => "standard-text",
            Self::MulticolorText => "multicolor-text",
            Self::ExtendedBackground => "ecm-text",
            Self::HiresBitmap => "hires-bitmap",
            Self::MulticolorBitmap => "multicolor-bitmap",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct VicState {
    pub bank_select_dd00: u8,
    pub memory_setup_d018: u8,
    /// $D011: raster bit 8, ECM (bit 6), bitmap (bit 5), vertical scroll.
    pub control_1_d011: u8,
    /// $D016: multicolor (bit 4), 38/40 columns, horizontal scroll.
    pub control_2_d016: u8,
    pub sprite_enable_d015: u8,
    pub sprite_multicolor_d01c: u8,
    pub sprite_y_expand_d017: u8,
    pub sprite_x_expand_d01d: u8,
    pub sprite_priority_d01b: u8,
    pub sprite_extra_x_d010: u8,
    pub background_color_d021: u8,
    pub background_1_d022: u8,
    pub background_2_d023: u8,
    pub background_3_d024: u8,
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

    /// In bitmap modes, $D018 bit 3 selects the 8K bitmap base within the bank.
    pub fn bitmap_base(self) -> u16 {
        let offset = u16::from((self.memory_setup_d018 >> 3) & 0x01) * 0x2000;
        self.vic_bank_base() + offset
    }

    pub fn sprite_pointer_table(self) -> u16 {
        self.screen_base() + 0x03f8
    }

    pub fn sprite_enabled(self, index: usize) -> bool {
        index < 8 && (self.sprite_enable_d015 & (1 << index)) != 0
    }

    pub fn sprite_multicolor(self, index: usize) -> bool {
        index < 8 && (self.sprite_multicolor_d01c & (1 << index)) != 0
    }

    pub fn sprite_x_expanded(self, index: usize) -> bool {
        index < 8 && (self.sprite_x_expand_d01d & (1 << index)) != 0
    }

    pub fn sprite_y_expanded(self, index: usize) -> bool {
        index < 8 && (self.sprite_y_expand_d017 & (1 << index)) != 0
    }

    pub fn display_mode(self) -> DisplayMode {
        let bitmap = self.control_1_d011 & 0x20 != 0;
        let ecm = self.control_1_d011 & 0x40 != 0;
        let multicolor = self.control_2_d016 & 0x10 != 0;
        if bitmap {
            if multicolor {
                DisplayMode::MulticolorBitmap
            } else {
                DisplayMode::HiresBitmap
            }
        } else if ecm {
            DisplayMode::ExtendedBackground
        } else if multicolor {
            DisplayMode::MulticolorText
        } else {
            DisplayMode::StandardText
        }
    }

    /// True when this state represents a ROM character set (VIC banks 0/2,
    /// charset base $1000 or $1800 map to character ROM).
    pub fn is_rom_charset(self) -> bool {
        let base = self.charset_base();
        let bank = self.vic_bank_base();
        matches!(bank, 0x0000 | 0x4000) && matches!(base, 0x1000 | 0x1800)
    }

    /// True when this state uses a hires/multicolor bitmap at the bitmap base.
    pub fn uses_bitmap(self) -> bool {
        self.display_mode() == DisplayMode::HiresBitmap
            || self.display_mode() == DisplayMode::MulticolorBitmap
    }
}
