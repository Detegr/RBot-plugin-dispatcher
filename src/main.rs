extern crate rbot_parser as parser;
extern crate unix_socket;
extern crate toml;

use unix_socket::UnixStream;
use std::io::{BufRead,BufReader,Write};
use std::process::Command;
use std::thread;
use std::sync::Arc;

mod config;
use config::{Config, Plugin};

mod error;
use error::Error;

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
                    println!("{}", line);
                    match parser::parse_message(&line[..]) {
                        Ok(parsed) => {
                            let plugin_list = plgs.iter()
                                                  .filter(|p| p.command
                                                               .iter()
                                                               .find(|&c| *c == parsed.command)
                                                               .is_some())
                                                  .collect::<Vec<&Plugin>>();
                            println!("{:?}", plugin_list);
                            for plugin in plugin_list {
                                match run_plugin(plugin, &line, &parsed)
                                          .and_then(|out| send_plugin_reply(conn.get_mut(), &out)) {
                                    Err(Error::Plugin) => {},
                                    Err(e) => println!("{}", e),
                                    _ => {}
                                }
                            }
                        }
                        Err(e) => println!("{}", e)
                    };
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

fn run_plugin(plugin: &Plugin, line: &String, parsed: &parser::Message) -> Result<String, Error> {
    if plugin.trigger.is_none() || plugin.trigger.as_ref().unwrap().is_match(line) {
        println!("Running plugin {}", plugin.executable);
        let params: Vec<&str> = parsed.params.iter().cloned().skip(1).collect();
        let output = match Command::new(&plugin.executable).arg(params.join(" ")).output() {
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
