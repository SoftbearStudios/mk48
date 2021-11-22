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

uniform float uTime;
uniform vec2 uMiddle;
uniform float uVisual;
uniform float uRestrict;
uniform float uBorder;

// Licensed under MIT license
// https://github.com/edankwan/hyper-mix/blob/master/src/glsl/helpers/noise3.glsl
vec4 mod289(vec4 x){return x - floor(x * (1.0 / 289.0)) * 289.0;}
vec4 perm(vec4 x){return mod289(((x * 34.0) + 1.0) * x);}

float noise(vec3 p){
    vec3 a = floor(p);
    vec3 d = p - a;
    d = d * d * (3.0 - 2.0 * d);

    vec4 b = a.xxyy + vec4(0.0, 1.0, 0.0, 1.0);
    vec4 k1 = perm(b.xyxy);
    vec4 k2 = perm(k1.xyxy + b.zzww);

    vec4 c = k2 + a.zzzz;
    vec4 k3 = perm(c);
    vec4 k4 = perm(c + 1.0);

    vec4 o1 = fract(k3 * (1.0 / 41.0));
    vec4 o2 = fract(k4 * (1.0 / 41.0));

    vec4 o3 = o2 * d.z + o1 * (1.0 - d.z);
    vec2 o4 = o3.yw * d.x + o3.xz * (1.0 - d.x);

    return o4.y * d.y + o4.x * (1.0 - d.y);
}

// Match Rust code "levels"
#define SAND 0.5
#define GRASS 0.64

#define DEEP 0.3
#define SHARPNESS 1.5
#define GRASS_SATURATION 2.0
#define WIND vec2(-0.21, -0.045)

void main() {
    float h = texture2D(uSampler, vUv).a * 1.0;

    // This noise, while potentially faster, behaves poorely on some iGPUs.
    //float nHeight = noise(vPosition * 0.1);
    float nHeight = noise(vec3(vPosition.x, vPosition.y, 0.0) * 0.1);

    // Noise must always increase height, as input texture is stratified by 4 bit representation, meaning that any
    // decrease could make the edge very noisy.
    float height = h + nHeight * 0.02 + 0.01;

    vec3 sand = texture2D(uSand, vUv2).rgb * 0.9;

    if (height >= GRASS) {
        vec3 grass = mix(texture2D(uGrass, vUv2).rgb * vec3(0.9, 1.25, 0.9), sand, 0.4);
        gl_FragColor = vec4(mix(sand, grass, min((height - GRASS) * GRASS_SATURATION / (1.0 - GRASS), 1.0)), 1.0); // Grass
    } else {
        vec3 shoal = sand * vec3(0.75, 0.78, 0.85);
        vec3 s = mix(shoal, sand, min((height - SAND) * SHARPNESS / (GRASS - SAND), 1.0)); // Sand

        #ifdef WAVES
            vec3 waterNoise = vec3(noise(vec3(vPosition * 0.07 + WIND * uTime, uTime * 0.2))) * vec3(0.05, 0.0375, 2.5);
        #else
            vec3 waterNoise = vec3(0.0);
        #endif

        float sandHeight = SAND - waterNoise.y;
        float waterDarkness = (1.429 - pow(0.00024, (abs(sandHeight - (height - waterNoise.x))))) * 0.7;

        vec3 w = mix(shoal, vec3(0.0, 0.2, 0.45), waterDarkness);

        #ifdef WAVES
            vec3 waterNormal = normalize(cross(vec3(dFdx(vPosition.x), dFdx(waterNoise.z), 0.0), vec3(0.0, dFdy(waterNoise.z), dFdy(vPosition.y))));;
            float reflectY = max(reflect(vec3(0.333, -0.666, 0.666), waterNormal).y, 0.0);

            // Manual pow(reflectY, 10.0) because 4 cycles instead of 9
            float _2reflectY = reflectY * reflectY;
            float _4reflectY = _2reflectY * _2reflectY;
            float _8reflectY = _4reflectY * _4reflectY;
            float reflectYPow10 = _8reflectY * _2reflectY;

            // Components of water.
            float waterSpecular = reflectYPow10 * smoothstep(SAND - 0.1, SAND - 0.3, h);

            w += waterSpecular * 0.3;

            float delta = fwidth(height) * 0.3;
        #else
            float delta = 0.035;
        #endif

        #ifdef FOAM
            float waterFoam = smoothstep(0.024, 0.006, sandHeight - height);
            #ifndef WAVES
                waterFoam *= 3.0;
            #endif
            w = mix(w, vec3(0.8), waterFoam * 0.7);
        #endif

        gl_FragColor = vec4(mix(s, w, smoothstep(sandHeight, sandHeight - delta, height)), 1.0);
    }

    gl_FragColor = mix(gl_FragColor, vec4(0.4, 0.15, 0.15, 1.0), clamp((length(vPosition) - uBorder) * 0.1, 0.0, 0.5));
    gl_FragColor = mix(gl_FragColor, vec4(0.0, 0.14, 0.32, 1.0), clamp((length(vPosition - uMiddle) - uVisual) * 0.1, 0.0, uRestrict));
}