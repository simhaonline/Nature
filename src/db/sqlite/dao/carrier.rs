use serde::Serialize;
use super::*;
use task::CarryError;

pub struct TableDelivery;

impl DeliveryDao for TableDelivery {
    fn insert<T: Sized + Serialize>(_carrier: &Carrier<T>) -> Result<u128> {
//        use self::schema::thing_defines;
//        let conn: &SqliteConnection = &CONN.lock().unwrap();
//        let rtn = diesel::insert_into(thing_defines::table)
//            .values(NewThingDefine::new(define))
//            .execute(conn);
//        match rtn {
//            Ok(x) => Ok(x),
//            Err(e) => Err(NatureError::from(e))
//        }
        unimplemented!()
    }

    fn delete(_id: &u128) -> Result<()> {
        unimplemented!()
    }

    fn move_to_error<T: Sized + Serialize>(_err: CarryError<T>) -> Result<()> {
        unimplemented!()
    }

    fn update_execute_time(_id: u128, _new_time: i64) -> Result<()> {
        unimplemented!()
    }

    fn get<T: Sized + Serialize>(_id: u128) -> Result<Carrier<T>> {
        unimplemented!()
    }
}
