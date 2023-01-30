pub mod file_handling;
pub mod server;

fn main() {
    file_handling::init_root();
    let _ = server::start_server();
}
