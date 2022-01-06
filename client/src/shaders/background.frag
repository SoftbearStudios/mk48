#ifdef WAVES
    #extension GL_OES_standard_derivatives : enable
    precision highp float;
#else
    precision mediump float;
#endif

varying vec2 vPosition;
varying vec2 vUv;
varying vec2 vUv2;

uniform sampler2D uSampler;
#ifndef SAND_COLOR
    uniform sampler2D uSand;
#endif
#ifndef GRASS_COLOR
uniform sampler2D uGrass;
#endif

uniform vec4 uMiddle_uDerivative;
uniform vec4 uTime_uVisual_uRestrict_uBorder;

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
        float a = 0.42;
        vec3 shift = vec3(100);
        mat2 rot = mat2(cos(0.5), sin(0.5), -sin(0.5), cos(0.5));
        for (int i = 0; i < WAVES; ++i) {
            v += a * noise(x);
            x = vec3(rot * x.xy, x.z) * vec3(vec2(2.0), 2.43) + shift;
            a *= 0.56;
        }
        return v;
    }
#endif

/* <----- End modified source from shadertoy. */

// Match Rust code "levels"
#define SAND 0.5
#define GRASS 0.64

#define WIND vec2(-0.21, -0.045)

void main() {
    float h = texture2D(uSampler, vUv).a;

    // Noise must always increase height, as input texture is stratified by 4 bit representation, meaning that any
    // decrease could make the edge very noisy.
    float height = h + noise(vPosition * 0.1) * 0.02 + 0.01;

    #ifdef SAND_COLOR
        vec3 sand = SAND_COLOR;
    #else
        vec3 sand = texture2D(uSand, vUv2).rgb;
    #endif
    sand *= 0.87;

    if (height >= GRASS) {
        #ifdef GRASS_COLOR
            vec3 grass = GRASS_COLOR;
        #else
            vec3 grass = texture2D(uGrass, vUv2).rgb;
        #endif
        grass = mix(grass * vec3(0.9, 1.25, 0.9), sand, 0.3);

        gl_FragColor = vec4(mix(sand, grass, min((height - GRASS) * (1.0 / (1.0 - GRASS)), 1.0)), 1.0); // Grass
    } else {
        vec3 shoal = sand * vec3(0.78, 0.74, 0.93);
        vec3 s = mix(shoal, sand, min((height - SAND) * (1.5 / (GRASS - SAND)), 1.0)); // Sand

        #ifdef WAVES
            float uTime = uTime_uVisual_uRestrict_uBorder.x;
            vec2 waterNoise = vec2(waveNoise(vec3(vPosition * 0.07 + WIND * uTime, uTime * 0.07))) * vec2(0.035, 2.5);
        #else
            vec2 waterNoise = vec2(0.0);
        #endif

        float sandHeight = SAND - waterNoise.x;
        float waterDarkness = (1.429 - pow(0.00024, (abs(sandHeight - height)))) * 0.7;

        vec3 w = mix(shoal, vec3(0.0, 0.2, 0.45), waterDarkness);

        #ifdef WAVES
            vec3 waterNormal = normalize(cross(vec3(uMiddle_uDerivative.z, dFdx(waterNoise.y), 0.0), vec3(0.0, dFdy(waterNoise.y), uMiddle_uDerivative.w)));
            float reflectY = max(reflect(vec3(0.333, -0.666, 0.666), waterNormal).y, 0.0);

            // Manual pow(reflectY, 10.0) because 4 cycles instead of 9
            float _2reflectY = reflectY * reflectY;
            float _4reflectY = _2reflectY * _2reflectY;
            float _8reflectY = _4reflectY * _4reflectY;
            float reflectYPow10 = _8reflectY * _2reflectY;

            // Add specular highlight of waves to water color.
            w += reflectYPow10 * smoothstep(sandHeight - 0.05, sandHeight - 0.3, h) * 0.3;
        #endif

        float foam = smoothstep(0.03, 0.01, sandHeight - height);
        w = mix(w, vec3(0.8), foam * 0.65);

        float delta = uMiddle_uDerivative.z * 0.0075;
        gl_FragColor = vec4(mix(s, w, smoothstep(sandHeight + delta, sandHeight - delta, height)), 1.0);
    }

    gl_FragColor = mix(gl_FragColor, vec4(0.4, 0.15, 0.15, 1.0), clamp((length(vPosition) - uTime_uVisual_uRestrict_uBorder.w) * 0.1, 0.0, 0.5));
    gl_FragColor = mix(gl_FragColor, vec4(0.0, 0.14, 0.32, 1.0), clamp((length(vPosition - uMiddle_uDerivative.xy) - uTime_uVisual_uRestrict_uBorder.y) * 0.1, 0.0, uTime_uVisual_uRestrict_uBorder.z));
}