use bit_field::BitField;

#[derive(Copy, Clone)]
pub enum Mode {
    // (ECM/BMM/MCM=0/0/0)
    Text = 0x00,
    // (ECM/BMM/MCM=0/0/1)
    McText = 0x01,
    // (ECM/BMM/MCM=0/1/0)
    Bitmap = 0x02,
    // (ECM/BMM/MCM=0/1/1)
    McBitmap = 0x03,
    // (ECM/BMM/MCM=1/0/0)
    EcmText = 0x04,
    // (ECM/BMM/MCM=1/0/1)
    InvalidText = 0x05,
    // (ECM/BMM/MCM=1/1/0)
    InvalidBitmap1 = 0x06,
    // (ECM/BMM/MCM=1/1/1)
    InvalidBitmap2 = 0x07,
}

impl Mode {
    pub fn from(mode: u8) -> Mode {
        match mode {
            0x00 => Mode::Text,
            0x01 => Mode::McText,
            0x02 => Mode::Bitmap,
            0x03 => Mode::McBitmap,
            0x04 => Mode::EcmText,
            0x05 => Mode::InvalidText,
            0x06 => Mode::InvalidBitmap1,
            0x07 => Mode::InvalidBitmap2,
            _ => panic!("invalid mode {}", mode),
        }
    }

    pub fn value(&self) -> u8 {
        *self as u8
    }
}

pub struct GfxSequencer {
    mode: Mode,
    bg_color: [u8; 4],
    border_color: u8,
    border_mff: bool,
    border_vff: bool,
    c_data: u8,
    c_color: u8,
    g_data: u8,
    mc_cycle: bool,
    output: u8,
}

impl GfxSequencer {
    pub fn new() -> Self {
        GfxSequencer {
            mode: Mode::Text,
            bg_color: [0; 4],
            border_color: 0,
            border_mff: false,
            border_vff: false,
            c_data: 0,
            c_color: 0,
            g_data: 0,
            mc_cycle: false,
            output: 0,
        }
    }

    pub fn get_bg_color(&self, index: usize) -> u8 {
        self.bg_color[index]
    }

    pub fn get_border_color(&self) -> u8 {
        self.border_color
    }

    pub fn get_border_vertical_ff(&self) -> bool {
        self.border_vff
    }

    pub fn get_mode(&self) -> Mode {
        self.mode
    }

    pub fn set_bg_color(&mut self, index: usize, color: u8) {
        self.bg_color[index] = color;
    }

    pub fn set_border_color(&mut self, color: u8) {
        self.border_color = color;
    }

    pub fn set_border_main_ff(&mut self, value: bool) {
        self.border_mff = value;
    }

    pub fn set_border_vertical_ff(&mut self, value: bool) {
        self.border_vff = value;
    }

    pub fn set_data(&mut self, c_data: u8, c_color: u8, g_data: u8) {
        self.c_data = c_data;
        self.c_color = c_color;
        self.g_data = g_data;
    }

    pub fn set_mode(&mut self, mode: Mode) {
        self.mode = mode;
    }

    #[inline]
    pub fn clock(&mut self) {
        if !self.border_vff && !self.border_mff {
            if !self.mc_cycle {
                self.output = match self.mode {
                    Mode::Text => self.output_text(),
                    Mode::McText => {
                        self.mc_cycle = self.c_color.get_bit(3);
                        self.output_text_mc()
                    },
                    Mode::Bitmap => self.output_bitmap(),
                    Mode::McBitmap => {
                        self.mc_cycle = true;
                        self.output_bitmap_mc()
                    },
                    Mode::EcmText => self.output_text_ecm(),
                    Mode::InvalidBitmap1 | Mode::InvalidBitmap2 => 0,
                    _ => panic!("unsupported graphics mode {}", self.mode.value()),
                };
                self.g_data = if !self.mc_cycle {
                    self.g_data << 1
                } else {
                    self.g_data << 2
                }
            } else {
                self.mc_cycle = false;
            }
        } else {
            self.output = self.border_color;
        }
    }

    #[inline]
    pub fn output(&self) -> u8 {
        self.output
    }

