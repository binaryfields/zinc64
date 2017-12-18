/*
 * Copyright (c) 2016-2017 Sebastian Jastrzebski. All rights reserved.
 *
 * This file is part of zinc64.
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

use device::joystick;
use util::{Dimension, Rect};

// http://codebase64.org/doku.php?id=base:cpu_clocking
// http://dustlayer.com/vic-ii/2013/4/25/vic-ii-for-beginners-beyond-the-screen-rasters-cycle
// http://www.antimon.org/dl/c64/code/missing.txt

/*
All clock frequencies in the C64 are derived from a single clock quartz which has the frequency of 4
times the frequency of the color carrier used for PAL or NTSC.

PAL C64 master clock: 17.734475 MHz

NTSC C64 master clock: 14.31818 MHz

The CPU frequency is then calculated from that by simply dividing the frequency by 18 (PAL) or 14
(NTSC). The VIC-II runs at a frequency which is exactly 8 times that of the CPU. This is the so called
“dot clock” which has to be very precise in order to keep the right timing needed to generate a video
signal compatible with all TVs. The CPU of the time could not go that fast, max. 1MHz, but the CPU
still needs to be phase synchronous to the VIC-II because they share control of the address/data bus
of the machine. That's why the VIC-II internally provides a clock divider which feeds the CPU.
*/

const CLOCK_MASTER_PAL: u32 = 17734475;
const CLOCK_CPU_PAL: u32 = CLOCK_MASTER_PAL / 18; // 985248 Hz
#[allow(dead_code)]
const CLOCK_VIC_PAL: u32 = CLOCK_CPU_PAL * 8; // 7881984 Hz

/*

The dimensions of the video display for the different VIC types are as
follows:

          | Video  | # of  | Visible | Cycles/ |  Visible
   Type   | system | lines |  lines  |  line   | pixels/line
 ---------+--------+-------+---------+---------+------------
 6567R56A | NTSC-M |  262  |   234   |   64    |    411
  6567R8  | NTSC-M |  263  |   235   |   65    |    418
   6569   |  PAL-B |  312  |   284   |   63    |    403

          | First  |  Last  |              |   First    |   Last
          | vblank | vblank | First X coo. |  visible   |  visible
   Type   |  line  |  line  |  of a line   |   X coo.   |   X coo.
 ---------+--------+--------+--------------+------------+-----------
 6567R56A |   13   |   40   |  412 ($19c)  | 488 ($1e8) | 388 ($184)
  6567R8  |   13   |   40   |  412 ($19c)  | 489 ($1e9) | 396 ($18c)
   6569   |  300   |   15   |  404 ($194)  | 480 ($1e0) | 380 ($17c)

*/

const DISPLAY_WIDTH_PAL: u16 = 504;
const DISPLAY_HEIGHT_PAL: u16 = 312;

const VISIBLE_WIDTH_PAL: u16 = 403;
const VISIBLE_HEIGHT_PAL: u16 = 284;
const VISIBLE_FIRST_COL_PAL: u16 = 80; // translated 76, original 480, but we offset x to beg of display
const VISIBLE_FIRST_LINE_PAL: u16 = 16;
#[allow(dead_code)]
const VISIBLE_LAST_COL_PAL: u16 = 484; // translated 480, original 380, but we offset x to beg of display
#[allow(dead_code)]
const VISIBLE_LAST_LINE_PAL: u16 = 299;

/*

The height and width of the display window can each be set to two different
values with the bits RSEL and CSEL in the registers $d011 and $d016:

 RSEL|  Display window height   | First line  | Last line
 ----+--------------------------+-------------+----------
   0 | 24 text lines/192 pixels |   55 ($37)  | 246 ($f6)
   1 | 25 text lines/200 pixels |   51 ($33)  | 250 ($fa)

 CSEL|   Display window width   | First X coo. | Last X coo.
 ----+--------------------------+--------------+------------
   0 | 38 characters/304 pixels |   31 ($1f)   |  334 ($14e)
   1 | 40 characters/320 pixels |   24 ($18)   |  343 ($157)

The X coordinates run up to $1ff (only $1f7 on the 6569) within a line, then comes X coordinate 0.

There are 2×2 comparators belonging to each of the two flip flops. There
comparators compare the X/Y position of the raster beam with one of two
hardwired values (depending on the state of the CSEL/RSEL bits) to control
the flip flops. The comparisons only match if the values are reached
precisely. There is no comparison with an interval.

The horizontal comparison values:

       |   CSEL=0   |   CSEL=1
 ------+------------+-----------
 Left  |  31 ($1f)  |  24 ($18)
 Right | 335 ($14f) | 344 ($158)

And the vertical ones:

        |   RSEL=0  |  RSEL=1
 -------+-----------+----------
 Top    |  55 ($37) |  51 ($33)
 Bottom | 247 ($f7) | 251 ($fb)

*/

