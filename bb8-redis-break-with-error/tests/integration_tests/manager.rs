use std::error;

use bb8_redis_break_with_error::{
    bb8::ManageConnection as _,
    redis::{aio::Monitor, cmd},
    RedisConnectionManager,
};
use futures_util::StreamExt as _;

use super::helpers::{get_conn_addr, init_logger};

#[tokio::test]
async fn simple() -> Result<(), Box<dyn error::Error>> {
    init_logger();

    //
    let manager = RedisConnectionManager::new(get_conn_addr()?)?;

    //
    let conn = manager.connect().await?.into_inner();
    let mut monitor = Monitor::new(conn);
    monitor.monitor().await?;
    let mut monitor_stream = monitor.into_on_message::<String>();

    //
    let mut conn = manager.connect().await?;
    manager.is_valid(&mut conn).await?;
    assert!(!manager.has_broken(&mut conn));

    //
    let pool = bb8::Pool::builder()
        .max_size(10)
        .test_on_check_out(true)
        .build(manager)
        .await?;

    let mut conn = pool.get().await?;
    let reply: String = cmd("PING")
        .arg("MyPing")
        .query_async(&mut *conn)
        .await
        .unwrap();
    assert_eq!("MyPing", reply);

    //
    while let Some(msg) = monitor_stream.next().await {
        println!("[{}]", msg);
        // .is_valid()
        if msg.ends_with(r#" "PING""#) {
            break;
        }
    }

    Ok(())
}
