#![allow(dead_code)]

mod ash;
mod config;
mod bridge;
mod buffers;
mod logging;
mod server;
mod spi;

use std::io;

use config::Config;
use logging::setup_logging;
use server::Server;

/// Bridge starts by listening on the chosen port for a connection.
/// Once a connection is established, the server initializes the SPI device and
/// starts in the FAILED state.
/// 
/// ## FAILED State
/// 
/// When the Host sends a ASH RST frame, the device will perform a reset
/// transaction on the SPI device, and send back the RST ACK frame. If any
/// other frame is received, the server will send back an ERROR frame with the
/// code 0x02. When the reset is complete, the server enters the CONNECTED state;
/// 
/// ## CONNECTED State
/// 
/// In this state, the server will receive DATA frames from the TCP stream. It
/// will store them in a buffer be sent as commands to the SPI device. The
/// server will also add the frame number to a queue for piggy-backed
/// acknowledgements.
/// 
/// When a SPI command receives a response, the response data is queued for
/// delivery to the client.
/// 
/// ## Sequence numbers
/// 
/// The server will track and rewrite the sequence number of EZSP commands from
/// the Host in order to seamlessly issue callback commands to the NCP.
/// 
/// ## Callbacks
/// 
/// When the server detects a callback is ready on the NCP, it will send a
/// callback command to the NCP to receive the callback data and send it to the
/// Host.
/// 
/// ## Piggy-backed acknowledgement
/// 
/// The server will wait for a DATA frame to be ready for the Host to send a
/// bulk acknowledgement on top of the DATA frame. If no such frame is ready
/// within the timeout period, or if the acknowledgement window becomes full,
/// the server will send out an ACK frame.
#[tokio::main]
async fn main() -> Result<(), io::Error>{
    setup_logging();

    let config = Config::new("0.0.0.0".parse().unwrap(), 5555);
    let mut server = Server::new(config);

    server.run().await
}
