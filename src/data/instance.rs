extern crate r2d2;

use data::*;
use db::*;
use global::*;
use std::collections::HashMap;
use std::collections::HashSet;
use std::ops::Deref;
use std::sync::Arc;
use util::*;

/// A snapshot for a particular `Thing`
#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq)]
pub struct Instance {
    /// A unique value used to distinguish other instance
    pub id: u128,
    pub data: InstanceNoID,
}

impl Deref for Instance {
    type Target = InstanceNoID;

    fn deref(&self) -> &<Self as Deref>::Target {
        &self.data
    }
}


/// A snapshot for a particular `Thing`
#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq)]
pub struct InstanceNoID {
    /// This instance's Type
    pub thing: Thing,
    /// The time that this instance exists
    pub event_time: i64,
    /// The time which plan to flow for this instance
    pub execute_time: i64,
    /// When this instance created in db
    pub create_time: i64,
    /// What contend in this instance for the `Thing`
    pub content: String,
    /// Is a json for a `Map[key, value]` which contents other instance for other `Thing`'s.
    /// `Nature` can transform those to `Instance`'s by flowing.
    ///
    /// # Key
    ///
    /// context name
    ///
    /// # Value
    ///
    /// json data for a `Instance`.
    pub context: HashMap<String, String>,
    pub status: HashSet<String>,
    pub status_version: i32,
    pub from: Option<FromInstance>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq)]
pub struct FromInstance {
    pub thing: Thing,
    pub status_version: i32,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct ParallelBatchInstance(pub Vec<Instance>);

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct SerialBatchInstance {
    pub context_for_finish: String,
    pub instances: Vec<Instance>,
}

pub trait InstanceServiceTrait {
    fn verify(instance: &mut Instance, root: Root) -> Result<u128>;
}

pub struct InstanceImpl;

impl InstanceServiceTrait for InstanceImpl {
    /// check key whether defined
    /// generate id by hashing if it is not set.
    fn verify(instance: &mut Instance, root: Root) -> Result<u128> {
        Thing::key_standardize(&mut instance.data.thing.key, root)?;
        // just see whether it was configured.
        ThingDefineCacheImpl::get(&instance.data.thing)?;
        Self::id_generate_if_not_set(instance)
    }
}

impl InstanceImpl {
    fn id_generate_if_not_set(instance: &mut Instance) -> Result<u128> {
        if instance.id == 0 {
            instance.id = generate_id(&instance.data)?;
        }
        Ok(instance.id)
    }
}

lazy_static! {
    pub static ref DATA_INSTANCE: Arc<InstanceImpl> = Arc::new(InstanceImpl);
}
