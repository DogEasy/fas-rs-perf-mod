mod read;
mod single;

// 全局配置，可以在任何地方线程安全的访问toml
pub use single::CONFIG;

use std::{
    collections::HashSet,
    fs,
    path::Path,
    process::Command,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
};

use fas_rs_fw::Fps;

use likely_stable::LikelyOption;
use parking_lot::RwLock;
use toml::Value;

use crate::{ThisOption, ThisResult};
use read::wait_and_read;

pub type ConfData = RwLock<Value>;
pub struct Config {
    toml: Arc<ConfData>,
    exit: Arc<AtomicBool>,
}

impl Drop for Config {
    fn drop(&mut self) {
        self.exit.store(true, Ordering::Release);
    }
}

impl Config {
    pub fn new(path: &Path) -> Self {
        let ori = fs::read_to_string(path).this_unwrap();
        let toml = toml::from_str(&ori).this_unwrap();
        let toml = Arc::new(RwLock::new(toml));
        let toml_clone = toml.clone();

        let exit = Arc::new(AtomicBool::new(false));
        let exit_clone = exit.clone();

        let path = path.to_owned();

        thread::spawn(move || wait_and_read(&path, &toml_clone, &exit_clone));

        Self { toml, exit }
    }

    pub fn cur_game_fps(&self) -> Option<(String, Fps)> {
        let toml = self.toml.read();
        #[allow(unused)]
        let list = toml
            .get("game_list")
            .and_then_likely(Value::as_table)
            .cloned()
            .this_unwrap();

        drop(toml); // early-drop

        let pkgs = Self::get_top_pkgname()?;
        let pkg = pkgs.into_iter().find(|key| list.contains_key(key))?;

        let (game, fps) = (
            &pkg,
            Fps::try_from(list.get(&pkg)?.as_integer().this_unwrap()).this_unwrap(),
        );

        Some((game.clone(), fps.to_owned()))
    }

    #[allow(unused)]
    pub fn get_conf(&self, label: &'static str) -> Option<Value> {
        let toml = self.toml.read();
        toml.get("config").this_unwrap().get(label).cloned()
    }

    fn get_top_pkgname() -> Option<HashSet<String>> {
        let dump = Command::new("dumpsys")
            .args(["window", "visible-apps"])
            .output()
            .ok()?;
        let dump = String::from_utf8_lossy(&dump.stdout).into_owned();

        Some(
            dump.lines()
                .filter(|l| l.contains("package="))
                .map(|p| {
                    p.split_whitespace()
                        .nth(2)
                        .and_then_unlikely(|p| p.split('=').nth(1))
                        .this_unwrap()
                })
                .zip(
                    dump.lines()
                        .filter(|l| l.contains("canReceiveKeys()"))
                        .map(|k| k.contains("canReceiveKeys()=true")),
                )
                .filter(|(_, k)| *k)
                .map(|(p, _)| p.to_owned())
                .collect(),
        )
    }
}
