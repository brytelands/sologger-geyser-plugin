use {
    crate::geyser_plugin_sologger::SologgerGeyserPlugin,
    agave_geyser_plugin_interface::geyser_plugin_interface::GeyserPlugin,
};

use crate::geyser_plugin_sologger::PluginContext;

#[no_mangle]
#[allow(improper_ctypes_definitions)]
/// # Safety
///
/// This function simply allocates a GeyserPluginHook,
/// and returns a pointer to it as trait GeyserPlugin.
pub unsafe extern "C" fn _create_plugin() -> *mut dyn GeyserPlugin {
    let plugin = SologgerGeyserPlugin { context: PluginContext::default() };
    let plugin: Box<dyn GeyserPlugin> = Box::new(plugin);
    Box::into_raw(plugin)
}