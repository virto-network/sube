use super::{Message, WalletApi, WalletCommand, WalletError, WalletEvent, WalletResult};
use crate::StateMachine;
use crate::utils::prelude::*;

#[derive(Deserialize, Serialize, Debug)]
pub struct Wallet {
    device_id: Option<String>,
    pending_messages: Vec<Message>,
}

#[async_trait]
impl StateMachine for Wallet {
    type Command = WalletCommand;
    type Event = WalletEvent;
    type Error = WalletError;
    type Services = Box<dyn WalletApi + Send + Sync>;

    fn apply(&mut self, event: Self::Event) {
        match event {
            WalletEvent::Signed(..) => {
                self.pending_messages = vec![];
            }
            WalletEvent::AddedMessageToSign(message) => {
                self.pending_messages.push(message);
            }
        }
    }

    async fn handle(
        &self,
        cmd: Self::Command,
        svc: &Self::Services,
    ) -> Result<Vec<Self::Event>, Self::Error> {
        match cmd {
            WalletCommand::AddMessageToSign(message) => {
                Ok(vec![WalletEvent::AddedMessageToSign(message)])
            }
            WalletCommand::Sign() => {
                if self.pending_messages.len() == 0 {
                    return Err(WalletError::NoMessagesToSign);
                }

                let messages_signatures =
                    future::try_join_all(self.pending_messages.iter().map(|m: &Message| async {
                        Ok((
                            svc.sign(m).await.map_err(|_| WalletError::Unknown)?,
                            m.to_owned(),
                        ))
                    }))
                    .await?;

                Ok(vec![WalletEvent::Signed(messages_signatures)])
            }
        }
    }
}

impl Default for Wallet {
    fn default() -> Self {
        Self {
            device_id: None,
            pending_messages: vec![],
        }
    }
}

#[cfg(test)]
mod wallet_test {
    use super::*;
    use crate::test::TestFramework;
    use crate::utils::prelude::*;

    struct SimpleWalletArgs {
        pass: String,
    }

    type WalletService = TestFramework<Wallet>;

    #[test]
    fn add_pending_tx() {
        let message = Message::try_from([0u8, 1u8, 2u8].as_slice()).expect("can not convert");

        WalletService::with(Box::new(MockWalletService::default()))
            .given_no_previous_events()
            .when(WalletCommand::AddMessageToSign(message.clone()))
            .then_expect_events(vec![WalletEvent::AddedMessageToSign(message)]);
    }

    #[async_std::test]
    async fn no_messages_to_sign() {
        let message = Message::try_from([0u8, 1u8, 2u8].as_slice()).expect("can not convert");

        let executor = WalletService::with(Box::new(MockWalletService::default()))
            .given_no_previous_events()
            .when(WalletCommand::Sign())
            .then_expect_error(WalletError::NoMessagesToSign);
    }

    #[async_std::test]
    async fn sign_tx_queues() {
        let message = Message::try_from([0u8, 1u8, 2u8].as_slice()).expect("can not convert");

        let executor = WalletService::with(Box::new(MockWalletService::default()))
            .given(vec![WalletEvent::AddedMessageToSign(message.clone())]);

        let validator = executor.when_async(WalletCommand::Sign()).await;
        validator.then_expect_events(vec![WalletEvent::Signed(vec![(
            Message::try_from([1u8].as_slice()).expect("can not convert"),
            message,
        )])])
    }

    #[derive(Default)]
    struct MockWalletService;

    #[async_trait]
    impl WalletApi for MockWalletService {
        async fn sign<'p>(&self, payload: &'p [u8]) -> WalletResult<Message> {
            Ok(Message::try_from([1u8].as_slice()).expect("hello world"))
        }
    }
}
