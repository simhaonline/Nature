use std::sync::mpsc::Sender;

use nature_common::{Meta, DynamicConverter, Instance, Result};
use nature_db::{Mission, OneStepFlow, RawTask};

use crate::task::TaskForConvert;

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct TaskForStore {
    pub instance: Instance,
    /// save outside has non converter info.
    pub upstream: Option<TaskForConvert>,
    pub mission: Option<Vec<Mission>>,
}


impl TaskForStore {
    pub fn gen_task<FG, FF>(instance: &Instance, step_getter: FG, mission_filter: FF) -> Result<Self> where
        FG: Fn(&Meta) -> Result<Option<Vec<OneStepFlow>>>,
        FF: FnOnce((&Instance, Vec<OneStepFlow>)) -> Option<Vec<Mission>>
    {
        let steps = match step_getter(&instance.thing)? {
            Some(steps) => {
                mission_filter((&instance, steps))
            }
            None => None
        };
        Ok(
            TaskForStore {
                instance: instance.clone(),
                upstream: None,
                mission: steps,
            }
        )
    }

    pub fn send(&self, raw: &RawTask, sender: &Sender<(TaskForStore, RawTask)>) {
        let _ = sender.send((self.to_owned(), raw.to_owned()));
    }

    pub fn for_dynamic(instance: &Instance, dynamic: Vec<DynamicConverter>) -> Result<TaskForStore> {
        let target = Mission::for_dynamic(dynamic)?;
        // save to task to make it can redo
        let task = TaskForStore {
            instance: instance.clone(),
            upstream: None,
            mission: Some(target),
        };
        Ok(task)
    }
}