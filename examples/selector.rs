extern crate pretty_env_logger;
extern crate tokio;
extern crate u8_selector;

use tokio::prelude::*;

// require rust 1.26
fn main() -> Result<(), std::io::Error> {
    pretty_env_logger::init();

    let addr = "127.0.0.1:7777".parse().map_err(|e| {
        eprintln!("{}", e);
        std::io::Error::from(std::io::ErrorKind::Other)
    })?;
    let connection = tokio::net::TcpStream::connect(&addr);

    let work = connection
        .and_then(|c| {
            let u8c = u8_selector::U8Codec::new(c);
            let mut selector = u8_selector::U8Frowarder::new(u8c);

            // Catch all `A`
            let worker1 = selector.catch_all(b'A').for_each(|(v, _tx)| {
                println!("Got from worker1 {}", v);
                assert!(v == b'A');
                Ok(())
            });

            // Catch all `B`
            let worker2 = selector.catch_all(b'B').for_each(|(v, tx)| {
                println!("Ping pong {}", v);
                // send it back
                tx.unbounded_send(v).unwrap();
                Ok(())
            });

            // Create selector work
            let selector_work = selector
                .and_then(|_| {
                    println!("End of selector work ");
                    Ok(())
                })
                .map_err(|e| {
                    eprintln!("selector error {}", e);
                });

            tokio::spawn(worker1);
            tokio::spawn(worker2);
            tokio::spawn(selector_work);
            Ok(())
        })
        .map_err(|e| {
            eprintln!("{}", e);
        });

    tokio::run(work);
    Ok(())
}