    pub fn reset(&mut self) {
        self.mode = Mode::Text;
        self.bg_color = [0x06, 0, 0, 0];
        self.border_color = 0x0e;
        self.border_mff = false;
        self.border_vff = false;
        self.c_data = 0;
        self.c_color = 0;
        self.g_data = 0;
        self.mc_cycle = false;
        self.output = 0;
    }

    /*
     +----+----+----+----+----+----+----+----+
     |  7 |  6 |  5 |  4 |  3 |  2 |  1 |  0 |
     +----+----+----+----+----+----+----+----+
     |         8 pixels (1 bit/pixel)        |
     |                                       |
     | "0": Color from bits 0-3 of c-data    |
     | "1": Color from bits 4-7 of c-data    |
     +---------------------------------------+
    */

    #[inline]
    fn output_bitmap(&self) -> u8 {
        if self.g_data.get_bit(7) {
            self.c_data >> 4
        } else {
            self.c_data & 0x0f
        }
    }

    /*
     +----+----+----+----+----+----+----+----+
     |  7 |  6 |  5 |  4 |  3 |  2 |  1 |  0 |
     +----+----+----+----+----+----+----+----+
     |         4 pixels (2 bits/pixel)       |
     |                                       |
     | "00": Background color 0 ($d021)      |
     | "01": Color from bits 4-7 of c-data   |
     | "10": Color from bits 0-3 of c-data   |
     | "11": Color from bits 8-11 of c-data  |
     +---------------------------------------+
    */

    #[inline]
    fn output_bitmap_mc(&self) -> u8 {
        match self.g_data >> 6 {
            0 => self.bg_color[0],
            1 => self.c_data >> 4,
            2 => self.c_data & 0x0f,
            3 => self.c_color,
            _ => panic!("invalid color source {}", self.g_data >> 6),
        }
    }

    /*
     +----+----+----+----+----+----+----+----+
     |  7 |  6 |  5 |  4 |  3 |  2 |  1 |  0 |
     +----+----+----+----+----+----+----+----+
     |         8 pixels (1 bit/pixel)        |
     |                                       |
     | "0": Background color 0 ($d021)       |
     | "1": Color from bits 8-11 of c-data   |
     +---------------------------------------+
    */

    #[inline]
    fn output_text(&self) -> u8 {
        if self.g_data.get_bit(7) {
            self.c_color
        } else {
            self.bg_color[0]
        }
    }

    /*
     +----+----+----+----+----+----+----+----+
     |  7 |  6 |  5 |  4 |  3 |  2 |  1 |  0 |
     +----+----+----+----+----+----+----+----+
     |         8 pixels (1 bit/pixel)        |
     |                                       |
     | "0": Depending on bits 6/7 of c-data  |
     |      00: Background color 0 ($d021)   |
     |      01: Background color 1 ($d022)   |
     |      10: Background color 2 ($d023)   |
     |      11: Background color 3 ($d024)   |
     | "1": Color from bits 8-11 of c-data   |
     +---------------------------------------+
    */

    #[inline]
    fn output_text_ecm(&self) -> u8 {
        if self.g_data.get_bit(7) {
            self.c_color
        } else {
            self.bg_color[(self.c_data >> 6) as usize]
        }
    }

    /*
     +----+----+----+----+----+----+----+----+
     |  7 |  6 |  5 |  4 |  3 |  2 |  1 |  0 |
     +----+----+----+----+----+----+----+----+
     |         8 pixels (1 bit/pixel)        |
     |                                       | MC flag = 0
     | "0": Background color 0 ($d021)       |
     | "1": Color from bits 8-10 of c-data   |
     +---------------------------------------+
     |         4 pixels (2 bits/pixel)       |
     |                                       |
     | "00": Background color 0 ($d021)      | MC flag = 1
     | "01": Background color 1 ($d022)      |
     | "10": Background color 2 ($d023)      |
     | "11": Color from bits 8-10 of c-data  |
     +---------------------------------------+
    */

    #[inline]
    fn output_text_mc(&self) -> u8 {
        if self.c_color.get_bit(3) {
            match self.g_data >> 6 {
                0 => self.bg_color[0],
                1 => self.bg_color[1],
                2 => self.bg_color[2],
                3 => self.c_color & 0x07,
                _ => panic!("invalid color source {}", self.g_data >> 6),
            }
        } else {
            self.output_text()
        }
    }

}