extern crate rbot_parser as parser;
extern crate unix_socket;
extern crate toml;

use unix_socket::UnixStream;
use std::io::{BufRead,BufReader,Write};
use std::process::Command;
use std::thread;
use std::sync::Arc;
use std::str::FromStr;

mod config;
use config::{Config, Plugin};

mod error;
use error::Error;

const TYPE: usize = 0;
const RETRIES: usize = 10;

fn main() {
    let config = match Config::new() {
        Ok(conf) => conf,
        Err(e) => {
            println!("{}", e);
            ::std::process::exit(1)
        }
    };
    println!("{:?}", config);

    let plugins = Arc::new(config.plugins);
    let mut threads = vec![];

    for socket in config.sockets {
        let plgs = plugins.clone();
        threads.push(thread::spawn(move || {
            let mut conn = match UnixStream::connect(&socket) {
                Ok(conn) => BufReader::new(conn),
                Err(e) => {
                    println!("{}", e.to_string());
                    match retry_connection(&socket) {
                        Some(conn) => conn,
                        None => ::std::process::exit(1)
                    }
                }
            };
            let mut line = String::new();
            loop {
                if let Ok(bytes) = conn.read_line(&mut line) {
                    if bytes == 0 {
                        break;
                    }
                    let splitted: Vec<&str> = line.split_whitespace().collect();
                    if splitted.len() == 0 {
                        continue;
                    }
                    let cmd = match FromStr::from_str(splitted[TYPE]) {
                        Ok(cmd) => parser::Command::Numeric(cmd),
                        Err(_) => parser::Command::Named(splitted[TYPE].into())
                    };
                    let plugin_list = plgs.iter()
                                          .filter(|p| p.command
                                                       .iter()
                                                       .find(|&c| *c == cmd)
                                                       .is_some())
                                          .collect::<Vec<&Plugin>>();
                    for plugin in plugin_list {
                        match run_plugin(plugin, &line)
                                  .and_then(|out| send_plugin_reply(conn.get_mut(), &out)) {
                            Err(Error::Plugin) => {},
                            Err(e) => println!("{}", e),
                            _ => {}
                        }
                    }
                }
                else {
                    println!("Could not read the socket");
                    break;
                }
                line.clear();
            }
        }));
    }
    for t in threads {
        let _ = t.join();
    }
}

fn retry_connection(socket: &str) -> Option<BufReader<UnixStream>> {
    for _ in 0..RETRIES {
        println!("Retrying connection...");
        if let Ok(conn) = UnixStream::connect(socket) {
            return Some(BufReader::new(conn));
        }
        else {
            thread::sleep_ms(1000);
        }
    }
    None
}

fn run_plugin(plugin: &Plugin, line: &String) -> Result<String, Error> {
    if plugin.trigger.is_none() || plugin.trigger.as_ref().unwrap().is_match(line) {
        println!("Running plugin {}", plugin.executable);
        let output = match Command::new(&plugin.executable).output() {
            Ok(output) => output.stdout,
            Err(e) => {
                return Err(e.into())
            }
        };
        return Ok(try!(String::from_utf8(output)));
    }
    Err(Error::Plugin)
}

fn send_plugin_reply(s: &mut UnixStream, output: &str) -> Result<(), Error> {
    Ok(try!(s.write(output.trim_right().as_ref())
        .and_then(|_| s.write(b"\r\n"))
        .and_then(|_| s.flush())))
}
