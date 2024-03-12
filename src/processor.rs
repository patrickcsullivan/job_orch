use async_trait::async_trait;

use crate::task::Run;
use either::Either;

/// The sending side of a channel for a job processor.
#[async_trait]
pub trait RequestSender<R>: Clone
where
    R: Run,
{
    /// An error that may occor when sending a job request to the processor.
    type Error;

    /// Sends a job and any preprocessed work to the processor.
    fn send(&self, request: R::Request) -> Result<(), Self::Error>;
}

#[async_trait]
pub trait ResponseReceiver<R>: Clone
where
    R: Run,
{
    type Error;

    async fn receive(&self) -> Result<R::Response, Either<R::Error, Self::Error>>;
}

#[async_trait]
pub trait Processor<R>
where
    R: Run,
{
    type Error;
    type Sender: RequestSender<R>;
    type Receiver: ResponseReceiver<R>;

    /// Returns a sender for sending requests to the processor.
    fn request_sender(&self) -> Self::Sender;

    /// Returns a receiver for receieving responses from the processor.
    fn response_receiver(&self) -> Self::Receiver;

    /// Process requests sent to the processor and emit responses.
    async fn run(&self) -> Result<(), Self::Error>;
}

pub struct JobAndProcessor<J, P>
where
    J: Run,
    P: Processor<J>,
{
    job: J,
    processor: P,
}

// impl<J> Sender<J> for async_channel::Sender<(J::Preprocessed, J)>
// where
//     J: Job,
// {
//     type Error = ();

//     fn send(&self, pre: J::Preprocessed, job: J) -> Result<(), Self::Error> {
//         self.send((pre, job));
//         Ok(())
//     }
// }
