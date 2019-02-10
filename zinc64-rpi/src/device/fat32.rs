// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

// SPEC: https://en.wikipedia.org/wiki/Design_of_the_FAT_file_system

use alloc::prelude::*;
use core::alloc::{GlobalAlloc, Layout};
use core::cmp;
use core::mem;
use core::ptr;
use core::slice;
use core::str;

use super::sd::Sd;

type Result<T> = core::result::Result<T, &'static str>;

const BLOCK_SIZE: usize = 512;

#[repr(C, packed)]
struct MasterBootRecord {
    pub reserved: [u8; 446],
    pub partition_entries: [PartitionEntry; 4],
    pub boot_signature: u16,
}

#[repr(C, packed)]
struct PartitionEntry {
    pub status: u8,
    pub chs_start: [u8; 3],
    pub partition_type: u8,
    pub chs_end: [u8; 3],
    pub first_sector: u32,
    pub num_sectors: u32,
}

#[repr(C, packed)]
struct BootSector {
    pub jmp_boot: [u8; 3],
    pub oem_name: [u8; 8],
}

#[repr(C, packed)]
struct BiosParamBlock {
    pub reserved_bs: [u8; 11],
    pub bytes_per_sector: u16,
    pub sectors_per_cluster: u8,
    pub reserved_sector_count: u16,
    pub num_of_fats: u8,
    pub root_entry_count: u16,
    pub total_sectors_16: u16,
    pub media: u8,
    pub fat_size_16: u16,
    pub sectors_per_track: u16,
    pub num_heads: u16,
    pub hidden_sectors: u32,
    pub total_sectors: u32,
}

#[repr(C, packed)]
struct ExtBiosParamBlock {
    pub reserved_bs: [u8; 11],
    pub reserved_bpb: [u8; 25],
    pub drive_number: u8,
    pub reserved_1: u8,
    pub boot_signature: u8,
    pub volume_id: u32,
    pub volume_label: [u8; 11],
    pub file_system_type: [u8; 8],
}

#[repr(C, packed)]
struct ExtBiosParamBlock32 {
    pub reserved_bs: [u8; 11],
    pub reserved_bpb: [u8; 25],
    pub fat_size_32: u32,
    pub ext_flags: u16,
    pub fs_version: u16,
    pub root_cluster: u32,
    pub fs_info: u16,
    pub backup_boot_sector: u16,
    pub reserved: [u8; 12],
    pub drive_number: u8,
    pub reserved_1: u8,
    pub boot_signature: u8,
    pub volume_id: u32,
    pub volume_label: [u8; 11],
    pub file_system_type: [u8; 8],
}

#[repr(C, packed)]
#[derive(Clone, Copy, Default)]
struct DirectoryEntry {
    pub name: [u8; 8],
    pub ext: [u8; 3],
    pub attributes: u8,
    pub nt_reserved: u8,
    pub created_time_tenth: u8,
    pub created_time: u16,
    pub created_date: u16,
    pub last_access_date: u16,
    pub first_cluster_hi: u16,
    pub write_time: u16,
    pub write_date: u16,
    pub first_cluster_low: u16,
    pub file_size: u32,
}

impl DirectoryEntry {
    pub fn get_cluster(&self) -> u32 {
        self.first_cluster_low as u32 | (self.first_cluster_hi as u32) << 16
    }

    pub fn get_ext(&self) -> Result<String> {
        Ok(util::to_str_trimmed(&self.ext)?.to_owned())
    }

    pub fn get_file_name(&self) -> Result<String> {
        let mut name = String::new();
        if self.is_dir() {
            name.push_str(util::to_str_trimmed(&self.name)?);
        } else {
            name.push_str(util::to_str_trimmed(&self.name)?);
            name.push_str(".");
            name.push_str(util::to_str_trimmed(&self.ext)?);
        }
        Ok(name)
    }

    pub fn get_size(&self) -> Option<usize> {
        if self.is_dir() { None } else { Some(self.file_size as usize) }
    }

    pub fn is_empty(&self) -> bool {
        self.name[0] == 0x00 as u8
    }

    pub fn is_deleted(&self) -> bool {
        self.name[0] == 0xe5 as u8
    }

    pub fn is_dir(&self) -> bool {
        self.attributes & (FileAttribute::Directory as u8) != 0
    }

    pub fn is_label(&self) -> bool {
        self.attributes == (FileAttribute::Label as u8) || self.attributes == 0x0f
    }
}

struct Buffer<'a> {
    pub buf: &'a mut [u8],
}

