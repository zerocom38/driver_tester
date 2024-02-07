use std::fs::{File, OpenOptions};
use std::io::Write;
use std::time::Duration;

use clap::Parser;
use drm::buffer::DrmFourcc;
use drm::control::{connector, crtc};
use nix::fcntl::OFlag;
use nix::sys::inotify::{AddWatchFlags, InitFlags, Inotify};
use nix::sys::stat::Mode;
use nix::{ioctl_read, ioctl_write_ptr};
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;

use nix::fcntl::open;

pub use drm::control::Device as ControlDevice;
pub use drm::Device;

use sysfs_gpio::{Direction, Pin};

#[derive(Parser)]
#[command(name = "", no_binary_name = true)] // This name will show up in clap's error messages, so it is important to set it to "".
enum Command {
    Test {
        arg: Option<String>,
    },
    List(ListCommand),
    #[clap(name = "pwm_set")]
    PwmSet(PwmSetCommand),
    #[clap(name = "pwm_get")]
    PwmGet,
    DmaSend,
    Drm,
    GpioSet,
}

#[derive(Parser)]
struct ListCommand {
    /// An argument for the list command
    #[clap(long)]
    arg: Option<String>,
}

#[derive(Parser)]
struct PwmSetCommand {
    #[clap(short, long, value_parser=clap_num::maybe_hex::<u32>)]
    value: u32,
}

const PWM_MODULE: u8 = b'p'; // Defined in linux/spi/spidev.h
ioctl_read!(pwm_get_pwm, PWM_MODULE, 2, u32);
ioctl_write_ptr!(pwm_set_pwm, PWM_MODULE, 1, u32);

fn main() {
    std::thread::spawn(|| {
        let instance = Inotify::init(InitFlags::empty()).unwrap();

        // We add a new watch on directory "test" for all events.
        let wd = instance
            .add_watch(
                "/sys/class/qube/dummy/device/checksum",
                AddWatchFlags::IN_ALL_EVENTS,
            )
            .unwrap();

        loop {
            // We read from our inotify instance for events.
            let events = instance.read_events().unwrap();
            println!("Events: {:?}", events);
        }
    });

    let gpio = Pin::new(444);
    match gpio.export() {
        Ok(()) => {
            println!("Gpio {} exported!", gpio.get_pin());
            gpio.set_direction(Direction::Out).unwrap();
        }
        Err(err) => println!("Gpio {} could not be exported: {}", gpio.get_pin(), err),
    }

    let mut rl = DefaultEditor::new().unwrap();
    #[cfg(feature = "with-file-history")]
    if rl.load_history("history.txt").is_err() {
        println!("No previous history.");
    }
    loop {
        let readline = rl.readline(">> ");
        match readline {
            Ok(l) => {
                let line = l.trim();
                if line.is_empty() {
                    continue;
                }
                rl.add_history_entry(line).unwrap();
                let line_split = shlex::split(&line).unwrap();

                let res = Command::try_parse_from(line_split);
                match res {
                    Ok(cli) => {
                        match cli {
                            Command::Test { arg } => {
                                println!("Running test with argument: {:?}", arg);
                                // Here you would actually run the test
                            }
                            Command::List(list_command) => {
                                println!("Listing with argument: {:?}", list_command.arg);
                                // Here you would actually list the values
                            }
                            Command::PwmGet => {
                                println!("Getting PWM");
                                let file =
                                    open("/dev/dummy_sink", OFlag::O_RDWR, Mode::empty()).unwrap();

                                // Prepare a place for the ioctl result
                                let mut result: u32 = 0;

                                // Send the ioctl command
                                let ret = unsafe { pwm_get_pwm(file, &mut result).unwrap() };
                                if ret == -1 {
                                    println!("ioctl failed");
                                } else {
                                    println!("ioctl succeeded, result = {}", result);
                                }
                                // Here you would actually list the values
                            }
                            Command::PwmSet(cmd) => {
                                println!("Setting PWM");
                                let file =
                                    open("/dev/dummy_sink", OFlag::O_RDWR, Mode::empty()).unwrap();

                                // Send the ioctl command
                                let ret = unsafe { pwm_set_pwm(file, &cmd.value).unwrap() };
                                if ret == -1 {
                                    println!("ioctl failed");
                                } else {
                                    println!("ioctl succeeded, result = {}", ret);
                                }
                                // Here you would actually list the values
                            }
                            Command::DmaSend => {
                                let mut file = OpenOptions::new()
                                    .write(true)
                                    .open("/dev/dummy_sink")
                                    .unwrap();

                                let mut data = Vec::new();
                                data.resize(65536, 5u8);
                                file.write(&data).unwrap();
                            }
                            Command::Drm => drm_test(&gpio),
                            Command::GpioSet => {
                                println!("Setting GPIO:");
                                gpio.set_value(0);
                                std::thread::sleep(Duration::from_millis(100));
                                gpio.set_value(1);
                                println!("GPIO is now: {}", gpio.get_value().unwrap());
                            }
                        }
                    }
                    Err(err) => {
                        println!("Failed to parse command: {}", err);
                    }
                }
            }
            Err(ReadlineError::Interrupted) => {
                println!("CTRL-C");
                break;
            }
            Err(ReadlineError::Eof) => {
                println!("CTRL-D");
                break;
            }
            Err(err) => {
                println!("Error: {:?}", err);
                break;
            }
        }
    }
    #[cfg(feature = "with-file-history")]
    rl.save_history("history.txt");
}

