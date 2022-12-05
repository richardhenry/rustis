use crate::{
    client::{ClientPreparedCommand, InnerClient},
    commands::InternalPubSubCommands,
    resp::{FromValue, Value},
    Error, PubSubReceiver, Result,
};
use futures::{Stream, StreamExt};
use std::{
    pin::Pin,
    task::{Context, Poll},
};

/// Pub/Sub Message that can be streamed from [`PubSubStream`](PubSubStream)
pub struct PubSubMessage {
    pub pattern: Value,
    pub channel: Value,
    pub payload: Value,
}

impl PubSubMessage {
    pub(crate) fn from_message(channel: Value, payload: Value) -> Self {
        Self {
            pattern: Value::Nil,
            channel,
            payload,
        }
    }

    pub(crate) fn from_pmessage(pattern: Value, channel: Value, payload: Value) -> Self {
        Self {
            pattern,
            channel,
            payload,
        }
    }

    /// Convert and consumes the message's pattern
    /// 
    /// For subscriptions with no pattern ([`PubSubCommands::subscribe`](crate::commands::PubSubCommands::subscribe)), 
    /// the message's pattern is internally stored as [`Value::Nil`](Value::Nil).
    pub fn get_pattern<P: FromValue>(&mut self) -> Result<P> {
        let mut pattern = Value::Nil;
        std::mem::swap(&mut pattern, &mut self.pattern);
        pattern.into()
    }

    /// Convert and consumes the message's channel
    pub fn get_channel<P: FromValue>(&mut self) -> Result<P> {
        let mut channel = Value::Nil;
        std::mem::swap(&mut channel, &mut self.channel);
        channel.into()
    }

    /// Convert and consumes the message's payload
    pub fn get_payload<P: FromValue>(&mut self) -> Result<P> {
        let mut payload = Value::Nil;
        std::mem::swap(&mut payload, &mut self.payload);
        payload.into()
    }
}

/// Stream to get messages from the channels or patterns [`subscribed`](https://redis.io/docs/manual/pubsub/) to
///
/// # Example
/// ```
/// use rustis::{
///     client::{Client, ClientPreparedCommand},
///     commands::{FlushingMode, PubSubCommands, ServerCommands},
///     resp::cmd,
///     Result,
/// };
/// use futures::StreamExt;
///
/// #[tokio::main]
/// async fn main() -> Result<()> {
///     let mut pub_sub_client = Client::connect("127.0.0.1:6379").await?;
///     let mut regular_client = Client::connect("127.0.0.1:6379").await?;
///
///     regular_client.flushdb(FlushingMode::Sync).await?;
///
///     let mut pub_sub_stream = pub_sub_client.subscribe("mychannel").await?;
///
///     regular_client.publish("mychannel", "mymessage").await?;
///
///     let mut message = pub_sub_stream.next().await.unwrap()?;
///     let channel: String = message.get_channel()?;
///     let payload: String = message.get_payload()?;
///
///     assert_eq!("mychannel", channel);
///     assert_eq!("mymessage", payload);
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
    shardchannels: Vec<String>,
    receiver: PubSubReceiver,
    client: InnerClient,
}

impl PubSubStream {
    pub(crate) fn from_channels(
        channels: Vec<String>,
        receiver: PubSubReceiver,
        client: InnerClient,
    ) -> Self {
        Self {
            closed: false,
            channels,
            patterns: Vec::new(),
            shardchannels: Vec::new(),
            receiver,
            client,
        }
    }

    pub(crate) fn from_patterns(
        patterns: Vec<String>,
        receiver: PubSubReceiver,
        client: InnerClient,
    ) -> Self {
        Self {
            closed: false,
            channels: Vec::new(),
            patterns,
            shardchannels: Vec::new(),
            receiver,
            client,
        }
    }

    pub(crate) fn from_shardchannels(
        shardchannels: Vec<String>,
        receiver: PubSubReceiver,
        client: InnerClient,
    ) -> Self {
        Self {
            closed: false,
            channels: Vec::new(),
            patterns: Vec::new(),
            shardchannels,
            receiver,
            client,
        }
    }

    pub async fn close(&mut self) -> Result<()> {
        let mut channels = Vec::<String>::new();
        std::mem::swap(&mut channels, &mut self.channels);
        if !channels.is_empty() {
            self.client.unsubscribe(channels).await?;
        }

        let mut patterns = Vec::<String>::new();
        std::mem::swap(&mut patterns, &mut self.patterns);
        if !patterns.is_empty() {
            self.client.punsubscribe(patterns).await?;
        }

        let mut shardchannels = Vec::<String>::new();
        std::mem::swap(&mut shardchannels, &mut self.shardchannels);
        if !shardchannels.is_empty() {
            self.client.sunsubscribe(shardchannels).await?;
        }

        self.closed = true;

        Ok(())
    }
}

impl Stream for PubSubStream {
    type Item = Result<PubSubMessage>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        fn extract_message(message: Value) -> Result<PubSubMessage> {
            let parts: Vec<Value> = message.into()?;
            let mut iter = parts.into_iter();

            match (iter.next(), iter.next(), iter.next(), iter.next()) {
                (Some(pattern), Some(channel), Some(payload), None) => {
                    Ok(PubSubMessage::from_pmessage(pattern, channel, payload))
                }
                (Some(channel), Some(payload), None, None) => {
                    Ok(PubSubMessage::from_message(channel, payload))
                }
                _ => Err(Error::Client("Cannot parse PubSubMessage".to_owned())),
            }
        }

        if self.closed {
            Poll::Ready(None)
        } else {
            match self.get_mut().receiver.poll_next_unpin(cx) {
                Poll::Ready(Some(Ok(message))) => Poll::Ready(Some(extract_message(message))),
                Poll::Ready(None) => Poll::Ready(None),
                Poll::Ready(Some(Err(e))) => Poll::Ready(Some(Err(e))),
                Poll::Pending => Poll::Pending,
            }
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
            let _result = self.client.unsubscribe(channels).forget();
        }

        let mut patterns = Vec::<String>::new();
        std::mem::swap(&mut patterns, &mut self.patterns);
        if !patterns.is_empty() {
            let _result = self.client.punsubscribe(patterns).forget();
        }

        let mut shardchannels = Vec::<String>::new();
        std::mem::swap(&mut shardchannels, &mut self.shardchannels);
        if !shardchannels.is_empty() {
            let _result = self.client.sunsubscribe(shardchannels).forget();
        }
    }
}