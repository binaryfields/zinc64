/*
 * Copyright (c) 2016-2018 Sebastian Jastrzebski. All rights reserved.
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

// SPEC: https://sites.google.com/site/h2obsession/CBM/petscii

pub fn pet_to_ascii(code: u8) -> u8 {
    match code {
        // Block 0, Low Control Codes
        0 ... 31 => 0,
        // Block 1, Numbers and Punctuation
        32 => 32, // sp
        33 => 33, // !
        34 => 34, // "
        35 => 35, // #
        36 => 36, // $
        37 => 37, // %
        38 => 38, // &
        39 => 39, // '
        40 => 40, // (
        41 => 41, // )
        42 => 42, // *
        43 => 43, // +
        44 => 44, // ,
        45 => 45, // -
        46 => 46, // .
        47 => 47, // /
        48 => 48, // 0
        49 => 49, // 1
        50 => 50, // 2
        51 => 51, // 3
        52 => 52, // 4
        53 => 53, // 5
        54 => 54, // 6
        55 => 55, // 7
        56 => 56, // 8
        57 => 57, // 9
        58 => 58, // :
        59 => 59, // ;
        60 => 60, // <
        61 => 61, // =
        62 => 62, // >
        63 => 63, // ?
        // Block 2, Lowercase Letters
        64 => 64, // @
        65 => 97, // a
        66 => 98, // b
        67 => 99, // c
        68 => 100, // d
        69 => 101, // e
        70 => 102, // f
        71 => 103, // g
        72 => 104, // h
        73 => 105, // i
        74 => 106, // j
        75 => 107, // k
        76 => 108, // l
        77 => 109, // m
        78 => 110, // n
        79 => 111, // o
        80 => 112, // p
        81 => 113, // q
        82 => 114, // r
        83 => 115, // s
        84 => 116, // t
        85 => 117, // u
        86 => 118, // v
        87 => 119, // w
        88 => 120, // x
        89 => 121, // y
        90 => 122, // z
        91 => 91, // [
        92 => 0, // £
        93 => 93, // ]
        94 => 94, // ↑
        95 => 0, // ←
        // Block 3, Uppercase Letters (Alternate)
        96 => 0, // ─
        97 => 65, // A
        98 => 66, // B
        99 => 67, // C
        100 => 68, // D
        101 => 69, // E
        102 => 70, // F
        103 => 71, // G
        104 => 72, // H
        105 => 73, // I
        106 => 74, // J
        107 => 75, // K
        108 => 76, // L
        109 => 77, // M
        110 => 78, // N
        111 => 79, // O
        112 => 80, // P
        113 => 81, // Q
        114 => 82, // R
        115 => 83, // S
        116 => 84, // T
        117 => 85, // U
        118 => 86, // V
        119 => 87, // W
        120 => 88, // X
        121 => 89, // Y
        122 => 90, // Z
        123 => 0, // ┼
        124 => 0, // ▦
        125 => 124, // │
        126 => 0, // ▩
        127 => 0, // ▧
        // Block 4, High Control Codes
        128 ... 159 => 0,
        // Block 5, Common Graphics (Primary)
        160 => 32, // SP
        161 ... 191 => 0,
        // Block 6, Uppercase Letters (Primary)
        192 => 0, // ─
        193 => 65, // A
        194 => 66, // B
        195 => 67, // C
        196 => 68, // D
        197 => 69, // E
        198 => 70, // F
        199 => 71, // G
        200 => 72, // H
        201 => 73, // I
        202 => 74, // J
        203 => 75, // K
        204 => 76, // L
        205 => 77, // M
        206 => 78, // N
        207 => 79, // O
        208 => 80, // P
        209 => 81, // Q
        210 => 82, // R
        211 => 83, // S
        212 => 84, // T
        213 => 85, // U
        214 => 86, // V
        215 => 87, // W
        216 => 88, // X
        217 => 89, // Y
        218 => 90, // Z
        219 => 0, // ┼
        220 => 0, // ▦
        221 => 124, // │
        222 => 0, // ▩
        223 => 0, // ▧
        // Block 7, Common Graphics (Alternate)
        224 ... 255 => 0,
        _ => 0,
    }
}

pub fn screen_code_to_ascii(code: u8) -> u8 {
    match code {
        0 => 64,
        1...31 => 96 + code,
        32...90 => code,
        _ => 0,
    }
}

