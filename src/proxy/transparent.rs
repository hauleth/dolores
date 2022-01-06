use tokio::io;
use tokio::io::AsyncWriteExt;

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

        loop {
            let up_down = async {
                io::copy(&mut ru, &mut wd).await?;
                wd.shutdown().await
            };
            let down_up = async {
                io::copy(&mut rd, &mut wu).await?;
                wu.shutdown().await
            };

            // XXX: Check how to handle this result and whether it makes sense to report anything
            // meaningful there
            let _ = tokio::try_join!(up_down, down_up);
        }
    }
}
