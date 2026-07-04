pub type SonateId = u64;

pub type SonateEventCallback =
    Option<extern "C" fn(usize, *const crate::SonateEvent, *mut std::ffi::c_void)>;

pub trait EngineBackend: Send + Sync {
    fn add_stylesheet(&self, css: String);
    fn create_node(&self, node_id: SonateId, text: Option<String>);
    fn destroy_node(&self, node_id: SonateId);
    fn set_parent(&self, parent_id: SonateId, child_id: SonateId);
    fn set_attribute(&self, node_id: SonateId, key: String, value: String);
    fn set_text(&self, node_id: SonateId, text: Option<String>);
    fn root_id(&self) -> SonateId;
    fn set_event_callback(
        &self,
        callback: SonateEventCallback,
        user_data: *mut std::ffi::c_void,
    ) -> i32;
    fn run(&self) -> i32;
    fn destroy(&self) -> i32;
}
