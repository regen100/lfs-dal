mod protocol;

use anyhow::{Context as _, Result};
use futures_util::{io::AsyncReadExt, AsyncWriteExt};
use log::debug;
use opendal::Operator;
use protocol::*;
use std::path::PathBuf;
use tokio_util::compat::{TokioAsyncReadCompatExt as _, TokioAsyncWriteCompatExt as _};

const DEFAULT_BUF_SIZE: usize = 8 * 1024 * 1024;

pub struct Agent {
    remote_op: Operator,
    sender: tokio::sync::mpsc::Sender<String>,
    tasks: tokio::task::JoinSet<()>,
    root: PathBuf,
}

impl Agent {
    pub fn new(remote_op: Operator, sender: tokio::sync::mpsc::Sender<String>) -> Self {
        Self {
            remote_op,
            sender,
            tasks: tokio::task::JoinSet::new(),
            root: PathBuf::from(""),
        }
    }

    pub async fn process(&mut self, request: &str) -> Result<()> {
        debug!("request: {}", request);
        let request: Request = serde_json::from_str(request).context("invalid request")?;
        match request {
            Request::Init => self.init().await,
            Request::Upload { oid, path } => self.upload(oid, path).await,
            Request::Download { oid } => self.download(oid).await,
            Request::Terminate => self.terminate().await,
        };
        Ok(())
    }

    async fn init(&mut self) {
        send_response(&self.sender, InitResponse::new().json()).await;
    }

    async fn upload(&mut self, oid: String, path: String) {
        let remote_op = self.remote_op.clone();
        let sender = self.sender.clone();
        self.tasks.spawn(async move {
            let status: Result<Option<String>> = async {
                let mut reader =
                    tokio::io::BufReader::new(tokio::fs::File::open(path).await?).compat();
                let mut writer = remote_op
                    .writer_with(&oid)
                    .chunk(DEFAULT_BUF_SIZE)
                    .await?
                    .into_futures_async_write();
                copy_with_progress(&sender, &oid, &mut reader, &mut writer).await?;
                writer.close().await?;
                Ok(None)
            }
            .await;
            send_response(&sender, TransferResponse::new(oid, status).json()).await;
        });
    }

    async fn download(&mut self, oid: String) {
        let remote_op = self.remote_op.clone();
        let sender = self.sender.clone();
        let path = self.root.join(lfs_object_path(&oid));
        self.tasks.spawn(async move {
            let status: Result<Option<String>> = async {
                tokio::fs::create_dir_all(path.parent().unwrap()).await?;
                let meta = remote_op.stat(&oid).await?;
                let mut reader = remote_op
                    .reader_with(&oid)
                    .chunk(DEFAULT_BUF_SIZE)
                    .await?
                    .into_futures_async_read(0..meta.content_length())
                    .await?;
                let mut writer =
                    tokio::io::BufWriter::new(tokio::fs::File::create(&path).await?).compat_write();
                copy_with_progress(&sender, &oid, &mut reader, &mut writer).await?;
                writer.close().await?;
                Ok(Some(path.to_string_lossy().into()))
            }
            .await;
            send_response(&sender, TransferResponse::new(oid, status).json()).await;
        });
    }

    async fn terminate(&mut self) {
        while self.tasks.join_next().await.is_some() {}
    }
}

async fn send_response(sender: &tokio::sync::mpsc::Sender<String>, msg: String) {
    debug!("response: {}", &msg);
    sender.send(msg).await.unwrap();
}

async fn copy_with_progress<R, W>(
    progress_sender: &tokio::sync::mpsc::Sender<String>,
    oid: &str,
    mut reader: R,
    mut writer: W,
) -> tokio::io::Result<usize>
where
    R: AsyncReadExt + Unpin,
    W: AsyncWriteExt + Unpin,
{
    let mut bytes_so_far: usize = 0;
    let mut buf = vec![0; DEFAULT_BUF_SIZE];

    loop {
        let bytes_since_last = reader.read(&mut buf).await?;
        if bytes_since_last == 0 {
            break;
        }
        writer.write_all(&buf[..bytes_since_last]).await?;
        bytes_so_far += bytes_since_last;
        send_response(
            progress_sender,
            ProgressResponse::new(oid.into(), bytes_so_far, bytes_since_last).json(),
        )
        .await;
    }

    Ok(bytes_so_far)
}

fn lfs_object_path(oid: &str) -> PathBuf {
    PathBuf::from(".git/lfs/objects")
        .join(&oid[0..2])
        .join(&oid[2..4])
        .join(oid)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as _;
    use tempfile::NamedTempFile;
    use tokio::sync::mpsc::error::TryRecvError;

    fn agent_with_buf() -> (Agent, tokio::sync::mpsc::Receiver<String>) {
        let remote_op = opendal::Operator::new(opendal::services::Memory::default())
            .unwrap()
            .finish();
        let (tx, rx) = tokio::sync::mpsc::channel(32);
        let agent = Agent::new(remote_op, tx);
        (agent, rx)
    }

    #[tokio::test]
    async fn init() {
        let (mut agent, mut output) = agent_with_buf();
        agent.process(r#"{"event":"init"}"#).await.unwrap();
        assert_eq!(&output.recv().await.unwrap(), "{}");
        assert_eq!(output.try_recv(), Err(TryRecvError::Empty));
    }

    #[tokio::test]
    async fn upload() {
        let (mut agent, mut output) = agent_with_buf();
        let mut file = NamedTempFile::new().unwrap();
        file.write_all("test".as_bytes()).unwrap();
        agent
            .process(
                &serde_json::json!({
                    "event": "upload",
                    "oid": "aabbcc",
                    "path": file.path(),
                })
                .to_string(),
            )
            .await
            .unwrap();
        assert_eq!(
            output.recv().await.unwrap(),
            r#"{"event":"progress","oid":"aabbcc","bytesSoFar":4,"bytesSinceLast":4}"#
        );
        assert_eq!(
            output.recv().await.unwrap(),
            r#"{"event":"complete","oid":"aabbcc"}"#
        );
        assert_eq!(output.try_recv(), Err(TryRecvError::Empty));
        assert_eq!(
            agent.remote_op.read("aabbcc").await.unwrap().to_bytes(),
            "test".as_bytes()
        );
    }

    #[tokio::test]
    async fn download() {
        let tempdir = tempfile::tempdir().unwrap();
        let (mut agent, mut output) = agent_with_buf();
        agent.root = tempdir.path().into();
        agent.remote_op.write("aabbcc", "test").await.unwrap();
        agent
            .process(r#"{"event":"download","oid":"aabbcc"}"#)
            .await
            .unwrap();
        assert_eq!(
            output.recv().await.unwrap(),
            r#"{"event":"progress","oid":"aabbcc","bytesSoFar":4,"bytesSinceLast":4}"#
        );
        let file_name = tempdir
            .path()
            .join(".git/lfs/objects")
            .join("aa")
            .join("bb")
            .join("aabbcc");
        assert_eq!(
            output.recv().await.unwrap(),
            serde_json::json!({
                "event": "complete",
                "oid": "aabbcc",
                "path": file_name,
            })
            .to_string()
        );
        assert_eq!(output.try_recv(), Err(TryRecvError::Empty));
        assert_eq!(
            std::fs::read_to_string(file_name).unwrap(),
            "test".to_string()
        );
    }

    #[tokio::test]
    async fn terminate() {
        let (mut agent, _) = agent_with_buf();
        agent.process(r#"{"event":"terminate"}"#).await.unwrap();
    }
}
