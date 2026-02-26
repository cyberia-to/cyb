use crate::sugarloaf::{Colorspace, SugarloafWindow, SugarloafWindowSize};
use crate::SugarloafRenderer;

pub struct Context<'a> {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub format: wgpu::TextureFormat,
    pub size: SugarloafWindowSize,
    pub scale: f32,
    pub supports_f16: bool,
    pub colorspace: Colorspace,
    pub max_texture_dimension_2d: u32,
    /// Offscreen render target for embedded/headless mode.
    /// When Some, render() uses this texture instead of the surface swapchain.
    pub offscreen_texture: Option<wgpu::Texture>,
    /// Read-back buffer for copying offscreen texture to CPU.
    pub readback_buffer: Option<wgpu::Buffer>,
    // --- Fields only present in standalone mode (own surface) ---
    surface: Option<wgpu::Surface<'a>>,
    alpha_mode: Option<wgpu::CompositeAlphaMode>,
    pub adapter_info: Option<wgpu::AdapterInfo>,
    surface_caps: Option<wgpu::SurfaceCapabilities>,
}

#[inline]
#[cfg(not(target_os = "macos"))]
fn find_best_texture_format(
    formats: &[wgpu::TextureFormat],
    colorspace: Colorspace,
) -> wgpu::TextureFormat {
    let mut format: wgpu::TextureFormat = formats.first().unwrap().to_owned();

    // TODO: Fix formats with signs
    // FIXME: On Nvidia GPUs usage Rgba16Float texture format causes driver to enable HDR.
    // Reason for this is currently output color space is poorly defined in wgpu and
    // anything other than Srgb texture formats can cause undeterministic output color
    // space selection which also causes colors to mismatch. Optionally we can whitelist
    // only the Srgb texture formats for now until output color space selection lands in wgpu. See #205
    // TODO: use output color format for the CanvasConfiguration when it lands on the wgpu
    #[cfg(windows)]
    let unsupported_formats = [
        wgpu::TextureFormat::Rgba8Snorm,
        wgpu::TextureFormat::Rgba16Float,
    ];

    // not reproduce-able on mac
    #[cfg(not(windows))]
    let unsupported_formats = [
        wgpu::TextureFormat::Rgba8Snorm,
        // Features::TEXTURE_FORMAT_16BIT_NORM must be enabled to use these texture format.
        wgpu::TextureFormat::R16Unorm,
        wgpu::TextureFormat::R16Snorm,
    ];

    // Bgra8Unorm is the most widely supported and guaranteed format in wgpu
    // Prefer it explicitly if available
    if formats.contains(&wgpu::TextureFormat::Bgra8Unorm) {
        format = wgpu::TextureFormat::Bgra8Unorm;
        tracing::info!(
            "Sugarloaf selected format: {format:?} from {:?} for colorspace {:?}",
            formats,
            colorspace
        );
        return format;
    }

    let filtered_formats: Vec<wgpu::TextureFormat> = formats
        .iter()
        .copied()
        .filter(|&x| {
            // On non-macOS platforms, always avoid sRGB formats
            // This maintains compatibility with existing Linux/Windows color handling
            !wgpu::TextureFormat::is_srgb(&x) && !unsupported_formats.contains(&x)
        })
        .collect();

    // If no compatible formats found, fall back to any non-unsupported format
    let final_formats = if filtered_formats.is_empty() {
        formats
            .iter()
            .copied()
            .filter(|&x| !unsupported_formats.contains(&x))
            .collect()
    } else {
        filtered_formats
    };

    if !final_formats.is_empty() {
        final_formats.first().unwrap().clone_into(&mut format);
    }

    tracing::info!(
        "Sugarloaf selected format: {format:?} from {:?} for colorspace {:?}",
        formats,
        colorspace
    );

    format
}

#[inline]
#[cfg(target_os = "macos")]
fn get_macos_texture_format(colorspace: Colorspace) -> wgpu::TextureFormat {
    match colorspace {
        Colorspace::Srgb => wgpu::TextureFormat::Bgra8UnormSrgb,
        Colorspace::DisplayP3 | Colorspace::Rec2020 => wgpu::TextureFormat::Bgra8Unorm,
    }
}

