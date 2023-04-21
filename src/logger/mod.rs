use crate::WsMessage;

pub mod console;

pub trait MessageLogger {
    fn log(&self, message: &WsMessage);
}
