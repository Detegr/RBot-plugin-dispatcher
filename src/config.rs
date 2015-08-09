extern crate rbot_parser as parser;
extern crate regex;

use toml;
use error::Error;
use std::fs::OpenOptions;
use std::io::Read;
use std::collections::BTreeMap;
use self::regex::Regex;

static CONFIG_FILE: &'static str = "plugins.toml";

#[derive(Debug)]
pub struct Plugin<'a> {
    command: Vec<parser::Command<'a>>,
    executable: String,
    trigger: Option<Regex>,
}
#[derive(Debug)]
pub struct Config<'a> {
    plugins: Vec<Plugin<'a>>,
    sockets: Vec<String>,
}
fn plugin_from_map<'a>(map: &BTreeMap<String, toml::Value>) -> Option<Plugin<'a>> {
    let executable = match map.get("executable").and_then(toml::Value::as_str) {
        Some(exe) => exe,
        None => return None
    }.to_owned();
    let command = map.get("command")
                     .and_then(toml::Value::as_slice)
                     .and_then(|commands| Some(commands.into_iter().filter_map(|c| {
                         match *c {
                             toml::Value::Integer(i) => {
                                 Some(parser::Command::Numeric(i as u16))
                             },
                             toml::Value::String(ref s) => {
                                 Some(parser::Command::Named(s.clone().into()))
                             },
                             _ => None
                         }
                     })
                     .collect()));
    let trigger = map.get("trigger")
                     .and_then(toml::Value::as_str)
                     .and_then(|t| Regex::new(t).ok());
    Some(Plugin {
        command: match command {
            Some(command) => command,
            None => vec![]
        },
        executable: executable,
        trigger: trigger,
    })
}
impl<'a> Config<'a> {
    pub fn new() -> Result<Config<'a>, Error> {
        let mut config_file = try!(OpenOptions::new()
                                       .create(true)
                                       .append(true)
                                       .open(CONFIG_FILE));
        let mut contents = String::new();
        try!(config_file.read_to_string(&mut contents));
        let mut config = match toml::Parser::new(&contents).parse() {
            Some(parsed) => parsed,
            None => return Err(Error::Config(None))
        };
        let sockets = try!(config.remove("sockets")
                          .unwrap_or(toml::Value::Array(vec![]))
                          .as_slice()
                          .ok_or(Error::Config(Some("Value of 'sockets' in the config file must be an array".into()))))
                          .into_iter()
                          .filter_map(|v| v.as_str().map(|s| s.to_owned()))
                          .collect();
        let plugins = config.remove("plugins")
                         .unwrap_or(toml::Value::Table(BTreeMap::new()))
                         .as_table()
                         .ok_or(Error::Config(Some("Malformed config file. Use [plugins.pluginname] to define new plugins".into())))
                         .into_iter()
                         .flat_map(|plugins| {
                             plugins.values()
                                .filter_map(|plugin| {
                                   plugin.as_table()
                                      .and_then(plugin_from_map)
                                      /*
                                      .and_then(|e| Some(Plugin {
                                         executable: e.to_owned()
                                      }))*/
                                })
                         })
                         .collect();
        Ok(Config {
            plugins: plugins,
            sockets: sockets,
        })
    }
}
