use crate::{resp::Value, Client, ClientCommandResult, PubSubCommands, PubSubReceiver, Result};
use futures::{Stream, StreamExt};
use std::{
    pin::Pin,
    task::{Context, Poll},
};

/// Stream to get messages from the channels or patterns [`subscribed`](https://redis.io/docs/manual/pubsub/) to
///
/// # Example
/// ```
/// use redis_driver::{
///     resp::cmd, Client, ClientCommandResult, FlushingMode,
///     PubSubCommands, ServerCommands, Result
/// };
/// use futures::StreamExt;
///
/// #[tokio::main]
/// async fn main() -> Result<()> {
///     let pub_sub_client = Client::connect("127.0.0.1:6379").await?;
///     let regular_client = Client::connect("127.0.0.1:6379").await?;
///
///     regular_client.flushdb(FlushingMode::Sync).send().await?;
///
///     let mut pub_sub_stream = pub_sub_client.subscribe("mychannel").await?;
/// 
///     regular_client
///         .publish("mychannel", "mymessage")
///         .send()
///         .await?;
///
///     let (channel, message): (String, String) = pub_sub_stream
///         .next()
///         .await
///         .unwrap()?
///         .into()?;
/// 
///     assert_eq!("mychannel", channel);
///     assert_eq!("mymessage", message);
/// 
///     pub_sub_stream.close().await?;
///
///     Ok(())
/// }
/// ```
pub struct PubSubStream {
    closed: bool,
    channels: Vec<String>,
    patterns: Vec<String>,
    receiver: PubSubReceiver,
    client: Client,
}

impl PubSubStream {
    pub(crate) fn from_channels(
        channels: Vec<String>,
        receiver: PubSubReceiver,
        client: Client,
    ) -> Self {
        Self {
            closed: false,
            channels,
            patterns: Vec::new(),
            receiver,
            client,
        }
    }

    pub(crate) fn from_patterns(
        patterns: Vec<String>,
        receiver: PubSubReceiver,
        client: Client,
    ) -> Self {
        Self {
            closed: false,
            channels: Vec::new(),
            patterns,
            receiver,
            client,
        }
    }

    pub async fn close(&mut self) -> Result<()> {
        let mut channels = Vec::<String>::new();
        std::mem::swap(&mut channels, &mut self.channels);
        if !channels.is_empty() {
            let _result = self.client.unsubscribe(channels).send().await?;
        }

        let mut patterns = Vec::<String>::new();
        std::mem::swap(&mut patterns, &mut self.patterns);
        if !patterns.is_empty() {
            let _result = self.client.punsubscribe(patterns).send().await?;
        }

        self.closed = true;

        Ok(())
    }
}

impl Stream for PubSubStream {
    type Item = Result<Value>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        if self.closed {
            Poll::Ready(None)
        } else {
            self.get_mut().receiver.poll_next_unpin(cx)
        }
    }
}

impl Drop for PubSubStream {
    fn drop(&mut self) {
        if self.closed {
            return;
        }

        let mut channels = Vec::<String>::new();
        std::mem::swap(&mut channels, &mut self.channels);
        if !channels.is_empty() {
            let _result = self.client.unsubscribe(channels).send_and_forget();
        }

        let mut patterns = Vec::<String>::new();
        std::mem::swap(&mut patterns, &mut self.patterns);
        if !patterns.is_empty() {
            let _result = self.client.punsubscribe(patterns).send_and_forget();
        }
    }
}