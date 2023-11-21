use std::fmt::Debug;

use log::{error, info};

pub fn log_error(name: &str, res: Result<(), impl Debug>) {
    if let Err(e) = res {
        error!("{} failed with error: {:?}", name, e)
    } else {
        info!("{} succeeded", name);
    }
}
