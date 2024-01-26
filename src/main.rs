use clap::Parser;
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;

#[derive(Parser)]
#[command(name = "", no_binary_name = true)] // This name will show up in clap's error messages, so it is important to set it to "".
enum Command {
    Test {
        arg: Option<String>,
    },
    List(ListCommand),
    #[clap(name = "pwm_set")]
    PwmSet,
}

#[derive(Parser)]
struct ListCommand {
    /// An argument for the list command
    #[clap(long)]
    arg: Option<String>,
}

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
                            Command::PwmSet => {
                                println!("Setting PWM");
                                // Here you would actually list the values
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
