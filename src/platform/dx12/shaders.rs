use windows::Win32::Graphics::{
    Direct3D12::{
        D3D12SerializeRootSignature, ID3D12Device, ID3D12PipelineState, ID3D12RootSignature,
        D3D12_BLEND_DESC, D3D12_DEPTH_STENCIL_DESC, D3D12_GRAPHICS_PIPELINE_STATE_DESC,
        D3D12_INPUT_LAYOUT_DESC, D3D12_PRIMITIVE_TOPOLOGY_TYPE_TRIANGLE, D3D12_RASTERIZER_DESC,
        D3D12_ROOT_SIGNATURE_DESC, D3D12_ROOT_SIGNATURE_FLAG_ALLOW_INPUT_ASSEMBLER_INPUT_LAYOUT,
        D3D12_SHADER_BYTECODE, D3D_ROOT_SIGNATURE_VERSION_1,
    },
    Dxgi::Common::{DXGI_FORMAT_UNKNOWN, DXGI_SAMPLE_DESC},
};

pub struct Shader {
    pub root_signature: ID3D12RootSignature,
    pub pipeline_state: ID3D12PipelineState,
}

pub fn create_rect_shader(device: &ID3D12Device) -> Shader {
    const VS: &[u8] = &[]; // include_bytes!(concat!(env!("OUT_DIR"), "/rect_vs.cso")));
    const PS: &[u8] = &[]; // include_bytes!(concat!(env!("OUT_DIR"), "/rect_ps.cso")));

    let root_signature = {
        let blob = {
            let desc = D3D12_ROOT_SIGNATURE_DESC {
                NumParameters: 0,
                pParameters: std::ptr::null(),
                NumStaticSamplers: 0,
                pStaticSamplers: std::ptr::null(),
                Flags: D3D12_ROOT_SIGNATURE_FLAG_ALLOW_INPUT_ASSEMBLER_INPUT_LAYOUT,
            };

            // todo: handle error
            let mut blob = None;
            let mut error = None;
            unsafe {
                D3D12SerializeRootSignature(
                    &desc,
                    D3D_ROOT_SIGNATURE_VERSION_1,
                    &mut blob,
                    Some(&mut error),
                )
            }
            .unwrap();

            blob.unwrap()
        };

        let blob = unsafe {
            std::slice::from_raw_parts(blob.GetBufferPointer().cast(), blob.GetBufferSize())
        };

        unsafe { device.CreateRootSignature(0, blob) }.unwrap()
    };

    let pipeline_state = {
        // todo: cached pso?
        let desc = D3D12_GRAPHICS_PIPELINE_STATE_DESC {
            pRootSignature: unsafe { std::mem::transmute_copy(&root_signature) },
            VS: D3D12_SHADER_BYTECODE {
                pShaderBytecode: todo!(),
                BytecodeLength: todo!(),
            },
            PS: D3D12_SHADER_BYTECODE {
                pShaderBytecode: todo!(),
                BytecodeLength: todo!(),
            },
            BlendState: D3D12_BLEND_DESC {
                AlphaToCoverageEnable: todo!(),
                IndependentBlendEnable: todo!(),
                RenderTarget: todo!(),
            },
            SampleMask: u32::MAX,
            RasterizerState: D3D12_RASTERIZER_DESC {
                FillMode: todo!(),
                CullMode: todo!(),
                FrontCounterClockwise: todo!(),
                DepthBias: todo!(),
                DepthBiasClamp: todo!(),
                SlopeScaledDepthBias: todo!(),
                DepthClipEnable: todo!(),
                MultisampleEnable: todo!(),
                AntialiasedLineEnable: todo!(),
                ForcedSampleCount: todo!(),
                ConservativeRaster: todo!(),
            },
            DepthStencilState: D3D12_DEPTH_STENCIL_DESC {
                DepthEnable: todo!(),
                DepthWriteMask: todo!(),
                DepthFunc: todo!(),
                StencilEnable: todo!(),
                StencilReadMask: todo!(),
                StencilWriteMask: todo!(),
                FrontFace: todo!(),
                BackFace: todo!(),
            },
            InputLayout: D3D12_INPUT_LAYOUT_DESC {
                pInputElementDescs: todo!(),
                NumElements: todo!(),
            },
            PrimitiveTopologyType: D3D12_PRIMITIVE_TOPOLOGY_TYPE_TRIANGLE,
            NumRenderTargets: 1,
            RTVFormats: todo!(),
            DSVFormat: DXGI_FORMAT_UNKNOWN,
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            ..Default::default()
        };

        unsafe { device.CreateGraphicsPipelineState(&desc) }.unwrap()
    };

    Shader {
        root_signature,
        pipeline_state,
    }
}
