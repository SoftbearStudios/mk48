#ifdef WAVES
    #extension GL_OES_standard_derivatives : enable
#endif
precision highp float;

varying vec2 vPosition;
varying vec2 vUv;
varying vec2 vUv2;

uniform sampler2D uSampler;
uniform sampler2D uSand;
uniform sampler2D uGrass;
uniform sampler2D uSnow;

uniform vec4 uMiddle_uDerivative;
uniform float uTime;

/* Modified source from https://www.shadertoy.com/view/4dS3Wd ----> */
// By Morgan McGuire @morgan3d, http://graphicscodex.com
// Reuse permitted under the BSD license.

// Precision-adjusted variations of https://www.shadertoy.com/view/4djSRW
float hash(float p) { p = fract(p * 0.011); p *= p + 7.5; p *= p + p; return fract(p); }
float hash(vec2 p) {vec3 p3 = fract(vec3(p.xyx) * 0.13); p3 += dot(p3, p3.yzx + 3.333); return fract((p3.x + p3.y) * p3.z); }

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

// Match Rust code "levels"
#define LOW_LAND 0.5
#define HIGH_LAND 0.64

#define BORDER 200.0
#define WIND vec2(-0.21, -0.045)

void main() {
    float h = texture2D(uSampler, vUv).a;
    float height = h;

    float arctic = smoothstep(ARCTIC - BORDER, ARCTIC + BORDER, vPosition.y - noise(vPosition.x * 0.005 + 139.21) * (BORDER * 0.5));
    bool ocean = vPosition.y < ARCTIC;

    if (ocean) {
        // Noise must always increase height, as input texture is stratified by 4 bit representation, meaning that any
        // decrease could make the edge very noisy.
        float n = noise(vPosition * 0.1);
        height += (n * 0.02 + 0.01) * (0.5 - arctic);
    } else {
        // Noise can decrease height to form separate icebergs.
        vec2 f = worley(vPosition * 0.02);
        float d = f.y - f.x;
        height -= (0.4 - d) * 0.15 * smoothstep(0.0625 * 2.0, 0.0, abs(height - (LOW_LAND)));
    }

    vec3 lowLand;
    if (ocean) {
        lowLand = texture2D(uSand, vUv2).rgb * 0.87;
    } else {
        lowLand = texture2D(uSnow, vUv2).rgb * 0.8;
    }

    if (height >= HIGH_LAND) {
        vec3 highLand;
        if (ocean) {
            highLand = mix(texture2D(uGrass, vUv2).rgb * vec3(0.9, 1.25, 0.9), lowLand, 0.25);
        } else {
            highLand = lowLand * 1.2;
        }
        gl_FragColor = vec4(mix(lowLand, highLand, min((height - HIGH_LAND) * (1.0 / (1.0 - HIGH_LAND)), 1.0)), 1.0); // Low land to high land
    } else {
        #define WAVE_HEIGHT 0.035

        vec3 beach = lowLand * (ocean ? vec3(0.78, 0.74, 0.93) : vec3(0.6, 0.8, 1.0));
        vec3 s = mix(beach, lowLand, min((height - (LOW_LAND + WAVE_HEIGHT * 0.5)) * (1.5 / (HIGH_LAND - LOW_LAND)), 1.0)); // Beach to low land

        float sandHeight = LOW_LAND;
        if (height >= sandHeight + WAVE_HEIGHT * 0.3) {
            gl_FragColor = vec4(s, 1.0);
        } else {
            #ifdef WAVES
                vec2 waterNoise = vec2(waveNoise(vec3(vPosition * 0.07 + WIND * uTime, uTime * 0.07))) * vec2(WAVE_HEIGHT, 2.2);
                sandHeight += waterNoise.x - WAVE_HEIGHT * 0.5;
            #endif

            vec3 deep = mix(vec3(0.0, 0.2, 0.45), vec3(0.0, 0.3, 0.4), arctic);
            vec3 shallow = mix(vec3(0.2, 0.37, 0.53), vec3(0.0, 0.4, 0.53), arctic);
            vec3 w = mix(deep, shallow, pow(0.01, abs(sandHeight - height))); // Deep to shallow water.

            #ifdef WAVES
                vec3 waterNormal = normalize(cross(vec3(uMiddle_uDerivative.z, dFdx(waterNoise.y), 0.0), vec3(0.0, dFdy(waterNoise.y), uMiddle_uDerivative.w)));
                float reflectY = max(reflect(vec3(0.333, -0.666, 0.666), waterNormal).y, 0.0);

                // Manual pow(reflectY, 10.0) because 4 cycles instead of 9
                float _2reflectY = reflectY * reflectY;
                float _4reflectY = _2reflectY * _2reflectY;
                float _8reflectY = _4reflectY * _4reflectY;
                float reflectYPow10 = _8reflectY * _2reflectY;

                // Add specular highlight of waves to water color.
                w += reflectYPow10 * smoothstep(sandHeight, sandHeight - (ocean ? 0.25 : 0.001), h) * 0.3;
            #endif

            // Foam appears near surface.
            float foam = smoothstep(0.034, 0.003, sandHeight - height);
            vec3 foamColor;
            if (ocean) {
                foamColor = vec3(foam * 0.65);
            } else {
                foamColor = mix(shallow, beach, 0.5) * foam;
            }
            w = max(w, foamColor);

            // Antialias foam and sand (slight bias towards positive height).
            // Scale args to avoid precision issue.
            float delta = uMiddle_uDerivative.z * 0.0075;
            vec3 args = vec3(sandHeight + delta * 1.5, sandHeight - delta, height) * 16.0;
            gl_FragColor = vec4(mix(s, w, smoothstep(args.x, args.y, args.z)), 1.0);
        }
    }
}