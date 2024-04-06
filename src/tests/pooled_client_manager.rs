use crate::{
    client::PooledClientManager, commands::PubSubCommands, commands::StringCommands,
    tests::get_default_addr, Result,
};
use futures_util::StreamExt;
use serial_test::serial;

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn pooled_client_manager() -> Result<()> {
    let manager = PooledClientManager::new(get_default_addr())?;
    let pool = crate::bb8::Pool::builder().build(manager).await?;
    let client = pool.get().await.unwrap();

    client.set("key", "value").await?;
    let value: String = client.get("key").await?;
    assert_eq!("value", value);

    Ok(())
}

#[cfg_attr(
    feature = "tokio-runtime",
    tokio::test(flavor = "multi_thread", worker_threads = 4)
)]
async fn pub_sub_pooling() -> Result<()> {
    let manager = PooledClientManager::new(get_default_addr())?;

    let p0 = crate::bb8::Pool::builder()
        .max_size(3)
        .build(manager)
        .await
        .unwrap();

    let p1 = p0.clone();

    let h0 = tokio::spawn(async move {
        for _ in 0..1000 {
            tokio::time::sleep(tokio::time::Duration::from_millis(15)).await;

            let (mut sink, mut stream) = p0.get().await.unwrap().create_pub_sub().split();
            sink.subscribe("foo").await.unwrap();

            p0.get().await.unwrap().publish("foo", "bar").await.unwrap();

            let message = stream.next().await.unwrap().unwrap();
            assert_eq!(b"foo".to_vec(), message.channel);
            assert_eq!(b"bar".to_vec(), message.payload);
        }
    });

    let h1 = tokio::spawn(async move {
        for _ in 0..3000 {
            tokio::time::sleep(tokio::time::Duration::from_millis(7)).await;

            p1.get().await.unwrap().set("foo", 1).await.unwrap();
        }
    });

    _ = tokio::join!(h0, h1);

    Ok(())
}
