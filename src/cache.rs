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

// todo: GetCacheWithContext

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
        > + Send
        + Sync,
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

// #[async_trait]
// impl<First, Second, F> GetCache for RunnerThen<First, Second, F>
// where
//     First: Runner
//         + GetCache<
//             Request = <<First as Runner>::S as RequestSender>::Request,
//             Response = <<First as Runner>::R as ResponseReceiver>::Response,
//             Resources = <<First as Runner>::J as Run>::Resources,
//         > + Send
//         + Sync,
//     <<First as Runner>::S as RequestSender>::Request: Sync,
//     <<First as Runner>::R as ResponseReceiver>::Response: Send,
//     <<First as Runner>::J as Run>::Resources: Send,
//     <First as GetCache>::Error: Send,
//     <First::S as RequestSender>::Request: Clone + Send,
//     First::E: From<<Second::S as MpSender>::SenderError>
//         + From<<Second::R as McReceiver>::ReceiverError>
//         + From<<Second::R as ResponseReceiver>::JobError>,
//     Second: Runner<E = First::E>
//         + GetCache<
//             Request = <<Second as Runner>::S as RequestSender>::Request,
//             Response = <<Second as Runner>::R as ResponseReceiver>::Response,
//             Resources = <<Second as Runner>::J as Run>::Resources,
//         > + Send
//         + Sync,
//     <<Second as Runner>::S as RequestSender>::Request: Send + Sync,
//     <<Second as Runner>::R as ResponseReceiver>::Response: Send,
//     <<Second as Runner>::J as Run>::Resources: Send,
//     <Second as GetCache>::Error: Send,
//     F: Fn(First::NewCtx) -> <Second::J as Run>::Request + Send + Sync,
// {
//     type Request = <<First as Runner>::S as RequestSender>::Request;

//     type Response = <<Second as Runner>::R as ResponseReceiver>::Response;

//     type Resources = (
//         <<First as Runner>::J as Run>::Resources,
//         <<Second as Runner>::J as Run>::Resources,
//     );

//     type Error = Either<<First as GetCache>::Error, <Second as GetCache>::Error>;

//     async fn get_cached(
//         &self,
//         req: &Self::Request,
//         rsrcs: &mut Self::Resources,
//     ) -> Result<Option<Self::Response>, Self::Error> {
//         let (rsrcs1, rsrcs2) = rsrcs;
//         let rslt1 = self
//             .first
//             .get_cached(req, rsrcs1)
//             .await
//             .map_err(either::Left)?;

//         if let Some(rsp1) = rslt1 {
//             let req2 = (self.f)(req.clone(), rsp1);
//             let cache2 = self
//                 .then
//                 .get_cached(&req2, rsrcs2)
//                 .await
//                 .map_err(either::Right)?;
//             return Ok(cache2);
//         }
//         Ok(None)
//     }
// }
