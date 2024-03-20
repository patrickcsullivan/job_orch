use async_trait::async_trait;

/// The sending side of a multiple-producer channel.
#[async_trait]
pub trait MpSender: Clone {
    /// The type of messages sent along the channel.
    type Message;

    /// The type of error that can occur when sending a message along the
    /// channel.
    type Error;

    /// Attempts to send a message on the channel.
    async fn send(&self, msg: Self::Message) -> Result<(), Self::Error>;
}

/// The receiving side of a multiple-consumer channel.
#[async_trait]
pub trait McReceiver: Clone {
    /// The type of messages sent along the channel.
    type Message;

    /// The type of error that can occur when receiving a message from the
    /// channel.
    type Error;

    /// Attempts to wait for a message on the channel.
    async fn receive(&self) -> Result<Self::Message, Self::Error>;
}
