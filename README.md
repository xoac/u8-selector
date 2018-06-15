# Example of using tokio

Run server that can work with our app:
-----
```
printf 'AAAAAAABBBBBABBBBBBBBAAAABOB\r\n' | netcat -l -p 7777  
```


Run application:
----
```
cargo build --examples
./target/debug/examples/selector
```

Run application with trace debug level:
-----
```
cargo build --examples
RUST_LOG=trace ./target/debug/examples/selector 
```

