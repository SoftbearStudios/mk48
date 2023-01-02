#version 300 es

in vec4 position;
in vec2 uv;
uniform mat3 uCamera;
uniform mat3 uTexture;
out vec2 vPosition; // world position.
out vec2 vUv; // terrain texture uv.
out vec2 vUv2; // sand/grass texture uv.

void main() {
    gl_Position = position;
    vPosition = (uCamera * vec3(uv, 1.0)).xy;
    vUv = (uTexture * vec3(vPosition, 1.0)).xy;
    vUv2 = vPosition * 0.005;
}
