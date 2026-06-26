use std::error::Error;
use futures::Future;

/// Trait defining the application use cases. These are the core usages the user will be able to do.
/// They regroup different interactions (displays + user input)
/// Components like the CLI commands or the UI hook onto this abstraction to execute main logic.
pub trait AppUseCases {
    // Future: I promise to eventually give you a result (useful to make the UI wait for an async result)
    // Output = ... : When the task finishes, this is the type you get back.
    // "Result<(), Box<dyn Error>>": The task succeeds with nothing (()), 
    // or fails with "any kind of error" (Box<dyn Error>).
    // "+ Send": This task is safe to move to a different CPU thread if needed. Required to make it "Future"
    fn advertise(&self) -> impl Future<Output = Result<(), Box<dyn Error>>> + Send;
    fn discover(&self) -> impl Future<Output = Result<(), Box<dyn Error>>> + Send;
    fn send(&self, ip: String, port: Option<u16>, file_path: String) -> impl Future<Output = Result<(), Box<dyn Error>>> + Send;
    fn receive(&self) -> impl Future<Output = Result<(), Box<dyn Error>>> + Send;
    fn advertise_and_receive(&self) -> impl Future<Output = Result<(), Box<dyn Error>>> + Send;
    /// Pre: assumes a discovery has been made, and the peers list is populated
    /// Enables a user to send a file to a already detected other peer (asks for peer selection)
    fn interactive_send(&self, file_path: String) -> impl Future<Output = Result<(), Box<dyn Error>>> + Send;
    
    // User info management use cases
    fn get_ip(&self) -> impl Future<Output = Result<(), Box<dyn Error>>> + Send;
    fn get_name(&self) -> impl Future<Output = Result<(), Box<dyn Error>>> + Send;
    fn set_name(&self, name: String) -> impl Future<Output = Result<(), Box<dyn Error>>> + Send;
    fn set_name_default(&self) -> impl Future<Output = Result<(), Box<dyn Error>>> + Send;
    fn get_port(&self) -> impl Future<Output = Result<(), Box<dyn Error>>> + Send;
    fn set_port(&self, port: u16) -> impl Future<Output = Result<(), Box<dyn Error>>> + Send;
    fn set_port_default(&self) -> impl Future<Output = Result<(), Box<dyn Error>>> + Send;
    fn info(&self) -> impl Future<Output = Result<(), Box<dyn Error>>> + Send;
}