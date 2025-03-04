mod power_mode;

use crate::framework::error::{Error, Result};
use hashbrown::HashMap;
use likely_stable::unlikely;
pub use power_mode::Mode;
use std::{
    fs,
    path::Path,
    time::{Duration, Instant},
};

const NODE_PATH: &str = "/data/adb/fas-rs";
const REFRESH_TIME: Duration = Duration::from_secs(1);

pub struct Node {
    map: HashMap<String, String>,
    timer: Instant,
}

impl Node {
    pub fn init() -> Result<Self> {
        let _ = fs::create_dir(NODE_PATH);

        let mut result = Self {
            map: HashMap::new(),
            timer: Instant::now(),
        };

        let _ = result.remove_node("mode");
        result.create_node("mode", "balance")?;

        Ok(result)
    }

    pub fn create_node<S: AsRef<str>>(&mut self, i: S, d: S) -> Result<()> {
        let id = i.as_ref();
        let default = d.as_ref();

        let path = Path::new(NODE_PATH).join(id);
        fs::write(path, default)?;
        self.refresh()
    }

    pub fn remove_node<S: AsRef<str>>(&mut self, i: S) -> Result<()> {
        let id = i.as_ref();

        let path = Path::new(NODE_PATH).join(id);
        fs::remove_file(path)?;

        self.refresh()
    }

    pub fn get_node<S: AsRef<str>>(&mut self, id: S) -> Result<String> {
        let id = id.as_ref();

        if unlikely(self.timer.elapsed() > REFRESH_TIME) {
            self.refresh()?;
        }

        self.map
            .get_mut(id)
            .map_or_else(|| Err(Error::NodeNotFound), |value| Ok(value.clone()))
    }

    fn refresh(&mut self) -> Result<()> {
        for entry in fs::read_dir(NODE_PATH)? {
            let Ok(entry) = entry else {
                continue;
            };

            if entry.file_type()?.is_file() {
                let id = entry.file_name().into_string().unwrap();
                let value = fs::read_to_string(entry.path())?;
                self.map.insert(id, value);
            }
        }

        Ok(())
    }
}
