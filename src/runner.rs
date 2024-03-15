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
pub fn runner<J, JId, S, R, E>(
    request_sender: S,
    response_receiver: R,
    timeout: Duration,
) -> RunnerSingle<J, JId, S, R, E>
where
    J: Run,
    S: RequestSender<JobId = JId, Request = J::Request>,
    R: ResponseReceiver<JobId = JId, Response = J::Response, JobError = J::Error>,
    JId: Copy + Eq + for<'a> From<&'a J::Request>,
    E: From<<S as MpSender>::SenderError> + From<<R as McReceiver>::ReceiverError> + From<J::Error>,
{
    RunnerSingle {
        request_sender,
        response_receiver,
        timeout,
        _j: PhantomData,
        _jid: PhantomData,
        _e: PhantomData,
    }
}

/// Runs a sequence of asynchronous jobs that depend on one another and whose
/// input requests are passed to multiple-producer input channels and whose
/// responses are published to multiple-consumer output channels.
#[async_trait]
pub trait Runner {
    type J: Run;
    type S: RequestSender<Request = <Self::J as Run>::Request>;
    type R: ResponseReceiver<Response = <Self::J as Run>::Response>;
    type E: From<<Self::S as MpSender>::SenderError>
        + From<<Self::R as McReceiver>::ReceiverError>
        + From<<Self::R as ResponseReceiver>::JobError>;

    /// Runs the sequence of jobs.
    async fn run(
        &self,
        req: <Self::J as Run>::Request,
    ) -> Result<<Self::R as ResponseReceiver>::Response, RunnerError<Self::E>>;
}

/// A runner for a single job.
///
/// The runner will pass a given request to the [request_sender] and will wait
/// for a corresponding response from the [response_receiver].
pub struct RunnerSingle<J, JId, S, R, E>
where
    J: Run,
    S: RequestSender<JobId = JId, Request = J::Request>,
    R: ResponseReceiver<JobId = JId, Response = J::Response, JobError = J::Error>,
    JId: Copy + Eq + for<'a> From<&'a J::Request>,
    E: From<<S as MpSender>::SenderError> + From<<R as McReceiver>::ReceiverError> + From<J::Error>,
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

    _j: PhantomData<J>,

    _jid: PhantomData<JId>,

    _e: PhantomData<E>,
}

#[async_trait]
impl<J, JId, S, R, E> Runner for RunnerSingle<J, JId, S, R, E>
where
    J: Run + Send + Sync + 'static,
    J::Request: Send,
    J::Response: Send,
    J::Error: Send,
    S: RequestSender<JobId = JId, Request = J::Request> + Send + Sync,
    R: ResponseReceiver<JobId = JId, Response = J::Response, JobError = J::Error>
        + Send
        + Sync
        + 'static,
    JId: Copy + Eq + Send + Sync + 'static + for<'a> From<&'a J::Request>,
    E: From<<S as MpSender>::SenderError>
        + From<<R as McReceiver>::ReceiverError>
        + From<J::Error>
        + Send
        + Sync,
{
    type J = J;
    type S = S;
    type R = R;
    type E = E;

    async fn run(
        &self,
        req: <Self::J as Run>::Request,
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
