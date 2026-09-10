pub use crate::object::{Obj as Entry, ObjValue as RedisValue};

pub struct RedisCmd {
    pub cmd: String,
    pub args: Vec<String>
}



pub struct RedisCmds {
    pub cmds: Vec<RedisCmd>
}
