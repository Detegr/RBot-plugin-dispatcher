extern crate unix_socket;
use std::collections::HashMap;
use unix_socket::UnixStream;
use std::io::{BufRead,BufReader,Write};
use std::process::Command;
use std::thread;
use std::sync::Arc;

const SOCKETS: [&'static str; 1] = ["../RBot/sockets/irc.quakenet.org"];
const TYPE: usize = 0;

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
                Err(e) => panic!(e.to_string())
            };
            loop {
                let mut line = String::new();
                let res = conn.read_line(&mut line);
                match res {
                    Ok(0) => {
                        break;
                    }
                    Ok(_) => {
                        let splitted: Vec<&str> = line.split_whitespace().collect();
                        if splitted.len() == 0 {
                            continue;
                        }
                        let linetype = splitted[TYPE];
                        if let Some(plugin_list) = plgs.get(linetype) {
                            for plugin in plugin_list {
                                println!("Running plugin {}", plugin);
                                let output = match Command::new(plugin).output() {
                                    Ok(output) => output.stdout,
                                    Err(e) => {
                                        println!("{}", e.to_string());
                                        continue;
                                    }
                                };
                                match String::from_utf8(output) {
                                    Ok(out) => {
                                        println!("Output: {}", out.trim_right());
                                        let s = conn.get_mut();
                                        match s.write(out.trim_right().as_ref())
                                                .and_then(|_| s.write(b"\r\n"))
                                                .and_then(|_| s.flush()) {
                                            Err(e) => println!("{}", e.to_string()),
                                            _ => {},
                                        }
                                    }
                                    Err(e) => {
                                        println!("{}", e.to_string());
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        println!("{}", e.to_string());
                        break;
                    }
                }
            }
        }));
    }
    for t in threads {
        let _ = t.join();
    }
}
