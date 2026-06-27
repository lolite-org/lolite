pub type SonateId = u64;

pub trait EngineBackend: Send {
    fn add_stylesheet(&self, css: String);
    fn create_node(&self, node_id: SonateId, text: Option<String>);
    fn destroy_node(&self, node_id: SonateId);
    fn set_parent(&self, parent_id: SonateId, child_id: SonateId);
    fn set_attribute(&self, node_id: SonateId, key: String, value: String);
    fn root_id(&self) -> SonateId;
    fn run(&self) -> i32;
    fn destroy(&self) -> i32;
}
