precision highp float;

varying vec2 vPosition;
uniform mat3 uTexture;
uniform sampler2D uSampler;

uniform vec2 uMiddle;
uniform float uVisual;
uniform float uRestrict;
uniform float uBorder;

// Licensed under MIT license
// https://github.com/edankwan/hyper-mix/blob/master/src/glsl/helpers/noise3.glsl
float mod289(float x){return x - floor(x * (1.0 / 289.0)) * 289.0;}
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
    #define GRASS 0.5608

    #define SHOAL (SAND - 0.0075)
    #define DEEP 0.3
    #define SHARPNESS 1.5
    #define GRASS_SATURATION 2.0

void main() {
    vec2 uv = (uTexture * vec3(vPosition, 1.0)).xy;
    float h = texture2D(uSampler, uv).a * 1.0;

    float nHeight = noise(vec3(vPosition.x / 10.0, vPosition.y / 10.0, 0));

    // Noise must always increase height, as input texture is stratified by 4 bit representation, meaning that any
    // decrease could make the edge very noisy.
    float height = h + nHeight * 0.02 + 0.01;

    if (height >= GRASS) {
        gl_FragColor = vec4(mix(vec3(0.73, 0.65, 0.45), vec3(0.25, 0.6 + sin(vPosition.x * 0.075 + 0.5 * cos(vPosition.y * 0.05)) * 0.005, 0.2), min((height - GRASS) * GRASS_SATURATION / (1.0 - GRASS), 1.0)), 1.0); // Grass
    } else if (height >= SAND) {
        gl_FragColor = vec4(mix(vec3(0.6, 0.56, 0.40), vec3(0.73, 0.65, 0.45), min((height - SAND) * SHARPNESS / (GRASS - SAND), 1.0)), 1.0); // Sand
    } else if (height > SHOAL) {
        gl_FragColor = vec4(mix(vec3(0.0, 0.3, 0.5), vec3(0.6, 0.56, 0.45), (height - SHOAL) / (SAND - SHOAL)), 1.0); // Water to sand
    } else {
        gl_FragColor = vec4(mix(vec3(0.0, 0.2, 0.45), vec3(0.0, 0.3, 0.5), max((height - DEEP) / (SHOAL - DEEP), -0.35)), 1.0); // Water
    }

    gl_FragColor = mix(gl_FragColor, vec4(0.4, 0.15, 0.15, 1.0), max(min((length(vPosition) - uBorder) * 0.1, 0.5), 0.0));
    gl_FragColor = mix(gl_FragColor, vec4(vec3(0.0, 0.14, 0.32), 1.0), max(min((length(vPosition - uMiddle) - uVisual) * 0.1, 1.0) * uRestrict, 0.0));
}