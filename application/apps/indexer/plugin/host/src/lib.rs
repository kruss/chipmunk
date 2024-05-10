pub mod wasi;
pub mod wasm;

use log::info;
use plugin_rpc::*;
use std::{
    any::Any,
    cell::RefCell,
    collections::HashMap,
    fmt::Debug,
    marker::{Send, Sync},
    rc::Rc,
};
use strum_macros::Display;

pub type PluginName<'a> = &'a str;
pub type PluginId = usize;
pub type PluginFactoryObj = Box<dyn PluginFactory>;
pub type PluginProxyObj = Box<dyn PluginProxy + Send + Sync>;
pub type PluginProxyRef = Rc<RefCell<PluginProxyObj>>;

#[derive(Debug, Display, PartialEq)]
pub enum PluginError {
    Invalid,
    Unsupported,
}

#[derive(Default)]
pub struct PluginRuntime<'a> {
    pub(crate) factories: HashMap<PluginName<'a>, PluginFactoryObj>,
    next_id: PluginId,
}

impl<'a> PluginRuntime<'a> {
    pub fn add_factory(&mut self, name: PluginName<'a>, factory: Box<dyn PluginFactory>) {
        info!("add plugin factory '{}'", name);
        self.factories.insert(name, factory);
    }

    pub fn create_proxy(&mut self, name: PluginName<'a>) -> Result<PluginProxyRef, PluginError> {
        if let Some(factory) = &mut self.factories.get_mut(&name) {
            let mut proxy = factory.create(self.next_id)?;
            self.next_id += 1;
            proxy.init()?;
            Ok(Rc::new(RefCell::new(proxy)))
        } else {
            Err(PluginError::Invalid)
        }
    }
}

pub trait PluginFactory {
    fn create(&self, id: PluginId) -> Result<PluginProxyObj, PluginError>;
}

pub trait PluginProxy {
    fn as_any(&self) -> &dyn Any;
    fn id(&self) -> PluginId;
    fn call(&mut self, request: &[u8]) -> Result<Vec<u8>, PluginError>;

    fn init(&mut self) -> Result<(), PluginError> {
        let request: PluginRequest<()> = PluginRequest::Runtime(PluginRuntimeRequest::Init);
        let input = rkyv::to_bytes::<_, 256>(&request).unwrap();
        info!("send init request with {} bytes : {:?}", input.len(), input);
        match self.call(&input) {
            Ok(output) => {
                info!(
                    "received init response with {} bytes : {:?}",
                    output.len(),
                    output
                );
                let result: PluginResponse<()> = rkyv::from_bytes(&output).unwrap();
                match result {
                    PluginResponse::Runtime(PluginRuntimeResponse::Init) => Ok(()),
                    _ => Err(PluginError::Invalid),
                }
            }
            _ => Err(PluginError::Invalid),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rkyv::{Archive, Deserialize, Serialize};

    #[derive(Default)]
    struct TestFactory {}

    impl PluginFactory for TestFactory {
        fn create(&self, id: PluginId) -> Result<PluginProxyObj, PluginError> {
            Ok(Box::new(TestPlugin::new(id)))
        }
    }

    struct TestPlugin {
        id: PluginId,
        state: Option<TestPluginState>,
    }

    impl TestPlugin {
        fn new(id: PluginId) -> Self {
            TestPlugin { id, state: None }
        }

        fn call_ping(proxy: &mut PluginProxyRef) {
            let mut proxy = proxy.borrow_mut();
            let request = PluginRequest::Plugin(TestPluginRequest::Ping);
            let input = rkyv::to_bytes::<_, 256>(&request).unwrap();

            let output = proxy.call(&input).expect("call");
            let result = rkyv::from_bytes(&output).unwrap();
            assert_eq!(PluginResponse::Plugin(TestPluginResponse::Ping), result);
        }

        fn assert_state(proxy: &PluginProxyRef, state: TestPluginState) {
            let proxy = proxy.borrow_mut();
            let plugin: &TestPlugin = proxy
                .as_any()
                .downcast_ref::<TestPlugin>()
                .expect("downcast");
            assert_eq!(Some(state), plugin.state);
        }
    }

    #[derive(Debug, PartialEq)]
    enum TestPluginState {
        Init,
        Ping,
    }

    #[derive(Archive, Serialize, Deserialize, Debug, Display, PartialEq)]
    #[archive(check_bytes)]
    pub enum TestPluginRequest {
        Ping,
    }

    #[derive(Archive, Serialize, Deserialize, Debug, Display, PartialEq)]
    #[archive(check_bytes)]
    pub enum TestPluginResponse {
        Ping,
    }

    impl PluginProxy for TestPlugin {
        fn as_any(&self) -> &dyn Any {
            self
        }

        fn id(&self) -> PluginId {
            self.id
        }

        fn call(&mut self, input: &[u8]) -> Result<Vec<u8>, PluginError> {
            let request = rkyv::from_bytes(input).unwrap();
            let output = match request {
                PluginRequest::Runtime(PluginRuntimeRequest::Init) => {
                    self.state = Some(TestPluginState::Init);
                    let response: PluginResponse<()> =
                        PluginResponse::Runtime(PluginRuntimeResponse::Init);
                    let output = rkyv::to_bytes::<_, 256>(&response).unwrap();
                    output
                }
                PluginRequest::Plugin(TestPluginRequest::Ping) => {
                    self.state = Some(TestPluginState::Ping);
                    let response = PluginResponse::Plugin(TestPluginResponse::Ping);
                    let output = rkyv::to_bytes::<_, 256>(&response).unwrap();
                    output
                }
                _ => {
                    return Err(PluginError::Unsupported);
                }
            };

            Ok(output.to_vec())
        }
    }

    #[test]
    fn test_plugin_runtime() {
        println!("");
        let _ = env_logger::try_init();

        let mut runtime = PluginRuntime::default();
        assert_eq!(0, runtime.factories.len());

        runtime.add_factory("test", Box::new(TestFactory::default()));
        assert_eq!(1, runtime.factories.len());

        let mut proxy = runtime.create_proxy("test").expect("create");
        assert_eq!(0, proxy.borrow().id());

        TestPlugin::assert_state(&proxy, TestPluginState::Init);
        TestPlugin::call_ping(&mut proxy);
        TestPlugin::assert_state(&proxy, TestPluginState::Ping);
    }

    #[test]
    pub fn test_plugin_commands() {
        let items = [
            PluginRequest::Runtime(PluginRuntimeRequest::Init),
            PluginRequest::Plugin(TestPluginRequest::Ping),
        ];

        for item in items {
            let bytes = rkyv::to_bytes::<_, 256>(&item).unwrap();
            println!("req: {}\t: {:?}", item, bytes);
            let deserialized = rkyv::from_bytes(&bytes).unwrap();
            assert_eq!(item, deserialized);
        }
    }

    #[test]
    pub fn test_plugin_results() {
        let items = [
            PluginResponse::Runtime(PluginRuntimeResponse::Init),
            PluginResponse::Plugin(TestPluginResponse::Ping),
        ];

        for item in items {
            let bytes = rkyv::to_bytes::<_, 256>(&item).unwrap();
            println!("res: {}\t: {:?}", item, bytes);
            let deserialized = rkyv::from_bytes(&bytes).unwrap();
            assert_eq!(item, deserialized);
        }
    }
}