impl Context<'_> {
    /// Create a Context that owns its own wgpu Instance/Device/Queue/Surface (standalone mode).
    pub fn new<'a>(
        sugarloaf_window: SugarloafWindow,
        renderer_config: SugarloafRenderer,
    ) -> Context<'a> {
        let backend = wgpu::Backends::from_env().unwrap_or(renderer_config.backend);
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: backend,
            ..Default::default()
        });

        tracing::info!("selected instance: {instance:?}");

        #[cfg(not(target_arch = "wasm32"))]
        {
            tracing::info!("Available adapters:");
            for a in instance.enumerate_adapters(wgpu::Backends::all()) {
                tracing::info!("    {:?}", a.get_info())
            }
        }

        tracing::info!("initializing the surface");

        let size = sugarloaf_window.size;
        let scale = sugarloaf_window.scale;

        let surface: wgpu::Surface<'a> =
            instance.create_surface(sugarloaf_window).unwrap();
        let adapter = futures::executor::block_on(instance.request_adapter(
            &wgpu::RequestAdapterOptions {
                power_preference: renderer_config.power_preference,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            },
        ))
        .expect("Request adapter");

        let adapter_info = adapter.get_info();
        tracing::info!("Selected adapter: {:?}", adapter_info);

        let surface_caps = surface.get_capabilities(&adapter);

        #[cfg(target_os = "macos")]
        let format = get_macos_texture_format(renderer_config.colorspace);
        #[cfg(not(target_os = "macos"))]
        let format = find_best_texture_format(
            surface_caps.formats.as_slice(),
            renderer_config.colorspace,
        );

        let (device, queue, supports_f16) = {
            let base_features = wgpu::Features::ADDRESS_MODE_CLAMP_TO_BORDER;
            let base_f16_features = base_features | wgpu::Features::SHADER_F16;

            let device_configs = [(base_f16_features, true), (base_features, false)];

            let mut result = None;
            for (features, supports_f16_val) in device_configs {
                if let Ok(device_result) = futures::executor::block_on(
                    adapter.request_device(&wgpu::DeviceDescriptor {
                        required_features: features,
                        ..Default::default()
                    }),
                ) {
                    result = Some((device_result.0, device_result.1, supports_f16_val));
                    break;
                }
            }

            result.unwrap_or_else(|| {
                // Last resort: downlevel limits with no features
                let device_result = futures::executor::block_on(adapter.request_device(
                    &wgpu::DeviceDescriptor {
                        memory_hints: wgpu::MemoryHints::Performance,
                        label: None,
                        required_features: wgpu::Features::empty(),
                        required_limits: wgpu::Limits::downlevel_webgl2_defaults(),
                        ..Default::default()
                    },
                ))
                .expect("Request device");
                (device_result.0, device_result.1, false)
            })
        };

        let alpha_mode = if surface_caps
            .alpha_modes
            .contains(&wgpu::CompositeAlphaMode::PostMultiplied)
        {
            wgpu::CompositeAlphaMode::PostMultiplied
        } else if surface_caps
            .alpha_modes
            .contains(&wgpu::CompositeAlphaMode::PreMultiplied)
        {
            wgpu::CompositeAlphaMode::PreMultiplied
        } else {
            wgpu::CompositeAlphaMode::Auto
        };

        // Configure view formats for wide color gamut support
        let view_formats = match renderer_config.colorspace {
            Colorspace::DisplayP3 | Colorspace::Rec2020 => {
                vec![format]
            }
            Colorspace::Srgb => {
                vec![]
            }
        };

        surface.configure(
            &device,
            &wgpu::SurfaceConfiguration {
                usage: Self::get_texture_usage(&surface_caps),
                format,
                width: size.width as u32,
                height: size.height as u32,
                view_formats,
                alpha_mode,
                present_mode: wgpu::PresentMode::Fifo,
                desired_maximum_frame_latency: 2,
            },
        );

        let max_texture_dimension_2d = device.limits().max_texture_dimension_2d;

        tracing::info!("F16 shader support: {}", supports_f16);
        tracing::info!("Configured colorspace: {:?}", renderer_config.colorspace);
        tracing::info!("Surface format: {:?}", format);
        tracing::info!("Max texture dimension 2D: {}", max_texture_dimension_2d);

        Context {
            device,
            queue,
            format,
            size: SugarloafWindowSize {
                width: size.width,
                height: size.height,
            },
            scale,
            supports_f16,
            colorspace: renderer_config.colorspace,
            max_texture_dimension_2d,
            offscreen_texture: None,
            readback_buffer: None,
            surface: Some(surface),
            alpha_mode: Some(alpha_mode),
            adapter_info: Some(adapter_info),
            surface_caps: Some(surface_caps),
        }
    }

    /// Create a Context using an externally-provided Device and Queue (embedded mode).
    /// No wgpu Instance, Surface, or Adapter is created — the caller owns the GPU stack.
    pub fn new_external(
        device: wgpu::Device,
        queue: wgpu::Queue,
        format: wgpu::TextureFormat,
        size: SugarloafWindowSize,
        scale: f32,
    ) -> Context<'static> {
        let supports_f16 = device.features().contains(wgpu::Features::SHADER_F16);
        let max_texture_dimension_2d = device.limits().max_texture_dimension_2d;

        tracing::info!("Sugarloaf external context: format={:?}, f16={}, max_tex={}",
            format, supports_f16, max_texture_dimension_2d);

        Context {
            device,
            queue,
            format,
            size,
            scale,
            supports_f16,
            colorspace: Colorspace::default(),
            max_texture_dimension_2d,
            offscreen_texture: None,
            readback_buffer: None,
            surface: None,
            alpha_mode: None,
            adapter_info: None,
            surface_caps: None,
        }
    }

    /// Create a Context with an offscreen render target (no window surface needed for rendering).
    /// The Context still needs a window surface for wgpu initialization, but render() will
    /// target the offscreen texture instead of the swapchain.
    pub fn enable_offscreen(&mut self) {
        let width = self.size.width as u32;
        let height = self.size.height as u32;

        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("sugarloaf_offscreen"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        // Buffer for reading pixels back to CPU
        let bytes_per_row = Self::align_to_256(width * 4);
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sugarloaf_readback"),
            size: (bytes_per_row * height) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        self.offscreen_texture = Some(texture);
        self.readback_buffer = Some(buffer);
    }

    /// Get the offscreen texture view for rendering
    pub fn offscreen_view(&self) -> Option<wgpu::TextureView> {
        self.offscreen_texture.as_ref().map(|t| {
            t.create_view(&wgpu::TextureViewDescriptor::default())
        })
    }

    /// Read offscreen texture pixels into a Vec<u8> (RGBA8).
    /// Returns None if offscreen mode is not enabled.
    pub fn read_offscreen_pixels(&self) -> Option<Vec<u8>> {
        let texture = self.offscreen_texture.as_ref()?;
        let buffer = self.readback_buffer.as_ref()?;
        let width = self.size.width as u32;
        let height = self.size.height as u32;
        let bytes_per_row = Self::align_to_256(width * 4);

        let mut encoder = self.device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor { label: Some("readback_encoder") },
        );

        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        self.queue.submit(Some(encoder.finish()));

        let buffer_slice = buffer.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = tx.send(result);
        });
        let _ = self.device.poll(wgpu::PollType::Wait { submission_index: None, timeout: None });
        rx.recv().ok()?.ok()?;

        let data = buffer_slice.get_mapped_range();
        // Remove row padding
        let mut pixels = Vec::with_capacity((width * height * 4) as usize);
        for row in 0..height {
            let start = (row * bytes_per_row) as usize;
            let end = start + (width * 4) as usize;
            pixels.extend_from_slice(&data[start..end]);
        }
        drop(data);
        buffer.unmap();

        Some(pixels)
    }

    fn align_to_256(value: u32) -> u32 {
        (value + 255) & !255
    }

    /// Get the owned surface (standalone mode only).
    pub fn surface(&self) -> Option<&wgpu::Surface<'_>> {
        self.surface.as_ref()
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.size.width = width as f32;
        self.size.height = height as f32;

        if let (Some(ref surface), Some(ref surface_caps), Some(alpha_mode)) =
            (&self.surface, &self.surface_caps, self.alpha_mode)
        {
            // Configure view formats for wide color gamut support
            let view_formats = match self.colorspace {
                Colorspace::DisplayP3 | Colorspace::Rec2020 => {
                    vec![self.format]
                }
                Colorspace::Srgb => {
                    vec![]
                }
            };

            surface.configure(
                &self.device,
                &wgpu::SurfaceConfiguration {
                    usage: Self::get_texture_usage(surface_caps),
                    format: self.format,
                    width,
                    height,
                    view_formats,
                    alpha_mode,
                    present_mode: wgpu::PresentMode::Fifo,
                    desired_maximum_frame_latency: 2,
                },
            );
        }
    }

    pub fn surface_caps(&self) -> Option<&wgpu::SurfaceCapabilities> {
        self.surface_caps.as_ref()
    }

    pub fn supports_f16(&self) -> bool {
        self.supports_f16
    }

    pub fn get_optimal_texture_format(&self, channels: u32) -> wgpu::TextureFormat {
        if self.supports_f16 {
            match channels {
                1 => wgpu::TextureFormat::R16Float,
                2 => wgpu::TextureFormat::Rg16Float,
                4 => wgpu::TextureFormat::Rgba16Float,
                _ => wgpu::TextureFormat::Rgba8Unorm, // fallback
            }
        } else {
            wgpu::TextureFormat::Rgba8Unorm
        }
    }

    pub fn max_texture_dimension_2d(&self) -> u32 {
        self.max_texture_dimension_2d
    }

    pub fn get_optimal_texture_sample_type(&self) -> wgpu::TextureSampleType {
        // Both Rgba16Float (f16) and Rgba8Unorm (f32) use Float sample type with filtering
        wgpu::TextureSampleType::Float { filterable: true }
    }

    pub fn convert_rgba8_to_optimal_format(&self, rgba8_data: &[u8]) -> Vec<u8> {
        if self.supports_f16 {
            // Convert u8 RGBA to f16 RGBA
            let mut f16_data = Vec::with_capacity(rgba8_data.len() * 2);
            for chunk in rgba8_data.chunks(4) {
                if chunk.len() == 4 {
                    // Convert u8 [0-255] to f16 [0.0-1.0]
                    let r = half::f16::from_f32(chunk[0] as f32 / 255.0);
                    let g = half::f16::from_f32(chunk[1] as f32 / 255.0);
                    let b = half::f16::from_f32(chunk[2] as f32 / 255.0);
                    let a = half::f16::from_f32(chunk[3] as f32 / 255.0);

                    f16_data.extend_from_slice(&r.to_le_bytes());
                    f16_data.extend_from_slice(&g.to_le_bytes());
                    f16_data.extend_from_slice(&b.to_le_bytes());
                    f16_data.extend_from_slice(&a.to_le_bytes());
                }
            }
            f16_data
        } else {
            rgba8_data.to_vec()
        }
    }

    fn get_texture_usage(caps: &wgpu::SurfaceCapabilities) -> wgpu::TextureUsages {
        let mut usage = wgpu::TextureUsages::RENDER_ATTACHMENT;

        // COPY_DST and COPY_SRC are required for FiltersBrush
        // But some backends like OpenGL might not support COPY_DST and COPY_SRC
        // https://github.com/emilk/egui/pull/3078

        if caps.usages.contains(wgpu::TextureUsages::COPY_DST) {
            usage |= wgpu::TextureUsages::COPY_DST;
        }

        if caps.usages.contains(wgpu::TextureUsages::COPY_SRC) {
            usage |= wgpu::TextureUsages::COPY_SRC;
        }

        usage
    }
}
