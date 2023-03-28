use std::collections::{HashMap, HashSet};

use crate::{
    client::{Client, IntoConfig},
    commands::{
        ClientKillOptions, ClusterCommands, ClusterShardResult, ConnectionCommands, FlushingMode,
        PubSubChannelsOptions, PubSubCommands, ServerCommands, StringCommands,
    },
    tests::{get_cluster_test_client, get_default_addr, get_test_client, log_try_init},
    Result,
};
use futures_util::{FutureExt, StreamExt, TryStreamExt};
use serial_test::serial;

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn pubsub() -> Result<()> {
    log_try_init();

    let mut config = get_default_addr().into_config()?;
    config.connection_name = "pub/sub".to_owned();
    let pub_sub_client = Client::connect(config).await?;

    let mut config = get_default_addr().into_config()?;
    config.connection_name = "regular".to_owned();
    let regular_client = Client::connect(config).await?;

    // cleanup
    regular_client.flushdb(FlushingMode::Sync).await?;

    let mut pub_sub_stream = pub_sub_client.subscribe("mychannel").await?;
    regular_client.publish("mychannel", "mymessage").await?;

    let message = pub_sub_stream.next().await.unwrap()?;
    assert_eq!(b"mychannel".to_vec(), message.channel);
    assert_eq!(b"mymessage".to_vec(), message.payload);

    regular_client.set("key", "value").await?;
    let value: String = regular_client.get("key").await?;
    assert_eq!("value", value);

    pub_sub_stream.close().await?;

    let mut pub_sub_stream = pub_sub_client.subscribe("mychannel2").await?;
    regular_client.publish("mychannel2", "mymessage2").await?;

    let message = pub_sub_stream.next().await.unwrap()?;
    let channel: String = String::from_utf8(message.channel).unwrap();
    let payload: String = String::from_utf8(message.payload).unwrap();

    assert_eq!("mychannel2", channel);
    assert_eq!("mymessage2", payload);

    Ok(())
}

// #[cfg_attr(feature = "tokio-runtime", tokio::test)]
// #[cfg_attr(feature = "async-std-runtime", async_std::test)]
// #[serial]
// async fn forbidden_command() -> Result<()> {
//     let client = get_test_client().await?;

//     // cleanup
//     client.flushdb(FlushingMode::Sync).await?;

//     // regular mode, these commands are allowed
//     client.set("key", "value").await?;
//     let value: String = client.get("key").await?;
//     assert_eq!("value", value);

//     // subscribed mode
//     let pub_sub_stream = client.subscribe("mychannel").await?;

//     // Cannot send regular commands during subscribed mode
//     let result: Result<String> = client.get("key").await;
//     assert!(result.is_err());

//     pub_sub_stream.close().await?;

//     // After leaving subscribed mode, should work again
//     let value: String = client.get("key").await?;
//     assert_eq!("value", value);

