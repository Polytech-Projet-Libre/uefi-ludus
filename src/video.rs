use core::{cell::RefCell, cmp::Ordering, iter};

use alloc::{boxed::Box, vec, vec::Vec};
use log::info;
use uefi::{
    prelude::*,
    proto::console::gop::{BltOp, BltPixel, BltRegion, GraphicsOutput, Mode, PixelFormat},
    table::boot::ScopedProtocol,
};

use ludus::{NES_HEIGHT, NES_WIDTH};

pub struct Video<'a> {
    pixels: Box<[BltPixel]>,
    resolution: (usize, usize),
    scale: (usize, usize),
    gop: RefCell<ScopedProtocol<'a, GraphicsOutput>>,
}

impl ludus::VideoDevice for Video<'_> {
    fn blit_pixels(&mut self, pixels: &ludus::PixelBuffer) {
        for x in 0..NES_WIDTH {
            for y in 0..NES_HEIGHT {
                for (sub_x, sub_y) in iter::zip(0..self.scale.0, 0..self.scale.1) {
                    self.pixels[x * self.scale.0
                        + sub_x
                        + (y * self.scale.1 + sub_y) * self.resolution.0] =
                        pixels.as_ref()[x + y * NES_WIDTH].into();
                }
            }
        }
    }
}

fn is_nes_res_multiple((width, height): (usize, usize)) -> bool {
    ((width % NES_WIDTH) == 0) && ((height % NES_HEIGHT) == 0)
}

impl<'a> Video<'a> {
    pub fn new(bt: &'a BootServices) -> Self {
        let gop_handle = bt.get_handle_for_protocol::<GraphicsOutput>().unwrap();
        let mut gop = bt
            .open_protocol_exclusive::<GraphicsOutput>(gop_handle)
            .unwrap();

        let mut compatible: Vec<Mode> = gop
            .modes(bt)
            .filter(|mode| {
                let info = mode.info();
                let (w, h) = mode.info().resolution();

                w > NES_WIDTH
                    && h > NES_HEIGHT
                    && matches!(info.pixel_format(), PixelFormat::Rgb | PixelFormat::Bgr)
            })
            .collect();

        if compatible.is_empty() {
            panic!("No compatible framebuffer !");
        }

        info!("Compatible framebuffers: {compatible:#?}");

        compatible.sort_by(|a, b| {
            // Smallest resolution, prefer pixel perfect scale.
            let (a_res, b_res) = (a.info().resolution(), b.info().resolution());

            let a_mul = is_nes_res_multiple(a_res);
            let b_mul = is_nes_res_multiple(b_res);

            match (a_mul, b_mul) {
                (true, true) | (false, false) => {
                    // Take smaller res
                    (a_res.0 * a_res.1).cmp(&(b_res.0 * b_res.1))
                }
                (true, false) => Ordering::Greater,
                (false, true) => Ordering::Less,
            }
        });

        let mode = compatible.first().unwrap();

        info!("Choose {mode:?}");

        let x_scale = mode.info().resolution().0 / NES_WIDTH;
        let y_scale = mode.info().resolution().1 / NES_HEIGHT;

        gop.set_mode(mode).unwrap();
        let resolution = mode.info().resolution();

        let pixels = vec![BltPixel::new(0, 0, 0); resolution.0 * resolution.1];

        Self {
            resolution,
            scale: (x_scale, y_scale),
            gop: RefCell::new(gop),
            pixels: pixels.into_boxed_slice(),
        }
    }

    pub fn refresh(&self) {
        self.gop
            .borrow_mut()
            .blt(BltOp::BufferToVideo {
                buffer: &self.pixels,
                src: BltRegion::Full,
                dest: (0, 0),
                dims: self.resolution,
            })
            .unwrap()
    }
}
