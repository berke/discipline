use serde::{
    Deserialize,
    Serialize
};

#[derive(Debug,Serialize,Deserialize,Clone)]
pub enum Entity {
    Controller,
    Administrator(String),
    Subject(String)
}

#[derive(Debug,Serialize,Deserialize,Clone)]
pub struct Envelope<T> {
    pub sender:Entity,
    pub payload:T,
    pub signature:String
}

#[derive(Debug,Serialize,Deserialize,Clone)]
pub enum Command {
    Authorize { subject:String,
		duration:f64 },
    GetAuthorization { subject:String },
    GetStatus { subject:String },
    Cancel { subject:String },
}

#[derive(Debug,Serialize,Deserialize,Clone)]
pub enum Response {
    Ack,
    Authorization {
	subject:String,
	until:f64
    },
}
