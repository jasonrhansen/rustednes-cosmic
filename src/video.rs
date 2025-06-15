use std::mem;

use rustednes_core::sink::{VideoSink, XRGB8888_PALETTE};

pub struct VideoFrameSink<'a> {
    pixels: &'a mut [u8],
    frame_written: bool,
}

impl<'a> VideoFrameSink<'a> {
    pub fn new(pixels: &'a mut [u8]) -> Self {
        VideoFrameSink {
            pixels,
            frame_written: false,
        }
    }
}

impl<'a> VideoSink for VideoFrameSink<'a> {
    fn write_frame(&mut self, frame_buffer: &[u8]) {
        for (i, palette_index) in frame_buffer.iter().enumerate() {
            let pixel = XRGB8888_PALETTE[*palette_index as usize];
            let offset = i * 4;

            self.pixels[offset] = (pixel >> 16) as u8;
            self.pixels[offset + 1] = (pixel >> 8) as u8;
            self.pixels[offset + 2] = pixel as u8;
            self.pixels[offset + 3] = 0x77;
        }
        self.frame_written = true;
    }

    fn frame_written(&self) -> bool {
        self.frame_written
    }

    fn pixel_size(&self) -> usize {
        mem::size_of::<u32>()
    }
}