impl<'a> Buffer<'a> {
    pub fn build(size: usize) -> Result<Buffer<'a>> {
        let layout = Layout::from_size_align(size, 8)
            .map_err(|_| "invalid buffer alignment")?;
        let ptr = unsafe { crate::DMA_ALLOCATOR.alloc_zeroed(layout) };
        if ptr.is_null() {
            return Err("failed to allocate buffer");
        }
        Ok(Buffer {
            buf: unsafe { slice::from_raw_parts_mut(ptr as *mut u8, size) },
        })
    }
}

impl<'a> Drop for Buffer<'a> {
    fn drop(&mut self) {
        let layout = Layout::from_size_align(self.buf.len(), 8).unwrap();
        unsafe {
            crate::DMA_ALLOCATOR.dealloc(self.buf.as_mut_ptr(), layout);
        }
    }
}

pub struct DirEntry {
    pub name: String,
    pub ext: String,
    pub attibutes: u8,
    pub size: usize,
}

pub struct File<'a> {
    pub cluster: u32,
    pub sector: u8,
    pub pos: usize,
    pub size: Option<usize>,
    pub buffer: &'a mut [u8],
    pub buffer_pos: usize,
}

impl<'a> File<'a> {
    pub fn build(cluster: u32, size: Option<usize>) -> Result<File<'a>> {
        let layout = Layout::from_size_align(BLOCK_SIZE, 8)
            .map_err(|_| "invalid buffer alignment")?;
        let ptr = unsafe { crate::DMA_ALLOCATOR.alloc_zeroed(layout) };
        if ptr.is_null() {
            return Err("failed to allocate buffer");
        }
        Ok(File {
            cluster,
            sector: 0,
            pos: 0,
            size,
            buffer: unsafe { slice::from_raw_parts_mut(ptr as *mut u8, BLOCK_SIZE) },
            buffer_pos: BLOCK_SIZE,
        })
    }

    pub fn get_remaining_bytes(&self) -> usize {
        self.size
            .map(|size| size - self.pos)
            .unwrap_or(0xffff)
    }

    pub fn is_eof(&self) -> bool {
        !(self.cluster > 0
            && self.cluster < 0x0ffffff6
            && self.size.map(|size| self.pos < size)
            .unwrap_or(true))
    }
}

impl<'a> Drop for File<'a> {
    fn drop(&mut self) {
        let layout = Layout::from_size_align(self.buffer.len(), 8).unwrap();
        unsafe {
            crate::DMA_ALLOCATOR.dealloc(self.buffer.as_mut_ptr(), layout);
        }
    }
}

#[allow(unused)]
pub enum FileAttribute {
    Readonly = 0x01,
    Hidden = 0x02,
    System = 0x04,
    Label = 0x08,
    Directory = 0x10,
    Archive = 0x20,
    Device = 0x40,
    Normal = 0x80,
}

enum Mode {
    Fat16,
    Fat32,
}

pub struct Fat32 {
    device: Sd,
    mode: Mode,
    label: String,
    bytes_per_sector: u16,
    sectors_per_cluster: u8,
    reserved_sectors: u16,
    hidden_sectors: u32,
    total_sectors: u32,
    sectors_per_fat: u32,
    data_start_sector: u32,
    root_dir_cluster: u32,
    partition_lba: u32,
}

