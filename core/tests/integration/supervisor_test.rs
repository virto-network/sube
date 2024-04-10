
// mod supervisor_test {

//   use virto_sdk::prelude::*;
//   use virto_sdk::{
//       backend::matrix::MatrixAppStore, to_serialized_command_envelope, AppFactory, AppInfo, DomainEvent,
//       AppPermission, ConstructableService, DomainCommand, SDKBuilder, SDKCore, CommandRequest,
//       SerializedCommandEnvelope, SerializedEventEnvelope, StateMachine, Supervisor, CommittedEventEnvelope
//   };
//   use std::fmt::Result;
//   use async_once_cell::OnceCell;
//   use async_std::test;

//   #[derive(Deserialize, Serialize, Debug)]
//   enum TestCmd {
//       A,
//       B,
//   }

//   #[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
//   enum TestEvent {
//       A,
//       B(usize),
//   }

//   #[derive(Debug)]
//   enum TestError {
//       A,
//       B,
//   }

//   impl core::fmt::Display for TestError {
//       fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> std::fmt::Result {
//           match self {
//               Self::A => write!(f, "Error::A"),
//               Self::B => write!(f, "Error::B"),
//           }
//       }
//   }

//   impl std::error::Error for TestError {}

//   impl DomainEvent for TestEvent {
//       fn event_type(&self) -> String {
//           match self {
//               TestEvent::A => "A".into(),
//               TestEvent::B(_) => "B".into(),
//           }
//       }

//       fn event_version(&self) -> String {
//           "0.0.1".into()
//       }
//   }

//   impl DomainCommand for TestCmd {
//       fn command_name(&self) -> String {
//           match self {
//               TestCmd::A => "A".into(),
//               TestCmd::B => "B".into(),
//           }
//       }

//       fn command_payload(&self) -> Value {
//           serde_json::to_value(self).expect("Can not Serialize command")
//       }
//   }

//   #[derive(Serialize, Deserialize)]
//   struct TestAggregate {
//       pub sum: usize,
//   }

//   impl Default for TestAggregate {
//       fn default() -> Self {
//           Self { sum: 0 }
//       }
//   }

//   trait TestService: Sync + Send {
//       fn sign(&self) -> usize;
//   }

//   #[derive(Deserialize, Clone)]
//   struct ServiceConfig {
//       url: String,
//   }

//   #[derive(Default)]
//   struct Service {
//       url: String,
//   }

//   impl Service {
//       fn new(args: ServiceConfig) -> Self {
//           Service { url: args.url }
//       }
//   }

//   impl TestService for Service {
//       fn sign(&self) -> usize {
//           10
//       }
//   }

//   fn to_envelop_cmd(cmd: TestCmd) -> SerializedCommandEnvelope {
//       to_serialized_command_envelope("0.0.1", "david", cmd, HashMap::new())
//   }

//   impl ConstructableService for Box<dyn TestService> {
//       type Args = ServiceConfig;
//       type Service = Box<dyn TestService>;

//       fn new(args: Self::Args) -> Self::Service {
//           Box::new(Service::new(args))
//       }
//   }

//   #[async_trait]
//   impl StateMachine for TestAggregate {
//       type Command = TestCmd;
//       type Event = TestEvent;
//       type Error = TestError;
//       type Services = Box<dyn TestService + 'static>;

//       async fn handle(
//           &self,
//           command: Self::Command,
//           ctx: &Self::Services,
//       ) -> Result<Vec<Self::Event>, Self::Error> {
//           match command {
//               TestCmd::A => Ok(vec![TestEvent::A]),
//               TestCmd::B => Ok(vec![TestEvent::A, TestEvent::B(ctx.sign())]),
//           }
//       }

//       fn apply(&mut self, event: Self::Event) {
//           match event {
//               TestEvent::A => {
//                   self.sum += 1;
//               }
//               TestEvent::B(u) => {
//                   self.sum += u;
//               }
//           }
//       }
//   }

//   fn mock_app_info(id: String) -> AppInfo {
//       AppInfo {
//           author: "foo".into(),
//           description: "foo".into(),
//           id,
//           name: "hello".into(),
//           permissions: vec![],
//           version: "0.0.1".into(),
//       }
//   }

//   fn to_committed_event(
//       sequence: usize,
//       event: SerializedEventEnvelope,
//   ) -> CommittedEventEnvelope {
//       CommittedEventEnvelope {
//           aggregate_id: event.aggregate_id,
//           app_id: event.app_id,
//           event_type: event.event_type,
//           event_version: event.event_version,
//           metadata: event.metadata,
//           payload: event.payload,
//           sequence,
//       }
//   }

//   static mut SDK_CORE: OnceCell<SDKCore> = OnceCell::new();

//   async fn get_sdk() -> &'static mut SDKCore {
//       unsafe {
//           match SDK_CORE.get() {
//               Some(..) => SDK_CORE.get_mut().expect("error getting mut"),
//               None => {
//                   SDK_CORE
//                       .get_or_try_init(async {
//                           let mut core = SDKBuilder::new()
//                               .with_homeserver("https://matrix-client.matrix.org")
//                               .with_credentials("fooaccount2", "H0l4mund0@123")
//                               .with_device_name("iphone-dev")
//                               .build_and_login()
//                               .await
//                               .expect("Can not login");

//                           core.init().await;

//                           Ok::<_, ()>(core)
//                       })
//                       .await
//                       .expect("Erroor at getting core");

//                   SDK_CORE.get_mut().expect("error getting mut")
//               }
//           }
//       }
//   }

//   #[async_std::test]
//   async fn run_command_and_take_snapshot() {
//       let info = mock_app_info("foo".into());
//       let info_2 = mock_app_info("foo_2".into());

//       let sdk = get_sdk().await;

//       let registry = MatrixAppStore::new(sdk.client());

//       let app_factory = AppFactory::<TestAggregate, ServiceConfig>::new(
//           info,
//           ServiceConfig { url: "http".into() }, // container reference
//       );

//       let app_factory_2 = AppFactory::<TestAggregate, ServiceConfig>::new(
//           info_2,
//           ServiceConfig { url: "http".into() }, // container reference
//       );

      
//       // assert!(state.sum == 11);
//   }
// }
