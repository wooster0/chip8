mod display;
mod interpreter;
mod util;

use interpreter::Interpreter;
use std::{borrow::Cow, env, fs, io, process};
use terminal::Terminal;

type Error = Cow<'static, str>;

fn main() {
    let exit_code = match run() {
        Ok(()) => 0,
        Err(err) => {
            eprintln!("{}", err);
            1
        }
    };

    process::exit(exit_code);
}

fn get_args() -> env::ArgsOs {
    let mut args = env::args_os();

    args.next(); // This is probably the program name.

    args
}

fn get_binary() -> Result<Vec<u8>, Error> {
    let mut args = get_args();

    if let Some(arg) = args.next() {
        let path = match arg.as_os_str().to_str() {
            Some(path) => path,
            None => return Err("Given argument is not valid UTF-8.".into()),
        };
        let binary = fs::read(path);

        match binary {
            Ok(binary) => Ok(binary),
            Err(err) => {
                use io::ErrorKind::*;

                let err = match err.kind() {
                    PermissionDenied => "No permission to read binary.",
                    NotFound => "Binary was not found.",
                    _ => "Failed to read binary.",
                };

                Err(err.into())
            }
        }
    } else {
        Err("No path to the binary given.".into())
    }
}

// fn get_binary() -> Result<Vec<u8>, &'static str> {
//     let file = get_fvile()?;

//     let capacity = get_file_capacity(file);
//     let binary = Vec::<u8>::with_capacity(capacity);

//     file.read

//     Ok(binary)
// }

fn run() -> Result<(), Error> {
    let binary = get_binary()?;

    let stdout = io::stdout();

    let mut terminal = match Terminal::new(stdout.lock()) {
        Ok(mut terminal) => {
            terminal.initialize(Some("CHIP-8"), false);
            terminal.flush();
            terminal
        }
        Err(_) => {
            return Err("This is not a terminal.".into());
        }
    };

    await_fitting_window_width(&mut terminal);
    await_fitting_window_height(&mut terminal);

    let mut interpreter = Interpreter::new(binary)?;

    let result = interpreter.run(&mut terminal);

    terminal.reset_cursor();
    terminal.write("Program ended. Press any key to continue.");
    terminal.flush();

    crate::read_event(&mut terminal);

    terminal.deinitialize();
    terminal.flush();

    result
}

fn get_size_message(size: &str) -> String {
    format!("Please increase your window {}", size)
}

use terminal::event::{Event, Key};

pub fn exit(terminal: &mut Terminal) -> ! {
    terminal.deinitialize();
    terminal.flush();
    process::exit(0);
}

pub fn read_event(terminal: &mut Terminal) -> Option<Event> {
    let event = terminal.read_event();
    if let Some(Event::Key(Key::Esc)) = event {
        exit(terminal)
    } else {
        event
    }
}

fn await_window_resize(terminal: &mut Terminal) {
    loop {
        let event = read_event(terminal);
        if let Some(Event::Resize) = event {
            break;
        }
    }
}

fn window_size_alert(terminal: &mut Terminal, size: &str) {
    terminal.reset_cursor();
    terminal.write(&get_size_message(size));
    terminal.flush();
    await_window_resize(terminal);
}

pub fn await_fitting_window_width(terminal: &mut Terminal) {
    while terminal.size.width < display::SIZE.width * 2 {
        window_size_alert(terminal, "width");
    }
    //  terminal.clear();
}

pub fn await_fitting_window_height(terminal: &mut Terminal) {
    while terminal.size.height < display::SIZE.height {
        window_size_alert(terminal, "height");
    }
    // terminal.clear();
}
