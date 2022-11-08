use structopt::StructOpt;

use std::fs::{self, OpenOptions};
use std::io::{Error, ErrorKind, Read, Write};
use std::thread;
use std::time::Duration;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "kbsim",
    about = "Simulates a HID keyboard by writing a string to a HID device file. It supports a number of different keyboard layouts."
)]
struct CliOpt {
    #[structopt(
        long = "hid-file",
        short = "f",
        help = "The HID file to write to. Defaults to /dev/hidg0"
    )]
    hid_file: Option<String>,
    #[structopt(
        long = "layout",
        short = "l",
        help = "The keyboard layout to use. Specify 'list' to show all available layouts",
        default_value = "LAYOUT_UNITED_KINGDOM"
    )]
    layout: String,
    #[structopt(
        long = "newline",
        short = "n",
        help = "Hit the 'Enter' key after writing the string"
    )]
    newline: bool,
    #[structopt(
        long = "delay",
        short = "d",
        help = "Specify the number of seconds to wait before writing",
        default_value = "0"
    )]
    delay: u64,
    #[structopt(
        long = "cooldown",
        short = "c",
        help = "Specify the number of milliseconds to wait between sending each HID packet to the device file",
        default_value = "0"
    )]
    cooldown: u64,
    #[structopt(name = "STRING")]
    string: Option<String>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let CliOpt {
        hid_file,
        layout,
        newline,
        delay,
        cooldown,
        string,
    } = CliOpt::from_args();

    if layout.to_lowercase() == "list" {
        for l in keyboard_layouts::available_layouts() {
            println!("{}", l);
        }
        return Ok(());
    }

    let hid_file = hid_file.unwrap_or_else(|| "/dev/hidg0".to_string());
    if let Some(mut string) = string {
        if newline {
            string.push('\n');
        }

        let hid_bytes = keyboard_layouts::string_to_hid_packets(&layout, &string)
            .map_err(|e| Error::new(ErrorKind::Other, format!("{}", e)))?;

        thread::sleep(Duration::from_secs(delay));

        for packet in hid_bytes.chunks(keyboard_layouts::HID_PACKET_LEN) {
            fs::write(&hid_file, packet)?;
            thread::sleep(Duration::from_millis(cooldown));
        }
    } else {
        eprintln!("Reading from stdin");

        let mut hid_file = OpenOptions::new().write(true).open(hid_file)?;

        let mut term = terminal::stdout();
        term.act(terminal::Action::EnableRawMode)?;

        let mut stdin = std::io::stdin();
        let mut buf = [0u8; 128];
        while let Ok(n) = stdin.read(&mut buf[..]) {
            if n == 0 {
                continue;
            }
            if buf.contains(&3) {
                // Break on ctrl+c
                break;
            }
            match std::str::from_utf8(&buf[..n]) {
                Ok(text) => {
                    term.write(&buf[..n])?;
                    term.flush()?;
                    let hid_bytes = keyboard_layouts::string_to_hid_packets(&layout, text)
                        .map_err(|e| Error::new(ErrorKind::Other, format!("{}", e)))?;

                    for packet in hid_bytes.chunks(keyboard_layouts::HID_PACKET_LEN) {
                        hid_file.write(packet)?;
                        hid_file.flush()?;
                    }
                }
                Err(e) => eprintln!("Could not decode character {} {:?}", e, &buf),
            };
        }
        term.act(terminal::Action::DisableRawMode)?;
    }

    Ok(())
}
