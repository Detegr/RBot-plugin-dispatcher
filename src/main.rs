extern crate unix_socket;
use std::collections::HashMap;
use unix_socket::UnixStream;
use std::io::{BufRead,BufReader,Write};
use std::process::Command;
use std::thread;
use std::sync::Arc;

const SOCKETS: [&'static str; 1] = ["../RBot/sockets/irc.quakenet.org"];
const TYPE: usize = 0;
const RETRIES: usize = 10;

fn main() {
    // TODO: Read from a config file
    let mut plugins = HashMap::new();
    plugins.insert("376", vec!["./autojoin.sh"]); // End of motd
    plugins.insert("422", vec!["./autojoin.sh"]); // Motd file missing
    plugins.insert("PRIVMSG", vec!["./test.sh"]);
    let plugins = Arc::new(plugins);

    let mut threads = vec![];

    for socket in SOCKETS.iter().cloned() {
        let plgs = plugins.clone();
        threads.push(thread::spawn(move || {
            let mut conn = match UnixStream::connect(socket) {
                Ok(conn) => BufReader::new(conn),
                Err(e) => {
                    println!("{}", e.to_string());
                    match retry_connection(socket) {
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
                    let linetype = splitted[TYPE];
                    if let Some(plugin_list) = plgs.get(linetype) {
                        for plugin in plugin_list {
                            match run_plugin(plugin)
                                      .and_then(|out| send_plugin_reply(conn.get_mut(), &out)) {
                                Err(e) => println!("{}", e),
                                _ => {}
                            }
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

fn run_plugin(plugin: &str) -> Result<String, String> {
    println!("Running plugin {}", plugin);
    let output = match Command::new(plugin).output() {
        Ok(output) => output.stdout,
        Err(e) => {
            return Err(e.to_string());
        }
    };
    String::from_utf8(output).map_err(|e| e.to_string())
}

fn send_plugin_reply(s: &mut UnixStream, output: &str) -> Result<(),String> {
    match s.write(output.trim_right().as_ref())
        .and_then(|_| s.write(b"\r\n"))
        .and_then(|_| s.flush()) {
            Err(e) => Err(e.to_string()),
            _ => Ok(()),
        }
}
