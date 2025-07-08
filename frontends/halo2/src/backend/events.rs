use crate::halo2::Expression;
use crate::ir::CircuitStmt;
use anyhow::Result;

#[derive(Clone)]
pub struct EmitStmtsMessage<F: Clone>(pub(super) Vec<CircuitStmt<Expression<F>>>);

impl<F: Clone> Message for EmitStmtsMessage<F> {
    type Response = ();
}

pub trait Message {
    type Response;
}

pub trait EventReceiver<'c> {
    type Message: Message;

    fn accept(&'c self, msg: &Self::Message) -> Result<<Self::Message as Message>::Response>;
}

pub struct EventSender<'r, R> {
    receiver: &'r R,
}

impl<'r, R> EventSender<'r, R> {
    pub fn new(receiver: &'r R) -> Self {
        Self { receiver }
    }

    pub fn send<M>(&self, msg: &M) -> Result<M::Response>
    where
        M: Message,
        R: EventReceiver<'r, Message = M>,
    {
        self.receiver.accept(msg)
    }

    pub fn send_iter<'a, M>(
        &self,
        msgs: impl Iterator<Item = &'a M>,
    ) -> impl Iterator<Item = Result<M::Response>>
    where
        M: Message + 'a,
        R: EventReceiver<'r, Message = M>,
    {
        msgs.map(|msg| self.receiver.accept(msg))
    }
}