const WINDOW_WIDTH: u16 = 320;
const WINDOW_HEIGHT: u16 = 200;
const WINDOW_FIRST_COL: u16 = 128; // translated 124, original 24
const WINDOW_FIRST_LINE: u16 = 51;
#[allow(dead_code)]
const WINDOW_LAST_LINE: u16 = 250;
#[allow(dead_code)]
const WINDOW_LAST_COL: u16 = 447; // translated 443, original 343

const RASTER_TIME_BYTE_PAL: u16 = 1; // 8 pixels/cpu cycle
const RASTER_LINE_CYCLES_PAL: u16 = DISPLAY_WIDTH_PAL / 8 * RASTER_TIME_BYTE_PAL; // 63
const RASTER_FRAME_CYCLES_PAL: u16 = DISPLAY_HEIGHT_PAL * RASTER_LINE_CYCLES_PAL; // 19656
const RASTER_REFRESH_RATE_PAL: f64 = (CLOCK_CPU_PAL as f64) / (RASTER_FRAME_CYCLES_PAL as f64); // 50.125

#[derive(Copy, Clone)]
pub struct Config {
    // Cpu
    pub cpu_frequency: u32,
    // Video
    pub raster_size: Dimension,
    pub screen_size: Dimension,
    pub screen: Rect,
    pub window_size: Dimension,
    pub graphics: Rect,
    pub window: Rect,
    pub frame_cycles: u32,
    pub frame_duration_ns: u32,
    pub raster_line_cycles: u16,
    pub refresh_rate: f64,
    // Devices
    pub joystick1: joystick::Mode,
    pub joystick2: joystick::Mode,
}

impl Config {
    pub fn new(model: &str) -> Config {
        match model {
            "pal" => Config::pal(),
            _ => panic!("invalid model {}", model),
        }
    }

    pub fn pal() -> Config {
        let display_size = Dimension::new(DISPLAY_WIDTH_PAL, DISPLAY_HEIGHT_PAL);
        let visible_size = Dimension::new(VISIBLE_WIDTH_PAL, VISIBLE_HEIGHT_PAL);
        let window_size = Dimension::new(WINDOW_WIDTH, WINDOW_HEIGHT);
        let graphics = Rect::new_with_dim(WINDOW_FIRST_COL, WINDOW_FIRST_LINE - 3, window_size);
        let window = Rect::new_with_dim(
            WINDOW_FIRST_COL - VISIBLE_FIRST_COL_PAL,
            WINDOW_FIRST_LINE - VISIBLE_FIRST_LINE_PAL,
            window_size,
        );
        Config {
            cpu_frequency: CLOCK_CPU_PAL,
            raster_size: display_size,
            screen_size: visible_size,
            screen: Rect::new_with_dim(VISIBLE_FIRST_COL_PAL, VISIBLE_FIRST_LINE_PAL, visible_size),
            graphics: graphics,
            window_size: window_size,
            window: window,
            frame_cycles: RASTER_FRAME_CYCLES_PAL as u32,
            frame_duration_ns: ((1.0 / RASTER_REFRESH_RATE_PAL) * 1_000_000_000.0) as u32,
            raster_line_cycles: RASTER_LINE_CYCLES_PAL,
            refresh_rate: RASTER_REFRESH_RATE_PAL,
            joystick1: joystick::Mode::Numpad,
            joystick2: joystick::Mode::None,
        }
    }
}
