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
    let len = stream.read_u32().await? as usize;
    if len == 0 || len > MAX_FRAME {
        return Err("Invalid protocol frame length".into());
    }
    let mut buf = vec![0; len];
    stream.read_exact(&mut buf).await?;
    Ok(serde_json::from_slice(&buf)?)
}
pub async fn write<T: Serialize>(
    stream: &mut (impl AsyncWrite + Unpin),
    message: &T,
) -> Result<()> {
    let data = serde_json::to_vec(message)?;
    if data.is_empty() || data.len() > MAX_FRAME {
        return Err("Protocol message exceeds size limit".into());
    }
    let mut frame = Vec::with_capacity(4 + data.len());
    frame.extend_from_slice(&(data.len() as u32).to_be_bytes());
    frame.extend_from_slice(&data);
    stream.write_all(&frame).await?;
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
}
