use crate::{
    job::Run,
    processor::{McReceiver, MpSender, RequestSender, ResponseReceiver},
    runner::{Runner, RunnerSingle, RunnerThen},
};
use async_trait::async_trait;
use either::Either;

/// A trait for job whose cached response can be retrieved from [Rsrcs].
#[async_trait]
pub trait GetCache {
    type Request;
    type Response;
    type Resources;
    type Error;

    /// Try to get a cached version of the response.
    async fn get_cached(
        &self,
        req: &Self::Request,
        rsrcs: &mut Self::Resources,
    ) -> Result<Option<Self::Response>, Self::Error>;
}

#[async_trait]
impl<J, JId, S, R, E> GetCache for RunnerSingle<J, JId, S, R, E>
where
    J: Run
        + Default
        + GetCache<
            Request = <J as Run>::Request,
            Response = <J as Run>::Response,
            Resources = <J as Run>::Resources,
            Error = <J as Run>::Error,
        > + Sync,
    <J as Run>::Request: Send + Sync,
    <J as Run>::Resources: Send,
    S: RequestSender<JobId = JId, Request = <J as Run>::Request> + Sync,
    R: ResponseReceiver<JobId = JId, Response = <J as Run>::Response, JobError = <J as Run>::Error>
        + Sync,
    JId: Copy + Eq + Sync + for<'a> From<&'a <J as Run>::Request>,
    E: From<<S as MpSender>::SenderError>
        + From<<R as McReceiver>::ReceiverError>
        + From<<J as Run>::Error>
        + Sync,
{
    type Request = <J as Run>::Request;

    type Response = <J as Run>::Response;

    type Resources = <J as Run>::Resources;

    type Error = <J as Run>::Error;

    async fn get_cached(
        &self,
        req: &Self::Request,
        rsrcs: &mut Self::Resources,
    ) -> Result<Option<Self::Response>, Self::Error> {
        J::default().get_cached(req, rsrcs).await
    }
}

#[async_trait]
impl<First, Second, F> GetCache for RunnerThen<First, Second, F>
where
    First: Runner
        + GetCache<
            Request = <<First as Runner>::J as Run>::Request,
            Response = <<First as Runner>::J as Run>::Response,
            Resources = <<First as Runner>::J as Run>::Resources,
            // Error = <<First as Runner>::J as Run>::Error,
        > + Sync,
    <<First as Runner>::J as Run>::Request: Sync,
    <<First as Runner>::J as Run>::Resources: Send,
    <First::S as RequestSender>::Request: Clone + Send,
    First::E: From<<Second::S as MpSender>::SenderError>
        + From<<Second::R as McReceiver>::ReceiverError>
        + From<<Second::R as ResponseReceiver>::JobError>,
    Second: Runner<E = First::E>
        + GetCache<
            Request = <<Second as Runner>::J as Run>::Request,
            Response = <<Second as Runner>::J as Run>::Response,
            Resources = <<Second as Runner>::J as Run>::Resources,
            // Error = <<Second as Runner>::J as Run>::Error,
        > + Sync,
    <<Second as Runner>::J as Run>::Request: Sync,
    <<Second as Runner>::J as Run>::Resources: Send,
    F: Fn(
            <<First as Runner>::S as RequestSender>::Request,
            <<First as Runner>::R as ResponseReceiver>::Response,
        ) -> <<Second as Runner>::S as RequestSender>::Request
        + Sync,
{
    type Request = <<First as Runner>::J as Run>::Request;

    type Response = <<Second as Runner>::J as Run>::Response;

    type Resources = (
        <<First as Runner>::J as Run>::Resources,
        <<Second as Runner>::J as Run>::Resources,
    );

    type Error = Either<<First as GetCache>::Error, <Second as GetCache>::Error>;

    async fn get_cached(
        &self,
        req: &Self::Request,
        rsrcs: &mut Self::Resources,
    ) -> Result<Option<Self::Response>, Self::Error> {
        let (rsrcs1, rsrcs2) = rsrcs;
        if let Some(rsp1) = First::get_cached(req, rsrcs1).await.map_err(either::Left)? {
            todo!()
        }
        todo!()
    }
}