impl Fat32 {
    pub fn mount(device: Sd, partition_num: usize) -> Result<Fat32> {
        let mut buffer = Buffer::build(BLOCK_SIZE)?;
        info!("Reading boot sector ...");
        device.read(0, 1, &mut buffer.buf)?;
        let bs: BootSector = unsafe { ptr::read(buffer.buf.as_ptr() as *const _) };
        let partition_lba = if bs.jmp_boot[0] == 0xe9 || bs.jmp_boot[0] == 0xeb {
            0
        } else {
            info!("Reading MBR ...");
            let mbr: MasterBootRecord = unsafe { ptr::read(buffer.buf.as_ptr() as *const _) };
            // check if we have valid MBR
            if mbr.boot_signature != 0xaa55 {
                return Err("invalid MBR signature");
            }
            // check if the partition is FAT16 or FAT32 with LBA
            let partition = &mbr.partition_entries[partition_num];
            if partition.partition_type != 0x0e && partition.partition_type != 0x0c {
                return Err("wrong partition type");
            }
            let partition_lba = partition.first_sector;
            // read BPB
            device.read(partition_lba, 1, &mut buffer.buf)?;
            let bs: BootSector = unsafe { ptr::read(buffer.buf.as_ptr() as *const _) };
            if bs.jmp_boot[0] != 0xe9 && bs.jmp_boot[0] != 0xeb {
                return Err("invalid boot sector");
            }
            partition_lba
        };
        let bpb: BiosParamBlock = unsafe { ptr::read(buffer.buf.as_ptr() as *const _) };
        // "Determine (once) SSA=RSC+FN×SF+ceil((32×RDE)/SS), where the reserved sector count RSC
        // is stored at offset 0x00E, the number of FATs FN at offset 0x010, the sectors per FAT SF
        // at offset 0x016 (FAT12/FAT16) or 0x024 (FAT32), the root directory entries RDE
        // at offset 0x011, the sector size SS at offset 0x00B, and ceil(x) rounds up
        // to a whole number."
        let fat = if bpb.fat_size_16 == 0 && bpb.root_entry_count == 0 {
            info!("Found FAT32 ...");
            let ebpb: ExtBiosParamBlock32 = unsafe { ptr::read(buffer.buf.as_ptr() as *const _) };
            let ssa = bpb.reserved_sector_count as u32
                + (bpb.num_of_fats as u32 * ebpb.fat_size_32);
            Fat32 {
                device,
                mode: Mode::Fat32,
                label: util::to_str(ebpb.volume_label.as_ref())?.to_owned(),
                bytes_per_sector: bpb.bytes_per_sector,
                sectors_per_cluster: bpb.sectors_per_cluster,
                reserved_sectors: bpb.reserved_sector_count,
                hidden_sectors: bpb.hidden_sectors,
                total_sectors: bpb.total_sectors,
                sectors_per_fat: ebpb.fat_size_32,
                data_start_sector: ssa,
                root_dir_cluster: ebpb.root_cluster,
                partition_lba,
            }
        } else {
            info!("Found FAT16 ...");
            let ebpb: ExtBiosParamBlock = unsafe { ptr::read(buffer.buf.as_ptr() as *const _) };
            let mut ssa = bpb.reserved_sector_count as u32
                + (bpb.num_of_fats as u32 * bpb.fat_size_16 as u32);
            let root_size = bpb.root_entry_count as usize * mem::size_of::<DirectoryEntry>();
            ssa += (root_size as u32 + bpb.bytes_per_sector as u32 - 1) / bpb.bytes_per_sector as u32;
            Fat32 {
                device,
                mode: Mode::Fat16,
                label: util::to_str(ebpb.volume_label.as_ref())?.to_owned(),
                bytes_per_sector: bpb.bytes_per_sector,
                sectors_per_cluster: bpb.sectors_per_cluster,
                reserved_sectors: bpb.reserved_sector_count,
                hidden_sectors: bpb.hidden_sectors,
                total_sectors: bpb.total_sectors,
                sectors_per_fat: bpb.fat_size_16 as u32,
                data_start_sector: ssa,
                root_dir_cluster: 2,
                partition_lba,
            }
        };
        Ok(fat)
    }

    pub fn info(&self) {
        let mode = match &self.mode {
            Mode::Fat16 => "FAT16",
            Mode::Fat32 => "FAT32",
        };
        info!("Filesystem type: {}", mode);
        info!("Disk label: {}", self.label);
        info!("Total sectors: {}", self.total_sectors);
        info!("Disk size: {}", self.total_sectors * self.bytes_per_sector as u32);
        info!("Bytes per sector: {}", self.bytes_per_sector);
        info!("Sectors per cluster: {}", self.sectors_per_cluster);
        info!("Reserved sectors: {}", self.reserved_sectors);
        info!("Hidden sectors: {}", self.hidden_sectors);
        info!("Sectors per FAT: {}", self.sectors_per_fat);
        info!("FAT size: {}", self.sectors_per_fat * self.bytes_per_sector as u32);
        info!("Data start sector: {}", self.data_start_sector);
        info!("Root directory cluster: {}", self.root_dir_cluster);
        info!("Partition start sector: {}", self.partition_lba);
    }

    pub fn open(&self, path: &str) -> Result<File> {
        let mut root = File::build(self.root_dir_cluster, None)?;
        if let Some(entry) = self.find_dir_entry(&mut root, path)? {
            if entry.is_dir() {
                return Err("eisdir");
            }
            let file = File::build(entry.get_cluster(), entry.get_size())?;
            Ok(file)
        } else {
            Err("enoent")
        }
    }

    pub fn read(&self, file: &mut File, buffer: &mut [u8]) -> Result<usize> {
        let mut read = 0;
        while read < buffer.len() && !file.is_eof() {
            // Fill internal buffer if empty
            if file.buffer_pos >= file.buffer.len() {
                let lsn = self.to_lsn(file.cluster) + file.sector as u32;
                self.device.read(self.to_lba(lsn), 1, &mut file.buffer)?;
                file.buffer_pos = 0;
                file.sector += 1;
                if file.sector >= self.sectors_per_cluster {
                    file.cluster = self.fetch_fat_entry(file.cluster)?;
                    file.sector = 0;
                }
            }
            // Copy available or remaining bytes
            let available = file.buffer.len() - file.buffer_pos;
            let remaining = cmp::min(buffer.len() - read, file.get_remaining_bytes());
            let len = cmp::min(available, remaining);
            let src = &file.buffer[file.buffer_pos..file.buffer_pos + len];
            buffer[read..read + len].copy_from_slice(src);
            file.buffer_pos += len;
            file.pos += len;
            read += len;
        }
        Ok(read)
    }

