#![allow(unused_imports)]

mod resp;
mod cache;

use std::sync::{Arc, Mutex};
use tokio::net::{TcpListener, TcpStream};
use crate::resp::Value;
use anyhow::Result;
use crate::cache::Cache;

#[derive(Debug, Clone)]
enum Command {
    Ping,
    Echo(Value),
    Set((String, String, Option<usize>)),
    Get(String),
    Unknown(String)
}

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:6379").await.unwrap();

    let cache: Arc<Mutex<Cache>> = Arc::new(Mutex::new(Cache::new()));
    
    loop {
        let stream = listener.accept().await;
        
        match stream {
            Ok((stream, _)) => {
                println!("accepted a new connection");

                let cache = Arc::clone(&cache);
                
                tokio::spawn(async move {
                    handle_conn(stream, cache).await
                });
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}

async fn handle_conn(stream: TcpStream, cache: Arc<Mutex<Cache>>) {
    let mut handler = resp::RespHandler::new(stream);
    
    loop {
        let value = handler.read_value().await.unwrap(); 
       
        println!("received value {:?}", value);
        
        let response = if let Some(v) = value {
            let command = extract_command(v);
            
            match command {
                Command::Ping => Value::SimpleString(String::from("PONG")),
                Command::Echo(v) => v,
                Command::Set((k, v, e)) => {
                    let mut cache = cache.lock().expect("failed to get `set` lock");
                    cache.set(&k, &v, &e);
                    Value::SimpleString(String::from("OK"))
                }
                Command::Get(k) => {
                    let cache = cache.lock().expect("failed to get `get` lock");
                    match cache.get(&k) {
                        Some(v) => {
                            println!("got value {}", v);
                            Value::BulkString(String::from(v))
                        },
                        None => Value::NullBulkString()
                    }
                }
                Command::Unknown(e) => panic!("cannot handle command: {}", e)
            }
        } else { 
          break;  
        };
        
        println!("responding with: {:?}", response);
        
        handler.write_value(response).await.unwrap();
    }
}

fn extract_command(value: Value) -> Command {
    match value {
        Value::Array(a) => {
            
            let command = unpack_bulk_str(a.first().unwrap().clone()).unwrap();
            let args: Vec<Value> = a.into_iter().skip(1).collect();
            
            match command.to_lowercase().as_str() { 
                "ping" => Command::Ping,
                "echo" => Command::Echo(args.first().expect("echo must provide a value").clone()),
                "set" => {
                    let mut args = args.into_iter();
                   
                    println!("args: {:?}", args);
                    
                    let key = unpack_bulk_str(args.next().unwrap().clone()).expect("set must provide a key");
                    let value = unpack_bulk_str(args.next().unwrap().clone()).expect("set must provide a value");

                    let mut expire = None;

                    let modifier = args.next();

                    match modifier {
                        Some(modifier) => {
                            match unpack_bulk_str(modifier) {
                                Ok(m) if m.to_lowercase() == "px" => {
                                    let duration = unpack_bulk_str(args.next().unwrap().clone()).unwrap();
                                    let duration = duration.parse().expect("duration should be a valid number");
                                    expire = Some(duration);
                                }
                                Ok(m) if m.to_lowercase() == "ex" => {
                                    let duration = unpack_bulk_str(args.next().unwrap().clone()).unwrap();
                                    let duration : usize = duration.parse().expect("duration should be a valid number");
                                    expire = Some(duration/100);
                                }
                                _ => {
                                    println!("invalid duration type")
                                }
                            }
                        },
                        None => {
                            println!("no modifier matched")
                        }
                    }

                    Command::Set((key, value, expire))
                },
                "get" => {
                    let key = unpack_bulk_str(args.first().unwrap().clone()).expect("get must provide a key");
                    Command::Get(key)
                }
                s => Command::Unknown(s.to_string())
            }
        }
        _ => Command::Unknown(String::from("failed to parse command"))
    }
}

fn unpack_bulk_str(value: Value) -> Result<String> {
    match value {
        Value::BulkString(s) => Ok(s),
        _ => Err(anyhow::anyhow!("expected command to be a bulk string"))
    }
}
