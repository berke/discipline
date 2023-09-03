use std::error::Error;

pub type Res<T> = Result<T,Box<dyn Error>>;

pub fn error(msg:&str)->Box<dyn Error> {
    Box::new(std::io::Error::new(std::io::ErrorKind::Other,msg))
}