    pub fn read_dir(&self, path: &str) -> Result<Vec<DirEntry>> {
        let mut dir = self.open_dir(path)?;
        let mut entries = Vec::new();
        while let Some(entry) = self.find_dir_entry(&mut dir, "*")? {
            entries.push(DirEntry {
                name: entry.get_file_name()?,
                ext: entry.get_ext()?,
                attibutes: entry.attributes,
                size: entry.get_size().unwrap_or(0),
            });
        }
        Ok(entries)
    }

    fn fetch_fat_entry(&self, cluster: u32) -> Result<u32> {
        let mut buffer = Buffer::build(BLOCK_SIZE)?;
        let entry_size = match &self.mode {
            Mode::Fat32 => 4,
            Mode::Fat16 => 2,
        };
        let entry_sector = self.reserved_sectors as u32
            + cluster * entry_size / self.bytes_per_sector as u32;
        let entry_offset = (cluster * entry_size % self.bytes_per_sector as u32) as usize;
        self.device.read(self.to_lba(entry_sector), 1, &mut buffer.buf)?;
        let result = match &self.mode {
            Mode::Fat32 => {
                unsafe { ptr::read::<u32>(buffer.buf[entry_offset..].as_ptr() as *const _) }
            },
            Mode::Fat16 => {
                let result = unsafe { ptr::read::<u16>(buffer.buf[entry_offset..].as_ptr() as *const _) };
                result as u32
            },
        };
        Ok(result)
    }

    fn find_dir_entry(&self, file: &mut File, path: &str) -> Result<Option<DirectoryEntry>> {
        let mut buffer = [0; mem::size_of::<DirectoryEntry>()];
        let path_parts: Vec<&str> = path.split("/").collect();
        let mut path_components = &path_parts[0..];
        while self.read(file, &mut buffer)? == buffer.len() {
            let entry: DirectoryEntry = unsafe { ptr::read(buffer.as_ptr() as *const _) };
            if entry.is_empty() {
                break;
            }
            if entry.is_deleted() || entry.is_label() {
                continue;
            }
            if util::match_pattern(&entry.get_file_name()?, path_components[0]) {
                if path_components.len() == 1 {
                    return Ok(Some(entry.clone()));
                } else {
                    if entry.is_dir() {
                        file.cluster = entry.get_cluster();
                        file.sector = 0;
                        file.pos = 0;
                        file.buffer_pos = BLOCK_SIZE;
                        path_components = &path_components[1..];
                    } else {
                        break;
                    }
                }
            }
        }
        Ok(None)
    }

    fn open_dir(&self, path: &str) -> Result<File> {
        let mut root = File::build(self.root_dir_cluster, None)?;
        if path == "." || path == "/" {
            Ok(root)
        } else {
            if let Some(entry) = self.find_dir_entry(&mut root, path)? {
                if !entry.is_dir() {
                    return Err("enotdir");
                }
                let file = File::build(entry.get_cluster(), entry.get_size())?;
                Ok(file)
            } else {
                Err("enoent")
            }
        }
    }

    fn to_lba(&self, sector: u32) -> u32 {
        self.partition_lba + sector
    }

    fn to_lsn(&self, cluster: u32) -> u32 {
        // "A simple formula translates a volume's given cluster number CN
        // to a logical sector number LSN:
        // Determine LSN=SSA+(CN−2)×SC, where the sectors per cluster SCare stored at offset 0x00D."
        match &self.mode {
            Mode::Fat16 if cluster == 2 =>
                self.data_start_sector - 32 + (cluster - 2) * self.sectors_per_cluster as u32,
            _ =>
                self.data_start_sector + (cluster - 2) * self.sectors_per_cluster as u32,
        }
    }
}

mod util {
    use super::Result;
    use alloc::prelude::*;

    pub fn match_pattern(name: &String, pattern: &str) -> bool {
        if pattern == "*" {
            true
        } else {
            name.eq_ignore_ascii_case(pattern)
        }
    }

    pub fn to_str(buffer: &[u8]) -> Result<&str> {
        core::str::from_utf8(buffer.as_ref())
            .map_err(|_| "invalid utf8")
    }

    pub fn to_str_trimmed(buffer: &[u8]) -> Result<&str> {
        let mut end = buffer.len();
        for (i, byte) in buffer.iter().enumerate() {
            if *byte == 0x20 {
                end = i;
                break;
            }
        }
        core::str::from_utf8(&buffer[0..end])
            .map_err(|_| "invalid utf8")
    }
}
