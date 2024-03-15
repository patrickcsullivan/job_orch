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

    /// Context containing requests and responses to all runners that have
    /// executed before this runner.
    type OldCtx: Context;

    /// Context containing requests and responses to all runners that have
    /// executed including this runner.
    type NewCtx: Context;

    /// Runs the sequence of jobs.
    async fn run(
        &self,
        req: <Self::J as Run>::Request,
    ) -> Result<<Self::R as ResponseReceiver>::Response, RunnerError<Self::E>>;

    async fn run_with_ctx(
        &self,
        req: <Self::J as Run>::Request,
    ) -> Result<Self::NewCtx, RunnerError<Self::E>>;

    /// Returns a new runner which executes this runner, converts the response
    /// from this runner into a request for the [second] runner, and then
    /// executes the [second] runner.
    fn then<Second, F>(self, second: Second, f: F) -> RunnerThen<Self, Second, F>
    where
        Self: Sized,
        Second: Runner,
        F: Fn(Self::NewCtx) -> <Second::J as Run>::Request + Send + Sync,
    {
        RunnerThen {
            first: self,
            then: second,
            f,
        }
    }
}

pub trait Context {
    type Request;
    type Response;
    type OldCtx;

    fn request(&self) -> Self::Request;
    fn response(&self) -> Self::Response;
    fn old_ctx(&self) -> Self::OldCtx;
}

pub struct EmptyCtx;

impl Context for EmptyCtx {
    type Request = ();
    type Response = ();
    type OldCtx = ();

    fn request(&self) -> () {}

    fn response(&self) -> () {}

    fn old_ctx(&self) -> () {}
}

pub struct Ctx<Request, Response, OldCtx> {
    pub request: Request,
    pub response: Response,
    pub old_ctx: OldCtx,
}

impl<Request, Response, OldCtx> Context for Ctx<Request, Response, OldCtx> {
    type Request = Request;
    type Response = Response;
    type OldCtx = OldCtx;

    fn request(&self) -> Request {
        self.request
    }

    fn response(&self) -> Response {
        self.response
    }

    fn old_ctx(&self) -> OldCtx {
        self.old_ctx
    }
}

impl<Request, Response, OldCtx> Ctx<Request, Response, OldCtx> {
    pub fn new(request: Request, response: Response, old_ctx: OldCtx) -> Self {
        Self {
            request,
            response,
            old_ctx,
        }
    }
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
    type OldCtx = EmptyCtx;
    type NewCtx = Ctx<J::Request, J::Response, Self::OldCtx>;

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

    async fn run_with_ctx(
        &self,
        req: <Self::J as Run>::Request,
    ) -> Result<Self::NewCtx, RunnerError<Self::E>> {
        let resp = self.run(req).await?;
        let new_ctx = Ctx::new(req, resp, EmptyCtx);
        Ok(new_ctx)
    }
}

/// A runner composed of a chain of runners.
///
/// This will execute the [first] runner and then map the response from the
/// [first] runner into a request for the [second] runner.
pub struct RunnerThen<First, Second, F>
where
    First: Runner,
    Second: Runner,
    F: Fn(<First as Runner>::NewCtx) -> <<Second as Runner>::J as Run>::Request + Send + Sync,
{
    pub(crate) first: First,
    pub(crate) then: Second,
    pub(crate) f: F,
}

#[async_trait]
impl<First, Second, F> Runner for RunnerThen<First, Second, F>
where
    First: Runner + Send + Sync,
    <First::J as Run>::Request: Clone + Send + Sync,
    <First::J as Run>::Response: Send,
    First::E: From<<Second::S as MpSender>::SenderError>
        + From<<Second::R as McReceiver>::ReceiverError>
        + From<<Second::R as ResponseReceiver>::JobError>,
    Second: Runner<E = First::E> + Send + Sync,
    <Second as Runner>::NewCtx: Context<Response = <<Second as Runner>::J as Run>::Response>,
    <Second::J as Run>::Request: Send,
    F: Fn(First::NewCtx) -> <Second::J as Run>::Request + Send + Sync,
{
    type J = (First::J, Second::J);
    type S = First::S;
    type R = Second::R;
    type E = First::E;
    type OldCtx = First::OldCtx;
    type NewCtx = Ctx<
        <<Second as Runner>::J as Run>::Request,
        <<Second as Runner>::J as Run>::Response,
        First::NewCtx,
    >;

    async fn run(
        &self,
        req: <Self::J as Run>::Request,
    ) -> Result<<Self::R as ResponseReceiver>::Response, RunnerError<Self::E>> {
        let resp_ctx = self.run_with_ctx(req).await?;
        Ok(resp_ctx.response())
    }

    async fn run_with_ctx(
        &self,
        req: <Self::J as Run>::Request,
    ) -> Result<Self::NewCtx, RunnerError<Self::E>> {
        let first_resp_ctx = self.first.run_with_ctx(req.clone()).await?;
        let second_req = (self.f)(first_resp_ctx);
        let second_resp_ctx = self.then.run_with_ctx(second_req).await?;
        let second_resp = second_resp_ctx.response();
        let new_ctx: Self::NewCtx = Ctx::new(second_req, second_resp, first_resp_ctx);
        Ok(new_ctx)
    }
}

pub enum RunnerError<E> {
    E(E),
    Timeout,
    Join(JoinError),
}
