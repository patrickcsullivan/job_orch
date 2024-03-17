use async_trait::async_trait;
use std::time::Duration;
use tokio::{task, time};

use crate::{McReceiver, MpSender, Run};

/// A job to whom resquests can be sent over a multiple producer channel and
/// from whom responses can be received over a multiple consumer channel.
#[async_trait]
pub trait SendReceive<JId, ReqTx, RespRx>: Run
where
    JId: Copy + Eq + Send + Sync + 'static,
    <Self as Run>::Error: Clone + Send + 'static,
    <Self as Run>::Request: Send + 'static,
    <Self as Run>::Response: Send + 'static,
    ReqTx: MpSender<Message = (JId, <Self as Run>::Request)> + Send + Sync + 'static,
    RespRx: McReceiver<Message = (JId, Result<<Self as Run>::Response, <Self as Run>::Error>)>
        + Send
        + Sync
        + 'static,
{
    /// Passes the request along with the job ID to the given [request_sender]
    /// and waits on a corresponding response from the given
    /// [response_receiver].
    async fn send_receive(
        request_sender: ReqTx,
        response_receiver: RespRx,
        jid: JId,
        req: <Self as Run>::Request,
        timeout: Duration,
    ) -> Result<<Self as Run>::Response, SendReceiveError<<Self as Run>::Error>> {
        // Start listenting for responses before the request is actually sent so
        // a response can't get sent before we start listening.
        let poller = task::spawn(async move {
            loop {
                match response_receiver.receive().await {
                    Ok((resp_job_id, resp_rslt)) if jid == resp_job_id => return resp_rslt,
                    _ => {}
                }
            }
        });

        request_sender
            .send((jid, req))
            .await
            .map_err(|_| SendReceiveError::RequestSender)?;
        let resp = time::timeout(timeout, poller)
            .await
            .map_err(|_| SendReceiveError::Elapsed)?
            .map_err(|_| SendReceiveError::Join)?
            .map_err(SendReceiveError::Job)?;
        Ok(resp)
    }
}

#[derive(thiserror::Error, Debug, Clone)]
pub enum SendReceiveError<JErr>
where
    JErr: Clone,
{
    #[error("")]
    RequestSender,
    #[error("")]
    ResponseReceiver,
    #[error("")]
    Job(JErr),
    #[error("")]
    Join,
    #[error("")]
    Elapsed,
}
