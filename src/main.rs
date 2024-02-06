use std::fs::{File, OpenOptions};
use std::io::Write;

use clap::Parser;
use nix::fcntl::OFlag;
use nix::sys::stat::Mode;
use nix::{ioctl_read, ioctl_write_ptr};
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;

use nix::fcntl::open;
use nix::sys::ioctl;

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
                                data.resize(65536, 0u8);
                                file.write(&data).unwrap();
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
