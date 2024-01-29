#![no_main]
#![no_std]

extern crate alloc;

mod audio;
mod input;
mod video;

use log::{info, warn};
use uefi::{
    fs,
    prelude::*,
    proto::console::text::{Input, Key, ScanCode},
};

use ludus::{ButtonState, Cart, Console};

#[entry]
fn main(image: Handle, mut system_table: SystemTable<Boot>) -> Status {
    uefi_services::init(&mut system_table).unwrap();
    let bt = system_table.boot_services();
    let mut filesystem = fs::FileSystem::new(bt.get_image_file_system(image).unwrap());

    let cart_raw = filesystem.read(cstr16!("game.nes")).unwrap();
    let cart = Cart::from_bytes(&cart_raw).unwrap();

    let mut console = Console::new(cart, audio::SAMPLE_RATE);

    let mut video = video::Video::new(bt);
    let mut audio = audio::Audio;

    let input_handle = bt.get_handle_for_protocol::<Input>().unwrap();
    let mut input = bt.open_protocol_exclusive::<Input>(input_handle).unwrap();

    'l: loop {
        let mut buttons = ButtonState::default();

        while let Some(key) = input.read_key().unwrap() {
            match key {
                Key::Printable(c) if 122u16.eq(&(c.into())) => buttons.a = true, // W
                Key::Printable(c) if 120u16.eq(&(c.into())) => buttons.b = true, // X
                Key::Printable(c) if 13u16.eq(&(c.into())) => buttons.start = true, // enter
                Key::Printable(c) if 8u16.eq(&(c.into())) => buttons.select = true, // escape
                Key::Special(ScanCode::UP) => buttons.up = true,
                Key::Special(ScanCode::DOWN) => buttons.down = true,
                Key::Special(ScanCode::LEFT) => buttons.left = true,
                Key::Special(ScanCode::RIGHT) => buttons.right = true,
                Key::Special(ScanCode::ESCAPE) => break 'l,
                Key::Printable(c) => {
                    info!("Unknown key: {}", Into::<u16>::into(c));
                }
                Key::Special(code) => {
                    warn!("Unknown scancode: {code:?}")
                }
            }
        }

        console.update_controller(buttons);

        console.step_frame(&mut audio, &mut video);
        video.refresh();

        // 50 fps (PAL)
        system_table.boot_services().stall(1_000_000 / 50);
    }

    Status::SUCCESS
}