//     Ok(())
// }

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn subscribe_to_multiple_channels() -> Result<()> {
    let pub_sub_client = get_test_client().await?;
    let regular_client = get_test_client().await?;

    // cleanup
    regular_client.flushdb(FlushingMode::Sync).await?;

    let mut pub_sub_stream = pub_sub_client
        .subscribe(["mychannel1", "mychannel2"])
        .await?;
    regular_client.publish("mychannel1", "mymessage1").await?;
    regular_client.publish("mychannel2", "mymessage2").await?;

    let message = pub_sub_stream.next().await.unwrap()?;
    let channel: String = String::from_utf8(message.channel).unwrap();
    let payload: String = String::from_utf8(message.payload).unwrap();

    assert_eq!("mychannel1", channel);
    assert_eq!("mymessage1", payload);

    let message = pub_sub_stream.next().await.unwrap()?;
    let channel: String = String::from_utf8(message.channel).unwrap();
    let payload: String = String::from_utf8(message.payload).unwrap();

    assert_eq!("mychannel2", channel);
    assert_eq!("mymessage2", payload);

    pub_sub_stream.close().await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn subscribe_to_multiple_patterns() -> Result<()> {
    let pub_sub_client = get_test_client().await?;
    let regular_client = get_test_client().await?;

    // cleanup
    regular_client.flushdb(FlushingMode::Sync).await?;

    let mut pub_sub_stream = pub_sub_client
        .psubscribe(["mychannel1*", "mychannel2*"])
        .await?;

    regular_client.publish("mychannel11", "mymessage11").await?;
    regular_client.publish("mychannel12", "mymessage12").await?;
    regular_client.publish("mychannel21", "mymessage21").await?;
    regular_client.publish("mychannel22", "mymessage22").await?;

    let message = pub_sub_stream.next().await.unwrap()?;
    let pattern: String = String::from_utf8(message.pattern).unwrap();
    let channel: String = String::from_utf8(message.channel).unwrap();
    let payload: String = String::from_utf8(message.payload).unwrap();

    assert_eq!("mychannel1*", pattern);
    assert_eq!("mychannel11", channel);
    assert_eq!("mymessage11", payload);

    let message = pub_sub_stream.next().await.unwrap()?;
    let pattern: String = String::from_utf8(message.pattern).unwrap();
    let channel: String = String::from_utf8(message.channel).unwrap();
    let payload: String = String::from_utf8(message.payload).unwrap();

    assert_eq!("mychannel1*", pattern);
    assert_eq!("mychannel12", channel);
    assert_eq!("mymessage12", payload);

    let message = pub_sub_stream.next().await.unwrap()?;
    let pattern: String = String::from_utf8(message.pattern).unwrap();
    let channel: String = String::from_utf8(message.channel).unwrap();
    let payload: String = String::from_utf8(message.payload).unwrap();

    assert_eq!("mychannel2*", pattern);
    assert_eq!("mychannel21", channel);
    assert_eq!("mymessage21", payload);

    let message = pub_sub_stream.next().await.unwrap()?;
    let pattern: String = String::from_utf8(message.pattern).unwrap();
    let channel: String = String::from_utf8(message.channel).unwrap();
    let payload: String = String::from_utf8(message.payload).unwrap();

    assert_eq!("mychannel2*", pattern);
    assert_eq!("mychannel22", channel);
    assert_eq!("mymessage22", payload);

    pub_sub_stream.close().await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn pub_sub_channels() -> Result<()> {
    let pub_sub_client = get_test_client().await?;
    let regular_client = get_test_client().await?;

    let stream = pub_sub_client
        .subscribe(["mychannel1", "mychannel2", "mychannel3", "otherchannel"])
        .await?;

    let channels: HashSet<String> = regular_client.pub_sub_channels(Default::default()).await?;
    assert_eq!(4, channels.len());
    assert!(channels.contains("mychannel1"));
    assert!(channels.contains("mychannel2"));
    assert!(channels.contains("mychannel3"));
    assert!(channels.contains("otherchannel"));

    let channels: HashSet<String> = regular_client
        .pub_sub_channels(PubSubChannelsOptions::default().pattern("mychannel*"))
        .await?;
    assert_eq!(3, channels.len());
    assert!(channels.contains("mychannel1"));
    assert!(channels.contains("mychannel2"));
    assert!(channels.contains("mychannel3"));

    stream.close().await?;

    let channels: HashSet<String> = regular_client.pub_sub_channels(Default::default()).await?;
    assert_eq!(0, channels.len());

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn pub_sub_numpat() -> Result<()> {
    let pub_sub_client = get_test_client().await?;
    let regular_client = get_test_client().await?;

    let num_patterns = regular_client.pub_sub_numpat().await?;
    assert_eq!(0, num_patterns);

    let stream = pub_sub_client.psubscribe(["mychannel*"]).await?;

    let num_patterns = regular_client.pub_sub_numpat().await?;
    assert_eq!(1, num_patterns);

    stream.close().await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn pub_sub_numsub() -> Result<()> {
    let pub_sub_client = get_test_client().await?;
    let regular_client = get_test_client().await?;

    let num_sub: HashMap<String, usize> = regular_client
        .pub_sub_numsub(["mychannel1", "mychannel2"])
        .await?;
    assert_eq!(2, num_sub.len());
    assert_eq!(Some(&0usize), num_sub.get("mychannel1"));
    assert_eq!(Some(&0usize), num_sub.get("mychannel2"));

    let stream = pub_sub_client
        .subscribe(["mychannel1", "mychannel2"])
        .await?;

    let num_sub: HashMap<String, usize> = regular_client
        .pub_sub_numsub(["mychannel1", "mychannel2"])
        .await?;
    assert_eq!(2, num_sub.len());
    assert_eq!(Some(&1usize), num_sub.get("mychannel1"));
    assert_eq!(Some(&1usize), num_sub.get("mychannel2"));

    stream.close().await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn pubsub_shardchannels() -> Result<()> {
    let pub_sub_client = get_cluster_test_client().await?;
    let regular_client = get_cluster_test_client().await?;

    // cleanup
    regular_client.flushdb(FlushingMode::Sync).await?;

    let mut pub_sub_stream = pub_sub_client.ssubscribe("mychannel").await?;
    regular_client.spublish("mychannel", "mymessage").await?;

    let message = pub_sub_stream.next().await.unwrap()?;
    let channel: String = String::from_utf8(message.channel).unwrap();
    let payload: String = String::from_utf8(message.payload).unwrap();

    assert_eq!("mychannel", channel);
    assert_eq!("mymessage", payload);

    regular_client.set("key", "value").await?;
    let value: String = regular_client.get("key").await?;
    assert_eq!("value", value);

    pub_sub_stream.close().await?;

    let mut pub_sub_stream = pub_sub_client.ssubscribe("mychannel2").await?;
    regular_client.spublish("mychannel2", "mymessage2").await?;

    let message = pub_sub_stream.next().await.unwrap()?;
    let channel: String = String::from_utf8(message.channel).unwrap();
    let payload: String = String::from_utf8(message.payload).unwrap();

    assert_eq!("mychannel2", channel);
    assert_eq!("mymessage2", payload);

    pub_sub_stream.close().await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn subscribe_to_multiple_shardchannels() -> Result<()> {
    let pub_sub_client = get_cluster_test_client().await?;
    let regular_client = get_cluster_test_client().await?;

    // cleanup
    regular_client.flushdb(FlushingMode::Sync).await?;

    let mut pub_sub_stream = pub_sub_client
        .ssubscribe(["mychannel1{1}", "mychannel2{1}"])
        .await?;
    regular_client
        .spublish("mychannel1{1}", "mymessage1")
        .await?;
    regular_client
        .spublish("mychannel2{1}", "mymessage2")
        .await?;

    let message = pub_sub_stream.next().await.unwrap()?;
    let channel: String = String::from_utf8(message.channel).unwrap();
    let payload: String = String::from_utf8(message.payload).unwrap();

    assert_eq!("mychannel1{1}", channel);
    assert_eq!("mymessage1", payload);

    let message = pub_sub_stream.next().await.unwrap()?;
    let channel: String = String::from_utf8(message.channel).unwrap();
    let payload: String = String::from_utf8(message.payload).unwrap();

    assert_eq!("mychannel2{1}", channel);
    assert_eq!("mymessage2", payload);

    pub_sub_stream.close().await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn pub_sub_shardchannels() -> Result<()> {
    let pub_sub_client = get_cluster_test_client().await?;

    // find the master node matching the {1} hashtag
    let slot = pub_sub_client.cluster_keyslot("{1}").await?;
    let shard_results: Vec<ClusterShardResult> = pub_sub_client.cluster_shards().await?;
    let shard_index = shard_results
        .iter()
        .position(|s| s.slots[0].0 <= slot && slot <= s.slots[0].1)
        .unwrap();
    let shard_result = &shard_results[shard_index];
    let master_node = shard_result
        .nodes
        .iter()
        .find(|n| n.role == "master")
        .unwrap();

    let master_client =
        Client::connect((master_node.ip.clone(), master_node.port.unwrap()).into_config()?).await?;

    let pub_sub_stream = pub_sub_client
        .ssubscribe([
            "mychannel1{1}",
            "mychannel2{1}",
            "mychannel3{1}",
            "otherchannel{1}",
        ])
        .await?;

    let channels: HashSet<String> = master_client
        .pub_sub_shardchannels(Default::default())
        .await?;
    assert_eq!(4, channels.len());
    assert!(channels.contains("mychannel1{1}"));
    assert!(channels.contains("mychannel2{1}"));
    assert!(channels.contains("mychannel3{1}"));
    assert!(channels.contains("otherchannel{1}"));

    let channels: HashSet<String> = master_client
        .pub_sub_shardchannels(PubSubChannelsOptions::default().pattern("mychannel*"))
        .await?;
    assert_eq!(3, channels.len());
    assert!(channels.contains("mychannel1{1}"));
    assert!(channels.contains("mychannel2{1}"));
    assert!(channels.contains("mychannel3{1}"));

    pub_sub_stream.close().await?;

    let channels: HashSet<String> = master_client
        .pub_sub_shardchannels(Default::default())
        .await?;
    assert_eq!(0, channels.len());

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn pub_sub_shardnumsub() -> Result<()> {
    let pub_sub_client = get_cluster_test_client().await?;

    // find the master node matching the {1} hashtag
    let slot = pub_sub_client.cluster_keyslot("{1}").await?;
    let shard_results: Vec<ClusterShardResult> = pub_sub_client.cluster_shards().await?;
    let shard_index = shard_results
        .iter()
        .position(|s| s.slots[0].0 <= slot && slot <= s.slots[0].1)
        .unwrap();
    let shard_result = &shard_results[shard_index];
    let master_node = shard_result
        .nodes
        .iter()
        .find(|n| n.role == "master")
        .unwrap();

    let master_client =
        Client::connect((master_node.ip.clone(), master_node.port.unwrap()).into_config()?).await?;

    let num_sub: HashMap<String, usize> = master_client
        .pub_sub_shardnumsub(["mychannel1{1}", "mychannel2{1}"])
        .await?;
    assert_eq!(2, num_sub.len());
    assert_eq!(Some(&0usize), num_sub.get("mychannel1{1}"));
    assert_eq!(Some(&0usize), num_sub.get("mychannel2{1}"));

    let pub_sub_stream = pub_sub_client
        .ssubscribe(["mychannel1{1}", "mychannel2{1}"])
        .await?;

    let num_sub: HashMap<String, usize> = master_client
        .pub_sub_shardnumsub(["mychannel1{1}", "mychannel2{1}"])
        .await?;
    assert_eq!(2, num_sub.len());
    assert_eq!(Some(&1usize), num_sub.get("mychannel1{1}"));
    assert_eq!(Some(&1usize), num_sub.get("mychannel2{1}"));

    pub_sub_stream.close().await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn additional_sub() -> Result<()> {
    let pub_sub_client = get_test_client().await?;
    let regular_client = get_test_client().await?;

    // cleanup
    regular_client.flushdb(FlushingMode::Sync).await?;

    // 1st subscription
    let mut pub_sub_stream = pub_sub_client.subscribe("mychannel1").await?;

    // publish / receive
    regular_client.publish("mychannel1", "mymessage1").await?;

    let message = pub_sub_stream.next().await.unwrap()?;
    let channel: String = String::from_utf8(message.channel).unwrap();
    let payload: String = String::from_utf8(message.payload).unwrap();

    assert_eq!("mychannel1", channel);
    assert_eq!("mymessage1", payload);

    // 2nd subscription
    pub_sub_stream.subscribe("mychannel2").await?;

    // publish / receive
    regular_client.publish("mychannel1", "mymessage1").await?;
    regular_client.publish("mychannel2", "mymessage2").await?;

    let message = pub_sub_stream.next().await.unwrap()?;
    let channel: String = String::from_utf8(message.channel).unwrap();
    let payload: String = String::from_utf8(message.payload).unwrap();

    assert_eq!("mychannel1", channel);
    assert_eq!("mymessage1", payload);

    let message = pub_sub_stream.next().await.unwrap()?;
    let channel: String = String::from_utf8(message.channel).unwrap();
    let payload: String = String::from_utf8(message.payload).unwrap();

    assert_eq!("mychannel2", channel);
    assert_eq!("mymessage2", payload);

    // 3rd subscription
    pub_sub_stream.psubscribe("o*").await?;

    // publish / receive
    regular_client.publish("mychannel1", "mymessage1").await?;
    regular_client.publish("mychannel2", "mymessage2").await?;
    regular_client.publish("otherchannel", "mymessage3").await?;

    let message = pub_sub_stream.next().await.unwrap()?;
    let channel: String = String::from_utf8(message.channel).unwrap();
    let payload: String = String::from_utf8(message.payload).unwrap();

    assert_eq!("mychannel1", channel);
    assert_eq!("mymessage1", payload);

    let message = pub_sub_stream.next().await.unwrap()?;
    let channel: String = String::from_utf8(message.channel).unwrap();
    let payload: String = String::from_utf8(message.payload).unwrap();

    assert_eq!("mychannel2", channel);
    assert_eq!("mymessage2", payload);

    let message = pub_sub_stream.next().await.unwrap()?;
    let channel: String = String::from_utf8(message.channel).unwrap();
    let payload: String = String::from_utf8(message.payload).unwrap();

    assert_eq!("otherchannel", channel);
    assert_eq!("mymessage3", payload);

    // close
    pub_sub_stream.close().await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn auto_resubscribe() -> Result<()> {
    let pub_sub_client = get_test_client().await?;
    let regular_client = get_test_client().await?;

    let pub_sub_client_id = pub_sub_client.client_id().await?;
    let mut pub_sub_stream = pub_sub_client.subscribe("mychannel").await?;
    pub_sub_stream.psubscribe("o*").await?;

    let mut on_reconnect = pub_sub_client.on_reconnect();

    regular_client
        .client_kill(ClientKillOptions::default().id(pub_sub_client_id))
        .await?;

    // wait for reconnection before publishing
    on_reconnect.recv().await.unwrap();

    regular_client.publish("mychannel", "mymessage").await?;
    regular_client
        .publish("otherchannel", "othermessage")
        .await?;

    let message = pub_sub_stream.try_next().await?.unwrap();
    let channel: String = String::from_utf8(message.channel).unwrap();
    let payload: String = String::from_utf8(message.payload).unwrap();

    assert_eq!("mychannel", channel);
    assert_eq!("mymessage", payload);

    let message = pub_sub_stream.try_next().await?.unwrap();
    let pattern: String = String::from_utf8(message.pattern).unwrap();
    let channel: String = String::from_utf8(message.channel).unwrap();
    let payload: String = String::from_utf8(message.payload).unwrap();

    assert_eq!("otherchannel", channel);
    assert_eq!("o*", pattern);
    assert_eq!("othermessage", payload);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn no_auto_resubscribe() -> Result<()> {
    log_try_init();

    let mut config = get_default_addr().into_config()?;
    config.connection_name = "pub/sub".to_owned();
    config.auto_resubscribe = false;
    let pub_sub_client = Client::connect(config).await?;

    let mut config = get_default_addr().into_config()?;
    config.connection_name = "regular".to_owned();
    let regular_client = Client::connect(config).await?;

    let pub_sub_client_id = pub_sub_client.client_id().await?;
    let mut pub_sub_stream = pub_sub_client.subscribe("mychannel").await?;
    pub_sub_stream.psubscribe("o*").await?;

    let mut on_reconnect = pub_sub_client.on_reconnect();

    regular_client
        .client_kill(ClientKillOptions::default().id(pub_sub_client_id))
        .await?;

    // wait for reconnection before publishing
    on_reconnect.recv().await.unwrap();

    regular_client.publish("mychannel", "mymessage").await?;
    regular_client
        .publish("otherchannel", "othermessage")
        .await?;

    let message = pub_sub_stream.next().now_or_never();
    assert!(message.is_none());

    Ok(())
}
