use crate::tree::Node;
use async_trait::async_trait;

#[async_trait]
pub trait Run {
    type Request;
    type Response;
    type Error;

    async fn run(&self, req: &Self::Request) -> Result<Self::Response, Self::Error>;
}

#[async_trait]
pub trait Cache {
    type Request;
    type Response;
    type Error;

    async fn get(&mut self, req: &Self::Request) -> Result<Option<Self::Response>, Self::Error>;

    async fn set(&mut self, resp: &Self::Response) -> Result<(), Self::Error>;
}

#[async_trait]
impl<T> Node for T
where
    T: Run
        + Cache<
            Request = <T as Run>::Request,
            Response = <T as Run>::Response,
            Error = <T as Run>::Error,
        > + Send
        + Sync,
    <T as Run>::Request: Send + Sync,
    <T as Run>::Response: Send + Sync,
    <T as Run>::Error: Send + Sync,
{
    type Dependency = <T as Run>::Request;
    type Value = <T as Run>::Response;
    type Error = <T as Run>::Error;

    async fn drive(&mut self, deps: &Self::Dependency) -> Result<Self::Value, Self::Error> {
        if let Some(resp) = self.get(deps).await? {
            return Ok(resp);
        }

        let resp = self.run(deps).await?;
        self.set(&resp).await?;
        Ok(resp)
    }
}
