use crate::halo2::Expression;
use crate::ir::CircuitStmt;
use anyhow::Result;

#[derive(Clone)]
pub struct EmitStmtsMessage<F: Clone>(pub Vec<CircuitStmt<Expression<F>>>);

impl<F: Clone> Message for EmitStmtsMessage<F> {
    type Response = ();
}

pub trait Message {
    type Response;
}

pub trait EventReceiver {
    type Message: Message;

    fn accept<'c>(&'c self, msg: &Self::Message) -> Result<<Self::Message as Message>::Response>;
}

#[derive(Copy)]
pub struct EventSender<'r, R> {
    receiver: &'r R,
}

impl<'r, R> Clone for EventSender<'r, R> {
    fn clone(&self) -> Self {
        EventSender {
            receiver: self.receiver,
        }
    }
}

impl<'r, R> EventSender<'r, R> {
    pub fn new(receiver: &'r R) -> Self {
        Self { receiver }
    }

    pub fn send<M>(&self, msg: &M) -> Result<M::Response>
    where
        M: Message,
        R: EventReceiver<Message = M>,
    {
        self.receiver.accept(msg)
    }

    pub fn send_iter<'a, M>(
        &self,
        msgs: impl Iterator<Item = &'a M>,
    ) -> impl Iterator<Item = Result<M::Response>>
    where
        M: Message + 'a,
        R: EventReceiver<Message = M>,
    {
        msgs.map(|msg| self.receiver.accept(msg))
    }
}

pub struct OwnedEventSender<R> {
    receiver: R,
}

impl<'r, R: Clone> Clone for OwnedEventSender<R> {
    fn clone(&self) -> Self {
        OwnedEventSender {
            receiver: self.receiver.clone(),
        }
    }
}

impl<R> OwnedEventSender<R> {
    pub fn new(receiver: R) -> Self {
        Self { receiver }
    }

    pub fn send<M>(&self, msg: &M) -> Result<M::Response>
    where
        M: Message,
        R: EventReceiver<Message = M>,
    {
        self.receiver.accept(msg)
    }

    pub fn send_iter<'a, M>(
        &self,
        msgs: impl Iterator<Item = &'a M>,
    ) -> impl Iterator<Item = Result<M::Response>>
    where
        M: Message + 'a,
        R: EventReceiver<Message = M>,
    {
        msgs.map(|msg| self.receiver.accept(msg))
    }
}
