use windows::Win32::{
    Foundation::{FALSE, TRUE},
    Graphics::{
        Direct3D::D3D_PRIMITIVE_TOPOLOGY_TRIANGLESTRIP,
        Direct3D12::{
            ID3D12Device, ID3D12GraphicsCommandList, ID3D12PipelineState, ID3D12Resource,
            ID3D12RootSignature, D3D12_BLEND_DESC, D3D12_BLEND_INV_SRC_ALPHA, D3D12_BLEND_ONE,
            D3D12_BLEND_OP_ADD, D3D12_COLOR_WRITE_ENABLE_ALL, D3D12_CULL_MODE_BACK,
            D3D12_DEPTH_STENCIL_DESC, D3D12_FILL_MODE_SOLID, D3D12_GRAPHICS_PIPELINE_STATE_DESC,
            D3D12_INPUT_ELEMENT_DESC, D3D12_INPUT_LAYOUT_DESC, D3D12_LOGIC_OP_NOOP,
            D3D12_PRIMITIVE_TOPOLOGY_TYPE_TRIANGLE, D3D12_RASTERIZER_DESC,
            D3D12_RENDER_TARGET_BLEND_DESC, D3D12_SHADER_BYTECODE,
        },
        Dxgi::Common::{DXGI_FORMAT_R16G16B16A16_FLOAT, DXGI_FORMAT_UNKNOWN, DXGI_SAMPLE_DESC},
    },
};

pub struct RectShader {
    pub root_signature: ID3D12RootSignature,
    pub pipeline_state: ID3D12PipelineState,
}

impl RectShader {
    pub fn new(device: &ID3D12Device) -> Self {
        const VS: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/rect_vs.cso"));
        const PS: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/rect_ps.cso"));

        const INPUT_ELEMENTS: &[D3D12_INPUT_ELEMENT_DESC] = &[];

        let root_signature: ID3D12RootSignature =
            unsafe { device.CreateRootSignature(0, VS) }.unwrap();

        let pipeline_state = {
            let mut blend_targets: [_; 8] = Default::default();
            blend_targets[0] = D3D12_RENDER_TARGET_BLEND_DESC {
                BlendEnable: TRUE,
                LogicOpEnable: FALSE,
                SrcBlend: D3D12_BLEND_ONE,
                DestBlend: D3D12_BLEND_INV_SRC_ALPHA,
                BlendOp: D3D12_BLEND_OP_ADD,
                SrcBlendAlpha: D3D12_BLEND_ONE,
                DestBlendAlpha: D3D12_BLEND_ONE,
                BlendOpAlpha: D3D12_BLEND_OP_ADD,
                LogicOp: D3D12_LOGIC_OP_NOOP,
                RenderTargetWriteMask: u8::try_from(D3D12_COLOR_WRITE_ENABLE_ALL.0).unwrap(),
            };

            let mut rtv_formats = [DXGI_FORMAT_UNKNOWN; 8];
            rtv_formats[0] = DXGI_FORMAT_R16G16B16A16_FLOAT;

            let desc = D3D12_GRAPHICS_PIPELINE_STATE_DESC {
                pRootSignature: unsafe { std::mem::transmute_copy(&root_signature) },
                VS: D3D12_SHADER_BYTECODE {
                    pShaderBytecode: VS.as_ptr().cast(),
                    BytecodeLength: VS.len(),
                },
                PS: D3D12_SHADER_BYTECODE {
                    pShaderBytecode: PS.as_ptr().cast(),
                    BytecodeLength: PS.len(),
                },
                BlendState: D3D12_BLEND_DESC {
                    AlphaToCoverageEnable: FALSE,
                    IndependentBlendEnable: FALSE,
                    RenderTarget: blend_targets,
                },
                SampleMask: u32::MAX,
                RasterizerState: D3D12_RASTERIZER_DESC {
                    FillMode: D3D12_FILL_MODE_SOLID,
                    CullMode: D3D12_CULL_MODE_BACK,
                    FrontCounterClockwise: TRUE,
                    ..Default::default()
                },
                DepthStencilState: D3D12_DEPTH_STENCIL_DESC {
                    DepthEnable: FALSE,
                    StencilEnable: FALSE,
                    ..Default::default()
                },
                InputLayout: D3D12_INPUT_LAYOUT_DESC {
                    pInputElementDescs: INPUT_ELEMENTS.as_ptr(),
                    NumElements: u32::try_from(INPUT_ELEMENTS.len()).unwrap(),
                },
                PrimitiveTopologyType: D3D12_PRIMITIVE_TOPOLOGY_TYPE_TRIANGLE,
                NumRenderTargets: 1,
                RTVFormats: rtv_formats,
                DSVFormat: DXGI_FORMAT_UNKNOWN,
                SampleDesc: DXGI_SAMPLE_DESC {
                    Count: 1,
                    Quality: 0,
                },
                ..Default::default()
            };

            unsafe { device.CreateGraphicsPipelineState(&desc) }.unwrap()
        };

        Self {
            root_signature,
            pipeline_state,
        }
    }

    pub fn bind(
        &self,
        command_list: &ID3D12GraphicsCommandList,
        rects: &ID3D12Resource,
        viewport_scale: [f32; 2],
        viewport_height: f32,
    ) {
        unsafe {
            command_list.SetPipelineState(&self.pipeline_state);
            command_list.SetGraphicsRootSignature(&self.root_signature);
            command_list.SetGraphicsRoot32BitConstants(
                0,
                3,
                [viewport_scale[0], viewport_scale[1], viewport_height]
                    .as_ptr()
                    .cast(),
                0,
            );
            command_list.SetGraphicsRootShaderResourceView(1, rects.GetGPUVirtualAddress());
            command_list.IASetPrimitiveTopology(D3D_PRIMITIVE_TOPOLOGY_TRIANGLESTRIP);
        }
    }
}
