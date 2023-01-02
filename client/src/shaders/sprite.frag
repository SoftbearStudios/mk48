precision mediump float;

in highp vec4 vPosition;
in highp vec2 vUv;
in vec2 vColor;
in vec3 vTangent;

uniform sampler2D uColor;
uniform sampler2D uNormal;
uniform vec3 uSun;

#ifdef SHADOWS
    uniform highp mat4 uShadowMatrix;
    uniform highp sampler2DShadow uShadow;
#endif

out vec4 fragColor;

// https://bgolus.medium.com/sharper-mipmapping-using-shader-based-supersampling-ed7aadb47bec
vec4 textureSharp(sampler2D s, vec2 uv, float bias) {
    // return texture(s, uv, bias);

    float b = bias - 1.5;
    vec2 dx = dFdx(uv);
    vec2 dy = dFdy(uv);
    vec2 o = vec2(0.125, 0.375);

    vec4 v = vec4(0.0);
    v += texture(s, uv + o.x * dx + o.y * dy, b);
    v += texture(s, uv - o.x * dx - o.y * dy, b);
    v += texture(s, uv + o.y * dx - o.x * dy, b);
    v += texture(s, uv - o.y * dx + o.x * dy, b);
    return v * 0.25;
}

void main() {
    // Blur shadows with mipmap bias.
    // TODO see if branching early on this is a performance improvement (skips lighting and 8 texture samples).
    bool isDropShadow = vColor.x == 0.0;
    #ifdef SOFT_SHADOWS
        float bias = isDropShadow ? 2.0 : 0.0; // TODO blur shadows more once sprite sheet has more padding.
    #else
        float bias = 0.0;
    #endif

    // Use mipmap bias + multiple samples to make sprites appear sharper.
    // Makes a big difference on ships with high frequency detail like Yamato.
    vec4 color = textureSharp(uColor, vUv, bias) * vColor.xxxy;
    vec4 normalBump = textureSharp(uNormal, vUv, 0.0);
    vec3 tsn = normalize(normalBump.xyz * 2.0 - 1.0);

    vec3 vNormal = vec3(0.0, 0.0, 1.0);
    vec3 vBitangent = cross(vNormal, vTangent);
    vec3 N = tsn.x * vTangent + tsn.y * vBitangent + tsn.z * vNormal;

    float NDotL = clamp(dot(N, uSun), 0.0, 1.0);
    float NDotUp = clamp(dot(N, vec3(0.0, 0.0, 1.0)), 0.0, 1.0);

    highp vec3 position = vPosition.xyz;
    position.z = max(position.z + normalBump.w * vPosition.w, 0.0);

    #ifdef SHADOWS
        highp vec4 shadowProj = uShadowMatrix * vec4(position, 1.0);
        highp vec3 shadowUv = shadowProj.xyz / shadowProj.w;
        shadowUv.z += 0.005;

        #ifdef SOFT_SHADOWS
            float sun = 0.0;
            #define S(x, y) sun += textureOffset(uShadow, shadowUv, ivec2(x, y));
            #define R(y) S(-1, y) S(0, y) S(1, y)
            R(-1) R(0) R(1)
            sun *= 1.0 / 9.0;
        #else
            float sun = texture(uShadow, shadowUv);
        #endif
    #else
        float sun = 1.0;
    #endif

    color.a *= isDropShadow ? sun : 1.0; // premultiply not needed since color is 0.
    color.rgb *= (NDotL * 0.75 * sun + 0.1) / (NDotUp * 0.5 + 0.5);
    fragColor = color;
}
