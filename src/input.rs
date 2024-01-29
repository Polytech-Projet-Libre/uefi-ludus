use ludus::ButtonState;
use pc_keyboard::{
    layouts::{self, Azerty}, HandleControl, KeyCode, KeyEvent, KeyState, ScancodeSet1
};
use ps2::{error::ControllerError, flags::ControllerConfigFlags, Controller};

// Initialization as documented by ps2-rs library.
// Based on OSDev https://web.archive.org/web/20201112021519/https://wiki.osdev.org/%228042%22_PS/2_Controller#Initialising_the_PS.2F2_Controller
fn initialize() -> Result<Controller, ControllerError> {
    let mut controller = unsafe { Controller::new() };

    // Step 3: Disable devices
    controller.disable_keyboard()?;
    controller.disable_mouse()?;

    // Step 4: Flush data buffer
    let _ = controller.read_data();

    // Step 5: Set config
    let mut config = controller.read_config()?;
    // Disable interrupts and scancode translation
    config.set(
        ControllerConfigFlags::ENABLE_KEYBOARD_INTERRUPT
            | ControllerConfigFlags::ENABLE_MOUSE_INTERRUPT
            | ControllerConfigFlags::ENABLE_TRANSLATE,
        false,
    );
    controller.write_config(config)?;

    // Step 6: Controller self-test
    controller.test_controller()?;
    // Write config again in case of controller reset
    controller.write_config(config)?;

    // Step 7: Determine if there are 2 devices
    let has_mouse = if config.contains(ControllerConfigFlags::DISABLE_MOUSE) {
        controller.enable_mouse()?;
        config = controller.read_config()?;
        // If mouse is working, this should now be unset
        !config.contains(ControllerConfigFlags::DISABLE_MOUSE)
    } else {
        false
    };
    // Disable mouse. If there's no mouse, this is ignored
    controller.disable_mouse()?;

    // Step 8: Interface tests
    let keyboard_works = controller.test_keyboard().is_ok();
    let mouse_works = has_mouse && controller.test_mouse().is_ok();

    // Step 9 - 10: Enable and reset devices
    config = controller.read_config()?;
    if keyboard_works {
        controller.enable_keyboard()?;
        config.set(ControllerConfigFlags::DISABLE_KEYBOARD, false);
        config.set(ControllerConfigFlags::ENABLE_KEYBOARD_INTERRUPT, true);
        controller.keyboard().reset_and_self_test().unwrap();
    }
    if mouse_works {
        controller.enable_mouse()?;
        config.set(ControllerConfigFlags::DISABLE_MOUSE, false);
        config.set(ControllerConfigFlags::ENABLE_MOUSE_INTERRUPT, true);
        controller.mouse().reset_and_self_test().unwrap();
        // This will start streaming events from the mouse
        controller.mouse().enable_data_reporting().unwrap();
    }

    // Write last configuration to enable devices and interrupts
    controller.write_config(config)?;

    Ok(controller)
}

pub struct Input<'a> {
    pub buttons: ButtonState,
    controller: &'a mut Controller,
    keyboard: pc_keyboard::Keyboard<Azerty, ScancodeSet1>,
}

impl Input<'a> {
    pub unsafe fn new(controller: &'a mut Controller) -> Self {
        Self {
            controller,
            keyboard: pc_keyboard::Keyboard::new(
                ScancodeSet1::new(),
                Azerty,
                HandleControl::Ignore,
            ),
            buttons: ButtonState::default()
        }
    }

    pub fn process_event(&mut self, event: KeyEvent) {
        match event.code {
            KeyCode::W => self.buttons.a = matches!(event.state, KeyState::Down),
            KeyCode::X => self.buttons.b = matches!(event.state, KeyState::Down),
            KeyCode::Return => self.buttons.start = matches!(event.state, KeyState::Down),
            KeyCode::Backspace => self.buttons.select = matches!(event.state, KeyState::Down),
            KeyCode::ArrowUp => self.buttons.up = matches!(event.state, KeyState::Down),
            KeyCode::ArrowDown => self.buttons.down = matches!(event.state, KeyState::Down),
            KeyCode::ArrowLeft => self.buttons.left = matches!(event.state, KeyState::Down),
            KeyCode::ArrowRight => self.buttons.right = matches!(event.state, KeyState::Down),
            _ => {
                info!("Unknown key {}", event.code);
            }
        }
    }

    pub fn poll(&mut self) {
        while Ok(data) = self.controller.read_data() {
            if let Ok(Some(event)) = self.keyboard.add_byte(data) {
                self.process_event(event)
            }
        }
    }
}
