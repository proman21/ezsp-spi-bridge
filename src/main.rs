#![allow(dead_code)]

mod ash;
mod bridge;
mod buffers;
mod logging;
mod settings;
mod spi;
mod test;

use anyhow::{Context, Result};
use bridge::handle;
use logging::setup_logging;
use settings::Settings;
use spi::{create_spi_peripheral, spi_device_handle};
use tokio::net::TcpListener;
use tracing::{error, info, instrument};

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
#[instrument]
#[tokio::main]
async fn main() -> Result<()> {
    let settings = Settings::new()?;
    setup_logging(settings.loglevel);

    let addr = settings.socket_addr();
    let listener = TcpListener::bind(addr).await.map_err(|e| {
        error!({ error = ?e }, "Unable to bind listener at {}: {}", addr, e);
        e
    })?;
    let peripheral = create_spi_peripheral(&settings.spi)
        .await
        .context("Unable to open SPI peripheral")?;
    let (actor, device) = spi_device_handle(peripheral);
    info!("Server listening at {}", addr);

    loop {
        let (client, client_addr) = loop {
            match listener.accept().await {
                Ok(v) => break v,
                Err(e) => {
                    error!(error = ?e, "Failed to accept connection from client: {}", e);
                }
            };
        };
        info!(%client_addr, "Received connection from {}", client_addr);

        if let Err(e) = handle(client, device.clone()).await {
            error!(error = %e, %client_addr, "Bridge encountered an unrecoverable error: {}", e);
            break;
        } else {
            info!(%client_addr, "Connection to {} closed", client_addr);
        }
    }
    Ok(())
}
