/* Copyright 2023 shadow3aaa@gitbub.com
*
*  Licensed under the Apache License, Version 2.0 (the "License");
*  you may not use this file except in compliance with the License.
*  You may obtain a copy of the License at
*
*      http://www.apache.org/licenses/LICENSE-2.0
*
*  Unless required by applicable law or agreed to in writing, software
*  distributed under the License is distributed on an "AS IS" BASIS,
*  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
*  See the License for the specific language governing permissions and
*  limitations under the License. */
mod buffer;
mod mode_policy;
mod policy;
mod utils;
mod window;

use std::{
    collections::HashMap,
    sync::mpsc::{Receiver, RecvTimeoutError},
    time::Duration,
};

use super::{topapp::TimedWatcher, FasData};
use crate::{
    config::Config,
    error::{Error, Result},
    node::Node,
    PerformanceController,
};

use buffer::Buffer;
use policy::Event;

pub type Producer = (i64, i32); // buffer, pid
pub type Buffers = HashMap<Producer, Buffer>; // Process, (jank_scale, total_jank_time_ns)

pub struct Looper<P: PerformanceController> {
    rx: Receiver<FasData>,
    config: Config,
    node: Node,
    controller: P,
    topapp_checker: TimedWatcher,
    buffers: Buffers,
    started: bool,
}

impl<P: PerformanceController> Looper<P> {
    pub fn new(rx: Receiver<FasData>, config: Config, node: Node, controller: P) -> Self {
        Self {
            rx,
            config,
            node,
            controller,
            topapp_checker: TimedWatcher::new(),
            buffers: Buffers::new(),
            started: false,
        }
    }

    pub fn enter_loop(&mut self) -> Result<()> {
        loop {
            let data = match self.rx.recv_timeout(Duration::from_secs(1)) {
                Ok(d) => d,
                Err(e) => {
                    if e == RecvTimeoutError::Disconnected {
                        return Err(Error::Other("Binder Server Disconnected"));
                    }

                    self.retain_topapp()?;

                    if self.started {
                        self.controller.release_max(&self.config)?;
                    }

                    continue;
                }
            };

            self.retain_topapp()?;
            self.buffer_update(&data);

            let Some(cur_buffer) = self.buffers.get_mut(&(data.buffer, data.pid)) else {
                continue;
            };
            let cur_event =
                Self::get_event(cur_buffer, &self.config, &mut self.node).unwrap_or(Event::None);

            let events: Vec<_> = self
                .buffers
                .values_mut()
                .map(|buffer| {
                    Self::get_event(buffer, &self.config, &mut self.node).unwrap_or(Event::None)
                })
                .collect();

            if events.contains(&Event::ReleaseMax) {
                self.controller.release_max(&self.config)?;
            } else if events.contains(&Event::Release) {
                self.controller.release(&self.config)?;
            } else if cur_event == Event::Limit {
                self.controller.limit(&self.config)?;
            }
        }
    }
}
