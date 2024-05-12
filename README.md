### Trading cli 

### Requirements
- Cargo 
- Rust 


### Usage

1. build with ``` cargo build ```
2. To run the program, either use ``` Cargo run ``` or directly call the executable in the target/debug or target/release folder
3. testing, run  ``` Cargo test ``` 


### Todos
- [ ] Extend app to use Clap with the option of using different assets and exchanges 
- [ ] Build a tui around the cli with Table support for rendering the order table
- [x] Fix arbitrage detection


### Testing
To run tests, use ``` cargo test ``` in cli.
