#version 300 es
#defines
// TODO webgl -> webgl2 shader translation to support oes std derivatives.

precision highp float;

in vec2 vPosition;
in vec2 vUv;
in vec2 vUv2;
out vec4 fragColor;

uniform sampler2D uHeight;
uniform sampler2D uDetail;

uniform float uDerivative;
uniform float uTime;
uniform vec3 uSun;
uniform vec3 uWaterSun;
uniform vec2 uWind;

#ifdef SHADOWS
    uniform mat4 uShadowMatrix;
    uniform highp sampler2DShadow uShadow;
#endif

/* Modified source from https://www.shadertoy.com/view/4dS3Wd ----> */
// By Morgan McGuire @morgan3d, http://graphicscodex.com
// Reuse permitted under the BSD license.

// Precision-adjusted variations of https://www.shadertoy.com/view/4djSRW
float hash(float p) { p = fract(p * 0.011); p *= p + 7.5; p *= p + p; return fract(p); }
float hash(vec2 p) { vec3 p3 = fract(vec3(p.xyx) * 0.13); p3 += dot(p3, p3.yzx + 3.333); return fract((p3.x + p3.y) * p3.z); }

float noise(float x) {
    float i = floor(x);
    float f = fract(x);
    float u = f * f * (3.0 - 2.0 * f);
    return mix(hash(i), hash(i + 1.0), u);
}

float noise(vec2 x) {
    vec2 i = floor(x);
    vec2 f = fract(x);
    float a = hash(i);
    float b = hash(i + vec2(1.0, 0.0));
    float c = hash(i + vec2(0.0, 1.0));
    float d = hash(i + vec2(1.0, 1.0));
    vec2 u = f * f * (3.0 - 2.0 * f);
    return mix(a, b, u.x) + (c - a) * u.y * (1.0 - u.x) + (d - b) * u.x * u.y;
}

float noise(vec3 x) {
    const vec3 step = vec3(110, 241, 171);
    vec3 i = floor(x);
    vec3 f = fract(x);
    float n = dot(i, step);
    vec3 u = f * f * (3.0 - 2.0 * f);
    return mix(mix(mix( hash(n + dot(step, vec3(0, 0, 0))), hash(n + dot(step, vec3(1, 0, 0))), u.x),
    mix( hash(n + dot(step, vec3(0, 1, 0))), hash(n + dot(step, vec3(1, 1, 0))), u.x), u.y),
    mix(mix( hash(n + dot(step, vec3(0, 0, 1))), hash(n + dot(step, vec3(1, 0, 1))), u.x),
    mix( hash(n + dot(step, vec3(0, 1, 1))), hash(n + dot(step, vec3(1, 1, 1))), u.x), u.y), u.z);
}

#ifdef WAVES
    float waveNoise(vec3 x) {
        float v = 0.0;
        float a = 0.5;
        vec3 shift = vec3(100);
        mat2 rot = mat2(cos(0.5), sin(0.5), -sin(0.5), cos(0.5));
        for (int i = 0; i < WAVES; ++i) {
            v += a * noise(x);
            x = vec3(rot * x.xy, x.z) * vec3(vec2(2.0), 2.11) + shift;
            a *= 0.5;
        }
        return v;
    }
#endif
/* <----- End modified source from shadertoy. */

/* Modified source from https://github.com/Erkaman/glsl-worley/blob/master/worley2D.glsl liscensed under MIT liscense. */
vec3 mod289(vec3 x) {
    return x - floor(x * (1.0 / 289.0)) * 289.0;
}

vec3 perm(vec3 x) {
    return mod289(((x * 34.0) + 1.0) * x);
}

vec3 dist(vec3 x, vec3 y) {
    return (x * x + y * y);
}

