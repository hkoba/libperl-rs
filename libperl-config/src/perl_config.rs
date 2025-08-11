use super::process_util::*;

use super::PerlCommand;

use std::collections::HashMap;

type ConfigDict = HashMap<String, String>;

pub struct PerlConfig {
    command: PerlCommand,
    pub dict: ConfigDict,
}

impl Default for PerlConfig {
     fn default() -> Self {
        let cmd = PerlCommand::default();
        let dict = read_config(&cmd, &[]).expect("Failed to read Config.pm");
        Self {
            command: cmd,
            dict: dict,
        }
    }
}

impl PerlConfig {

    pub fn new(perl: &str) -> Self {
        let cmd = PerlCommand::new(perl);
        let dict = read_config(&cmd, &[]).expect("Failed to read Config.pm");
        Self {
            command: cmd,
            dict: dict,
        }
    }

    pub fn command(&self, args: &[&str]) -> Command {
        self.command.command(args)
    }

    pub fn read_ccopts(&self) -> Result<Vec<String>, Error> {
        self.command.read_ccopts()
    }

    pub fn read_ldopts(&self) -> Result<Vec<String>, Error> {
        self.command.read_ldopts()
    }

    pub fn is_defined(&self, name: &str) -> Result<bool, Error> {
        if let Some(value) = self.dict.get(name) {
            Ok(value == "define")
        } else {
            Err(other_error("No such entry".to_string()))
        }
    }

    pub fn emit_cargo_ldopts(&self) {
        self.command.emit_cargo_ldopts()
    }

    pub fn emit_perlapi_vers(&self, min: i32, max: i32) {
        let config = &self.dict["PERL_API_VERSION"];
        let config = config.trim();
        println!("# PERL_API_VERSION={}", config);
        let ver = i32::from_str_radix(String::from(config).trim(), 10).unwrap();
        for v in min..=max {
            if v % 2 == 1 {
                continue;
            }
            // Emit check-cfg for all possible versions (Rust 1.80+)
            println!("cargo::rustc-check-cfg=cfg(perlapi_ver{})", v);
            if ver >= v {
                println!("cargo:rustc-cfg=perlapi_ver{}", v);
            }
        }
    }

    pub fn emit_features(&self, configs: &[&str]) {
        for &cfg in configs.iter() {
            println!("# perl config {} = {:?}", cfg, self.dict.get(&String::from(cfg)));
            // Emit check-cfg for all features (Rust 1.80+)
            println!("cargo::rustc-check-cfg=cfg(perl_{})", cfg);
            if self.is_defined(cfg).unwrap() {
                println!("cargo:rustc-cfg=perl_{}", cfg);
            }
        }
    }
}

fn read_config(cmd: &PerlCommand, configs: &[&str]) -> Result<ConfigDict, Error> {
    let config = cmd.read_raw_config(configs)?;
    let lines = config.lines().map(String::from).collect();
    Ok(lines_to_hashmap(lines))
}

fn lines_to_hashmap(lines: Vec<String>) -> ConfigDict {
    let mut dict = HashMap::new();
    for line in lines.iter() {
        let kv: Vec<String> = line.splitn(2, '\t').map(String::from).collect();
        if kv.len() == 2 {
            dict.insert(kv[0].clone(), kv[1].clone());
        }
    }
    dict
}
