#![no_main]
#![no_std]

extern crate alloc;
use alloc::vec::Vec;
use alloc::{format, vec};
use uefi::prelude::*;
use uefi::proto::console::gop::{BltOp, BltPixel, BltRegion, GraphicsOutput};
use uefi::Result;

struct Buffer {
    width: usize,
    height: usize,
    pixels: Vec<BltPixel>,
}

impl Buffer {
    fn new(width: usize, height: usize) -> Self {
        Buffer {
            width,
            height,
            pixels: vec![BltPixel::new(0, 0, 0); width * height],
        }
    }

    fn pixel(&mut self, x: usize, y: usize) -> Option<&mut BltPixel> {
        self.pixels.get_mut(y * self.width + x)
    }

    fn blit(&self, gop: &mut GraphicsOutput) -> Result {
        gop.blt(BltOp::BufferToVideo {
            buffer: &self.pixels,
            src: BltRegion::Full,
            dest: (0, 0),
            dims: (self.width, self.height),
        })
    }

    fn blit_pixel(&self, gop: &mut GraphicsOutput, coords: (usize, usize)) -> Result {
        gop.blt(BltOp::BufferToVideo {
            buffer: &self.pixels,
            src: BltRegion::SubRectangle {
                coords,
                px_stride: self.width,
            },
            dest: coords,
            dims: (1, 1),
        })
    }
}

use uefi::fs::FileSystem;
use uefi::proto::media::fs::SimpleFileSystem;
use uefi::table::boot::ScopedProtocol;
use uefi::{println, CString16};

fn parse_bmp(raw_data: &[u8]) -> Result<Buffer> {
    let width = u32::from_le_bytes(raw_data[18..22].try_into().unwrap()) as usize;
    let height = u32::from_le_bytes(raw_data[22..26].try_into().unwrap()) as usize;
    let bits_per_pixel = u16::from_le_bytes(raw_data[28..30].try_into().unwrap());

    if bits_per_pixel != 24 {
        panic!("Image format not supported!");
    }

    let pixel_data_offset = u32::from_le_bytes(raw_data[10..14].try_into().unwrap()) as usize;
    let pixel_data = &raw_data[pixel_data_offset..];

    let mut buffer = Buffer::new(width, height);

    for y in 0..height {
        for x in 0..width {
            let offset = (y * width + x) * 3;
            let b = pixel_data[offset + 0];
            let g = pixel_data[offset + 1];
            let r = pixel_data[offset + 2];

            let pixel = BltPixel::new(r, g, b);
            buffer.pixels[y * width + x] = pixel;
        }
    }

    Ok(buffer)
}

#[entry]
fn main(_image_handle: Handle, mut system_table: SystemTable<Boot>) -> Status {
    uefi::helpers::init(&mut system_table).unwrap();
    system_table
        .boot_services()
        .set_watchdog_timer(0, 80000, None)
        .expect("Failed to set watchdog timer.");

    let bs = system_table.boot_services();

    println!("Welcome to Hentai OS");
    println!("-------------------------------------------------------------");

    let fs: ScopedProtocol<SimpleFileSystem> = bs
        .get_image_file_system(bs.image_handle())
        .expect("Failed to access filesystem.");

    let mut fs = FileSystem::new(fs);

    let path: CString16 = CString16::try_from("images").unwrap();

    let mut image_buffers: Vec<Buffer> = Vec::new();

    for file in fs.read_dir(path.as_ref()).unwrap() {
        let file = file.unwrap();

        if file.is_directory() {
            continue;
        }

        let filename = file.file_name();
        let filepath = CString16::try_from(format!("{path}\\{filename}").as_str()).unwrap();
        println!("Reading {filepath}");
        let raw_image_data = fs.read(filepath.as_ref()).expect("Failed to read image");
        let buffer = parse_bmp(&raw_image_data).expect("Failed to parse BMP data");

        image_buffers.push(buffer);
    }

    if image_buffers.is_empty() {
        println!("No images found!");
        return Status::NOT_FOUND;
    }

    let gop_handle = bs
        .get_handle_for_protocol::<GraphicsOutput>()
        .expect("Failed to get graphics output protocol handle.");

    let mut gop = bs
        .open_protocol_exclusive::<GraphicsOutput>(gop_handle)
        .expect("Failed to open graphics output protocol.");

    let mut image_cycle = image_buffers.iter().cycle();

    loop {
        if let Some(image) = image_cycle.next() {
            image
                .blit(&mut gop)
                .expect("Failed to blit buffer to video");

            bs.stall(3_000_000);
        }
    }
}