vec2 worley(vec2 P) {
    float K = 0.142857142857;
    float Ko = 0.428571428571;
    vec2 Pi = mod(floor(P), 289.0);
    vec2 Pf = fract(P);
    vec3 oi = vec3(-1.0, 0.0, 1.0);
    vec3 of = vec3(-0.5, 0.5, 1.5);
    vec3 px = perm(Pi.x + oi);
    vec3 p = perm(px.x + Pi.y + oi);
    vec3 ox = fract(p * K) - Ko;
    vec3 oy = mod(floor(p * K),7.0) * K - Ko;
    vec3 dx = Pf.x + 0.5 + ox;
    vec3 dy = Pf.y - of + oy;
    vec3 d1 = dist(dx,dy);
    p = perm(px.y + Pi.y + oi);
    ox = fract(p * K) - Ko;
    oy = mod(floor(p * K),7.0) * K - Ko;
    dx = Pf.x - 0.5 + ox;
    dy = Pf.y - of + oy;
    vec3 d2 = dist(dx,dy);
    p = perm(px.z + Pi.y + oi);
    ox = fract(p * K) - Ko;
    oy = mod(floor(p * K),7.0) * K - Ko;
    dx = Pf.x - 1.5 + ox;
    dy = Pf.y - of + oy;
    vec3 d3 = dist(dx,dy);
    vec3 d1a = min(d1, d2);
    d2 = max(d1, d2);
    d2 = min(d2, d3);
    d1 = min(d1a, d2);
    d2 = max(d1a, d2);
    d1.xy = (d1.x < d1.y) ? d1.xy : d1.yx;
    d1.xz = (d1.x < d1.z) ? d1.xz : d1.zx;
    d1.yz = min(d1.yz, d2.yz);
    d1.y = min(d1.y, d1.z);
    d1.y = min(d1.y, d2.x);
    return sqrt(d1.xy);
}
/* End modified source from github. */

vec4 cubic(float v) {
    vec4 n = vec4(1.0, 2.0, 3.0, 4.0) - v;
    vec4 s = n * n * n;
    float x = s.x;
    float y = s.y - 4.0 * s.x;
    float z = s.z - 4.0 * s.y + 6.0 * s.x;
    float w = 6.0 - x - y - z;
    return vec4(x, y, z, w);
}

float textureBicubic(sampler2D s, vec2 uv) {
    vec2 res = vec2(textureSize(s, 0));

    uv = uv * res - 0.5;
    ivec2 i = ivec2(uv);
    vec2 fuv = fract(uv);

    vec4 xcubic = cubic(fuv.x);
    vec4 ycubic = cubic(fuv.y);
    vec4 c = vec4(xcubic.xz + xcubic.yw, ycubic.xz + ycubic.yw);

    vec4 f = vec4(xcubic.yw, ycubic.yw) / c;
    float cx = c.x / (c.x + c.y);
    float cy = c.z / (c.z + c.w);

    // https://iquilezles.org/articles/hwinterpolation/
    #define T(offset, f) mix(mix(texelFetchOffset(s, i, 0, offset + ivec2(0, 0)).a, texelFetchOffset(s, i, 0, offset + ivec2(1, 0)).a, f.x), mix(texelFetchOffset(s, i, 0, offset + ivec2(0, 1)).a, texelFetchOffset(s, i, 0, offset + ivec2(1, 1)).a, f.x), f.y)

    return mix(mix(T(ivec2(1, 1), f.yw), T(ivec2(-1, 1), f.xw), cx), mix(T(ivec2(1, -1), f.yz), T(ivec2(-1, -1), f.xz), cx), cy);
}

// Match Rust code "levels"
#define LOW_LAND 0.5
#define HIGH_LAND 0.64
#define BORDER 200.0

