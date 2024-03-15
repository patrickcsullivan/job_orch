use crate::job::Run;
use async_trait::async_trait;

/// The sending side of a multiple-producer channel.
#[async_trait]
pub trait MpSender: Clone {
    type Message;
    type SenderError;

    async fn send(&self, msg: Self::Message) -> Result<(), Self::SenderError>;
}

/// The receiving side of a multiple-consumer channel.
#[async_trait]
pub trait McReceiver: Clone {
    type Message;
    type ReceiverError;

    async fn receive(&self) -> Result<Self::Message, Self::ReceiverError>;
}

// /// The sending side of a multiple-producer channel for a job processor.
// #[async_trait]
// pub trait RequestSender: MpSender<Message = (Self::JobId, Self::Request)> {
//     type JobId: Send;
//     type Request: Send;

//     /// Sends a request to perform a job to the processor.
//     async fn send_request(
//         &self,
//         job_id: Self::JobId,
//         request: Self::Request,
//     ) -> Result<(), Self::SenderError> {
//         self.send((job_id, request)).await
//     }
// }

// /// The receiving side of a multiple-consumer channel for a job processor.
// #[async_trait]
// pub trait ResponseReceiver:
//     McReceiver<Message = (Self::JobId, Result<Self::Response, Self::JobError>)>
// {
//     type JobId;
//     type Response;
//     type JobError;

//     /// Wait to receive a job response from the processor.
//     async fn receive_response(
//         &self,
//     ) -> Result<(Self::JobId, Result<Self::Response, Self::JobError>), Self::ReceiverError> {
//         self.receive().await
//     }
// }

/// A processor that receives asynchronous job requests, executes the jobs, and
/// then emits the job responses.  
#[async_trait]
pub trait Processor<J, JId>
where
    J: Run,
    JId: Clone + Eq,
{
    type RequestSender: MpSender<Message = (JId, J::Request)>;
    type RequestReceiver: McReceiver<Message = (JId, J::Request)>;
    type ResponseSender: MpSender<Message = (JId, Result<J::Response, J::Error>)>;
    type ResponseReceiver: McReceiver<Message = (JId, Result<J::Response, J::Error>)>;
    type Error;

    /// Returns a sender for sending job requests to the processor.
    fn request_sender(&self) -> Self::RequestSender;

    /// Returns a receiver for receieving job responses from the processor.
    fn response_receiver(&self) -> Self::ResponseReceiver;

    /// Processes job requests sent to the processor and emit job responses.
    async fn run(&self, rsrcs: &mut J::Resources) -> Result<(), Self::Error>;
}
