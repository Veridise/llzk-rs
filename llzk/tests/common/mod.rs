use log::LevelFilter;
use simplelog::{Config, TestLogger};

pub fn setup() {
    let _ = TestLogger::init(LevelFilter::Debug, Config::default());
}
