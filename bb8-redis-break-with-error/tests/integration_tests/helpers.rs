use std::{env, error, net::IpAddr};

use log::debug;

//
pub(super) const PASSWORD: &str = "mypass";

//
pub(super) fn get_conn_addr() -> Result<String, Box<dyn error::Error>> {
    let port = env::var("REDIS_TCP_PORT")?;
    debug!("REDIS_TCP_PORT {}", port);

    let ip_addr = "127.0.0.1".parse::<IpAddr>()?;
    let port = port.parse::<u16>()?;

    Ok(format!("redis://:{}@{}:{}", PASSWORD, ip_addr, port))
}

pub(super) fn get_conn_addr_without_password() -> Result<String, Box<dyn error::Error>> {
    let port = env::var("REDIS_TCP_PORT")?;
    debug!("REDIS_TCP_PORT {}", port);

    let ip_addr = "127.0.0.1".parse::<IpAddr>()?;
    let port = port.parse::<u16>()?;

    Ok(format!("redis://{}:{}", ip_addr, port))
}

pub(super) fn init_logger() {
    let _ = env_logger::builder().is_test(true).try_init();
}
