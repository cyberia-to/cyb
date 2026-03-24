---
tags: cyb, core
crystal-type: entity
crystal-domain: cyber
diffusion: 0.00014274588386253872
springs: 0.0010764554648910948
heat: 0.0007949381557019456
focus: 0.0005532972125390032
gravity: 2
density: 5.14
---
# wgpu

cross-platform GPU abstraction layer in [[cyb]]. renders every [[particle]] type — text, pixels, vector, video, formula — through a unified shader pipeline

maps to native backends: Vulkan on Linux, Metal on macOS/iOS, DX12 on Windows, OpenGL ES on Android/web. same pixels everywhere

used by [[cyb/features]] PureRender for all visual output: glyph atlases, Vello path rasterization, texture sampling, compute shaders for waveform visualization and table virtualization

see [[cyb/features]] for the render pipeline. see [[cyb/onnx]] for the neural inference complement