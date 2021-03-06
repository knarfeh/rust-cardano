use std::path::{Path, PathBuf};
use std::env::{home_dir};
use clap::{ArgMatches, Arg, App};
use serde_yaml;

use account::{Account};
use command::{Wallet};
use command::{HasCommand};

use storage;
use storage::config::StorageConfig;

/// Configuration file for the Wallet CLI
#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub accounts: Vec<Account>,
    pub wallet: Option<Wallet>,
    pub network: String,
    pub root_dir: PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        let mut storage_dir = home_dir().unwrap();
        storage_dir.push(".ariadne/");
        Config::new_mainnet(storage_dir)
    }
}

impl Config {
    pub fn new(root_dir: PathBuf, network: String) -> Self {
        Config {
            accounts: vec![Account::default()],
            wallet: None,
            network: network,
            root_dir: root_dir,
        }
    }
    pub fn new_mainnet(root_dir: PathBuf) -> Self {
        Self::new(root_dir, "mainnet".to_string())
    }

    pub fn new_testnet(root_dir: PathBuf) -> Self {
        Self::new(root_dir, "testnet".to_string())
    }

    pub fn get_network_dir(&self) -> PathBuf {
        // TODO: check if `network`  starts with a `/`. if that is the case
        // it is an absolute path and it means the user wanted to use this
        // directly instead of our standard profile.
        let mut blk_dir_default = self.root_dir.clone();
        blk_dir_default.push("networks");
        blk_dir_default.push(self.network.as_str());
        blk_dir_default
    }

    pub fn get_storage_config(&self) -> StorageConfig {
        StorageConfig::new(&self.get_network_dir())
    }
    pub fn get_storage(&self) -> storage::Result<storage::Storage> {
        storage::Storage::init(&self.get_storage_config())
    }

    /// read the file associated to the given filepath, if the file does not exists
    /// this function creates the default `Config`;
    ///
    pub fn from_file<P: AsRef<Path>>(p: P) -> Self {
        use std::fs::{File};

        let path = p.as_ref();
        if ! path.is_file() {
            return Self::default();
        }

        let mut file = File::open(path).unwrap();
        serde_yaml::from_reader(&mut file).unwrap()
    }

    pub fn find_account(&self, account: &Account) -> Option<u32> {
        self.accounts.iter().position(|acc| acc == account)
            .map(|v| v as u32)
    }

    /// write the config in the given file
    ///
    /// if the file already exists it will erase the original data.
    pub fn to_file<P: AsRef<Path>>(&self, p: P) {
        use std::fs::{File};

        let mut file = File::create(p.as_ref()).unwrap();
        serde_yaml::to_writer(&mut file, &self).unwrap();
    }

    pub fn to_yaml(&self) -> serde_yaml::Value {
        serde_yaml::to_value(self).unwrap()
    }
    pub fn from_yaml(value: serde_yaml::Value) -> Self {
        serde_yaml::from_value(value).unwrap()
    }

    fn get(&self, path: &[serde_yaml::Value]) -> serde_yaml::Value {
        let mut obj = self.to_yaml();

        for e in path {
            obj = if obj.is_sequence() {
                obj.as_sequence().unwrap().get(e.as_u64().unwrap() as usize).unwrap().clone()
            } else {
                obj.get(e).unwrap().clone()
            }
        }

        obj
    }

    fn set(&mut self, path: &[serde_yaml::Value], value: serde_yaml::Value) {
        let mut obj = self.to_yaml();

        {
            let mut objr = &mut obj;

            for e in path {
                let mut objr_c = objr;
                objr = if objr_c.is_sequence() {
                    objr_c.as_sequence_mut().unwrap().get_mut(e.as_u64().unwrap() as usize).unwrap()
                } else if objr_c.is_mapping() {
                    objr_c.as_mapping_mut().unwrap().get_mut(e).unwrap()
                } else {
                    panic!("not a value")
                };
            }

            *objr = value;
        }

        *self = Self::from_yaml(obj)
    }
}

impl HasCommand for Config {
    type Output = Option<Config>;
    type Config = Config;

    const COMMAND : &'static str = "config";

    fn clap_options<'a, 'b>(app: App<'a, 'b>) -> App<'a, 'b> {
        app.about("get or set info from the config")
            .arg(Arg::with_name("path").help("path to a given option in the config").index(1).required(true))
            .arg(Arg::with_name("value").help("value to set to the given option").index(2).required(false))
    }

    fn run(cfg: Config, args: &ArgMatches) -> Self::Output {
        let path   : Vec<serde_yaml::Value> = args.value_of("path").unwrap().split('.').map(|s| serde_yaml::from_str(s).unwrap()).collect();

        match args.value_of("value") {
            None => {
                let r = cfg.get(&path);
                match r {
                    serde_yaml::Value::Null => println!(""),
                    serde_yaml::Value::Bool(b) => println!("{}", b),
                    serde_yaml::Value::Number(n) => println!("{}", n),
                    serde_yaml::Value::String(n) => println!("{}", n),
                    serde_yaml::Value::Sequence(n) => {
                        for e in n {
                            println!("{:?}", e);
                        }
                    },
                    serde_yaml::Value::Mapping(n) => {
                        for e in n {
                            println!("{:?}", e);
                        }
                    }
                };
                None
            },
            Some(val) => {
                let value : serde_yaml::Value = serde_yaml::from_str(val).unwrap();
                let mut cpy = cfg;
                cpy.set(&path, value);
                Some(cpy)
             },
        }
    }
}
