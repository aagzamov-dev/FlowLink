use crate::{
    input::Input,
    model::{Display, InputSettings, Placement, PROTOCOL_VERSION},
    Result,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
pub const MAX_FRAME: usize = 64 * 1024;
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Hello {
    pub version: u16,
    pub name: String,
    pub certificate: String,
    pub pair: bool,
}
impl Hello {
    pub fn validate(&self) -> Result<()> {
        if self.version != PROTOCOL_VERSION {
            return Err("Incompatible FlowLink protocol version".into());
        }
        if self.name.is_empty() || self.name.len() > 80 || self.certificate.len() > 16000 {
            return Err("Invalid peer identity".into());
        }
        Ok(())
    }
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", deny_unknown_fields)]
pub enum PairMessage {
    Known {
        certificate: String,
        name: String,
    },
    Ready {
        public: String,
        certificate: String,
        name: String,
    },
    Proof {
        public: String,
        proof: String,
    },
    Confirm {
        proof: String,
    },
    Rejected {
        reason: String,
    },
}
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ControlToken {
    pub revision: u64,
    pub handoff: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", deny_unknown_fields)]
pub enum Message {
    Welcome {
        device_id: String,
        name: String,
        displays: Vec<Display>,
        os: String,
    },
    ControlRequest {
        controller_id: String,
        from_either: bool,
    },
    ControlState {
        controller_id: String,
        from_either: bool,
        revision: u64,
    },
    Topology {
        displays: Vec<Display>,
    },
    Layout {
        revision: u64,
        placements: Vec<Placement>,
        host_displays: Vec<Display>,
    },
    Configuration {
        revision: u64,
        settings: InputSettings,
    },
    Handoff {
        token: ControlToken,
        display_id: String,
        x: f64,
        y: f64,
    },
    HandoffAck {
        token: ControlToken,
    },
    Return {
        token: ControlToken,
        x: f64,
        y: f64,
    },
    ReturnAck {
        token: ControlToken,
    },
    Input {
        token: ControlToken,
        sequence: u64,
        event: Input,
    },
    Heartbeat {
        nonce: u64,
    },
    Pong {
        nonce: u64,
    },
    Release {
        token: ControlToken,
    },
    Pause,
    Resume,
    Goodbye,
    Identify,
}
pub async fn read<T: DeserializeOwned>(stream: &mut (impl AsyncRead + Unpin)) -> Result<T> {
    read_buffered(stream, &mut Vec::new()).await
}
pub async fn read_buffered<T: DeserializeOwned>(
    stream: &mut (impl AsyncRead + Unpin),
    buffer: &mut Vec<u8>,
) -> Result<T> {
    let len = stream.read_u32().await? as usize;
    if len == 0 || len > MAX_FRAME {
        return Err("Invalid protocol frame length".into());
    }
    buffer.resize(len, 0);
    stream.read_exact(buffer).await?;
    Ok(serde_json::from_slice(buffer)?)
}
pub async fn write<T: Serialize>(
    stream: &mut (impl AsyncWrite + Unpin),
    message: &T,
) -> Result<()> {
    write_buffered(stream, message, &mut Vec::new()).await
}
pub async fn write_buffered<T: Serialize>(
    stream: &mut (impl AsyncWrite + Unpin),
    message: &T,
    buffer: &mut Vec<u8>,
) -> Result<()> {
    buffer.clear();
    buffer.extend_from_slice(&[0; 4]);
    serde_json::to_writer(&mut *buffer, message)?;
    let len = buffer.len() - 4;
    if len == 0 || len > MAX_FRAME {
        return Err("Protocol message exceeds size limit".into());
    }
    buffer[..4].copy_from_slice(&(len as u32).to_be_bytes());
    stream.write_all(buffer).await?;
    stream.flush().await?;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn oversized_frames_rejected_before_allocation() {
        let (mut a, mut b) = tokio::io::duplex(16);
        a.write_u32((MAX_FRAME + 1) as u32).await.unwrap();
        assert!(read::<Message>(&mut b).await.is_err());
    }
    #[tokio::test]
    async fn frames_roundtrip() {
        let (mut a, mut b) = tokio::io::duplex(256);
        write(&mut a, &Message::Heartbeat { nonce: 5 })
            .await
            .unwrap();
        assert!(matches!(
            read::<Message>(&mut b).await.unwrap(),
            Message::Heartbeat { nonce: 5 }
        ));
    }
    #[tokio::test]
    async fn reused_buffers_handle_different_frame_sizes_without_stale_bytes() {
        let (mut a, mut b) = tokio::io::duplex(4096);
        let mut write_buffer = Vec::new();
        let mut read_buffer = Vec::new();
        for nonce in [u64::MAX, 1, 1000000, 0] {
            write_buffered(&mut a, &Message::Heartbeat { nonce }, &mut write_buffer)
                .await
                .unwrap();
            let message = read_buffered::<Message>(&mut b, &mut read_buffer)
                .await
                .unwrap();
            assert!(matches!(message, Message::Heartbeat { nonce: actual } if actual == nonce));
        }
        assert!(write_buffer.capacity() >= write_buffer.len());
        a.write_u32((MAX_FRAME + 1) as u32).await.unwrap();
        let capacity = read_buffer.capacity();
        assert!(read_buffered::<Message>(&mut b, &mut read_buffer)
            .await
            .is_err());
        assert_eq!(read_buffer.capacity(), capacity);
    }
    #[tokio::test]
    async fn oversized_writes_never_send_partial_frames() {
        let (mut a, mut b) = tokio::io::duplex(4096);
        let mut buffer = Vec::new();
        assert!(write_buffered(&mut a, &"x".repeat(MAX_FRAME), &mut buffer)
            .await
            .is_err());
        write_buffered(&mut a, &Message::Pause, &mut buffer)
            .await
            .unwrap();
        assert!(matches!(
            read::<Message>(&mut b).await.unwrap(),
            Message::Pause
        ));
    }
}
