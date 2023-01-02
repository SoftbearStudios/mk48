#version 300 es

in vec2 position;
uniform mat4 uModel;
uniform mat4 uViewProjection;

uniform sampler2D uHeight;

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

void main() {
    float height = textureBicubic(uHeight, position);
    vec3 vPosition = (uModel * vec4(position, height, 1.0)).xyz;
    gl_Position = uViewProjection * vec4(vPosition, 1.0);
}
