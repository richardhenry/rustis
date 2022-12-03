An asynchronous Redis client for Rust.


[![Crate](https://img.shields.io/crates/v/rustis.svg)](https://crates.io/crates/rustis)
[![Build](https://github.com/dahomey-technologies/rustis/actions/workflows/compile_and_test.yml/badge.svg)](https://github.com/dahomey-technologies/rustis/actions/workflows/compile_and_test.yml)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

## Documentation
[Official Documentation](https://docs.rs/rustis/latest/rustis/)

## Philosophy
* Low allocations
* Full async library
* Lock free implementation
* Rust idiomatic API

## Features
* Support all [Redis Commands](https://redis.io/commands/) until Redis 7.0
* Async support ([tokio](https://tokio.rs/) or [async-std](https://async.rs/))
* Different client types:
  * Single client
  * [Multiplexed](https://redis.com/blog/multiplexing-explained/) client
  * Pooled client manager (based on [bb8](https://docs.rs/bb8/latest/bb8/))
* Automatic command batching
* [Pipelining](https://redis.io/docs/manual/pipelining/) support
* Configuration with Redis URL or dedicated builder
* [TLS](https://redis.io/docs/manual/security/encryption/) support
* [Transaction](https://redis.io/docs/manual/transactions/) support
* [Pub/sub](https://redis.io/docs/manual/pubsub/) support
* [Sentinel](https://redis.io/docs/manual/sentinel/) support
* [LUA Scripts/Functions](https://redis.io/docs/manual/programmability/) support
* [Cluster](https://redis.io/docs/manual/scaling/) support
* [Redis Stack](https://redis.io/docs/stack/) support:
  * [RedisJSON v2.4](https://redis.io/docs/stack/json/) support
  * [RedisSearch v2.6](https://redis.io/docs/stack/search/) support
  * [RedisGraph v2.10](https://redis.io/docs/stack/graph/) support
  * [RedisBloom v2.4](https://redis.io/docs/stack/bloom/) support
  * [RedisTimeSeries v1.8](https://redis.io/docs/stack/timeseries/) support

## Roadmap
* Advanced reconnection strategy
* Advanced configuration (timeouts)
* Improve documentation 

## Basic Usage

 ```rust
 use rustis::{
     Client, FlushingMode,
     Result, ServerCommands, StringCommands
 };

 #[tokio::main]
 async fn main() -> Result<()> {
     let mut client = Client::connect("127.0.0.1:6379").await?;
     client.flushdb(FlushingMode::Sync).await?;

     client.set("key", "value").await?;
     let value: String = client.get("key").await?;
     println!("value: {value:?}");

     Ok(())
 }
 ```
