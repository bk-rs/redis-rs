use std::error;

use bb8_redis_break_with_error::{
    bb8,
    redis::{cmd, ErrorKind as RedisErrorKind},
    RedisConnectionManager,
};
use futures_util::future::join_all;
use tokio::task::JoinHandle;

use super::helpers::{get_conn_addr_without_password, init_logger, PASSWORD};

#[tokio::test]
async fn simple() -> Result<(), Box<dyn error::Error>> {
    init_logger();

    //
    let manager = RedisConnectionManager::new(get_conn_addr_without_password()?)?;
    let pool = bb8::Pool::builder()
        .max_size(10)
        .test_on_check_out(false)
        .build(manager)
        .await?;

    let mut handles = vec![];

    for i in 0..3 {
        let pool = pool.clone();

        let handle: JoinHandle<Result<(), Box<dyn error::Error + Send + Sync>>> =
            tokio::spawn(async move {
                let mut conn = pool.get().await.unwrap();

                #[allow(clippy::single_match)]
                match i {
                    0 => {
                        let _: () = cmd("AUTH")
                            .arg(PASSWORD)
                            .query_async(&mut *conn)
                            .await
                            .unwrap();
                    }
                    _ => {
                        let _: () = cmd("AUTH")
                            .arg("xxx")
                            .query_async(&mut *conn)
                            .await
                            .map_err(|err| {
                                assert_eq!(err.kind(), RedisErrorKind::ExtensionError);
                                assert!(err.to_string().starts_with("WRONGPASS:"));
                                assert_eq!(err.code(), Some("WRONGPASS"));

                                err
                            })?;
                    }
                }

                let reply: String = cmd("PING")
                    .arg(format!("PingMsg{}", i))
                    .query_async(&mut *conn)
                    .await
                    .unwrap();
                assert_eq!(format!("PingMsg{}", i), reply);

                Ok(())
            });
        handles.push(handle);
    }

    let rets = join_all(handles).await;
    assert!(rets[0].as_ref().ok().unwrap().is_ok());
    assert!(rets[1].as_ref().ok().unwrap().is_err());
    assert!(rets[2].as_ref().ok().unwrap().is_err());

    println!("{:?}", pool.state());
    assert_eq!(pool.state().connections, 1);

    Ok(())
}