void main() {
    float height = textureBicubic(uHeight, vUv);

    float arctic = smoothstep(ARCTIC - BORDER, ARCTIC + BORDER, vPosition.y - noise(vPosition.x * 0.005 + 139.21) * (BORDER * 0.5));
    bool ocean = vPosition.y < ARCTIC;

    if (ocean) {
        // Noise must always increase height, as input texture is stratified by 4 bit representation, meaning that any
        // decrease could make the edge very noisy.
        float n = noise(vPosition * 0.1);
        height += (n * 0.02 - 0.01) * 0.5 * (1.0 - arctic);
    } else {
        // Noise can decrease height to form separate icebergs.
        vec2 f = worley(vPosition * 0.02);
        float d = f.y - f.x;
        height -= (0.4 - d) * 0.15 * smoothstep(0.0625 * 2.0, 0.0, abs(height - (LOW_LAND)));
    }

    float heightMeters = height * 400.0 - 200.0;
    vec3 N = normalize(cross(vec3(uDerivative, 0.0, dFdx(heightMeters)), vec3(0.0, uDerivative, dFdy(heightMeters))));

    // Contains sand, grass, snow, and waves.
    vec4 detail = texture(uDetail, vUv2);

    vec3 sand = (vec3(0.76816154, 1.0870991, 0.82120496) * detail.x + vec3(0.30392796, -0.067789495, -0.22626717)) * 0.8;
    vec3 snow = (vec3(1.0810544, 0.9797763, 0.95707744) * detail.z + vec3(-0.078879535, 0.018439114, 0.05167395)) * 0.8;
    vec3 lowLand = ocean ? sand : snow;

    #ifdef SHADOWS
        vec3 position = vec3(vPosition, max(heightMeters, 0.0));
        vec4 shadowProj = uShadowMatrix * vec4(position, 1.0);
        vec3 shadowUv = shadowProj.xyz / shadowProj.w;
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

    float NDotL = clamp(dot(N, uSun), 0.0, 1.0);
    float NDotUp = clamp(dot(N, vec3(0.0, 0.0, 1.0)), 0.0, 0.5);
    float light = NDotL * sun * 0.6 + NDotUp * 0.3 + 0.1;

    float waterLight = sun * 0.5 + 0.5;

    if (height >= HIGH_LAND) {
        vec3 highLand;
        if (ocean) {
            highLand = mix((vec3(0.68371207, 1.1779866, 0.16889346) * detail.y + vec3(0.007670317, -0.0036472604, 0.013303946)) * vec3(0.9, 1.25, 0.9), lowLand, 0.02);
        } else {
            highLand = lowLand * 1.2;
        }
        fragColor = vec4(mix(lowLand, highLand, min((height - HIGH_LAND) * (1.0 / (1.0 - HIGH_LAND)), 1.0)) * light, 1.0); // Low land to high land
    } else {
        #define WAVE_HEIGHT 0.035

        vec3 beach = lowLand * (ocean ? vec3(0.78, 0.74, 0.93) : vec3(0.6, 0.8, 1.0));
        vec3 s = mix(beach, lowLand, min((height - (LOW_LAND + WAVE_HEIGHT * 0.5)) * (1.5 / (HIGH_LAND - LOW_LAND)), 1.0)) * light; // Beach to low land

        float sandHeight = LOW_LAND;
        if (height >= sandHeight + WAVE_HEIGHT * 0.3) {
            fragColor = vec4(s, 1.0);
        } else {
            vec2 wavePos = vPosition + uWind * (uTime * -0.42);
            #ifdef WAVES
                vec2 wn = waveNoise(vec3(wavePos, uTime) * 0.07) * 0.8 * vec2(WAVE_HEIGHT, 2.5);
            #else
                #ifdef ANIMATIONS
                    float v = texture(uDetail, wavePos * 0.005).w;
                #else
                    float v = detail.w;
                #endif
                // Multiply by factor to account for mipmapping a value that is used non-linearly.
                vec2 wn = v * vec2(WAVE_HEIGHT, (uDerivative * 0.35 + 0.9) * 2.5);
            #endif
            sandHeight += wn.x - WAVE_HEIGHT * 0.5;

            vec3 deep = mix(vec3(0, 0.0331, 0.171) * 0.82, vec3(0.0, 0.0331, 0.0763), arctic) * (mix(light, waterLight, 0.6));
            vec3 shallow = mix(vec3(0.0331, 0.113, 0.242) * 0.9, vec3(0.0, 0.05, 0.115), arctic) * waterLight;
            vec3 w = mix(deep, shallow, pow(0.005, abs(sandHeight - height))); // Deep to shallow water.

            vec3 waveN = normalize(cross(vec3(uDerivative, 0.0, dFdx(wn.y)), vec3(0.0, uDerivative, dFdy(wn.y))));

            vec3 viewDir = vec3(0.0, 0.0, 1.0);
            float r = clamp(dot(reflect(-uWaterSun, waveN), viewDir), 0.0, 1.0);

            // r = pow(r, 16.0);
            r *= r;
            r *= r;
            r *= r;
            r *= r;

            // Add specular highlight of waves to water color.
            w += r * smoothstep(-sandHeight, -(sandHeight - (ocean ? 0.25 : 0.001)), -height) * 0.3 * (sun * 0.85 + 0.15);

            // Foam appears near surface and is uniform width.
            float t = sandHeight - height;
            float foam = smoothstep(-0.05, 0.0, -t / (1.0001 - N.z));
            vec3 foamColor = mix(mix(shallow, beach, 0.5), vec3(0.5), float(ocean)) * (foam * foam);
            w = max(w, foamColor * light);

            // Antialias foam and sand.
            float delta = uDerivative * 0.015;
            fragColor = vec4(mix(s, w, smoothstep(-delta, delta, t)), 1.0);
        }
    }
}
