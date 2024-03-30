#define RS "RootFlags (ALLOW_INPUT_ASSEMBLER_INPUT_LAYOUT),"       \
           "RootConstants(num32BitConstants=3, b0),"               \
           "RootConstants(num32BitConstants=1, b1),"               \
           "SRV(t0),"                                              \
           "DescriptorTable(SRV(t1, numDescriptors=unbounded)),"   \
           "StaticSampler(s0, filter = FILTER_MIN_MAG_MIP_POINT)," \
           "StaticSampler(s1, filter = FILTER_MIN_MAG_MIP_LINEAR)"

struct Rect
{
    // position of the top-left corner, in pixels, Y-axis pointing down
    float2 origin;
    // size of the rectangle, in pixels
    float2 size;
    float4 uvwh;
    float4 color;
};

cbuffer properties : register(b0, space0)
{
    float2 viewport_scale;
    float viewport_height;
};

cbuffer texture : register(b1, space0)
{
    int texture_id;
};

StructuredBuffer<Rect> rects : register(t0);
Texture2D<float4> textures[] : register(t1);

SamplerState point_sampler : register(s0);
SamplerState linear_sampler : register(s1);

struct VS_OUT
{
    // all values in clip space
    float4 origin : SV_POSITION;
    float2 uv : TEXCOORD0;
    uint instance : SV_INSTANCEID;
};

float4 point_to_clip_space(float2 value)
{
    // flip the y axis
    float2 flipped = float2(value.x, viewport_height - value.y);

    // scale to [0, 1]
    float2 scaled = flipped * viewport_scale;

    // transform to [-1, 1]
    return float4(scaled * 2.0 - 1.0, 0.0, 1.0);
}

float2 scale_to_viewport(float2 value)
{
    return value * viewport_scale * 2.0 - 1.0;
}

static const int2 positions[4] = {
    int2(0.0, 0.0),
    int2(0.0, 1.0),
    int2(1.0, 0.0),
    int2(1.0, 1.0)};

[RootSignature(RS)] VS_OUT vs_main(uint vertex : SV_VERTEXID, uint instance : SV_INSTANCEID)
{
    VS_OUT output;

    output.origin = point_to_clip_space(rects[instance].origin + rects[instance].size * positions[vertex]);
    output.uv = rects[instance].uvwh.xy + rects[instance].uvwh.zw * positions[vertex];
    output.instance = instance;

    return output;
}

float4 ps_main(VS_OUT input) : SV_TARGET
{
    // return rects[input.instance].color;
    return rects[input.instance].color * textures[texture_id].Sample(point_sampler, input.uv);
    // return textures[texture_id].Sample(point_sampler, input.uv);
}