fn drm_test(gpio: &Pin) {
    println!("DRM test...");
    let card = Card::open_global();

    // Load the information.
    let res = card
        .resource_handles()
        .expect("Could not load normal resource ids.");
    let coninfo: Vec<connector::Info> = res
        .connectors()
        .iter()
        .flat_map(|con| card.get_connector(*con, true))
        .collect();
    let crtcinfo: Vec<crtc::Info> = res
        .crtcs()
        .iter()
        .flat_map(|crtc| card.get_crtc(*crtc))
        .collect();

    // Filter each connector until we find one that's connected.
    let con = coninfo
        .iter()
        .find(|&i| i.state() == connector::State::Connected)
        .expect("No connected connectors");

    // Get the first (usually best) mode
    let &mode = con.modes().first().expect("No modes found on connector");

    let (disp_width, disp_height) = mode.size();

    println!(
        "Mode: {}x{} -> {}",
        disp_width,
        disp_height,
        mode.name().to_str().unwrap()
    );

    // Find a crtc and FB
    let crtc = crtcinfo.first().expect("No crtcs found");

    // Select the pixel format
    let fmt = DrmFourcc::Rgb888;

    // Create a DB
    // If buffer resolution is larger than display resolution, an ENOSPC (not enough video memory)
    // error may occur
    let mut db = card
        .create_dumb_buffer((disp_width.into(), disp_height.into()), fmt, 24)
        .expect("Could not create dumb buffer");

    // Map it and grey it out.
    {
        let mut map = card
            .map_dumb_buffer(&mut db)
            .expect("Could not map dumbbuffer");
        for b in map.as_mut() {
            *b = 128;
        }
    }
    // Create an FB:
    let fb = card
        .add_framebuffer(&db, 24, 24)
        .expect("Could not create FB");

    // println!("{:#?}", mode);
    // println!("{:#?}", fb);
    // println!("{:#?}", db);

    // Set the crtc
    // On many setups, this requires root access.
    // card.set_crtc(crtc.handle(), Some(fb), (0, 0), &[con.handle()], Some(mode))
    //     .expect("Could not set CRTC");
    card.set_crtc(crtc.handle(), Some(fb), (0, 0), &[con.handle()], Some(mode))
        .expect("Could not set CRTC");

    gpio.set_value(0);
    std::thread::sleep(Duration::from_millis(100));
    gpio.set_value(1);

    println!("GPIO toggled");

    let five_seconds = ::std::time::Duration::from_millis(5000);
    ::std::thread::sleep(five_seconds);

    card.destroy_framebuffer(fb).unwrap();
    card.destroy_dumb_buffer(db).unwrap();
}

#[derive(Debug)]
/// A simple wrapper for a device node.
pub struct Card(std::fs::File);

/// Implementing `AsFd` is a prerequisite to implementing the traits found
/// in this crate. Here, we are just calling `as_fd()` on the inner File.
impl std::os::unix::io::AsFd for Card {
    fn as_fd(&self) -> std::os::unix::io::BorrowedFd<'_> {
        self.0.as_fd()
    }
}

/// With `AsFd` implemented, we can now implement `drm::Device`.
impl Device for Card {}
impl ControlDevice for Card {}

/// Simple helper methods for opening a `Card`.
impl Card {
    pub fn open(path: &str) -> Self {
        let mut options = std::fs::OpenOptions::new();
        options.read(true);
        options.write(true);
        Card(options.open(path).unwrap())
    }

    pub fn open_global() -> Self {
        Self::open("/dev/dri/card0")
    }
}

pub mod capabilities {
    use drm::ClientCapability as CC;
    pub const CLIENT_CAP_ENUMS: &[CC] = &[CC::Stereo3D, CC::UniversalPlanes, CC::Atomic];

    use drm::DriverCapability as DC;
    pub const DRIVER_CAP_ENUMS: &[DC] = &[
        DC::DumbBuffer,
        DC::VBlankHighCRTC,
        DC::DumbPreferredDepth,
        DC::DumbPreferShadow,
        DC::Prime,
        DC::MonotonicTimestamp,
        DC::ASyncPageFlip,
        DC::CursorWidth,
        DC::CursorHeight,
        DC::AddFB2Modifiers,
        DC::PageFlipTarget,
        DC::CRTCInVBlankEvent,
        DC::SyncObj,
        DC::TimelineSyncObj,
    ];
}
