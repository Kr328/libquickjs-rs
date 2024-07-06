use rquickjs_sys::{JSClassID, JS_NewClassID};

use crate::{error::Error, Context};

pub struct FunctionID(JSClassID);

impl FunctionID {
    pub(crate) fn js_class_id(&self) -> JSClassID {
        unsafe { JS_NewClassID(&self.0 as *const _ as _) }
    }
}

pub trait Function {
    type Data: Send + 'static;

    fn id() -> &'static FunctionID;

    fn call(ctx: &mut Context, this: &u32, args: &[u32]) -> Result<(), Error>;
    fn construct(ctx: &mut Context, this: &u32, args: &[u32]) -> Result<(), Error>;
}
