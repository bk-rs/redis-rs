use std::error;

use bb8_redis_break_with_error::{
    bb8,
    redis::{cmd, ErrorKind as RedisErrorKind},
    RedisConnectionManager,
};
use futures_util::future::join_all;
use tokio::task::JoinHandle;

use super::helpers::{get_conn_addr, init_logger};

#[tokio::test]
async fn simple() -> Result<(), Box<dyn error::Error>> {
    init_logger();

    //
    let manager = RedisConnectionManager::new(get_conn_addr()?)?;
    let pool = bb8::Pool::builder()
        .max_size(10)
        .test_on_check_out(false)
        .build(manager)
        .await?;

    let mut handles = vec![];

    for i in 0..10 {
        let pool = pool.clone();

        let handle: JoinHandle<Result<(), Box<dyn error::Error + Send + Sync>>> =
            tokio::spawn(async move {
                let mut conn = pool.get().await.map_err(|err| match err {
                    bb8::RunError::User(err) => err,
                    bb8::RunError::TimedOut => panic!(""),
                })?;

                #[allow(clippy::single_match)]
                match i {
                    0 => {
                        cmd("AUTH mypass").query_async(&mut *conn).await?;
                    }
                    _ => {}
                }

                let reply: String = cmd(format!("PING PingMsg{}", i).as_str())
                    .query_async(&mut *conn)
                    .await
                    .map_err(|err| {
                        assert_eq!(err.kind(), RedisErrorKind::ResponseError);
                        conn.set_close_required_with_error(&err);

                        err
                    })?;
                assert_eq!(format!("PingMsg{}", i), reply);

                Ok(())
            });
        handles.push(handle);
    }

    join_all(handles).await;

    println!("{:?}", pool.state());
    assert_eq!(pool.state().connections, 1);

    Ok(())
}
