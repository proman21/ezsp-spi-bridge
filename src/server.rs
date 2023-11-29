use std::io;

use tokio::{net::TcpListener, select, spawn};
use tracing::{error, info, instrument, warn};

use crate::{bridge::Bridge, config::Config};

#[derive(Debug)]
pub struct Server {
    config: Config,
}

impl Server {
    pub fn new(config: Config) -> Server {
        Server { config }
    }

    #[instrument]
    pub async fn run(&mut self) -> io::Result<()> {
        let addr = self.config.socket_addr();
        let listener = TcpListener::bind(addr).await.map_err(|e| {
            error!({ error = ?e }, "Unable to bind listener at {}: {}", addr, e);
            e
        })?;
        info!("Server listening at {}", addr);

        loop {
            let (client, client_addr) = listener.accept().await.map_err(|e| {
                error!({ error = ?e }, "Couldn't establish connection to client: {}", e);
                e
            })?;
            info!("Received connection from {}", client_addr);

            let mut bridge = spawn(async {
                let mut bridge = Bridge::new(client);
                bridge.handle().await
            });

            select! {
                res = (&mut bridge) => {
                    match res {
                        Ok(r) => {
                            info!("Connection to {} closed: {:?}", client_addr, r);
                            continue;
                        },
                        Err(e) if e.is_cancelled() => {
                            warn!("Bridge to {} was cancelled", client_addr);
                            continue;
                        },
                        Err(e) => {
                            let error = if e.is_panic() {
                                Some(e.into_panic())
                            } else {
                                None
                            };
                            error!({ ?error }, "Bridge task paniced!");
                            break;
                        }
                    }
                },
                accept = listener.accept() => {
                    match accept {
                        Ok((_c, a)) => {
                            info!("Additional connection received from {}, dropping.", a);
                        }
                        Err(e) => {
                            warn!("Error while accepting additional connection: {}", e)
                        },
                    }
                }
            }
        }
        Ok(())
    }
}
