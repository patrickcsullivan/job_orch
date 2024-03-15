use crate::{
    job::Run,
    processor::{McReceiver, MpSender, RequestSender, ResponseReceiver},
};
use async_trait::async_trait;
use std::{marker::PhantomData, time::Duration};
use tokio::{
    task::{self, JoinError},
    time,
};

/// Returns a job runner that will pass a request to the given [request_sender]
/// and will wait on a corresponding response from the given
/// [response_receiver].
pub fn runner<JId, S, R, E>(
    request_sender: S,
    response_receiver: R,
    timeout: Duration,
) -> RunnerSingle<JId, S, R, E>
where
    S: RequestSender<JobId = JId>,
    R: ResponseReceiver<JobId = JId>,
    JId: Copy + Eq + for<'a> From<&'a S::Request>,
    E: From<<S as MpSender>::SenderError>
        + From<<R as McReceiver>::ReceiverError>
        + From<<R as ResponseReceiver>::JobError>,
{
    RunnerSingle {
        request_sender,
        response_receiver,
        timeout,
        _jid: PhantomData,
        _e: PhantomData,
    }
}

/// Runs a sequence of asynchronous jobs that depend on one another and whose
/// input requests are passed to multiple-producer input channels and whose
/// responses are published to multiple-consumer output channels.
#[async_trait]
pub trait Runner {
    type S: RequestSender;
    type R: ResponseReceiver;
    type E: From<<Self::S as MpSender>::SenderError>
        + From<<Self::R as McReceiver>::ReceiverError>
        + From<<Self::R as ResponseReceiver>::JobError>;

    /// Runs the sequence of jobs.
    async fn run(
        &self,
        req: <Self::S as RequestSender>::Request,
    ) -> Result<<Self::R as ResponseReceiver>::Response, RunnerError<Self::E>>;
}

/// A runner for a single job.
///
/// The runner will pass a given request to the [request_sender] and will wait
/// for a corresponding response from the [response_receiver].
pub struct RunnerSingle<JId, S, R, E>
where
    S: RequestSender<JobId = JId>,
    R: ResponseReceiver<JobId = JId>,
    JId: Copy + Eq + for<'a> From<&'a S::Request>,
    E: From<<S as MpSender>::SenderError>
        + From<<R as McReceiver>::ReceiverError>
        + From<<R as ResponseReceiver>::JobError>,
{
    /// The sending side of a channel to a job processor to which requests are
    /// sent.
    request_sender: S,

    /// The receiving side of a channel from a job processor from which
    /// responses are received.
    response_receiver: R,

    /// Maximum amount of time to wait for a completed job before assuming the
    /// job has failed or been lost.
    timeout: Duration,

    _jid: PhantomData<JId>,

    _e: PhantomData<E>,
}

#[async_trait]
impl<JId, S, R, E> Runner for RunnerSingle<JId, S, R, E>
where
    S: RequestSender<JobId = JId> + Send + Sync,
    R: ResponseReceiver<JobId = JId> + Send + Sync + 'static,
    <R as ResponseReceiver>::Response: Send,
    <R as ResponseReceiver>::JobError: Send,
    JId: Copy + Eq + Send + Sync + 'static + for<'a> From<&'a <S as RequestSender>::Request>,
    E: From<<S as MpSender>::SenderError>
        + From<<R as McReceiver>::ReceiverError>
        + From<<R as ResponseReceiver>::JobError>
        + Send
        + Sync,
{
    type S = S;
    type R = R;
    type E = E;

    async fn run(
        &self,
        req: <Self::S as RequestSender>::Request,
    ) -> Result<<Self::R as ResponseReceiver>::Response, RunnerError<Self::E>> {
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

        self.request_sender
            .send_request(req_job_id, req)
            .await
            .map_err(|e| RunnerError::E(e.into()))?;

        match time::timeout(self.timeout, poller).await {
            Ok(Ok(x)) => x.map_err(|e| RunnerError::E(e.into())),
            Ok(Err(e)) => Err(RunnerError::Join(e)),
            Err(_) => Err(RunnerError::Timeout),
        }
    }
}

pub enum RunnerError<E> {
    E(E),
    Timeout,
    Join(JoinError),
}
