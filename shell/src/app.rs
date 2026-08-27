//! The one cyb App — every platform builds it here.
//!
//! Desktop (`main.rs`) and Android (`lib.rs`'s `android_main`) call
//! [`build_app`] and get the same Bevy app: same worlds, same chrome, same
//! nav. Only the surfaces that name a platform differ — global hotkeys, the
//! tray, and the hide-on-close window contract exist where a desktop does.

use bevy::prelude::*;
use bevy::render::RenderApp;
use bevy::render::renderer::{RenderDevice, RenderQueue};

use crate::{agent, shell, worlds};

/// Exposes the render device/queue on the main world so worlds can allocate
/// GPU resources without reaching into the render sub-app.
struct GpuBridgePlugin;

impl Plugin for GpuBridgePlugin {
    fn build(&self, _app: &mut App) {}

    fn finish(&self, app: &mut App) {
        let (device, queue) = {
            let render_app = app.sub_app(RenderApp);
            let device = render_app.world().resource::<RenderDevice>().clone();
            let queue = render_app.world().resource::<RenderQueue>().clone();
            (device, queue)
        };
        // Off Apple, mir's device facade shares Bevy's wgpu device instead of
        // opening a second one: two VkDevices in one process hang the PowerVR
        // driver on the Pixel 10 (intermittently, mid-dispatch) where MoltenVK
        // tolerated it. One device, one queue, no cross-device races.
        #[cfg(not(target_vendor = "apple"))]
        mir::gpu::install_shared(device.wgpu_device().clone(), (***queue).clone());

        app.insert_resource(device);
        app.insert_resource(queue);

        // Android: switch bevy's GPU mesh preprocessing off. cyb's scene is
        // UI + mir's own compute — there are no 3D meshes to preprocess — and
        // the occlusion-culling variant of mesh_preprocess.wgsl samples a
        // depth pyramid from a compute shader, which the PowerVR driver on
        // the Pixel 10 (Tensor G5) aborts on: `spvcompiler: Unhandled
        // sampler flag combo` → SIGABRT at pipeline compile. This plugin's
        // finish runs after bevy_render's, so the override wins.
        #[cfg(target_os = "android")]
        {
            use bevy::render::batching::gpu_preprocessing::{
                GpuPreprocessingMode, GpuPreprocessingSupport,
            };
            app.sub_app_mut(RenderApp).insert_resource(GpuPreprocessingSupport {
                max_supported_mode: GpuPreprocessingMode::None,
            });
        }
    }
}

pub fn build_app() -> App {
    let mut app = App::new();

    // Vulkan platforms: ask the device for VK_EXT_external_memory_host when
    // the adapter has it. mir's unimem wrap imports pinned blocks through it
    // — the GPU reads the block's own pages, nothing is copied. The callback
    // runs at device creation; mir checks enabled_device_extensions() at
    // wrap time, so a driver without the extension just means the copy path.
    #[cfg(any(target_os = "android", target_os = "linux"))]
    {
        use bevy::render::renderer::raw_vulkan_init::RawVulkanInitSettings;
        let mut raw = RawVulkanInitSettings::default();
        // SAFETY: adds an extension only when the physical device lists it;
        // removes nothing, changes nothing else.
        unsafe {
            raw.add_create_device_callback(|args, adapter, _features| {
                // Every handle type mir can import from, in the order it
                // tries them. AHardwareBuffer is Android's IOSurface and
                // needs queue_family_foreign alongside; host-pointer import
                // is the desktop-Vulkan route. wgpu-hal already enables the
                // fd/dma-buf pair, so those are not listed here.
                const WANTED: &[&std::ffi::CStr] = &[
                    ash::android::external_memory_android_hardware_buffer::NAME,
                    ash::ext::queue_family_foreign::NAME,
                    ash::ext::external_memory_host::NAME,
                ];
                let Ok(props) = adapter
                    .shared_instance()
                    .raw_instance()
                    .enumerate_device_extension_properties(adapter.raw_physical_device())
                else {
                    return;
                };
                for ext in WANTED {
                    let supported = props.iter().any(|p| {
                        p.extension_name_as_c_str().is_ok_and(|n| n == *ext)
                    });
                    if supported && !args.extensions.contains(ext) {
                        args.extensions.push(ext);
                    }
                }
            });
        }
        app.insert_resource(raw);
    }

    #[allow(unused_mut)]
    let mut window_plugin = WindowPlugin {
        primary_window: Some(Window {
            title: "cyb".into(),
            resolution: (1280u32, 800u32).into(),
            ..default()
        }),
        ..default()
    };

    // The robot outlives its window: closing hides, the tray brings it back,
    // only tray → Quit ends the process. See `shell/window.rs`. On Android
    // the OS owns the app lifecycle, so the defaults stand.
    #[cfg(not(target_os = "android"))]
    {
        window_plugin.close_when_requested = false;
        window_plugin.exit_condition = bevy::window::ExitCondition::DontExit;
    }

    app.add_plugins(
        DefaultPlugins.set(window_plugin).set(bevy::render::RenderPlugin {
            render_creation: bevy::render::settings::RenderCreation::Automatic(
                bevy::render::settings::WgpuSettings { ..default() },
            ),
            ..default()
        }),
    )
    .insert_resource(ClearColor(bevy::color::Color::BLACK))
    .add_plugins(GpuBridgePlugin)
    .add_plugins(prysm::PrysmPlugin)
    .add_plugins(worlds::WorldsPlugin)
    .add_plugins(shell::chrome::ChromePlugin)
    .add_plugins(shell::platform::PlatformPlugin)
    .add_plugins(shell::nav::NavPlugin)
    .add_plugins(mir::bevy::GraphWorldPlugin)
    .add_plugins(worlds::graph::GraphBridgePlugin)
    .add_plugins(worlds::terminal::TerminalWorldPlugin)
    .add_plugins(worlds::cell::CellWorldPlugin)
    .add_plugins(worlds::sigma::SigmaWorldPlugin)
    .add_plugins(agent::AgentPlugin);

    #[cfg(not(target_os = "android"))]
    app.add_plugins(shell::hotkeys::HotkeysPlugin)
        .add_plugins(shell::window::WindowLifecyclePlugin)
        .add_plugins(shell::tray::TrayPlugin);

    app
}
