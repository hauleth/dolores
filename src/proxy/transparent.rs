use tokio::io;
use tokio::io::AsyncWriteExt;

/// Transparent proxy
///
/// This proxy will forward all data **as is** so it is the downstream responsibility to handle TLS
/// termination
#[derive(Clone, Debug)]
pub struct Transparent;

#[async_trait]
impl super::Proxy for Transparent {
    type Up = tokio::net::TcpStream;
    type Down = tokio::net::TcpStream;

    async fn run(&self, mut up: Self::Up, mut down: Self::Down) -> io::Result<()> {
        tracing::debug!("Proxy started");

        let (mut ru, mut wu) = up.split();
        let (mut rd, mut wd) = down.split();

        let up_down = async {
            io::copy(&mut ru, &mut wd).await?;
            wd.shutdown().await
        };
        let down_up = async {
            io::copy(&mut rd, &mut wu).await?;
            wu.shutdown().await
        };

        if let Err(error) = tokio::try_join!(up_down, down_up) {
            tracing::error!(?error, "Proxy failed");
            return Err(error);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::Transparent;
    use crate::proxy::Proxy;
    use std::time::Duration;
    use tokio::net::{TcpListener, TcpStream};

    async fn tcp_pair() -> (TcpStream, TcpStream) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let connection = TcpStream::connect(listener.local_addr().unwrap());
        let accepted = listener.accept();
        let (connection, accepted) = tokio::join!(connection, accepted);

        (accepted.unwrap().0, connection.unwrap())
    }

    #[tokio::test]
    async fn stops_after_both_peers_close() {
        let (up, up_peer) = tcp_pair().await;
        let (down, down_peer) = tcp_pair().await;
        let proxy = Transparent;
        let mut task = tokio::spawn(async move { proxy.run(up, down).await });

        drop(up_peer);
        drop(down_peer);

        tokio::time::timeout(Duration::from_secs(1), &mut task)
            .await
            .expect("proxy did not stop after both peers closed")
            .expect("proxy task failed")
            .expect("proxy returned an error");
    }
}
