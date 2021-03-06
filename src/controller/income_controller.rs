use std::convert::TryFrom;

use nature_common::{ConverterReturned, DelayedInstances, generate_id, Instance, KeyCondition, Meta, MetaType, NatureError, Result, SelfRouteInstance};
use nature_db::{C_M, C_R, D_M, D_R, D_T, InstanceDaoImpl, MetaCache, Mission, RawTask, RelationCache, TaskDao, TaskType};
use nature_db::flow_tool::{context_check, state_check};

use crate::channels::CHANNEL_CONVERT;
use crate::controller::*;
use crate::task::{TaskForConvert, TaskForStore};

pub struct IncomeController {}

impl IncomeController {
    /// born an instance which is the beginning of the changes.
    pub async fn input(mut instance: Instance) -> Result<u128> {
        let _ = check_and_revise(&mut instance).await?;
        let relations = C_R.get(&instance.meta, &*D_R, &*C_M, &*D_M).await?;
        let mission = Mission::get_by_instance(&instance, &relations, context_check, state_check);
        // for o in &mission {
        //     debug!("--generate mission from:{},to:{}", &instance.meta, o.to.meta_string());
        // }
        let task = TaskForStore::new(instance.clone(), mission, None, false);
        let raw = task.to_raw()?;
        let num = D_T.insert(&raw).await?;
        if num == 1 {
            channel_store(task, raw).await?;
        }
        Ok(instance.id)
    }


    /// born an instance which is the beginning of the changes.
    pub async fn self_route(instance: SelfRouteInstance) -> Result<u128> {
        let _ = instance.verify()?;
        // Convert a Self-Route-Instance to Normal Instance
        let mut ins = instance.to_instance();
        MetaType::check_type(&ins.meta, MetaType::Dynamic)?;
        let uuid = ins.revise()?.id;
        let task = TaskForStore::for_dynamic(&ins, instance.converter, None, false)?;
        let raw = task.to_raw()?;
        let num = D_T.insert(&raw).await?;
        if num == 1 {
            channel_store(task, raw).await?;
        }
        Ok(uuid)
    }

    pub async fn callback(delayed: DelayedInstances) -> Result<()> {
        match D_T.get(&delayed.task_id).await {
            Ok(raw) => {
                match raw {
                    None => Err(NatureError::VerifyError("task data missed, maybe it had done already.".to_string())),
                    Some(carrier) => match delayed.result {
                        ConverterReturned::LogicalError(err) => {
                            let err = NatureError::LogicalError(err);
                            warn!("{}", err);
                            let _ = D_T.raw_to_error(&err, &carrier).await?;
                            Ok(())
                        }
                        ConverterReturned::EnvError(e) => {
                            warn!("{}", e);
                            Ok(())
                        }
                        ConverterReturned::Delay(_) => {
                            Err(NatureError::VerifyError("callback can not process [ConverterReturned::Delay]".to_string()))
                        }
                        ConverterReturned::Instances(ins) => {
                            let (task, last) = get_task_and_last(&carrier).await?;
                            after_converted(&task, &carrier, ins, &last).await
                        }
                        ConverterReturned::SelfRoute(sf) => {
                            let (task, _last) = get_task_and_last(&carrier).await?;
                            received_self_route(&task, &carrier, sf)
                        }
                        ConverterReturned::None => {
                            let (task, _last) = get_task_and_last(&carrier).await?;
                            process_null(task.target.to.get_meta_type(), &delayed.task_id).await
                        }
                    }
                }
            }
            Err(e) => Err(e)
        }
    }

    pub async fn redo_task(raw: RawTask) -> Result<()> {
        // TODO check busy first
        match TaskType::try_from(raw.task_type)? {
            TaskType::Store => {
                let rtn = TaskForStore::from_raw(&raw, &*C_M, &*D_M).await?;
                debug!("--redo store task for task : {:?}", &rtn);
                channel_stored(rtn, raw).await;
            }
            TaskType::Convert => {
                let rtn = TaskForConvert::from_raw(&raw, InstanceDaoImpl::get_by_key, &*C_M, &*D_M).await?;
                debug!("--redo convert task: from:{}, to:{}", rtn.from.meta, rtn.target.to.meta_string());
                CHANNEL_CONVERT.sender.lock().unwrap().send((rtn, raw))?;
            }
            TaskType::Batch => {
                let rtn = serde_json::from_str(&raw.data)?;
                debug!("--redo batch task for task : {:?}", &rtn);
                channel_batch(rtn, raw).await;
            }
        }
        Ok(())
    }

    pub async fn batch(batch: Vec<Instance>) -> Result<()> {
        let id = generate_id(&batch)?;
        let raw = RawTask::new(&batch, &id.to_string(), TaskType::Batch as i8, &batch[0].meta)?;
        let num = D_T.insert(&raw).await?;
        if num != 1 {
            return Ok(());
        }
        let rtn = channel_batch(batch, raw).await;
        Ok(rtn)
    }
}

async fn get_task_and_last(task: &RawTask) -> Result<(TaskForConvert, Option<Instance>)> {
    let mut task: TaskForConvert = TaskForConvert::from_raw(task, InstanceDaoImpl::get_by_key, &*C_M, &*D_M).await?;
    let last = InstanceDaoImpl::get_last_target(&task.from, &mut task.target).await?;
    Ok((task, last))
}

async fn check_and_revise(instance: &mut Instance) -> Result<&mut Instance> {
    let meta: Meta = C_M.get(&instance.meta, &*D_M).await?;    // verify meta
    // check previous state version
    let version = instance.state_version;
    if meta.is_state() && version > 1 {
        let mut kc: KeyCondition = instance.clone().into();
        kc.state_version = version - 1;
        let rtn = InstanceDaoImpl::get_by_id(kc).await?;
        if rtn.is_none() {
            return Err(NatureError::VerifyError("you can't skip state_version for instance".to_string()));
        }
    }
    instance.revise()
}
