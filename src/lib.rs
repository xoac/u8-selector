extern crate bytes;
#[macro_use]
extern crate futures;
extern crate tokio;
use bytes::{BufMut, BytesMut};
use futures::sync::mpsc::{unbounded, UnboundedReceiver, UnboundedSender};
use std::collections::HashMap;
use std::io;
use tokio::prelude::*;
#[macro_use]
extern crate log;

type Tx = UnboundedSender<u8>;
type Rx = UnboundedReceiver<u8>;

use tokio::net::TcpStream;

/// Get bytes from socket
pub struct U8Codec {
    socket: TcpStream,
    rd: BytesMut,
    wr: BytesMut,
}

impl U8Codec {
    pub fn new(socket: TcpStream) -> U8Codec {
        U8Codec {
            socket: socket,
            rd: BytesMut::new(),
            wr: BytesMut::new(),
        }
    }

    fn fill_read_buf(&mut self) -> Poll<(), io::Error> {
        loop {
            // Ensure the read buffer has capacity.
            //
            // This might result in an internal allocation.
            self.rd.reserve(1024);

            // Read data into the buffer.
            let n = try_ready!(self.socket.read_buf(&mut self.rd));

            if n == 0 {
                return Ok(Async::Ready(()));
            }
        }
    }

    fn buffer(&mut self, b: u8) {
        // Ensure the buffer has capacity. Ideally this would not be unbounded,
        // but to keep the example simple, we will not limit this.
        self.wr.reserve(1);

        // Push the line onto the end of the write buffer.
        //
        // The `put` function is from the `BufMut` trait.
        self.wr.put(b);
    }

    /// Flush the write buffer to the socket
    fn poll_flush(&mut self) -> Poll<(), io::Error> {
        // As long as there is buffered data to write, try to write it.
        while !self.wr.is_empty() {
            // Try to write some bytes to the socket
            let n = try_ready!(self.socket.poll_write(&self.wr));

            // As long as the wr is not empty, a successful write should
            // never write 0 bytes.
            assert!(n > 0);

            // This discards the first `n` bytes of the buffer.
            let _ = self.wr.split_to(n);
        }

        Ok(Async::Ready(()))
    }
}

impl Stream for U8Codec {
    type Item = u8;
    type Error = io::Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        let sock_closed = self.fill_read_buf()?.is_ready();
        debug!("Poll U8Codec!");
        if self.rd.len() > 0 {
            let r = self.rd.split_to(1);
            // first return Some(first_value)
            let v = Some(*r.first().unwrap());
            return Ok(Async::Ready(v));
        }

        if sock_closed {
            Ok(Async::Ready(None))
        } else {
            Ok(Async::NotReady)
        }
    }
}

pub struct U8Frowarder {
    io: U8Codec,
    forwarder: HashMap<u8, Tx>,
    // here we reciving u8 we wanna send to socket
    rx: Rx,
    // a tx connection just for clone() purpose
    tx: Tx,
}

impl U8Frowarder {
    pub fn new(io: U8Codec) -> U8Frowarder {
        let (tx, rx) = unbounded();

        U8Frowarder {
            io,
            forwarder: HashMap::new(),
            rx,
            tx,
        }
    }

    /// Catch all u8 from socket that are equal to `c`
    ///
    /// # Panic
    /// if called twice with the same `c`
    pub fn catch_all(&mut self, c: u8) -> U8Stream {
        let (tx, rx) = unbounded();

        if let Some(_sth) = self.forwarder.insert(c, tx) {
            panic!("catch_all called twice with the same value!");
        }

        // we clone tx to allow U8Stream write to socet
        U8Stream {
            tx: self.tx.clone(),
            rx,
        }
    }
}

impl Future for U8Frowarder {
    type Item = ();
    type Error = io::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        debug!("Poll U8Frowarder!");
        loop {
            match self.rx.poll().unwrap() {
                Async::Ready(Some(v)) => {
                    // Buffer the line. Once all lines are buffered, they will
                    // be flushed to the socket (right below).
                    self.io.buffer(v);
                }
                _ => break,
            }
        }

        let _ = self.io.poll_flush()?;

        while let Async::Ready(b) = self.io.poll()? {
            debug!("Received u8  {:?}", b);

            if let Some(message) = b {
                if let Some(tx) = self.forwarder.get(&message) {
                    tx.unbounded_send(message).unwrap();
                } else {
                    debug!("\t not captured!");
                }
            } else {
                return Ok(Async::Ready(()));
            }
        }

        Ok(Async::NotReady)
    }
}

pub struct U8Stream {
    tx: Tx,
    rx: Rx,
}

impl Stream for U8Stream {
    type Item = (u8, Tx);
    type Error = ();
    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        debug!("Poll U8Stream!");
        while let Ok(Async::Ready(v)) = self.rx.poll() {
            if let Some(v) = v {
                return Ok(Async::Ready(Some((v, self.tx.clone()))));
            } else {
                return Ok(Async::Ready(None));
            }
        }
        Ok(Async::NotReady)
    }
}
