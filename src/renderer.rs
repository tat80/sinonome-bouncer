use windows::core::Interface;

pub struct LayeredRenderer {
    context: windows::Win32::Graphics::Direct3D11::ID3D11DeviceContext,
    target: windows::Win32::Graphics::Direct3D11::ID3D11RenderTargetView,
    back_buffer: windows::Win32::Graphics::Direct3D11::ID3D11Texture2D,
    upload: windows::Win32::Graphics::Direct3D11::ID3D11Texture2D,
    width: u32,
    height: u32,
    swap_chain: windows::Win32::Graphics::Dxgi::IDXGISwapChain1,
    composition: Option<windows::Win32::Graphics::DirectComposition::IDCompositionDevice>,
    _composition_target: Option<windows::Win32::Graphics::DirectComposition::IDCompositionTarget>,
    _visual: Option<windows::Win32::Graphics::DirectComposition::IDCompositionVisual>,
}

impl LayeredRenderer {
    pub fn new(hwnd: *mut core::ffi::c_void, width: u32, height: u32) -> Self {
        use windows::Win32::Foundation::{HMODULE, HWND};
        use windows::Win32::Graphics::Direct3D::D3D_DRIVER_TYPE_HARDWARE;
        use windows::Win32::Graphics::Direct3D11::{
            D3D11CreateDevice, D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_SDK_VERSION,
        };
        use windows::Win32::Graphics::DirectComposition::DCompositionCreateDevice;
        use windows::Win32::Graphics::Dxgi::{
            Common::{DXGI_ALPHA_MODE_PREMULTIPLIED, DXGI_FORMAT_B8G8R8A8_UNORM},
            CreateDXGIFactory2, IDXGIDevice, IDXGIFactory2, DXGI_CREATE_FACTORY_FLAGS,
            DXGI_SCALING_STRETCH, DXGI_SWAP_CHAIN_DESC1, DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL,
            DXGI_USAGE_RENDER_TARGET_OUTPUT,
        };

        let hwnd = HWND(hwnd as _);

        unsafe {
            let mut device = None;
            let mut context = None;
            D3D11CreateDevice(
                None,
                D3D_DRIVER_TYPE_HARDWARE,
                HMODULE::default(),
                D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                None,
                D3D11_SDK_VERSION,
                Some(&mut device),
                None,
                Some(&mut context),
            )
            .expect("D3D11デバイスを作成できません");
            let device = device.expect("D3D11デバイスがありません");
            let context = context.expect("D3D11コンテキストがありません");
            let dxgi_device: IDXGIDevice = device.cast().expect("DXGIデバイスを取得できません");
            let factory: IDXGIFactory2 = CreateDXGIFactory2(DXGI_CREATE_FACTORY_FLAGS(0))
                .expect("DXGIファクトリを作成できません");
            let swap_chain = factory
                .CreateSwapChainForComposition(
                    &device,
                    &DXGI_SWAP_CHAIN_DESC1 {
                        Width: width,
                        Height: height,
                        Format: DXGI_FORMAT_B8G8R8A8_UNORM,
                        Stereo: false.into(),
                        SampleDesc: windows::Win32::Graphics::Dxgi::Common::DXGI_SAMPLE_DESC {
                            Count: 1,
                            Quality: 0,
                        },
                        BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
                        BufferCount: 2,
                        Scaling: DXGI_SCALING_STRETCH,
                        SwapEffect: DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL,
                        AlphaMode: DXGI_ALPHA_MODE_PREMULTIPLIED,
                        Flags: 0,
                    },
                    None,
                )
                .expect("composition swap chain を作成できません");
            let back_buffer: windows::Win32::Graphics::Direct3D11::ID3D11Texture2D = swap_chain
                .GetBuffer(0)
                .expect("swap chain のバッファを取得できません");
            let mut target = None;
            device
                .CreateRenderTargetView(&back_buffer, None, Some(&mut target))
                .expect("swap chain の描画先を作成できません");
            let target = target.expect("swap chain の描画先がありません");
            let composition: windows::Win32::Graphics::DirectComposition::IDCompositionDevice =
                DCompositionCreateDevice(&dxgi_device)
                    .expect("DirectCompositionデバイスを作成できません");
            let composition_target = composition
                .CreateTargetForHwnd(hwnd, false)
                .expect("DirectCompositionターゲットを作成できません");
            let visual = composition
                .CreateVisual()
                .expect("DirectCompositionビジュアルを作成できません");
            visual
                .SetContent(&swap_chain)
                .expect("DirectCompositionへ swap chain を設定できません");
            composition_target
                .SetRoot(&visual)
                .expect("DirectCompositionのルートを設定できません");
            composition
                .Commit()
                .expect("DirectCompositionを確定できません");
            let upload = create_upload_texture(&device, width, height);
            Self {
                context,
                target,
                back_buffer,
                upload,
                width,
                height,
                swap_chain,
                composition: Some(composition),
                _composition_target: Some(composition_target),
                _visual: Some(visual),
            }
        }
    }

