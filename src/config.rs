use toml;
use error::Error;
use std::fs::OpenOptions;
use std::io::Read;
use std::collections::BTreeMap;

static CONFIG_FILE: &'static str = "plugins.toml";

#[derive(Debug)]
pub struct Plugin {
    executable: String
}
#[derive(Debug)]
pub struct Config {
    plugins: Vec<Plugin>,
    sockets: Vec<String>,
}
//fn get_toml_string_value(val: toml::Value
impl Config {
    pub fn new() -> Result<Config, Error> {
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
                                      .and_then(|t| t.get("executable"))
                                      .and_then(toml::Value::as_str)
                                      .and_then(|e| Some(Plugin {
                                         executable: e.to_owned()
                                      }))
                                })
                         })
                         .collect();
        Ok(Config {
            plugins: plugins,
            sockets: sockets,
        })
    }
}
