use crate::processor::{McReceiver, MpSender, RequestSender, ResponseReceiver};
use std::{marker::PhantomData, time::Duration};
use tokio::{
    task::{self, JoinError},
    time,
};

/// Returns a job runner that will pass a request to the given [request_sender]
/// and will wait on a corresponding response from the given
/// [response_receiver].
pub fn runner<S, R, JId, E>(
    request_sender: S,
    response_receiver: R,
    timeout: Duration,
) -> RunnerSingle<S, R, JId, E>
where
    S: RequestSender<JobId = JId>,
    R: ResponseReceiver<JobId = JId>,
    JId: Copy + Eq + for<'a> From<&'a <S as RequestSender>::Request>,
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
/// responses are published to multiple-consumer output channels.ß
pub trait Runner {
    type S: RequestSender;
    type R: ResponseReceiver;
    type E: From<<Self::S as MpSender>::SenderError>
        + From<<Self::R as McReceiver>::ReceiverError>
        + From<<Self::R as ResponseReceiver>::JobError>;

    /// Runs the sequence of jobs.
    #[allow(async_fn_in_trait)]
    async fn run(
        &self,
        req: <Self::S as RequestSender>::Request,
    ) -> Result<<Self::R as ResponseReceiver>::Response, RunnerError<Self::E>>;

    /// Returns a new runner which executes this runner, converts the response
    /// from this runner into a request for the [second] runner, and then
    /// executes the [second] runner.
    fn then<Second, F>(self, f: F, second: Second) -> RunnerThen<Self, Second, F>
    where
        Self: Sized,
        Second: Runner,
        F: Fn(
            <<Self as Runner>::S as RequestSender>::Request,
            <<Self as Runner>::R as ResponseReceiver>::Response,
        ) -> <<Second as Runner>::S as RequestSender>::Request,
    {
        RunnerThen {
            first: self,
            then: second,
            f,
        }
    }
}

/// A runner for a single job.
///
/// The runner will pass a given request to the [request_sender] and will wait
/// for a corresponding response from the [response_receiver].
pub struct RunnerSingle<S, R, JId, E>
where
    S: RequestSender<JobId = JId>,
    R: ResponseReceiver<JobId = JId>,
    JId: Copy + Eq + for<'a> From<&'a <S as RequestSender>::Request>,
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

impl<S, R, JId, E> Runner for RunnerSingle<S, R, JId, E>
where
    S: RequestSender<JobId = JId>,
    R: ResponseReceiver<JobId = JId> + Send + Sync + 'static,
    R::Response: Send + Sync + 'static,
    R::JobError: Send + Sync + 'static,
    JId: Copy + Eq + Send + for<'a> From<&'a <S as RequestSender>::Request> + 'static,
    E: From<<S as MpSender>::SenderError>
        + From<<R as McReceiver>::ReceiverError>
        + From<<R as ResponseReceiver>::JobError>,
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
            .map_err(|e| RunnerError::E(e.into()))?;

        match time::timeout(self.timeout, poller).await {
            Ok(Ok(x)) => x.map_err(|e| RunnerError::E(e.into())),
            Ok(Err(e)) => Err(RunnerError::Join(e)),
            Err(_) => Err(RunnerError::Timeout),
        }
    }
}

/// A runner composed of a chain of runners.
///
/// This will execute the [first] runner and then map the response from the
/// [first] runner into a request for the [second] runner.
pub struct RunnerThen<First, Then, F>
where
    First: Runner,
    Then: Runner,
    F: Fn(
        <<First as Runner>::S as RequestSender>::Request,
        <<First as Runner>::R as ResponseReceiver>::Response,
    ) -> <<Then as Runner>::S as RequestSender>::Request,
{
    first: First,
    then: Then,
    f: F,
}

impl<First, Then, F> Runner for RunnerThen<First, Then, F>
where
    First: Runner,
    <First::S as RequestSender>::Request: Clone,
    First::E: From<<Then::S as MpSender>::SenderError>
        + From<<Then::R as McReceiver>::ReceiverError>
        + From<<Then::R as ResponseReceiver>::JobError>,
    Then: Runner<E = First::E>,
    F: Fn(
        <<First as Runner>::S as RequestSender>::Request,
        <<First as Runner>::R as ResponseReceiver>::Response,
    ) -> <<Then as Runner>::S as RequestSender>::Request,
{
    type S = First::S;
    type R = Then::R;
    type E = First::E;

    async fn run(
        &self,
        req: <Self::S as RequestSender>::Request,
    ) -> Result<<Self::R as ResponseReceiver>::Response, RunnerError<Self::E>> {
        let first_resp = self.first.run(req.clone()).await?;
        let second_req = (self.f)(req, first_resp);
        let second_resp = self.then.run(second_req).await?;
        Ok(second_resp)
    }
}

pub enum RunnerError<E> {
    E(E),
    Timeout,
    Join(JoinError),
}