    pub fn present(&mut self, rgba: &[u8]) {
        use windows::Win32::Graphics::Dxgi::DXGI_PRESENT;
        unsafe {
            if rgba.len() == (self.width * self.height * 4) as usize {
                let mut mapped =
                    windows::Win32::Graphics::Direct3D11::D3D11_MAPPED_SUBRESOURCE::default();
                self.context
                    .Map(
                        &self.upload,
                        0,
                        windows::Win32::Graphics::Direct3D11::D3D11_MAP_WRITE_DISCARD,
                        0,
                        Some(&mut mapped),
                    )
                    .expect("GPU テクスチャを map できません");
                let source_stride = (self.width * 4) as usize;
                let destination_stride = mapped.RowPitch as usize;
                let destination = std::slice::from_raw_parts_mut(
                    mapped.pData as *mut u8,
                    destination_stride * self.height as usize,
                );
                for row in 0..self.height as usize {
                    let source = &rgba[row * source_stride..(row + 1) * source_stride];
                    let destination_row = &mut destination
                        [row * destination_stride..row * destination_stride + source_stride];
                    for pixel in 0..self.width as usize {
                        let source = &source[pixel * 4..pixel * 4 + 4];
                        let destination = &mut destination_row[pixel * 4..pixel * 4 + 4];
                        let alpha = source[3] as u16;
                        destination.copy_from_slice(&[
                            ((source[2] as u16 * alpha) / 255) as u8,
                            ((source[1] as u16 * alpha) / 255) as u8,
                            ((source[0] as u16 * alpha) / 255) as u8,
                            source[3],
                        ]);
                    }
                }
                self.context.Unmap(&self.upload, 0);
                self.context.CopyResource(&self.back_buffer, &self.upload);
            } else {
                self.context
                    .ClearRenderTargetView(&self.target, &[1.0, 0.0, 0.0, 1.0]);
            }
            self.swap_chain
                .Present(1, DXGI_PRESENT(0))
                .ok()
                .expect("swap chain を表示できません");
            if let Some(composition) = &self.composition {
                composition
                    .Commit()
                    .expect("DirectCompositionを確定できません");
            }
        }
    }
}

unsafe fn create_upload_texture(
    device: &windows::Win32::Graphics::Direct3D11::ID3D11Device,
    width: u32,
    height: u32,
) -> windows::Win32::Graphics::Direct3D11::ID3D11Texture2D {
    use windows::Win32::Graphics::Direct3D11::{
        D3D11_BIND_SHADER_RESOURCE, D3D11_CPU_ACCESS_WRITE, D3D11_TEXTURE2D_DESC,
        D3D11_USAGE_DYNAMIC,
    };
    use windows::Win32::Graphics::Dxgi::Common::{DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_SAMPLE_DESC};
    let mut texture = None;
    device
        .CreateTexture2D(
            &D3D11_TEXTURE2D_DESC {
                Width: width,
                Height: height,
                MipLevels: 1,
                ArraySize: 1,
                Format: DXGI_FORMAT_B8G8R8A8_UNORM,
                SampleDesc: DXGI_SAMPLE_DESC {
                    Count: 1,
                    Quality: 0,
                },
                Usage: D3D11_USAGE_DYNAMIC,
                BindFlags: D3D11_BIND_SHADER_RESOURCE.0 as u32,
                CPUAccessFlags: D3D11_CPU_ACCESS_WRITE.0 as u32,
                MiscFlags: 0,
            },
            None,
            Some(&mut texture),
        )
        .expect("GPU テクスチャを作成できません");
    texture.expect("GPU テクスチャがありません")
}
