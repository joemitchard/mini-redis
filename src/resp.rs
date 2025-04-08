use std::string;
use std::sync::Mutex;
use bytes::BytesMut;
use tokio::net::TcpStream;
use anyhow::Result;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[derive(Clone, Debug)]
pub enum Value {
    SimpleString(String),
    BulkString(String),
    NullBulkString(),
    Array(Vec<Value>),
}

impl Value {
    pub fn serialise(self) -> String {
        match &self {
            Value::SimpleString(s) => format!("+{}\r\n", s),
            Value::BulkString(s) => format!("${}\r\n{}\r\n", s.len(), s),
            Value::NullBulkString() => String::from("$-1\r\n"),
            _ => panic!("serialising value is not supported")
        }
    }
}

pub struct RespHandler {
    stream: TcpStream,
    buffer: BytesMut,
}

impl RespHandler {
    pub fn new(stream: TcpStream) -> Self {
        RespHandler {
            stream,
            buffer: BytesMut::with_capacity(512),
        }
    }

    pub async fn read_value(&mut self) -> Result<Option<Value>> {
        let bytes_read = self.stream.read_buf(&mut self.buffer).await?;
       
        if bytes_read == 0 {
            return Ok(None);
        }
        
        let (value, _) = parse_message(self.buffer.split())?;
        Ok(Some(value))
    }

    pub async fn write_value(&mut self, value: Value) -> Result<()> {
        println!("responding with value: {:?}", value.clone());
        println!("responding with bytes: {:?}", value.clone().serialise());
        self.stream.write( value.serialise().as_bytes()).await?;
        Ok(())
    }
}

fn parse_message(buffer: BytesMut) -> Result<(Value, usize)> {
    match buffer[0] as char {
        '+' => parse_simple_string(buffer),
        '*' => parse_array(buffer),
        '$' => parse_bulk_string(buffer),
        _ => Err(anyhow::anyhow!("unknown value type {:?}", buffer))
    }
}

fn parse_simple_string(buffer: BytesMut) -> Result<(Value, usize)> {
    // skipping first char
    if let Some((line, length)) = read_until_crlf(&buffer[1..]) {
        let string = String::from_utf8(line.to_vec())?;
        return Ok((Value::SimpleString(string), length + 1));
    }

    Err(anyhow::anyhow!("invalid string {:?}", buffer))
}

fn parse_array(buffer: BytesMut) -> Result<(Value, usize)> {
    // skipping first char
    let (array_length, mut bytes_consumed) = if let Some((line, length)) = read_until_crlf(&buffer[1..]) {
        let array_length = parse_int(line)?;

        (array_length, length + 1)
    } else {
        return Err(anyhow::anyhow!("invalid array format, {:?}", buffer));
    };
    let mut items = vec![];

    for _ in 0..array_length {
        let (array_item, length) = parse_message(BytesMut::from(&buffer[bytes_consumed..]))?;
        bytes_consumed += length;
        items.push(array_item);
    }

    Ok((Value::Array(items), bytes_consumed))
}



fn parse_bulk_string(buffer: BytesMut) -> Result<(Value, usize)> {
    // skipping first char
    let (bulk_str_length, bytes_consumed) = if let Some((line, length)) = read_until_crlf(&buffer[1..]) {
        let bulk_str_length = parse_int(line)?;

        (bulk_str_length, length + 1)
    } else {
        return Err(anyhow::anyhow!("invalid array format, {:?}", buffer));
    };

    let end_of_bulk_str = bytes_consumed + bulk_str_length as usize;
    // +1 for crlf
    let total_parsed = end_of_bulk_str + 2;

    let value = String::from_utf8(buffer[bytes_consumed..end_of_bulk_str].to_vec())?;

    Ok((Value::BulkString(value), total_parsed))
}

fn read_until_crlf(buffer: &[u8]) -> Option<(&[u8], usize)> {
    for i in 1..buffer.len() {
        if buffer[i - 1] == b'\r' && buffer[i] == b'\n' {
            return Some((&buffer[0..i - 1], i + 1));
        }
    }

    None
}

fn parse_int(buffer: &[u8]) -> Result<i64> {
    Ok(String::from_utf8(buffer.to_vec())?.parse::<i64>()?)
}