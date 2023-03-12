use bb8_redis_break_with_error::{
    bb8::ManageConnection as _,
    redis::{aio::Monitor, cmd},
    RedisConnectionManager,
};
use futures_util::StreamExt as _;

use super::helpers::{get_conn_addr, init_logger};

#[tokio::test]
async fn simple() -> Result<(), Box<dyn std::error::Error>> {
    init_logger();

    //
    let manager = RedisConnectionManager::new(get_conn_addr()?)?;

    //
    let mut conn = manager.connect().await?;
    manager.is_valid(&mut conn).await?;
    assert!(!manager.has_broken(&mut conn));

    //
    let monitor_conn = manager.connect().await?.into_inner();

    //
    let pool = bb8::Pool::builder()
        .max_size(1)
        .test_on_check_out(true)
        .build(manager)
        .await?;

    //
    let mut monitor = Monitor::new(monitor_conn);
    monitor.monitor().await?;
    let mut monitor_stream = monitor.into_on_message::<String>();

    //
    for i in 0..4 {
        let mut conn = pool.get().await.unwrap();

        let reply: String = cmd("PING")
            .arg(format!("MyPingMsg{}", i))
            .query_async(&mut *conn)
            .await
            .unwrap();
        assert_eq!(format!("MyPingMsg{}", i), reply);
    }

    //
    let mut n_call_is_valid = 0;
    while let Some(msg) = monitor_stream.next().await {
        println!("[{}]", msg);
        if msg.ends_with(r#" "PING""#) {
            n_call_is_valid += 1;
        }
        if n_call_is_valid >= 3 {
            break;
        }
    }

    Ok(())
}
