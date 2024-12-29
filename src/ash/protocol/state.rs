use super::handles::AshStreamTaskHandles;
use crate::ash::{
    constants::{ASH_VERSION_2, RESET_POWERON},
    frame::Frame,
    Error, FrameNumber,
};
use anyhow::{bail, Result};
use bytes::BytesMut;
use tokio::select;
use tracing::{debug, warn};

pub enum State {
    Failed(FailedState),
    Connected(ConnectedState),
}

impl State {
    pub(crate) fn initial() -> State {
        State::Failed(FailedState::default())
    }

    pub(crate) async fn process(&mut self, handles: &mut AshStreamTaskHandles) -> Result<()> {
        let res = match self {
            State::Failed(state) => state.process(handles).await?,
            State::Connected(state) => state.process(handles).await?,
        };
        if let Some(next_state) = res {
            *self = next_state;
        }
        Ok(())
    }
}

pub struct FailedState {
    pub reason: u8,
}

impl FailedState {
    async fn process(&mut self, handles: &mut AshStreamTaskHandles) -> Result<Option<State>> {
        // Wait for a RST frame, replying to all other frames with an ERROR
        let frame = handles.receive_frame().await?;

        if !matches!(frame, Ok(Frame::Rst)) {
            handles
                .send_frame(Frame::error(ASH_VERSION_2, self.reason))
                .await?;
            return Ok(None);
        }

        // Send a reset request to the NCP and wait for a response
        let code = handles.reset_ncp().await?;
        handles
            .send_frame(Frame::rst_ack(ASH_VERSION_2, code))
            .await?;

        // Before we transition to the Connected state, peek at the next frame
        // and discard any other RST frames.
        handles.discard_extra_rst_frames().await?;

        // Transition to connected
        Ok(Some(State::Connected(ConnectedState::default())))
    }
}

impl Default for FailedState {
    fn default() -> Self {
        Self {
            reason: RESET_POWERON,
        }
    }
}

#[derive(Default)]
pub struct ConnectedState {
    reject: bool,
    inflight_frame_number: FrameNumber,
    acked_frame_number: FrameNumber,
}

impl ConnectedState {
    async fn process(&mut self, handles: &mut AshStreamTaskHandles) -> Result<Option<State>> {
        select! {
            Ok(res) = handles.receive_frame() => {
                self.handle_frame(res, handles).await?;
            }
        }
        Ok(None)
    }

    async fn handle_frame(
        &mut self,
        frame: Result<Frame, Error>,
        handles: &mut AshStreamTaskHandles,
    ) -> Result<()> {
        match frame {
            Ok(Frame::Data {
                frm_num,
                re_tx,
                ack_num,
                body,
            }) => {
                self.process_data_frame(frm_num, re_tx, ack_num, body, handles)
                    .await?
            }
            Err(
                Error::InvalidChecksum(Frame::Data { frm_num, .. })
                | Error::InvalidDataField(Frame::Data { frm_num, .. }),
            ) => {
                self.set_reject_condition_and_send_nak(frm_num, handles)
                    .await?
            }
            Err(e) => warn!("Received an invalid frame: {}", e),
            _ => bail!("Frame type not yet implemented"),
        };
        Ok(())
    }

    async fn process_data_frame(
        &mut self,
        frm_num: FrameNumber,
        re_tx: bool,
        ack_num: FrameNumber,
        body: BytesMut,
        handles: &mut AshStreamTaskHandles,
    ) -> Result<()> {
        // Check frame number is in sequence
        if frm_num != self.inflight_frame_number + 1 {
            debug!(
                frm_num = *frm_num,
                re_tx,
                ack_num = *ack_num,
                "Rejected DATA frame with out-of-sequence frame number {}",
                frm_num
            );
            self.set_reject_condition_and_send_nak(frm_num, handles)
                .await?;
            return Ok(());
        }
        // Check that the host hasn't exceeded the in-flight limit for ACKs
        if self
            .inflight_frame_number
            .abs_diff(*self.acked_frame_number)
            > 7
        {
            debug!(
                frm_num = *frm_num,
                re_tx,
                ack_num = *ack_num,
                "Rejected DATA frame {} as the in-flight window is full",
                frm_num
            );
            self.set_reject_condition_and_send_nak(frm_num, handles)
                .await?;
            return Ok(());
        }
        self.inflight_frame_number += 1;

        // Send frame data to outbox
        handles.send_data(body)?;
        
        // Add ACK to
        Ok(())
    }

    async fn set_reject_condition_and_send_nak(
        &mut self,
        frm_num: FrameNumber,
        handles: &mut AshStreamTaskHandles,
    ) -> Result<()> {
        if !self.reject {
            self.reject = true;
            handles.send_frame(Frame::nak(false, frm_num)).await?;
        }
        Ok(())
    }

    fn clear_reject_condition(&mut self) {
        self.reject = false;
    }
}
