#define RS "RootFlags (ALLOW_INPUT_ASSEMBLER_INPUT_LAYOUT)," \
    "RootConstants(num32BitConstants=2, b0)," \
    "SRV(t0)"

struct Rect {
    float2 origin;
    float2 size;
    float4 color;
};

cbuffer properties: register(b0, space0) {
    float2 viewport_scale;
};

StructuredBuffer<Rect> rects: register(t0);

struct VS_OUT {
    // all values in clip space
    float4 origin : SV_POSITION;
    float2 size : SIZE;
    uint instance : SV_INSTANCEID;
};

float2 scale_to_viewport(float2 value) {
    return value * viewport_scale * 2.0 - 1.0;
}

static const int2 positions[4] = {
    int2(0.0, 0.0),
    int2(0.0, 1.0),
    int2(1.0, 0.0),
    int2(1.0, 1.0)
};

[RootSignature(RS)]
VS_OUT vs_main(uint vertex: SV_VERTEXID, uint instance: SV_INSTANCEID) {
    VS_OUT output;

    output.origin = float4(
        scale_to_viewport(rects[instance].origin + rects[instance].size * positions[vertex]),
        0.0, 1.0
    );
    output.size = scale_to_viewport(rects[instance].size);
    output.instance = instance;

    return output;
}

float4 ps_main(VS_OUT input): SV_TARGET {
    return rects[input.instance].color;
}
