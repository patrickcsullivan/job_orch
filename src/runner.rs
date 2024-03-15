use crate::processor::{McReceiver, MpSender};
use std::{marker::PhantomData, time::Duration};
use tokio::{
    task::{self, JoinError},
    time::{self, error::Elapsed},
};

/// Returns a job runner that will pass a request to the given [request_sender]
/// and will wait on a corresponding response from the given
/// [response_receiver].
pub fn runner<JId, JErr, Req, Resp, ReqTx, RespRx, E>(
    request_sender: ReqTx,
    response_receiver: RespRx,
    timeout: Duration,
) -> Runner<JId, JErr, Req, Resp, ReqTx, RespRx, E>
where
    JId: Copy + Eq + for<'a> From<&'a Req>,
    ReqTx: MpSender<Message = (JId, Req)>,
    RespRx: McReceiver<Message = (JId, Result<Resp, JErr>)>,
    E: From<JErr> + From<ReqTx::SenderError> + From<RespRx::ReceiverError>,
{
    Runner {
        request_sender,
        response_receiver,
        timeout,
        _jid: PhantomData,
        _je: PhantomData,
    }
}

/// A runner for a single job.
///
/// The runner will pass a given request to the [request_sender] and will wait
/// for a corresponding response from the [response_receiver].
pub struct Runner<JId, JErr, Req, Resp, ReqTx, RespRx, E>
where
    JId: Copy + Eq + for<'a> From<&'a Req>,
    ReqTx: MpSender<Message = (JId, Req)>,
    RespRx: McReceiver<Message = (JId, Result<Resp, JErr>)>,
    E: From<JErr> + From<ReqTx::SenderError> + From<RespRx::ReceiverError>,
{
    /// The sending side of a channel to a job processor to which requests are
    /// sent.
    request_sender: ReqTx,

    /// The receiving side of a channel from a job processor from which
    /// responses are received.
    response_receiver: RespRx,

    /// Maximum amount of time to wait for a completed job before assuming the
    /// job has failed or been lost.
    timeout: Duration,

    _jid: PhantomData<JId>,

    _je: PhantomData<E>,
}

impl<JId, JErr, Req, Resp, ReqTx, RespRx, E> Runner<JId, JErr, Req, Resp, ReqTx, RespRx, E>
where
    JId: Copy + Eq + Send + for<'a> From<&'a Req> + 'static,
    JErr: Send + 'static,
    Resp: Send + 'static,
    ReqTx: MpSender<Message = (JId, Req)> + 'static,
    RespRx: McReceiver<Message = (JId, Result<Resp, JErr>)> + Send + 'static,
    E: From<JErr>
        + From<ReqTx::SenderError>
        + From<RespRx::ReceiverError>
        + From<JoinError>
        + From<Elapsed>
        + Send,
{
    async fn run(&self, req: Req) -> Result<Resp, E> {
        let req_job_id: JId = (&req).into();
        let receiver = self.response_receiver.clone();

        // Start listenting for responses before the request is actually sent so
        // a response can't get sent before we start listening.
        let poller = task::spawn(async move {
            loop {
                match receiver.receive().await {
                    Ok((resp_job_id, resp_rslt)) if req_job_id == resp_job_id => return resp_rslt,
                    _ => {}
                }
            }
        });

        self.request_sender.send((req_job_id, req)).await?;
        let resp = time::timeout(self.timeout, poller).await???;
        Ok(resp)
    }
}
