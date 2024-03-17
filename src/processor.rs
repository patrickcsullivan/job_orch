use crate::{McReceiver, MpSender, Run};
use async_trait::async_trait;

/// A processor that receives asynchronous job requests, executes the jobs, and
/// then emits the job responses.  
#[async_trait]
pub trait Processor<J, JId>
where
    J: Run,
    JId: Clone + Eq,
{
    /// The type of the sender end of the channel for job requests.
    type RequestSender: MpSender<Message = (JId, J::Request)>;

    /// The type of the receiver end of the channel for job requests.
    type RequestReceiver: McReceiver<Message = (JId, J::Request)>;

    /// The type of the sender end of the channel for job responses.
    type ResponseSender: MpSender<Message = (JId, Result<J::Response, J::Error>)>;

    /// The type of the receiver end of the channel for job responses.
    type ResponseReceiver: McReceiver<Message = (JId, Result<J::Response, J::Error>)>;

    /// The type of error that can occur when processing asynchronous job
    /// requests.
    type Error;

    /// Returns a sender for sending job requests to the processor.
    fn request_sender(&self) -> Self::RequestSender;

    /// Returns a receiver for receieving job responses from the processor.
    fn response_receiver(&self) -> Self::ResponseReceiver;

    /// Processes job requests sent to the processor and emit job responses.
    async fn run(&self, rsrcs: &mut J::Resources) -> Result<(), Self::Error>;
}
